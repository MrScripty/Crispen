use std::ffi::{CStr, CString};
use std::path::Path;
use std::ptr::NonNull;

use crate::error::{OcioError, ffi_error};
use crate::processor::OcioProcessor;
use crate::sys;

pub struct OcioConfig {
    pub(crate) ptr: NonNull<sys::OcioConfig>,
}

impl OcioConfig {
    pub fn from_env() -> Result<Self, OcioError> {
        // SAFETY: FFI constructor returns owned opaque pointer or null on error.
        let ptr = unsafe { sys::ocio_config_create_from_env() };
        NonNull::new(ptr)
            .map(|ptr| Self { ptr })
            .ok_or_else(ffi_error)
    }

    pub fn from_file(path: &Path) -> Result<Self, OcioError> {
        let path = CString::new(path.to_string_lossy().as_bytes())?;
        // SAFETY: FFI constructor returns owned opaque pointer or null on error.
        let ptr = unsafe { sys::ocio_config_create_from_file(path.as_ptr()) };
        NonNull::new(ptr)
            .map(|ptr| Self { ptr })
            .ok_or_else(ffi_error)
    }

    pub fn builtin(uri: &str) -> Result<Self, OcioError> {
        let uri = CString::new(uri)?;
        // SAFETY: FFI constructor returns owned opaque pointer or null on error.
        let ptr = unsafe { sys::ocio_config_create_builtin(uri.as_ptr()) };
        NonNull::new(ptr)
            .map(|ptr| Self { ptr })
            .ok_or_else(ffi_error)
    }

    pub fn color_space_names(&self) -> Vec<String> {
        // SAFETY: `self.ptr` is valid for the life of `self`.
        let count = unsafe { sys::ocio_config_get_num_color_spaces(self.ptr.as_ptr()) };
        if count <= 0 {
            return Vec::new();
        }

        (0..count)
            .filter_map(|i| {
                // SAFETY: index in range and `self` alive.
                let ptr = unsafe { sys::ocio_config_get_color_space_name(self.ptr.as_ptr(), i) };
                cstr_to_string(ptr)
            })
            .collect()
    }

    pub fn role(&self, name: &str) -> Option<String> {
        let name = CString::new(name).ok()?;
        // SAFETY: pointers are valid while called.
        let ptr = unsafe { sys::ocio_config_get_role(self.ptr.as_ptr(), name.as_ptr()) };
        cstr_to_string(ptr)
    }

    pub fn displays(&self) -> Vec<String> {
        // SAFETY: `self.ptr` is valid for the life of `self`.
        let count = unsafe { sys::ocio_config_get_num_displays(self.ptr.as_ptr()) };
        if count <= 0 {
            return Vec::new();
        }

        (0..count)
            .filter_map(|i| {
                // SAFETY: index in range and `self` alive.
                let ptr = unsafe { sys::ocio_config_get_display(self.ptr.as_ptr(), i) };
                cstr_to_string(ptr)
            })
            .collect()
    }

    pub fn default_display(&self) -> String {
        // SAFETY: `self.ptr` is valid for the life of `self`.
        let ptr = unsafe { sys::ocio_config_get_default_display(self.ptr.as_ptr()) };
        cstr_to_string(ptr).unwrap_or_default()
    }

    pub fn views(&self, display: &str) -> Vec<String> {
        let display = match CString::new(display) {
            Ok(v) => v,
            Err(_) => return Vec::new(),
        };

        // SAFETY: pointers are valid while called.
        let count = unsafe { sys::ocio_config_get_num_views(self.ptr.as_ptr(), display.as_ptr()) };
        if count <= 0 {
            return Vec::new();
        }

        (0..count)
            .filter_map(|i| {
                // SAFETY: pointers are valid while called.
                let ptr =
                    unsafe { sys::ocio_config_get_view(self.ptr.as_ptr(), display.as_ptr(), i) };
                cstr_to_string(ptr)
            })
            .collect()
    }

    pub fn default_view(&self, display: &str) -> String {
        let display = match CString::new(display) {
            Ok(v) => v,
            Err(_) => return String::new(),
        };

        // SAFETY: pointers are valid while called.
        let ptr = unsafe { sys::ocio_config_get_default_view(self.ptr.as_ptr(), display.as_ptr()) };
        cstr_to_string(ptr).unwrap_or_default()
    }

    pub fn processor(&self, src: &str, dst: &str) -> Result<OcioProcessor, OcioError> {
        let src = CString::new(src)?;
        let dst = CString::new(dst)?;

        // SAFETY: pointers are valid while called.
        let ptr = unsafe {
            sys::ocio_config_get_processor_by_names(self.ptr.as_ptr(), src.as_ptr(), dst.as_ptr())
        };

        OcioProcessor::from_raw(ptr)
    }

    pub fn display_view_processor(
        &self,
        src: &str,
        display: &str,
        view: &str,
    ) -> Result<OcioProcessor, OcioError> {
        let src = CString::new(src)?;
        let display = CString::new(display)?;
        let view = CString::new(view)?;

        // SAFETY: pointers are valid while called.
        let ptr = unsafe {
            sys::ocio_config_get_display_view_processor(
                self.ptr.as_ptr(),
                src.as_ptr(),
                display.as_ptr(),
                view.as_ptr(),
            )
        };

        OcioProcessor::from_raw(ptr)
    }
}

impl Drop for OcioConfig {
    fn drop(&mut self) {
        // SAFETY: pointer came from FFI constructor and is owned by this wrapper.
        unsafe { sys::ocio_config_destroy(self.ptr.as_ptr()) };
    }
}

fn cstr_to_string(ptr: *const std::ffi::c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    // SAFETY: FFI contract returns valid NUL-terminated strings.
    let s = unsafe { CStr::from_ptr(ptr) }.to_string_lossy();
    if s.is_empty() {
        None
    } else {
        Some(s.into_owned())
    }
}
