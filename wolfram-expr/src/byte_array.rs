//! [`ByteArray`][ref/ByteArray]<sub>WL</sub> data type — `ByteArray["..."]`.
//!
//! Wire-distinct from a `String` in WXF (BinaryString token `'B'` vs String `'S'`).
//! Represents an opaque sequence of bytes with no encoding interpretation.
//!
//! [ref/ByteArray]: https://reference.wolfram.com/language/ref/ByteArray.html

use std::sync::Arc;

/// Owned [`ByteArray`][ref/ByteArray]<sub>WL</sub> value — refcounted byte buffer.
///
/// [ref/ByteArray]: https://reference.wolfram.com/language/ref/ByteArray.html
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ByteArray {
    bytes: Arc<[u8]>,
}

impl ByteArray {
    /// Construct from any byte slice (copies into a fresh `Arc<[u8]>`).
    pub fn new(bytes: &[u8]) -> Self {
        ByteArray {
            bytes: Arc::from(bytes),
        }
    }

    /// Construct from an owned `Vec<u8>` without copying.
    pub fn from_vec(bytes: Vec<u8>) -> Self {
        ByteArray {
            bytes: Arc::from(bytes.into_boxed_slice()),
        }
    }

    /// Construct from an existing `Arc<[u8]>` (no copy, refcount bump).
    pub fn from_arc(bytes: Arc<[u8]>) -> Self {
        ByteArray { bytes }
    }

    /// Borrow the bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Length in bytes.
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Borrow the underlying refcounted handle.
    pub fn as_arc(&self) -> &Arc<[u8]> {
        &self.bytes
    }
}

impl AsRef<[u8]> for ByteArray {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

impl From<&[u8]> for ByteArray {
    fn from(bytes: &[u8]) -> Self {
        ByteArray::new(bytes)
    }
}

impl From<Vec<u8>> for ByteArray {
    fn from(bytes: Vec<u8>) -> Self {
        ByteArray::from_vec(bytes)
    }
}

impl From<Arc<[u8]>> for ByteArray {
    fn from(bytes: Arc<[u8]>) -> Self {
        ByteArray { bytes }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty() {
        let b = ByteArray::new(&[]);
        assert!(b.is_empty());
        assert_eq!(b.len(), 0);
    }

    #[test]
    fn from_slice_and_vec() {
        let b1 = ByteArray::new(&[1u8, 2, 3, 4]);
        let b2 = ByteArray::from_vec(vec![1u8, 2, 3, 4]);
        assert_eq!(b1, b2);
        assert_eq!(b1.as_bytes(), &[1u8, 2, 3, 4]);
    }

    #[test]
    fn arc_sharing() {
        let arc: Arc<[u8]> = Arc::from(vec![5u8, 6, 7].into_boxed_slice());
        let b = ByteArray::from_arc(arc.clone());
        assert!(Arc::ptr_eq(b.as_arc(), &arc));
    }
}
