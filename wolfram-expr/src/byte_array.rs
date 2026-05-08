//! [`ByteArray`][ref/ByteArray]<sub>WL</sub> data type — a byte buffer.
//!
//! `ByteArray` is a type alias for [`Vec<u8>`]. Wire-distinct from a `String` in WXF
//! (BinaryString token `'B'` vs String `'S'`). The variant identity that distinguishes
//! "this is a `ByteArray` expression" from "this is a `Vec<u8>` to send as a `List`"
//! lives at the [`ExprKind::ByteArray`][crate::ExprKind::ByteArray] variant level —
//! a `Vec<u8>` becomes a `ByteArray` expression by going through [`Expr::from`].
//!
//! [ref/ByteArray]: https://reference.wolfram.com/language/ref/ByteArray.html

/// Owned byte buffer — Wolfram Language `ByteArray["..."]`.
pub type ByteArray = Vec<u8>;
