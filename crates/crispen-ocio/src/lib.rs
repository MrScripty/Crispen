//! OCIO integration crate for Crispen.
//!
//! This crate provides a minimal safe wrapper over a thin C ABI layer built on
//! top of OpenColorIO's C++ API.
#![allow(unsafe_code)]
// FFI wrappers necessarily use unsafe externs and raw pointers.

mod config;
mod error;
mod processor;
mod sys;

pub use config::OcioConfig;
pub use error::OcioError;
pub use processor::{OcioCpuProcessor, OcioProcessor};
