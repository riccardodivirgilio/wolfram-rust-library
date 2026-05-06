//! WXF binary wire format — serializer + deserializer.

pub mod constants;
pub mod deserialize;
pub mod varint;

use std::io::Write;

use wolfram_expr::{NumericArray, NumericArrayRead, PackedArray};

#[cfg(feature = "bignum")]
use wolfram_expr::{BigInteger, BigReal};

use crate::serializer::{Serializer, ToWolfram};
use crate::Error;

use self::constants::*;
use self::varint::write_varint;

pub use self::deserialize::deserialize;

/// WXF binary serializer. Wraps any [`Write`] sink.
pub struct WxfSerializer<'w, W: Write> {
    out: &'w mut W,
}

impl<'w, W: Write> WxfSerializer<'w, W> {
    /// Construct + write the WXF header (`b"8:"`).
    pub fn new(writer: &'w mut W) -> Result<Self, Error> {
        writer.write_all(&[WXF_VERSION, WXF_HEADER_SEPARATOR])?;
        Ok(WxfSerializer { out: writer })
    }
}

fn write_length_prefixed_bytes<W: Write>(
    w: &mut W,
    tag: u8,
    bytes: &[u8],
) -> Result<(), Error> {
    w.write_all(&[tag])?;
    write_varint(w, bytes.len() as u64)?;
    w.write_all(bytes)?;
    Ok(())
}

impl<'w, W: Write> Serializer for WxfSerializer<'w, W> {
    fn serialize_integer(&mut self, n: i64) -> Result<(), Error> {
        if let Ok(v) = i8::try_from(n) {
            self.out.write_all(&[TOKEN_INTEGER8])?;
            self.out.write_all(&v.to_le_bytes())?;
        } else if let Ok(v) = i16::try_from(n) {
            self.out.write_all(&[TOKEN_INTEGER16])?;
            self.out.write_all(&v.to_le_bytes())?;
        } else if let Ok(v) = i32::try_from(n) {
            self.out.write_all(&[TOKEN_INTEGER32])?;
            self.out.write_all(&v.to_le_bytes())?;
        } else {
            self.out.write_all(&[TOKEN_INTEGER64])?;
            self.out.write_all(&n.to_le_bytes())?;
        }
        Ok(())
    }

    fn serialize_real(&mut self, f: f64) -> Result<(), Error> {
        self.out.write_all(&[TOKEN_REAL64])?;
        self.out.write_all(&f.to_le_bytes())?;
        Ok(())
    }

    fn serialize_string(&mut self, s: &str) -> Result<(), Error> {
        write_length_prefixed_bytes(self.out, TOKEN_STRING, s.as_bytes())
    }

    fn serialize_symbol(&mut self, name: &str) -> Result<(), Error> {
        write_length_prefixed_bytes(self.out, TOKEN_SYMBOL, name.as_bytes())
    }

    fn serialize_byte_array(&mut self, bytes: &[u8]) -> Result<(), Error> {
        write_length_prefixed_bytes(self.out, TOKEN_BINARY_STRING, bytes)
    }

    fn serialize_function(
        &mut self,
        head: &dyn ToWolfram,
        args: &[&dyn ToWolfram],
    ) -> Result<(), Error> {
        self.out.write_all(&[TOKEN_FUNCTION])?;
        write_varint(self.out, args.len() as u64)?;
        head.serialize(self)?;
        for arg in args {
            arg.serialize(self)?;
        }
        Ok(())
    }

    fn serialize_association(
        &mut self,
        entries: &[(&dyn ToWolfram, &dyn ToWolfram, bool)],
    ) -> Result<(), Error> {
        self.out.write_all(&[TOKEN_ASSOCIATION])?;
        write_varint(self.out, entries.len() as u64)?;
        for (k, v, delayed) in entries {
            self.out
                .write_all(&[if *delayed { TOKEN_RULE_DELAYED } else { TOKEN_RULE }])?;
            k.serialize(self)?;
            v.serialize(self)?;
        }
        Ok(())
    }

    fn serialize_numeric_array(&mut self, arr: &NumericArray) -> Result<(), Error> {
        self.out.write_all(&[TOKEN_NUMERIC_ARRAY])?;
        self.out.write_all(&[array_type_to_wxf(arr.data_type())])?;
        let dims = arr.dimensions();
        write_varint(self.out, dims.len() as u64)?;
        for d in dims {
            write_varint(self.out, *d as u64)?;
        }
        self.out.write_all(arr.as_bytes())?;
        Ok(())
    }

    fn serialize_packed_array(&mut self, arr: &PackedArray) -> Result<(), Error> {
        self.out.write_all(&[TOKEN_PACKED_ARRAY])?;
        let dt = NumericArrayRead::data_type(arr);
        self.out.write_all(&[array_type_to_wxf(dt)])?;
        let dims = arr.dimensions();
        write_varint(self.out, dims.len() as u64)?;
        for d in dims {
            write_varint(self.out, *d as u64)?;
        }
        self.out.write_all(arr.as_bytes())?;
        Ok(())
    }

    #[cfg(feature = "bignum")]
    fn serialize_big_integer(&mut self, n: &BigInteger) -> Result<(), Error> {
        let s = n.to_decimal_string();
        write_length_prefixed_bytes(self.out, TOKEN_BIG_INTEGER, s.as_bytes())
    }

    #[cfg(feature = "bignum")]
    fn serialize_big_real(&mut self, r: &BigReal) -> Result<(), Error> {
        write_length_prefixed_bytes(self.out, TOKEN_BIG_REAL, r.as_str().as_bytes())
    }
}
