//! OIIO integration crate for Crispen.
//!
//! This crate provides a minimal safe wrapper over a thin C ABI layer built on
//! top of OpenImageIO's C++ API. It supports reading images and extracting
//! color space metadata detected from file headers.
#![allow(unsafe_code)]
// FFI wrappers necessarily use unsafe externs and raw pointers.

mod error;
mod image_input;
mod sys;

pub use error::OiioError;
pub use image_input::OiioImageInput;
