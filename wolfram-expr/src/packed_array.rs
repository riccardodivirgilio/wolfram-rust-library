//! [`PackedArray`][ref/PackedArray]<sub>WL</sub> data type.
//!
//! Packed arrays are dense, homogeneous N-dimensional numeric arrays. They are
//! semantically distinct from [`NumericArray`][crate::NumericArray] — both have a
//! type tag, dimensions, and a flat byte buffer, but WXF gives them separate
//! wire tokens (`'N'` for NumericArray, `0xC1` for PackedArray) and Wolfram
//! Language pattern-matching distinguishes them.
//!
//! Packed arrays are restricted to a smaller set of element types than NumericArray
//! (no unsigned integers, no complex pair representations).
//!
//! [ref/PackedArray]: https://reference.wolfram.com/language/ref/Developer/PackedArrayQ.html

use crate::array_buf::{ArrayBuf, ArrayElement, ArrayTag};
use crate::NumericArrayDataType;

/// Element-type tag for a [`PackedArray`].
///
/// Subset of [`NumericArrayDataType`][crate::NumericArrayDataType] — only the types
/// that the Wolfram Language treats as valid packed-array element types.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
#[allow(missing_docs)]
pub enum PackedArrayDataType {
    Integer8 = 0,
    Integer16 = 1,
    Integer32 = 2,
    Integer64 = 3,
    Real32 = 4,
    Real64 = 5,
    ComplexReal32 = 6,
    ComplexReal64 = 7,
}

impl PackedArrayDataType {
    /// Element size in bytes.
    pub fn size_in_bytes(&self) -> usize {
        match *self {
            PackedArrayDataType::Integer8 => 1,
            PackedArrayDataType::Integer16 => 2,
            PackedArrayDataType::Integer32 | PackedArrayDataType::Real32 => 4,
            PackedArrayDataType::Integer64
            | PackedArrayDataType::Real64
            | PackedArrayDataType::ComplexReal32 => 8,
            PackedArrayDataType::ComplexReal64 => 16,
        }
    }

    /// Wolfram Language name (e.g. `"Integer32"`, `"Real64"`).
    pub fn name(&self) -> &'static str {
        match *self {
            PackedArrayDataType::Integer8 => "Integer8",
            PackedArrayDataType::Integer16 => "Integer16",
            PackedArrayDataType::Integer32 => "Integer32",
            PackedArrayDataType::Integer64 => "Integer64",
            PackedArrayDataType::Real32 => "Real32",
            PackedArrayDataType::Real64 => "Real64",
            PackedArrayDataType::ComplexReal32 => "ComplexReal32",
            PackedArrayDataType::ComplexReal64 => "ComplexReal64",
        }
    }
}

/// Marker trait: Rust primitives valid as a [`PackedArray`] element.
///
/// Equivalent to [`ArrayElement<PackedArrayDataType>`][ArrayElement] — kept as
/// a stand-alone name (and `const TYPE` field) for ergonomics and symmetry with
/// [`NumericArrayElement`][crate::NumericArrayElement]. New code can equivalently
/// use `ArrayElement<PackedArrayDataType>`.
pub trait PackedArrayElement: ArrayElement<PackedArrayDataType> {
    /// Equivalent to `<Self as ArrayElement<PackedArrayDataType>>::TAG`.
    const TYPE: PackedArrayDataType = <Self as ArrayElement<PackedArrayDataType>>::TAG;
}
impl<T: ArrayElement<PackedArrayDataType>> PackedArrayElement for T {}

