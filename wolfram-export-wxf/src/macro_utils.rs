//! WXF wrapper runtime: the proc-macro emits an inline `fn(NumericArray<u8>)
//! -> NumericArray<u8>` shim around the user's typed function. That shim
//! reads the bytes off the input NumericArray, calls
//! `wolfram_serializer::deserialize::<A>()` to get the typed argument,
//! invokes the user function, and `serialize`s the result back into a fresh
//! UInt8 NumericArray.
//!
//! This module provides the helper trait + small helpers the macro expansion
//! references. The actual MArgument C ABI is delegated to
//! `wolfram_library_link::macro_utils::call_native_wolfram_library_function`
//! — we just stack a Rust-level typed-bytes layer on top of the existing
//! native call path.

use wolfram_expr::Expr;
use wolfram_library_link::NumericArray;
use wolfram_serializer::{deserialize, serialize, Format, FromWolfram, ToWolfram};

/// Compute the (arg types, return type) signature for a `#[export(wxf)]`
/// function. Used by the inventory entry's `signature` closure. For now
/// the entries are placeholders (`Expr::string("Expr")`); a follow-up
/// `wolfram_type()` trait method on `FromWolfram` / `ToWolfram` will yield
/// concrete type names.
pub fn wxf_signature() -> Result<(Vec<Expr>, Expr), String> {
    Ok((vec![Expr::string("Expr")], Expr::string("Expr")))
}

/// Deserialize WXF bytes from `input` into a typed value of type `A`.
/// Helper for macro-emitted code so the call sites don't have to import
/// `wolfram_serializer` directly.
pub fn decode<A: FromWolfram>(input: &NumericArray<u8>) -> A {
    let bytes: &[u8] = input.as_slice();
    deserialize::<A>(bytes, Format::Wxf)
        .unwrap_or_else(|e| panic!("WXF deserialize failed: {}", e))
}

/// Serialize `value` to WXF bytes and wrap them in a UInt8 NumericArray.
pub fn encode<R: ToWolfram>(value: &R) -> NumericArray<u8> {
    let bytes: Vec<u8> = serialize(value, Format::Wxf)
        .unwrap_or_else(|e| panic!("WXF serialize failed: {}", e));
    NumericArray::<u8>::from_slice(&bytes)
}

/// Marker trait used by the proc-macro to constrain the user function's
/// argument and return types at expansion time. The macro emits a closure
/// `fn(NumericArray<u8>) -> NumericArray<u8>` whose body uses [`decode`] /
/// [`encode`] around `user_fn(arg)`; the trait is decorative — actual
/// dispatch is type-driven by the call to `decode<A>()` / `encode<R>()`.
pub trait WxfFunction {}
impl<A: FromWolfram, R: ToWolfram> WxfFunction for fn(A) -> R {}

/// Bridge to `wolfram_library_link::macro_utils::call_native_wolfram_library_function`
/// — exposed under our own path so the proc-macro emits a single tidy reference.
pub use wolfram_library_link::macro_utils::call_native_wolfram_library_function
    as call_wxf_wolfram_library_function;

/// Macro-emitted code references `crate::macro_utils::LibraryLinkFunction::Wxf`
/// for inventory submission. Type-aliased to the shared `ExportEntry`.
pub use wolfram_export_core::ExportEntry as LibraryLinkFunction;
