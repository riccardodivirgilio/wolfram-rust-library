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
/// function with `n` parameters. Each arg maps to a `ByteArray` on the WL side.
pub fn wxf_signature(n: usize) -> Result<(Vec<Expr>, Expr), String> {
    Ok((
        vec![Expr::symbol(wolfram_expr::Symbol::new("System`ByteArray")); n],
        Expr::symbol(wolfram_expr::Symbol::new("System`ByteArray")),
    ))
}

/// Deserialize WXF bytes from `input` into a typed value of type `A`.
/// Helper for macro-emitted code so the call sites don't have to import
/// `wolfram_serializer` directly. Returns the error message as a `String`
/// for the bridge to surface back to WL as a `Failure["WxfDeserialize", …]`.
pub fn decode<A: FromWolfram>(input: &NumericArray<u8>) -> Result<A, String> {
    let bytes: &[u8] = input.as_slice();
    deserialize::<A>(bytes, Format::Wxf).map_err(|e| e.to_string())
}

/// Build a `Failure["WxfDeserialize", <|"MessageTemplate" -> msg|>]` Expr that
/// the bridge encodes back to WL when a typed-arg decode fails.
pub fn deserialize_failure_expr(msg: &str) -> wolfram_expr::Expr {
    use wolfram_expr::{Expr, Symbol};
    let assoc_entry = Expr::normal(
        Symbol::new("System`Rule"),
        vec![Expr::string("MessageTemplate"), Expr::string(msg)],
    );
    let assoc = Expr::normal(Symbol::new("System`Association"), vec![assoc_entry]);
    Expr::normal(
        Symbol::new("System`Failure"),
        vec![Expr::string("WxfDeserialize"), assoc],
    )
}

/// Serialize `value` to WXF bytes and wrap them in a UInt8 NumericArray.
pub fn encode<R: ToWolfram>(value: &R) -> NumericArray<u8> {
    let bytes: Vec<u8> = serialize(value, Format::Wxf)
        .unwrap_or_else(|e| panic!("WXF serialize failed: {}", e));
    NumericArray::<u8>::from_slice(&bytes)
}

/// Run `func` (the body of a WXF bridge), catch any panic, and return either
/// the successful `NumericArray<u8>` result or a WXF-serialized
/// `Failure["RustPanic", …]` expression.  The caller always gets a valid
/// `NumericArray<u8>` back — on panic the kernel receives the failure
/// expression rather than an opaque error code.
pub fn call_and_encode_panic<F>(func: F) -> NumericArray<u8>
where
    F: FnOnce() -> NumericArray<u8>,
{
    use std::panic::AssertUnwindSafe;
    match wolfram_library_link::macro_utils::call_and_catch_as_expr(AssertUnwindSafe(
        func,
    )) {
        Ok(result) => result,
        Err(failure_expr) => encode(&failure_expr),
    }
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
pub use wolfram_library_link::macro_utils::call_native_wolfram_library_function as call_wxf_wolfram_library_function;

/// Macro-emitted code references `crate::macro_utils::LibraryLinkFunction::Wxf`
/// for inventory submission. Type-aliased to the shared `ExportEntry`.
pub use wolfram_export_core::ExportEntry as LibraryLinkFunction;
