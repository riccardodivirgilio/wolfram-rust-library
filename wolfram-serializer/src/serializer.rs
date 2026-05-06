//! [`Serializer`] trait (format-agnostic) and [`ToWolfram`] trait
//! (per-Rust-type, format-independent encoder).

use crate::Error;
use wolfram_expr::{
    Association, ByteArray, Expr, ExprKind, NumericArray, PackedArray, Symbol,
};

#[cfg(feature = "bignum")]
use wolfram_expr::{BigInteger, BigReal};

/// Format-specific serializer: knows how to lay out atoms and compounds for a particular
/// output format (WL text, WXF binary, …). Mirrors wolframclient's `FormatSerializer`.
///
/// All methods take `&mut self` and write through to the underlying sink.
pub trait Serializer {
    //---- atoms ----

    /// Write an integer atom.
    fn serialize_integer(&mut self, n: i64) -> Result<(), Error>;
    /// Write a real (machine-precision f64) atom.
    fn serialize_real(&mut self, f: f64) -> Result<(), Error>;
    /// Write a string atom.
    fn serialize_string(&mut self, s: &str) -> Result<(), Error>;
    /// Write a symbol atom (fully-qualified name like `"System`Plus"`).
    fn serialize_symbol(&mut self, name: &str) -> Result<(), Error>;
    /// Write a ByteArray atom.
    fn serialize_byte_array(&mut self, bytes: &[u8]) -> Result<(), Error>;

    //---- compounds ----

    /// Write a function-application `head[args...]`.
    fn serialize_function(
        &mut self,
        head: &dyn ToWolfram,
        args: &[&dyn ToWolfram],
    ) -> Result<(), Error>;

    /// Write an Association `<|k -> v, k :> v|>`. Each entry carries a `delayed` flag:
    /// `false` for `Rule`, `true` for `RuleDelayed`.
    fn serialize_association(
        &mut self,
        entries: &[(&dyn ToWolfram, &dyn ToWolfram, bool)],
    ) -> Result<(), Error>;

    /// Write a NumericArray (typed N-dim flat buffer).
    fn serialize_numeric_array(&mut self, arr: &NumericArray) -> Result<(), Error>;

    /// Write a PackedArray (typed N-dim flat buffer, narrower element-type set).
    fn serialize_packed_array(&mut self, arr: &PackedArray) -> Result<(), Error>;

    //---- arbitrary precision (feature-gated) ----

    /// Write a BigInteger.
    #[cfg(feature = "bignum")]
    fn serialize_big_integer(&mut self, n: &BigInteger) -> Result<(), Error>;

    /// Write a BigReal.
    #[cfg(feature = "bignum")]
    fn serialize_big_real(&mut self, r: &BigReal) -> Result<(), Error>;
}

/// Implemented by Rust types that know how to serialize themselves into any
/// [`Serializer`]. Mirrors wolframclient's encoder dispatch — except in Rust the
/// dispatch is type-driven at compile time (zero overhead).
pub trait ToWolfram {
    /// Serialize `self` to `s`.
    fn serialize(&self, s: &mut dyn Serializer) -> Result<(), Error>;
}

//==============================================================================
// Primitive impls
//==============================================================================

macro_rules! impl_to_wolfram_int {
    ($($t:ty),+) => {
        $(
            impl ToWolfram for $t {
                fn serialize(&self, s: &mut dyn Serializer) -> Result<(), Error> {
                    s.serialize_integer(i64::from(*self))
                }
            }
        )+
    };
}
impl_to_wolfram_int!(i8, i16, i32, i64, u8, u16, u32);

impl ToWolfram for u64 {
    fn serialize(&self, s: &mut dyn Serializer) -> Result<(), Error> {
        // u64 may exceed i64::MAX; truncate for now (full BigInteger support needs the
        // bignum feature). For values that fit in i64, behavior is correct.
        s.serialize_integer(i64::try_from(*self).unwrap_or(i64::MAX))
    }
}

impl ToWolfram for f32 {
    fn serialize(&self, s: &mut dyn Serializer) -> Result<(), Error> {
        s.serialize_real(f64::from(*self))
    }
}

impl ToWolfram for f64 {
    fn serialize(&self, s: &mut dyn Serializer) -> Result<(), Error> {
        s.serialize_real(*self)
    }
}

impl ToWolfram for bool {
    fn serialize(&self, s: &mut dyn Serializer) -> Result<(), Error> {
        s.serialize_symbol(if *self { "System`True" } else { "System`False" })
    }
}

