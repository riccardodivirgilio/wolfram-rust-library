//! Serialize and deserialize [Wolfram Language expressions][wolfram_expr::Expr] to and
//! from Wolfram Language `InputForm` text and the WXF binary wire format.
//!
//! Mirrors the architectural pattern of [`wolframclient.serializers`][wolframclient]
//! in Python: a single [`export`] entry point produces bytes (WL or WXF), a single
//! [`import`] entry point reads WXF bytes back into [`Expr`].
//!
//! WL parsing (text → Expr) is out of V1 scope: [`import`] called with [`Format::Wl`]
//! returns [`Error::UnsupportedImportFormat`].
//!
//! [wolframclient]: https://github.com/WolframResearch/WolframClientForPython

#![warn(missing_docs)]

pub mod consumer;
pub mod serializer;
pub mod wl;
pub mod wxf;

use std::io::Write;

use wolfram_expr::Expr;

pub use crate::consumer::{ExprConsumer, WolframConsumer};
pub use crate::serializer::{Serializer, ToWolfram};

/// Output format selector for [`export`] / [`import`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Format {
    /// Wolfram Language `InputForm` (UTF-8 text). Export-only in V1.
    Wl,
    /// WXF binary wire format. Round-trippable.
    Wxf,
}

/// Errors returned by [`export`] / [`import`].
#[derive(Debug)]
pub enum Error {
    /// Wraps an underlying [`std::io::Error`] from a writer or reader.
    Io(std::io::Error),
    /// `import(_, Format::Wl)` — WL parsing is not implemented in V1.
    UnsupportedImportFormat,
    /// WXF byte stream is malformed (header mismatch, unexpected token, truncation, …).
    InvalidWxf(String),
    /// A consumer rejected a value with a domain-specific error.
    Consumer(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "I/O error: {}", e),
            Error::UnsupportedImportFormat => write!(
                f,
                "import(): the requested Format does not support deserialization"
            ),
            Error::InvalidWxf(msg) => write!(f, "invalid WXF: {}", msg),
            Error::Consumer(msg) => write!(f, "consumer error: {}", msg),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

//==============================================================================
// Top-level API
//==============================================================================

/// Serialize `value` using `format`, returning the bytes.
pub fn export<T: ToWolfram + ?Sized>(value: &T, format: Format) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    export_to(value, format, &mut out)?;
    Ok(out)
}

/// Serialize `value` using `format`, writing to `writer`.
pub fn export_to<T, W>(value: &T, format: Format, writer: &mut W) -> Result<(), Error>
where
    T: ToWolfram + ?Sized,
    W: Write,
{
    match format {
        Format::Wl => {
            let mut s = wl::WlSerializer::new(writer);
            value.serialize(&mut s)
        }
        Format::Wxf => {
            let mut s = wxf::WxfSerializer::new(writer)?;
            value.serialize(&mut s)
        }
    }
}

/// Deserialize `bytes` using `format`, returning an [`Expr`]. Uses the default
/// [`ExprConsumer`].
///
/// `format = Format::Wl` returns [`Error::UnsupportedImportFormat`] — text WL parsing
/// is not implemented in V1.
pub fn import(bytes: &[u8], format: Format) -> Result<Expr, Error> {
    let mut c = ExprConsumer;
    import_with(bytes, format, &mut c)
}

/// Deserialize `bytes` using `format` and a custom consumer. Returns the consumer's
/// [`Value`][WolframConsumer::Value].
pub fn import_with<C: WolframConsumer>(
    bytes: &[u8],
    format: Format,
    consumer: &mut C,
) -> Result<C::Value, Error> {
    match format {
        Format::Wl => Err(Error::UnsupportedImportFormat),
        Format::Wxf => wxf::deserialize(bytes, consumer),
    }
}
