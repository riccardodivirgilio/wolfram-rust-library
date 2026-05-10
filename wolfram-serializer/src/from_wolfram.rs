//! [`FromWolfram`] trait — typed deserialization from a parsed [`Expr`] tree.
//!
//! Top-level entry: [`crate::from_wxf`] reads WXF bytes into an [`Expr`] then
//! delegates to `<T as FromWolfram>::from_wolfram(&expr)`. Unlike
//! [`WolframConsumer`][crate::WolframConsumer] (the per-token visitor used to
//! build [`Expr`]), `FromWolfram` operates structurally on the already-parsed
//! tree — simpler, and exactly what's needed for derive-driven typed
//! deserialization.
//!
//! Hand-written impls live below for primitive scalars and the `wolfram-expr`
//! value types. Container types whose wire shape doesn't depend on the element
//! type — `Option<T>`, `HashMap<K,V>`, `BTreeMap<K,V>`, `()` — also have
//! blanket impls. Container types whose wire shape *does* depend on the
//! element type — `Vec<T>`, `[T; N]`, tuples — are intentionally NOT
//! implemented here; the `#[derive(FromWolfram)]` macro emits inline code at
//! the field site so it can pick the right wire shape.

use std::collections::{BTreeMap, HashMap};

use wolfram_expr::{
    Association, BigInteger, BigReal, ByteArray, Expr, ExprKind, NumericArray, PackedArray,
    RuleEntry, Symbol,
};

use crate::Error;

/// Deserialize a typed value from a parsed [`Expr`].
///
/// Implemented by hand for primitive scalars and the `wolfram-expr` value
/// types, and derivable via `#[derive(FromWolfram)]` for user types. The
/// derive emits structural code that walks the [`Expr`] tree according to
/// the wire-format conventions of [`#[derive(ToWolfram)]`].
pub trait FromWolfram: Sized {
    /// Try to deserialize a `Self` from `expr`. Returns
    /// [`Error::Deserialize`] (typically) on shape mismatch.
    fn from_wolfram(expr: &Expr) -> Result<Self, Error>;
}

/// Human-readable name of an [`ExprKind`] variant — used to fill the `got`
/// field of [`Error::Deserialize`] when we hit an unexpected shape.
pub fn kind_name(expr: &Expr) -> String {
    match expr.kind() {
        ExprKind::Integer(_) => "Integer".into(),
        ExprKind::Real(_) => "Real".into(),
        ExprKind::String(_) => "String".into(),
        ExprKind::Symbol(s) => format!("Symbol({:?})", s.as_str()),
        ExprKind::Normal(n) => {
            // Show head if it's a symbol; otherwise just say Function.
            match n.head().try_as_symbol() {
                Some(s) => format!("Function[{}, …]", s.as_str()),
                None => "Function[…]".into(),
            }
        }
        ExprKind::ByteArray(_) => "ByteArray".into(),
        ExprKind::Association(_) => "Association".into(),
        ExprKind::NumericArray(arr) => {
            format!("NumericArray<{}, {:?}>", arr.data_type().name(), arr.dimensions())
        }
        ExprKind::PackedArray(arr) => {
            format!("PackedArray<{}, {:?}>", arr.data_type().name(), arr.dimensions())
        }
        ExprKind::BigInteger(_) => "BigInteger".into(),
        ExprKind::BigReal(_) => "BigReal".into(),
        _ => "<unknown ExprKind>".into(),
    }
}

/// Helper used by the derive: build a Deserialize error tagged with a path.
pub fn err_at(path: impl Into<String>, expected: &'static str, got: String) -> Error {
    Error::Deserialize {
        path: path.into(),
        expected,
        got,
    }
}

//==============================================================================
// Primitive scalar impls
//==============================================================================

macro_rules! impl_int {
    ($($t:ty),+) => {
        $(
            impl FromWolfram for $t {
                fn from_wolfram(expr: &Expr) -> Result<Self, Error> {
                    match expr.kind() {
                        ExprKind::Integer(n) => <$t>::try_from(*n).map_err(|_| Error::Deserialize {
                            path: String::new(),
                            expected: concat!(stringify!($t), " (Integer in range)"),
                            got: format!("Integer({})", n),
                        }),
                        _ => Err(Error::Deserialize {
                            path: String::new(),
                            expected: stringify!($t),
                            got: kind_name(expr),
                        }),
                    }
                }
            }
        )+
    };
}
impl_int!(i8, i16, i32, i64, u8, u16, u32, u64);