impl ToWolfram for str {
    fn serialize(&self, s: &mut dyn Serializer) -> Result<(), Error> {
        s.serialize_string(self)
    }
}

impl ToWolfram for String {
    fn serialize(&self, s: &mut dyn Serializer) -> Result<(), Error> {
        s.serialize_string(self.as_str())
    }
}

impl<T: ToWolfram + ?Sized> ToWolfram for &T {
    fn serialize(&self, s: &mut dyn Serializer) -> Result<(), Error> {
        (*self).serialize(s)
    }
}

impl<T: ToWolfram + ?Sized> ToWolfram for Box<T> {
    fn serialize(&self, s: &mut dyn Serializer) -> Result<(), Error> {
        (**self).serialize(s)
    }
}

impl<T: ToWolfram> ToWolfram for Vec<T> {
    fn serialize(&self, s: &mut dyn Serializer) -> Result<(), Error> {
        let head = Symbol::new("System`List");
        let args: Vec<&dyn ToWolfram> = self.iter().map(|e| e as &dyn ToWolfram).collect();
        s.serialize_function(&head, &args)
    }
}

impl<T: ToWolfram> ToWolfram for [T] {
    fn serialize(&self, s: &mut dyn Serializer) -> Result<(), Error> {
        let head = Symbol::new("System`List");
        let args: Vec<&dyn ToWolfram> = self.iter().map(|e| e as &dyn ToWolfram).collect();
        s.serialize_function(&head, &args)
    }
}

//==============================================================================
// wolfram-expr type impls
//==============================================================================

impl ToWolfram for Symbol {
    fn serialize(&self, s: &mut dyn Serializer) -> Result<(), Error> {
        s.serialize_symbol(self.as_str())
    }
}

impl ToWolfram for ByteArray {
    fn serialize(&self, s: &mut dyn Serializer) -> Result<(), Error> {
        s.serialize_byte_array(self.as_bytes())
    }
}

impl ToWolfram for NumericArray {
    fn serialize(&self, s: &mut dyn Serializer) -> Result<(), Error> {
        s.serialize_numeric_array(self)
    }
}

impl ToWolfram for PackedArray {
    fn serialize(&self, s: &mut dyn Serializer) -> Result<(), Error> {
        s.serialize_packed_array(self)
    }
}

impl ToWolfram for Association {
    fn serialize(&self, s: &mut dyn Serializer) -> Result<(), Error> {
        let entries: Vec<(&dyn ToWolfram, &dyn ToWolfram, bool)> = self
            .iter()
            .map(|(k, e)| (k as &dyn ToWolfram, &e.value as &dyn ToWolfram, e.delayed))
            .collect();
        s.serialize_association(&entries)
    }
}

#[cfg(feature = "bignum")]
impl ToWolfram for BigInteger {
    fn serialize(&self, s: &mut dyn Serializer) -> Result<(), Error> {
        s.serialize_big_integer(self)
    }
}

#[cfg(feature = "bignum")]
impl ToWolfram for BigReal {
    fn serialize(&self, s: &mut dyn Serializer) -> Result<(), Error> {
        s.serialize_big_real(self)
    }
}

//==============================================================================
// The big one: ToWolfram for Expr (dispatches by ExprKind)
//==============================================================================

impl ToWolfram for Expr {
    fn serialize(&self, s: &mut dyn Serializer) -> Result<(), Error> {
        match self.kind() {
            ExprKind::Integer(n) => s.serialize_integer(*n),
            ExprKind::Real(r) => s.serialize_real(**r),
            ExprKind::String(t) => s.serialize_string(t.as_str()),
            ExprKind::Symbol(sym) => s.serialize_symbol(sym.as_str()),
            ExprKind::Normal(normal) => {
                let args: Vec<&dyn ToWolfram> =
                    normal.elements().iter().map(|e| e as &dyn ToWolfram).collect();
                s.serialize_function(normal.head(), &args)
            }
            ExprKind::ByteArray(b) => s.serialize_byte_array(b.as_bytes()),
            ExprKind::Association(a) => a.serialize(s),
            ExprKind::NumericArray(arr) => s.serialize_numeric_array(arr),
            ExprKind::PackedArray(arr) => s.serialize_packed_array(arr),
            #[cfg(feature = "bignum")]
            ExprKind::BigInteger(n) => s.serialize_big_integer(n),
            #[cfg(feature = "bignum")]
            ExprKind::BigReal(r) => s.serialize_big_real(r),
            other => Err(Error::Consumer(format!(
                "ToWolfram for Expr: unhandled ExprKind variant: {:?}",
                other
            ))),
        }
    }
}
