//! Procedural macros for `wolfram-serializer`.
//!
//! Provides `#[derive(ToWolfram)]` and `#[derive(FromWolfram)]` for structs
//! (named, tuple, unit) and enums. Field-level type pattern matching emits
//! the correct WXF representation for `Vec<u8>` (ByteArray), `Vec<numeric>`
//! and rectangular nested tuples / fixed-size arrays of numerics
//! (NumericArray), while everything else delegates through the
//! `ToWolfram` / `FromWolfram` traits.
//!
//! See the `wolfram-serializer` crate docs for usage and the wire-format
//! conventions emitted here.

#![allow(clippy::needless_doctest_main)]

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

mod deserialize;
mod serialize;
mod shared;
mod ty_classify;

/// Derive `ToWolfram` for a struct or enum.
#[proc_macro_derive(ToWolfram, attributes(wolfram))]
pub fn derive_to_wolfram(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    serialize::expand(&input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Derive `FromWolfram` for a struct or enum.
#[proc_macro_derive(FromWolfram, attributes(wolfram))]
pub fn derive_from_wolfram(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    deserialize::expand(&input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}