impl FromWolfram for f32 {
    fn from_wolfram(expr: &Expr) -> Result<Self, Error> {
        match expr.kind() {
            ExprKind::Real(r) => Ok(**r as f32),
            ExprKind::Integer(n) => Ok(*n as f32),
            _ => Err(Error::Deserialize {
                path: String::new(),
                expected: "f32",
                got: kind_name(expr),
            }),
        }
    }
}

impl FromWolfram for f64 {
    fn from_wolfram(expr: &Expr) -> Result<Self, Error> {
        match expr.kind() {
            ExprKind::Real(r) => Ok(**r),
            ExprKind::Integer(n) => Ok(*n as f64),
            _ => Err(Error::Deserialize {
                path: String::new(),
                expected: "f64",
                got: kind_name(expr),
            }),
        }
    }
}

impl FromWolfram for bool {
    fn from_wolfram(expr: &Expr) -> Result<Self, Error> {
        expr.try_as_bool().ok_or_else(|| Error::Deserialize {
            path: String::new(),
            expected: "bool (System`True / System`False)",
            got: kind_name(expr),
        })
    }
}

impl FromWolfram for String {
    fn from_wolfram(expr: &Expr) -> Result<Self, Error> {
        expr.try_as_str()
            .map(String::from)
            .ok_or_else(|| Error::Deserialize {
                path: String::new(),
                expected: "String",
                got: kind_name(expr),
            })
    }
}

impl FromWolfram for Expr {
    fn from_wolfram(expr: &Expr) -> Result<Self, Error> {
        Ok(expr.clone())
    }
}

impl FromWolfram for Symbol {
    fn from_wolfram(expr: &Expr) -> Result<Self, Error> {
        expr.try_as_symbol().cloned().ok_or_else(|| Error::Deserialize {
            path: String::new(),
            expected: "Symbol",
            got: kind_name(expr),
        })
    }
}

impl FromWolfram for ByteArray {
    fn from_wolfram(expr: &Expr) -> Result<Self, Error> {
        expr.try_as_byte_array()
            .cloned()
            .ok_or_else(|| Error::Deserialize {
                path: String::new(),
                expected: "ByteArray",
                got: kind_name(expr),
            })
    }
}

impl FromWolfram for NumericArray {
    fn from_wolfram(expr: &Expr) -> Result<Self, Error> {
        expr.try_as_numeric_array()
            .cloned()
            .ok_or_else(|| Error::Deserialize {
                path: String::new(),
                expected: "NumericArray",
                got: kind_name(expr),
            })
    }
}

impl FromWolfram for PackedArray {
    fn from_wolfram(expr: &Expr) -> Result<Self, Error> {
        expr.try_as_packed_array()
            .cloned()
            .ok_or_else(|| Error::Deserialize {
                path: String::new(),
                expected: "PackedArray",
                got: kind_name(expr),
            })
    }
}

impl FromWolfram for Association {
    fn from_wolfram(expr: &Expr) -> Result<Self, Error> {
        expr.try_as_association()
            .cloned()
            .ok_or_else(|| Error::Deserialize {
                path: String::new(),
                expected: "Association",
                got: kind_name(expr),
            })
    }
}

impl FromWolfram for BigInteger {
    fn from_wolfram(expr: &Expr) -> Result<Self, Error> {
        expr.try_as_big_integer()
            .cloned()
            .ok_or_else(|| Error::Deserialize {
                path: String::new(),
                expected: "BigInteger",
                got: kind_name(expr),
            })
    }
}

impl FromWolfram for BigReal {
    fn from_wolfram(expr: &Expr) -> Result<Self, Error> {
        expr.try_as_big_real()
            .cloned()
            .ok_or_else(|| Error::Deserialize {
                path: String::new(),
                expected: "BigReal",
                got: kind_name(expr),
            })
    }
}

