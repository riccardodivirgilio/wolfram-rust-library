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

use std::sync::Arc;

use crate::numeric_array::NumericArrayRead;

/// Element-type tag for a [`PackedArray`].
///
/// Subset of [`NumericArrayDataType`][crate::NumericArrayDataType] — only the types
/// that the Wolfram Language treats as valid packed-array element types.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
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
/// Sealed; impls cover `i8`, `i16`, `i32`, `i64`, `f32`, `f64`. No unsigned integers
/// (PackedArray does not support them) and complex representations require an external
/// type (use the byte-level constructors instead).
pub trait PackedArrayElement: Sized + Copy + 'static + sealed::Sealed {
    /// Dynamic discriminant for this Rust type.
    const TYPE: PackedArrayDataType;
}

mod sealed {
    pub trait Sealed {}
    impl Sealed for i8 {}
    impl Sealed for i16 {}
    impl Sealed for i32 {}
    impl Sealed for i64 {}
    impl Sealed for f32 {}
    impl Sealed for f64 {}
}

impl PackedArrayElement for i8 {
    const TYPE: PackedArrayDataType = PackedArrayDataType::Integer8;
}
impl PackedArrayElement for i16 {
    const TYPE: PackedArrayDataType = PackedArrayDataType::Integer16;
}
impl PackedArrayElement for i32 {
    const TYPE: PackedArrayDataType = PackedArrayDataType::Integer32;
}
impl PackedArrayElement for i64 {
    const TYPE: PackedArrayDataType = PackedArrayDataType::Integer64;
}
impl PackedArrayElement for f32 {
    const TYPE: PackedArrayDataType = PackedArrayDataType::Real32;
}
impl PackedArrayElement for f64 {
    const TYPE: PackedArrayDataType = PackedArrayDataType::Real64;
}

/// Owned [`PackedArray`][ref/PackedArray]<sub>WL</sub> value.
///
/// [ref/PackedArray]: https://reference.wolfram.com/language/ref/Developer/PackedArrayQ.html
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackedArray {
    data_type: PackedArrayDataType,
    dimensions: Vec<usize>,
    bytes: Arc<[u8]>,
}

impl PackedArray {
    /// Construct from raw parts.
    pub fn new(
        data_type: PackedArrayDataType,
        dimensions: Vec<usize>,
        bytes: Arc<[u8]>,
    ) -> Self {
        debug_assert_eq!(
            bytes.len(),
            dimensions.iter().product::<usize>() * data_type.size_in_bytes(),
        );
        PackedArray {
            data_type,
            dimensions,
            bytes,
        }
    }

    /// Construct from a typed slice.
    pub fn from_slice<T: PackedArrayElement>(dimensions: Vec<usize>, slice: &[T]) -> Self {
        assert_eq!(dimensions.iter().product::<usize>(), slice.len());
        let bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(
                slice.as_ptr() as *const u8,
                std::mem::size_of_val(slice),
            )
        };
        PackedArray {
            data_type: T::TYPE,
            dimensions,
            bytes: Arc::from(bytes),
        }
    }

    /// The element type.
    pub fn data_type(&self) -> PackedArrayDataType {
        self.data_type
    }

    /// Multi-dimensional shape.
    pub fn dimensions(&self) -> &[usize] {
        &self.dimensions
    }

    /// Raw byte buffer.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Number of dimensions.
    pub fn rank(&self) -> usize {
        self.dimensions.len()
    }

    /// Total element count.
    pub fn element_count(&self) -> usize {
        self.dimensions.iter().product()
    }

    /// Try to view the buffer as a slice of `T`. Returns `None` on type-tag mismatch.
    pub fn try_as_slice<T: PackedArrayElement>(&self) -> Option<&[T]> {
        if self.data_type != T::TYPE {
            return None;
        }
        let elem_size = std::mem::size_of::<T>();
        debug_assert_eq!(self.bytes.len() % elem_size, 0);
        Some(unsafe {
            std::slice::from_raw_parts(
                self.bytes.as_ptr() as *const T,
                self.bytes.len() / elem_size,
            )
        })
    }
}

/// Implementation note: `PackedArray` reuses the structural shape of [`NumericArrayRead`]
/// for its compatible types but doesn't share the trait directly because the type tags
/// differ. For uniform read-side polymorphism, use the inherent methods.
///
/// Bridge to `NumericArrayRead` when the packed type happens to fit numeric-array semantics:
impl NumericArrayRead for PackedArray {
    fn data_type(&self) -> crate::NumericArrayDataType {
        // Bridge: PackedArray → NumericArrayDataType for unified read API.
        // Lossless because every PackedArrayDataType has a NumericArrayDataType analog.
        match self.data_type {
            PackedArrayDataType::Integer8 => crate::NumericArrayDataType::Bit8,
            PackedArrayDataType::Integer16 => crate::NumericArrayDataType::Bit16,
            PackedArrayDataType::Integer32 => crate::NumericArrayDataType::Bit32,
            PackedArrayDataType::Integer64 => crate::NumericArrayDataType::Bit64,
            PackedArrayDataType::Real32 => crate::NumericArrayDataType::Real32,
            PackedArrayDataType::Real64 => crate::NumericArrayDataType::Real64,
            PackedArrayDataType::ComplexReal32 => crate::NumericArrayDataType::ComplexReal32,
            PackedArrayDataType::ComplexReal64 => crate::NumericArrayDataType::ComplexReal64,
        }
    }
    fn dimensions(&self) -> &[usize] {
        &self.dimensions
    }
    fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

impl<T: PackedArrayElement> From<(Vec<usize>, &[T])> for PackedArray {
    fn from((dims, slice): (Vec<usize>, &[T])) -> Self {
        PackedArray::from_slice(dims, slice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            crate::NumericArrayDataType::Bit32
        );
    }
}
