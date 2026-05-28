//! Automatically generated bindings to the Wolfram LibraryLink C API.

#![allow(non_snake_case, non_upper_case_globals, non_camel_case_types)]
#![allow(deref_nullptr)]

// `mcomplex` (the C ABI's `_Complex double`) is provided by `wolfram-expr` as
// `Complex64` instead of being bindgen-generated, so the same complex type is
// shared across the entire crate stack. The pre-generated bindings have the
// auto-generated `pub struct mcomplex { ri: [f64; 2] }` block stripped (and
// future bindgen runs blocklist it via `xtask`'s `--blocklist-type mcomplex`).
//
// `mcomplex` itself remains accessible at the original path via this re-export,
// so existing `wolfram_library_link_sys::mcomplex` and
// `wolfram_library_link::NumericArray<sys::mcomplex>` users see no API change.
pub use wolfram_expr::Complex64 as mcomplex;

// The name of this file comes from `build.rs`.
include!(env!("CRATE_WOLFRAM_LIBRARYLINK_SYS_BINDINGS"));