impl ArrayElement<PackedArrayDataType> for i8 { const TAG: PackedArrayDataType = PackedArrayDataType::Integer8; }
impl ArrayElement<PackedArrayDataType> for i16 { const TAG: PackedArrayDataType = PackedArrayDataType::Integer16; }
impl ArrayElement<PackedArrayDataType> for i32 { const TAG: PackedArrayDataType = PackedArrayDataType::Integer32; }
impl ArrayElement<PackedArrayDataType> for i64 { const TAG: PackedArrayDataType = PackedArrayDataType::Integer64; }
impl ArrayElement<PackedArrayDataType> for f32 { const TAG: PackedArrayDataType = PackedArrayDataType::Real32; }
impl ArrayElement<PackedArrayDataType> for f64 { const TAG: PackedArrayDataType = PackedArrayDataType::Real64; }
impl ArrayElement<PackedArrayDataType> for crate::complex::Complex32 { const TAG: PackedArrayDataType = PackedArrayDataType::ComplexReal32; }
impl ArrayElement<PackedArrayDataType> for crate::complex::Complex64 { const TAG: PackedArrayDataType = PackedArrayDataType::ComplexReal64; }

impl ArrayTag for PackedArrayDataType {
    fn size_in_bytes(self) -> usize {
        Self::size_in_bytes(&self)
    }
    fn name(self) -> &'static str {
        Self::name(&self)
    }
    fn to_numeric_array_data_type(self) -> NumericArrayDataType {
        // PackedArray's element types are a strict subset of NumericArray's —
        // the conversion is lossless.
        match self {
            PackedArrayDataType::Integer8 => NumericArrayDataType::Integer8,
            PackedArrayDataType::Integer16 => NumericArrayDataType::Integer16,
            PackedArrayDataType::Integer32 => NumericArrayDataType::Integer32,
            PackedArrayDataType::Integer64 => NumericArrayDataType::Integer64,
            PackedArrayDataType::Real32 => NumericArrayDataType::Real32,
            PackedArrayDataType::Real64 => NumericArrayDataType::Real64,
            PackedArrayDataType::ComplexReal32 => NumericArrayDataType::ComplexReal32,
            PackedArrayDataType::ComplexReal64 => NumericArrayDataType::ComplexReal64,
        }
    }
}

/// Owned [`PackedArray`][ref/PackedArray]<sub>WL</sub> value.
///
/// Type alias over [`ArrayBuf`] — `from_slice`, `try_as_slice`, and the rest of
/// the shape/buffer API live on `ArrayBuf<Tag>` and resolve here through the
/// alias. The element-type tag for this alias is [`PackedArrayDataType`].
/// The [`NumericArrayRead`][crate::NumericArrayRead] impl on `ArrayBuf` bridges
/// PackedArray into the unified read API.
///
/// [ref/PackedArray]: https://reference.wolfram.com/language/ref/Developer/PackedArrayQ.html
pub type PackedArray = ArrayBuf<PackedArrayDataType>;

impl<T: ArrayElement<PackedArrayDataType>> From<(Vec<usize>, &[T])> for PackedArray {
    fn from((dims, slice): (Vec<usize>, &[T])) -> Self {
        PackedArray::from_slice(dims, slice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NumericArrayRead;

    #[test]
    fn from_slice_basic() {
        let arr = PackedArray::from_slice::<f64>(vec![2, 2], &[1.0, 2.0, 3.0, 4.0]);
        assert_eq!(arr.data_type(), PackedArrayDataType::Real64);
        assert_eq!(arr.dimensions(), &[2, 2]);
        assert_eq!(arr.element_count(), 4);
        assert_eq!(arr.try_as_slice::<f64>(), Some([1.0, 2.0, 3.0, 4.0].as_slice()));
        assert_eq!(arr.try_as_slice::<i32>(), None);
    }

    #[test]
    fn bridge_to_numeric_array_read() {
        let arr = PackedArray::from_slice::<i32>(vec![3], &[10, 20, 30]);
        // Use NumericArrayRead methods via the bridge:
        assert_eq!(NumericArrayRead::rank(&arr), 1);
        assert_eq!(NumericArrayRead::byte_count(&arr), 12);
        assert_eq!(
            NumericArrayRead::data_type(&arr),
            crate::NumericArrayDataType::Integer32
        );
    }
}
