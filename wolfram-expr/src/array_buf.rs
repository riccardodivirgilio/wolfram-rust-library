//! Shared implementation backing both [`NumericArray`][crate::NumericArray] and
//! [`PackedArray`][crate::PackedArray].
//!
//! Both types are dense N-dimensional buffers with an element-type tag — the only
//! difference is the *set* of valid element types (PackedArray supports a strict
//! subset). [`ArrayBuf<Tag>`] captures the shared shape + bytes; the concrete tag
//! enums (`NumericArrayDataType`, `PackedArrayDataType`) implement [`ArrayTag`]
//! and provide the per-type specifics.

use std::hash::Hash;

use crate::{ByteArray, NumericArrayDataType, NumericArrayRead};

/// Element-type tag carried by an [`ArrayBuf`]. Implemented by
/// [`NumericArrayDataType`] and [`PackedArrayDataType`][crate::PackedArrayDataType].
pub trait ArrayTag:
    Copy + Eq + Ord + Hash + 'static + std::fmt::Debug + Send + Sync
{
    /// Bytes per element (1, 2, 4, 8, or 16).
    fn size_in_bytes(self) -> usize;

    /// Wolfram Language type name (e.g. `"Integer32"`, `"Real64"`).
    fn name(self) -> &'static str;

    /// Convert to a [`NumericArrayDataType`] — always lossless: PackedArray's
    /// element types are a strict subset of NumericArray's, and for NumericArray
    /// itself the conversion is the identity.
    fn to_numeric_array_data_type(self) -> NumericArrayDataType;
}

/// Generic dense N-dimensional buffer parameterized by an element-type tag.
///
/// `NumericArray = ArrayBuf<NumericArrayDataType>` and
/// `PackedArray   = ArrayBuf<PackedArrayDataType>`. Each provides specialized
/// constructors (`from_slice<T: …Element>`) and a typed slice view; everything
/// else (rank, element_count, byte_count, dimensions, as_bytes, the
/// [`NumericArrayRead`] impl) is shared via this struct.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArrayBuf<Tag: ArrayTag> {
    pub(crate) data_type: Tag,
    pub(crate) dimensions: Vec<usize>,
    pub(crate) bytes: ByteArray,
}

impl<Tag: ArrayTag> ArrayBuf<Tag> {
    /// Construct from raw parts. Caller is responsible for ensuring
    /// `bytes.len() == prod(dimensions) * data_type.size_in_bytes()`.
    pub fn new(data_type: Tag, dimensions: Vec<usize>, bytes: ByteArray) -> Self {
        debug_assert_eq!(
            bytes.len(),
            dimensions.iter().product::<usize>() * data_type.size_in_bytes(),
            "ArrayBuf::new: byte length does not match dims * element size"
        );
        ArrayBuf {
            data_type,
            dimensions,
            bytes,
        }
    }

    /// The concrete element-type tag.
    pub fn data_type(&self) -> Tag {
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

    /// Total element count = product of dimensions.
    pub fn element_count(&self) -> usize {
        self.dimensions.iter().product()
    }

    /// Total byte length of the buffer.
    pub fn byte_count(&self) -> usize {
        self.bytes.len()
    }

    /// Bytes per element (per the element-type tag).
    pub fn element_size(&self) -> usize {
        self.data_type.size_in_bytes()
    }
}

/// Bridge to the unified [`NumericArrayRead`] read API. Applies to both
/// `NumericArray` and `PackedArray` automatically.
impl<Tag: ArrayTag> NumericArrayRead for ArrayBuf<Tag> {
    fn data_type(&self) -> NumericArrayDataType {
        self.data_type.to_numeric_array_data_type()
    }

    fn dimensions(&self) -> &[usize] {
        &self.dimensions
    }

    fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}
