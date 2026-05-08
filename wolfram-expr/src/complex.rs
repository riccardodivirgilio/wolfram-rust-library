//! Complex number primitives suitable as element types of [`NumericArray`] and
//! [`PackedArray`][crate::PackedArray].
//!
//! [`Complex64`] is byte-layout-compatible with the C ABI's `_Complex double`
//! (and thus `wolfram_library_link_sys::mcomplex`, which re-exports this same
//! type via `pub use`). [`Complex32`] is the `_Complex float` analog.
//!
//! Both types are `#[repr(C)]` with two interleaved real/imaginary scalar fields,
//! matching WXF's wire layout for `ComplexReal{32,64}` array elements.
//!
//! [`NumericArray`]: crate::NumericArray

/// Single 64-bit complex number — pair of `f64` (real, imaginary).
///
/// Layout matches the C ABI `_Complex double` and the WXF `ComplexReal64`
/// element wire format. `wolfram-library-link-sys::mcomplex` is `pub use`'d as
/// an alias for this type, so `wll::NumericArray<sys::mcomplex>` and
/// `wll::NumericArray<wolfram_expr::Complex64>` are the same instantiation.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Complex64 {
    /// Real part.
    pub re: f64,
    /// Imaginary part.
    pub im: f64,
}

impl Complex64 {
    /// Construct from `(real, imaginary)` parts.
    pub const fn new(re: f64, im: f64) -> Self {
        Complex64 { re, im }
    }

    /// Real part.
    pub const fn re(self) -> f64 {
        self.re
    }
    /// Imaginary part.
    pub const fn im(self) -> f64 {
        self.im
    }
}

impl Default for Complex64 {
    fn default() -> Self {
        Complex64 { re: 0.0, im: 0.0 }
    }
}

/// Single 32-bit complex number — pair of `f32` (real, imaginary). Layout matches
/// the WXF `ComplexReal32` element wire format. No `_Complex float` typedef
/// exists in `WolframLibrary.h`, so this type is wolfram-expr-only.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Complex32 {
    /// Real part.
    pub re: f32,
    /// Imaginary part.
    pub im: f32,
}

impl Complex32 {
    /// Construct from `(real, imaginary)` parts.
    pub const fn new(re: f32, im: f32) -> Self {
        Complex32 { re, im }
    }

    /// Real part.
    pub const fn re(self) -> f32 {
        self.re
    }
    /// Imaginary part.
    pub const fn im(self) -> f32 {
        self.im
    }
}

impl Default for Complex32 {
    fn default() -> Self {
        Complex32 { re: 0.0, im: 0.0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryInto;

    #[test]
    fn layout_matches_c_complex_double() {
        // C `_Complex double` is two contiguous doubles, 16 bytes, aligned to f64.
        assert_eq!(std::mem::size_of::<Complex64>(), 16);
        assert_eq!(std::mem::align_of::<Complex64>(), std::mem::align_of::<f64>());
        // Field offsets: re at byte 0, im at byte 8.
        let z = Complex64::new(1.0, 2.0);
        let bytes: [u8; 16] = unsafe { std::mem::transmute(z) };
        let re_back = f64::from_le_bytes(bytes[..8].try_into().unwrap());
        let im_back = f64::from_le_bytes(bytes[8..].try_into().unwrap());
        assert_eq!(re_back, 1.0);
        assert_eq!(im_back, 2.0);
    }

    #[test]
    fn layout_matches_c_complex_float() {
        assert_eq!(std::mem::size_of::<Complex32>(), 8);
        assert_eq!(std::mem::align_of::<Complex32>(), std::mem::align_of::<f32>());
    }
}
