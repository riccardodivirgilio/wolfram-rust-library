//! Expansion for `#[derive(FromWolfram)]`.
//!
//! Mirror image of `serialize.rs`: walks the same `FieldKind` taxonomy but
//! emits code that *reads* an `Expr` tree into a typed value. Top-level
//! container shape determines what we expect on the wire (Association /
//! Function[Symbol(name), …] / Symbol(name)); each field's emit path is
//! the inverse of `serialize::expand_field_value`.

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{Data, DataEnum, DataStruct, DeriveInput, Fields, Result};

use crate::shared::{parse_container_attrs, parse_field_attrs, qualify_symbol, ContainerAttrs};
use crate::ty_classify::{classify, FieldKind};

pub(crate) fn expand(input: &DeriveInput) -> Result<TokenStream> {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let container_attrs = parse_container_attrs(&input.attrs)?;
    let name_str = name.to_string();

    let body = match &input.data {
        Data::Struct(s) => expand_struct(name, &name_str, &container_attrs, s)?,
        Data::Enum(e) => expand_enum(name, &name_str, e)?,
        Data::Union(_) => {
            return Err(syn::Error::new_spanned(
                input,
                "#[derive(FromWolfram)] does not support unions",
            ))
        }
    };

    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics ::wolfram_serializer::FromWolfram for #name #ty_generics #where_clause {
            fn from_wolfram(
                __expr: &::wolfram_expr::Expr,
            ) -> ::core::result::Result<Self, ::wolfram_serializer::Error> {
                #body
            }
        }
    })
}

//==============================================================================
// Structs
//==============================================================================