//==============================================================================
// Blanket impls for containers with a single, type-uniform wire shape
//==============================================================================

impl FromWolfram for () {
    fn from_wolfram(expr: &Expr) -> Result<Self, Error> {
        match expr.try_as_symbol() {
            Some(s) if s.as_str() == "System`Null" => Ok(()),
            _ => Err(Error::Deserialize {
                path: String::new(),
                expected: "() (System`Null symbol)",
                got: kind_name(expr),
            }),
        }
    }
}

impl<T: FromWolfram> FromWolfram for Option<T> {
    fn from_wolfram(expr: &Expr) -> Result<Self, Error> {
        if let Some(s) = expr.try_as_symbol() {
            if s.as_str() == "System`Null" {
                return Ok(None);
            }
        }
        Ok(Some(T::from_wolfram(expr)?))
    }
}

impl<K, V> FromWolfram for HashMap<K, V>
where
    K: FromWolfram + Eq + std::hash::Hash,
    V: FromWolfram,
{
    fn from_wolfram(expr: &Expr) -> Result<Self, Error> {
        let assoc = expr.try_as_association().ok_or_else(|| Error::Deserialize {
            path: String::new(),
            expected: "Association (HashMap)",
            got: kind_name(expr),
        })?;
        let mut out = HashMap::with_capacity(assoc.len());
        for (k_expr, RuleEntry { value: v_expr, .. }) in assoc.iter() {
            out.insert(K::from_wolfram(k_expr)?, V::from_wolfram(v_expr)?);
        }
        Ok(out)
    }
}

impl<K, V> FromWolfram for BTreeMap<K, V>
where
    K: FromWolfram + Ord,
    V: FromWolfram,
{
    fn from_wolfram(expr: &Expr) -> Result<Self, Error> {
        let assoc = expr.try_as_association().ok_or_else(|| Error::Deserialize {
            path: String::new(),
            expected: "Association (BTreeMap)",
            got: kind_name(expr),
        })?;
        let mut out = BTreeMap::new();
        for (k_expr, RuleEntry { value: v_expr, .. }) in assoc.iter() {
            out.insert(K::from_wolfram(k_expr)?, V::from_wolfram(v_expr)?);
        }
        Ok(out)
    }
}

//==============================================================================
// Vec<T> per-primitive impls for numeric T (mirror of the ToWolfram blanket
// impls). `Vec<u8>` (= `ByteArray` per wolfram-expr's type alias) is already
// covered by the `impl FromWolfram for ByteArray` above.
//==============================================================================

macro_rules! impl_vec_numeric_from {
    ($($t:ty => $variant:ident),+ $(,)?) => {
        $(
            impl FromWolfram for Vec<$t> {
                fn from_wolfram(expr: &Expr) -> Result<Self, Error> {
                    let na = expr.try_as_numeric_array().ok_or_else(|| Error::Deserialize {
                        path: String::new(),
                        expected: stringify!(Vec<$t>),
                        got: kind_name(expr),
                    })?;
                    if na.data_type() != wolfram_expr::NumericArrayDataType::$variant {
                        return Err(Error::Deserialize {
                            path: String::new(),
                            expected: stringify!(Vec<$t>),
                            got: format!("NumericArray<{}>", na.data_type().name()),
                        });
                    }
                    if na.dimensions().len() != 1 {
                        return Err(Error::Deserialize {
                            path: String::new(),
                            expected: stringify!(Vec<$t>),
                            got: format!(
                                "NumericArray with rank {}",
                                na.dimensions().len()
                            ),
                        });
                    }
                    let slice: &[$t] = na.try_as_slice::<$t>().ok_or_else(|| Error::Deserialize {
                        path: String::new(),
                        expected: stringify!(Vec<$t>),
                        got: format!("NumericArray element-type mismatch"),
                    })?;
                    Ok(slice.to_vec())
                }
            }
        )+
    };
}

impl_vec_numeric_from!(
    i8  => Integer8,
    i16 => Integer16,
    i32 => Integer32,
    i64 => Integer64,
    u16 => UnsignedInteger16,
    u32 => UnsignedInteger32,
    u64 => UnsignedInteger64,
    f32 => Real32,
    f64 => Real64,
);
