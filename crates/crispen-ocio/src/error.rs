use std::ffi::{CStr, NulError};

use crate::sys;

#[derive(Debug, thiserror::Error)]
pub enum OcioError {
    #[error("string contains interior NUL: {0}")]
    Nul(#[from] NulError),
    #[error("OCIO error: {0}")]
    Ocio(String),
    #[error("invalid argument: {0}")]
    InvalidArgument(&'static str),
}

pub(crate) fn last_error_message() -> String {
    // SAFETY: FFI returns either null or a valid NUL-terminated string.
    unsafe {
        let ptr = sys::ocio_get_last_error();
        if ptr.is_null() {
            return "unknown OCIO error".to_string();
        }
        CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }
}

pub(crate) fn ffi_error() -> OcioError {
    OcioError::Ocio(last_error_message())
}