fn expand_struct(
    name: &syn::Ident,
    name_str: &str,
    attrs: &ContainerAttrs,
    data: &DataStruct,
) -> Result<TokenStream> {
    match &data.fields {
        Fields::Named(named) => {
            // Expect an Association; look up each field by its (renamed) key.
            let fields: Vec<&syn::Field> = named.named.iter().collect();
            let extracts = expand_named_field_extracts(name_str, &fields)?;
            let idents: Vec<&syn::Ident> = fields
                .iter()
                .map(|f| f.ident.as_ref().expect("named field"))
                .collect();
            Ok(quote! {
                let __assoc = __expr.try_as_association().ok_or_else(|| {
                    ::wolfram_serializer::from_wolfram::err_at(
                        #name_str,
                        "Association",
                        ::wolfram_serializer::from_wolfram::kind_name(__expr),
                    )
                })?;
                #extracts
                ::core::result::Result::Ok(#name { #(#idents),* })
            })
        }
        Fields::Unnamed(unnamed) => {
            // Expect Function[Symbol("Global`Name"), arg0, arg1, ...]
            let symbol = qualify_symbol(name_str, attrs);
            let fields: Vec<&syn::Field> = unnamed.unnamed.iter().collect();
            let mut extracts = Vec::with_capacity(fields.len());
            let mut bindings = Vec::with_capacity(fields.len());
            for (i, f) in fields.iter().enumerate() {
                let ident = format_ident!("__arg{}", i);
                let path = format!("{}.{}", name_str, i);
                let span = f.ty.span();
                let extract = expand_field_extract(
                    &quote! { &__args[#i] },
                    &f.ty,
                    &ident,
                    &path,
                    span,
                )?;
                extracts.push(quote_spanned! { span =>
                    let #ident = #extract;
                });
                bindings.push(quote! { #ident });
            }
            let arity = fields.len();
            Ok(quote! {
                let __normal = __expr.try_as_normal().ok_or_else(|| {
                    ::wolfram_serializer::from_wolfram::err_at(
                        #name_str,
                        "Function[Symbol(...), …]",
                        ::wolfram_serializer::from_wolfram::kind_name(__expr),
                    )
                })?;
                let __head = __normal.head().try_as_symbol().ok_or_else(|| {
                    ::wolfram_serializer::from_wolfram::err_at(
                        #name_str,
                        "Function head as Symbol",
                        ::wolfram_serializer::from_wolfram::kind_name(__normal.head()),
                    )
                })?;
                if __head.as_str() != #symbol {
                    return ::core::result::Result::Err(
                        ::wolfram_serializer::from_wolfram::err_at(
                            #name_str,
                            concat!("Function head ", stringify!(#symbol)),
                            format!("Symbol({:?})", __head.as_str()),
                        ),
                    );
                }
                let __args = __normal.elements();
                if __args.len() != #arity {
                    return ::core::result::Result::Err(
                        ::wolfram_serializer::from_wolfram::err_at(
                            #name_str,
                            concat!("Function with ", stringify!(#arity), " arguments"),
                            format!("Function with {} arguments", __args.len()),
                        ),
                    );
                }
                #(#extracts)*
                ::core::result::Result::Ok(#name(#(#bindings),*))
            })
        }
        Fields::Unit => {
            let symbol = qualify_symbol(name_str, attrs);
            Ok(quote! {
                let __sym = __expr.try_as_symbol().ok_or_else(|| {
                    ::wolfram_serializer::from_wolfram::err_at(
                        #name_str,
                        concat!("Symbol(", stringify!(#symbol), ")"),
                        ::wolfram_serializer::from_wolfram::kind_name(__expr),
                    )
                })?;
                if __sym.as_str() != #symbol {
                    return ::core::result::Result::Err(
                        ::wolfram_serializer::from_wolfram::err_at(
                            #name_str,
                            concat!("Symbol(", stringify!(#symbol), ")"),
                            format!("Symbol({:?})", __sym.as_str()),
                        ),
                    );
                }
                ::core::result::Result::Ok(#name)
            })
        }
    }
}

/// Build the per-field extract statements for a named struct or enum
/// struct-variant. The Association `__assoc` must be in scope. Yields a
/// sequence of `let <field_ident> = …;` bindings.
fn expand_named_field_extracts(
    container_path: &str,
    fields: &[&syn::Field],
) -> Result<TokenStream> {
    let mut out = TokenStream::new();
    for f in fields {
        let f_attrs = parse_field_attrs(&f.attrs)?;
        let ident = f.ident.as_ref().expect("named field");
        let key = f_attrs.rename.clone().unwrap_or_else(|| ident.to_string());
        let path = format!("{}.{}", container_path, ident);
        let span = f.ty.span();
        let extract = expand_field_extract(
            &quote! { &__entry.value },
            &f.ty,
            ident,
            &path,
            span,
        )?;
        out.extend(quote_spanned! { span =>
            let #ident = {
                let __entry = __assoc.get(&::wolfram_expr::Expr::from(#key)).ok_or_else(|| {
                    ::wolfram_serializer::from_wolfram::err_at(
                        #path,
                        "Association entry",
                        "missing key".into(),
                    )
                })?;
                #extract
            };
        });
    }
    Ok(out)
}

/// For one field, produce an *expression* (not statement!) that evaluates to
/// the typed field value, given that `expr_path` (a `&Expr`) is in scope.
fn expand_field_extract(
    expr_path: &TokenStream,
    ty: &syn::Type,
    _field_ident: &syn::Ident,
    err_path: &str,
    span: Span,
) -> Result<TokenStream> {
    let kind = classify(ty);

    match kind {
        FieldKind::VecOfU8 => Ok(quote_spanned! { span => {
            let __ba = (#expr_path).try_as_byte_array().ok_or_else(|| {
                ::wolfram_serializer::from_wolfram::err_at(
                    #err_path,
                    "ByteArray",
                    ::wolfram_serializer::from_wolfram::kind_name(#expr_path),
                )
            })?;
            __ba.as_slice().to_vec()
        }}),
        FieldKind::VecOfNumeric { elem_ty, dt } => Ok(quote_spanned! { span => {
            let __na = (#expr_path).try_as_numeric_array().ok_or_else(|| {
                ::wolfram_serializer::from_wolfram::err_at(
                    #err_path,
                    "NumericArray",
                    ::wolfram_serializer::from_wolfram::kind_name(#expr_path),
                )
            })?;
            if __na.data_type() != #dt {
                return ::core::result::Result::Err(
                    ::wolfram_serializer::from_wolfram::err_at(
                        #err_path,
                        "NumericArray with matching element type",
                        format!("NumericArray<{:?}>", __na.data_type()),
                    ),
                );
            }
            if __na.dimensions().len() != 1 {
                return ::core::result::Result::Err(
                    ::wolfram_serializer::from_wolfram::err_at(
                        #err_path,
                        "1-D NumericArray",
                        format!("NumericArray with rank {}", __na.dimensions().len()),
                    ),
                );
            }
            let __slice: &[#elem_ty] = __na.try_as_slice::<#elem_ty>().ok_or_else(|| {
                ::wolfram_serializer::from_wolfram::err_at(
                    #err_path,
                    "NumericArray element-type slice",
                    format!("element-type mismatch: {:?}", __na.data_type()),
                )
            })?;
            __slice.to_vec()
        }}),
        FieldKind::VecOfOther { elem_ty } => Ok(quote_spanned! { span => {
            let __normal = (#expr_path).try_as_normal().ok_or_else(|| {
                ::wolfram_serializer::from_wolfram::err_at(
                    #err_path,
                    "Function[List, …]",
                    ::wolfram_serializer::from_wolfram::kind_name(#expr_path),
                )
            })?;
            let mut __out: ::std::vec::Vec<#elem_ty> = ::std::vec::Vec::with_capacity(__normal.elements().len());
            for __elem in __normal.elements() {
                __out.push(<#elem_ty as ::wolfram_serializer::FromWolfram>::from_wolfram(__elem)?);
            }
            __out
        }}),
        FieldKind::NumericTensor {
            elem_ty,
            dt,
            dims,
            tuple_paths,
            original_ty,
        } => {
            let dim_lits: Vec<TokenStream> = dims.iter().map(|d| quote! { #d }).collect();
            let total_leaves: usize = dims.iter().product();
            let rank = dims.len();
            let dim_check = quote_spanned! { span =>
                let __expected_dims: [usize; #rank] = [ #(#dim_lits),* ];
                if __na.dimensions() != &__expected_dims[..] {
                    return ::core::result::Result::Err(
                        ::wolfram_serializer::from_wolfram::err_at(
                            #err_path,
                            "NumericArray with matching dimensions",
                            format!("NumericArray with dims {:?}", __na.dimensions()),
                        ),
                    );
                }
            };
            // For both array-rooted and tuple-rooted, we read a contiguous
            // `&[T]` slice in row-major order. Reconstruction differs:
            // - Array-rooted: `[T; N]` / `[[T; M]; N]` etc. — copy bytes
            //   directly into a stack-allocated array of the target type.
            // - Tuple-rooted: build the tuple by indexing into the slice.
            if let Some(_paths) = &tuple_paths {
                // Build the tuple from the flat slice in the same row-major
                // order the serialize side wrote it.
                // Construct the tuple by walking `paths` and assembling
                // nested tuples from the bottom up. Simpler approach: emit a
                // chained `(s[0], s[1], (s[2], s[3]), …)` matching the
                // original shape. We do that by walking the original_ty
                // shape recursively and emitting a constructor expression.
                let tup_ctor = build_tuple_ctor_from_slice(original_ty, &mut 0);
                Ok(quote_spanned! { span => {
                    let __na = (#expr_path).try_as_numeric_array().ok_or_else(|| {
                        ::wolfram_serializer::from_wolfram::err_at(
                            #err_path,
                            "NumericArray",
                            ::wolfram_serializer::from_wolfram::kind_name(#expr_path),
                        )
                    })?;
                            if __na.data_type() != #dt {
                        return ::core::result::Result::Err(
                            ::wolfram_serializer::from_wolfram::err_at(
                                #err_path,
                                "NumericArray with matching element type",
                                format!("NumericArray<{:?}>", __na.data_type()),
                            ),
                        );
                    }
                    #dim_check
                    let __slice: &[#elem_ty] = __na.try_as_slice::<#elem_ty>().ok_or_else(|| {
                        ::wolfram_serializer::from_wolfram::err_at(
                            #err_path,
                            "NumericArray element-type slice",
                            format!("element-type mismatch: {:?}", __na.data_type()),
                        )
                    })?;
                    if __slice.len() != #total_leaves {
                        return ::core::result::Result::Err(
                            ::wolfram_serializer::from_wolfram::err_at(
                                #err_path,
                                "NumericArray with expected leaf count",
                                format!("got {} leaves", __slice.len()),
                            ),
                        );
                    }
                    #tup_ctor
                }})
            } else {
                // Array-rooted: byte-copy into a stack array of the target
                // type. The original type is e.g. `[i32; 4]` or `[[f64; 3]; 2]`.
                Ok(quote_spanned! { span => {
                    let __na = (#expr_path).try_as_numeric_array().ok_or_else(|| {
                        ::wolfram_serializer::from_wolfram::err_at(
                            #err_path,
                            "NumericArray",
                            ::wolfram_serializer::from_wolfram::kind_name(#expr_path),
                        )
                    })?;
                            if __na.data_type() != #dt {
                        return ::core::result::Result::Err(
                            ::wolfram_serializer::from_wolfram::err_at(
                                #err_path,
                                "NumericArray with matching element type",
                                format!("NumericArray<{:?}>", __na.data_type()),
                            ),
                        );
                    }
                    #dim_check
                    let __slice: &[#elem_ty] = __na.try_as_slice::<#elem_ty>().ok_or_else(|| {
                        ::wolfram_serializer::from_wolfram::err_at(
                            #err_path,
                            "NumericArray element-type slice",
                            format!("element-type mismatch: {:?}", __na.data_type()),
                        )
                    })?;
                    if __slice.len() != #total_leaves {
                        return ::core::result::Result::Err(
                            ::wolfram_serializer::from_wolfram::err_at(
                                #err_path,
                                "NumericArray with expected leaf count",
                                format!("got {} leaves", __slice.len()),
                            ),
                        );
                    }
                    let mut __out: #original_ty = ::core::default::Default::default();
                    let __out_bytes = unsafe {
                        ::core::slice::from_raw_parts_mut(
                            (&mut __out) as *mut #original_ty as *mut u8,
                            ::core::mem::size_of::<#original_ty>(),
                        )
                    };
                    let __src_bytes = unsafe {
                        ::core::slice::from_raw_parts(
                            __slice.as_ptr() as *const u8,
                            ::core::mem::size_of_val(__slice),
                        )
                    };
                    __out_bytes.copy_from_slice(__src_bytes);
                    __out
                }})
            }
        }
        FieldKind::TupleHetero { tup } => {
            // Heterogeneous tuple — expect Function[List, …] with arity = tup.elems.len().
            let arity = tup.elems.len();
            let elem_extracts = tup.elems.iter().enumerate().map(|(i, t)| {
                let synth = format_ident!("__t{}", i);
                let inner_path = format!("{}.{}", err_path, i);
                let span = t.span();
                let extract =
                    expand_field_extract(&quote! { &__elems[#i] }, t, &synth, &inner_path, span)
                        .unwrap_or_else(|e| e.to_compile_error());
                quote_spanned! { span => #extract }
            });
            Ok(quote_spanned! { span => {
                let __normal = (#expr_path).try_as_normal().ok_or_else(|| {
                    ::wolfram_serializer::from_wolfram::err_at(
                        #err_path,
                        "Function[List, …]",
                        ::wolfram_serializer::from_wolfram::kind_name(#expr_path),
                    )
                })?;
                let __elems = __normal.elements();
                if __elems.len() != #arity {
                    return ::core::result::Result::Err(
                        ::wolfram_serializer::from_wolfram::err_at(
                            #err_path,
                            "Function[List, …] with matching arity",
                            format!("got {} elements", __elems.len()),
                        ),
                    );
                }
                ( #(#elem_extracts),* )
            }})
        }
        FieldKind::ArrayHetero { arr, len } => {
            // Heterogeneous fixed-size array — expect Function[List, …] with
            // exactly `len` elements; build a `[T; N]` from them.
            let elem_ty = &arr.elem;
            Ok(quote_spanned! { span => {
                let __normal = (#expr_path).try_as_normal().ok_or_else(|| {
                    ::wolfram_serializer::from_wolfram::err_at(
                        #err_path,
                        "Function[List, …]",
                        ::wolfram_serializer::from_wolfram::kind_name(#expr_path),
                    )
                })?;
                let __elems = __normal.elements();
                if __elems.len() != #len {
                    return ::core::result::Result::Err(
                        ::wolfram_serializer::from_wolfram::err_at(
                            #err_path,
                            "Function[List, …] with matching length",
                            format!("got {} elements", __elems.len()),
                        ),
                    );
                }
                let __vals: ::std::vec::Vec<#elem_ty> = {
                    let mut __v = ::std::vec::Vec::with_capacity(#len);
                    for __e in __elems {
                        __v.push(<#elem_ty as ::wolfram_serializer::FromWolfram>::from_wolfram(__e)?);
                    }
                    __v
                };
                <[#elem_ty; #len]>::try_from(__vals.as_slice()).map_err(|_| {
                    ::wolfram_serializer::from_wolfram::err_at(
                        #err_path,
                        concat!("array conversion of length ", stringify!(#len)),
                        "length mismatch".into(),
                    )
                })?
            }})
        }
        FieldKind::Other => Ok(quote_spanned! { span => {
            <#ty as ::wolfram_serializer::FromWolfram>::from_wolfram(#expr_path)?
        }}),
    }
}

/// Recursively build a tuple constructor expression from a flat slice
/// `__slice: &[T]`. Tracks an in-out cursor `idx` that marks the next slot to
/// consume. Used for tuple-rooted numeric tensors.
fn build_tuple_ctor_from_slice(ty: &syn::Type, idx: &mut usize) -> TokenStream {
    match ty {
        syn::Type::Tuple(tup) => {
            let parts = tup
                .elems
                .iter()
                .map(|inner| build_tuple_ctor_from_slice(inner, idx))
                .collect::<Vec<_>>();
            quote! { ( #(#parts),* ) }
        }
        _ => {
            let i = *idx;
            *idx += 1;
            quote! { __slice[#i] }
        }
    }
}

//==============================================================================
// Enums
//==============================================================================

fn expand_enum(name: &syn::Ident, name_str: &str, data: &DataEnum) -> Result<TokenStream> {
    // The matcher dispatches on the incoming Expr's shape:
    // - try_as_symbol() with name == "Global`UnitVariant" → unit variant
    // - try_as_normal() with head == "Global`TupleVariant" → tuple variant
    // - try_as_normal() with head == "Global`StructVariant" → struct variant
    //   (with a single Association argument)
    let mut unit_arms = Vec::new();
    let mut function_arms = Vec::new();

    for v in &data.variants {
        let v_attrs = parse_container_attrs(&v.attrs)?;
        let v_name = &v.ident;
        let v_symbol = qualify_symbol(&v_name.to_string(), &v_attrs);
        let v_path = format!("{}::{}", name_str, v_name);
        match &v.fields {
            Fields::Unit => {
                // Each unit-variant arm short-circuits the function with
                // `return Ok(...)` so the surrounding match's arms can all be
                // `()` and the type-check passes.
                unit_arms.push(quote! {
                    #v_symbol => return ::core::result::Result::Ok(#name :: #v_name),
                });
            }
            Fields::Unnamed(unnamed) => {
                let fields: Vec<&syn::Field> = unnamed.unnamed.iter().collect();
                let arity = fields.len();
                let mut bindings = Vec::with_capacity(arity);
                let mut extracts = Vec::with_capacity(arity);
                for (i, f) in fields.iter().enumerate() {
                    let bind = format_ident!("__a{}", i);
                    let span = f.ty.span();
                    let path = format!("{}.{}", v_path, i);
                    let extract = expand_field_extract(
                        &quote! { &__args[#i] },
                        &f.ty,
                        &bind,
                        &path,
                        span,
                    )?;
                    extracts.push(quote_spanned! { span =>
                        let #bind = #extract;
                    });
                    bindings.push(quote! { #bind });
                }
                function_arms.push(quote! {
                    #v_symbol => {
                        let __args = __normal.elements();
                        if __args.len() != #arity {
                            return ::core::result::Result::Err(
                                ::wolfram_serializer::from_wolfram::err_at(
                                    #v_path,
                                    concat!("Function with ", stringify!(#arity), " arguments"),
                                    format!("Function with {} arguments", __args.len()),
                                ),
                            );
                        }
                        #(#extracts)*
                        return ::core::result::Result::Ok(#name :: #v_name ( #(#bindings),* ));
                    }
                });
            }
            Fields::Named(named) => {
                let fields: Vec<&syn::Field> = named.named.iter().collect();
                let extracts = expand_named_field_extracts(&v_path, &fields)?;
                let idents: Vec<&syn::Ident> = fields
                    .iter()
                    .map(|f| f.ident.as_ref().expect("named field"))
                    .collect();
                function_arms.push(quote! {
                    #v_symbol => {
                        let __args = __normal.elements();
                        if __args.len() != 1 {
                            return ::core::result::Result::Err(
                                ::wolfram_serializer::from_wolfram::err_at(
                                    #v_path,
                                    "Function with 1 Association argument",
                                    format!("Function with {} arguments", __args.len()),
                                ),
                            );
                        }
                        let __assoc = __args[0].try_as_association().ok_or_else(|| {
                            ::wolfram_serializer::from_wolfram::err_at(
                                #v_path,
                                "Association in Function argument",
                                ::wolfram_serializer::from_wolfram::kind_name(&__args[0]),
                            )
                        })?;
                        #extracts
                        return ::core::result::Result::Ok(#name :: #v_name { #(#idents),* });
                    }
                });
            }
        }
    }

    Ok(quote! {
        if let ::core::option::Option::Some(__sym) = __expr.try_as_symbol() {
            match __sym.as_str() {
                #(#unit_arms)*
                _ => {}
            }
        }
        if let ::core::option::Option::Some(__normal) = __expr.try_as_normal() {
            if let ::core::option::Option::Some(__head) = __normal.head().try_as_symbol() {
                match __head.as_str() {
                    #(#function_arms)*
                    _ => {}
                }
            }
        }
        ::core::result::Result::Err(
            ::wolfram_serializer::from_wolfram::err_at(
                #name_str,
                "matching enum variant",
                ::wolfram_serializer::from_wolfram::kind_name(__expr),
            ),
        )
    })
}
