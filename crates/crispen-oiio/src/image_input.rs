use std::ffi::{CStr, CString};
use std::path::Path;
use std::ptr::NonNull;

use crispen_core::image::BitDepth;

use crate::error::{OiioError, ffi_error};
use crate::sys;

/// OIIO TypeDesc BASETYPE constants (mirrors the C++ enum).
mod basetype {
    pub const UINT8: i32 = 2;
    pub const UINT16: i32 = 4;
    pub const INT16: i32 = 5;
    pub const HALF: i32 = 10;
    pub const FLOAT: i32 = 11;
    pub const DOUBLE: i32 = 12;
}

/// A loaded image file opened via OpenImageIO.
///
/// The image data is read eagerly on [`open`](Self::open) and converted to
/// f32 internally. Use [`read_rgba_f32`](Self::read_rgba_f32) to retrieve
/// pixel data as RGBA f32.
pub struct OiioImageInput {
    ptr: NonNull<sys::OiioImageInput>,
}

// SAFETY: The handle wraps an OIIO ImageBuf which is safe to send across
// threads. We only expose shared (`&self`) access to the underlying data.
unsafe impl Send for OiioImageInput {}

impl OiioImageInput {
    /// Open an image file and read its pixel data.
    pub fn open(path: &Path) -> Result<Self, OiioError> {
        let path = CString::new(path.to_string_lossy().as_bytes())?;
        // SAFETY: FFI constructor returns owned opaque pointer or null on error.
        let ptr = unsafe { sys::oiio_image_input_open(path.as_ptr()) };
        NonNull::new(ptr)
            .map(|ptr| Self { ptr })
            .ok_or_else(ffi_error)
    }

    /// Image width in pixels.
    pub fn width(&self) -> u32 {
        // SAFETY: `self.ptr` is valid for the life of `self`.
        let v = unsafe { sys::oiio_image_input_width(self.ptr.as_ptr()) };
        v.max(0) as u32
    }

    /// Image height in pixels.
    pub fn height(&self) -> u32 {
        // SAFETY: `self.ptr` is valid for the life of `self`.
        let v = unsafe { sys::oiio_image_input_height(self.ptr.as_ptr()) };
        v.max(0) as u32
    }

    /// Number of channels in the source image (before RGBA conversion).
    pub fn nchannels(&self) -> u32 {
        // SAFETY: `self.ptr` is valid for the life of `self`.
        let v = unsafe { sys::oiio_image_input_nchannels(self.ptr.as_ptr()) };
        v.max(0) as u32
    }

    /// Original bit depth / format of the source image.
    pub fn bit_depth(&self) -> BitDepth {
        // SAFETY: `self.ptr` is valid for the life of `self`.
        let fmt = unsafe { sys::oiio_image_input_format(self.ptr.as_ptr()) };
        match fmt {
            basetype::UINT8 => BitDepth::U8,
            basetype::UINT16 | basetype::INT16 => BitDepth::U16,
            basetype::HALF => BitDepth::F16,
            basetype::FLOAT | basetype::DOUBLE => BitDepth::F32,
            _ => BitDepth::U8,
        }
    }

    /// The color space detected by OIIO from file metadata (`oiio:ColorSpace`).
    ///
    /// Returns `None` if no color space information was found.
    pub fn color_space(&self) -> Option<String> {
        // SAFETY: `self.ptr` is valid for the life of `self`.
        let ptr = unsafe { sys::oiio_image_input_color_space(self.ptr.as_ptr()) };
        if ptr.is_null() {
            return None;
        }
        // SAFETY: FFI contract returns valid NUL-terminated strings.
        let s = unsafe { CStr::from_ptr(ptr) }.to_string_lossy();
        if s.is_empty() { None } else { Some(s.into_owned()) }
    }

    /// Read the full image as RGBA f32 pixels.
    ///
    /// Channels are mapped as follows:
    /// - 1 channel (grayscale): R=G=B=value, A=1
    /// - 3 channels (RGB): A=1
    /// - 4 channels (RGBA): used directly
    /// - >4 channels: first 4 used
    pub fn read_rgba_f32(&self) -> Result<Vec<[f32; 4]>, OiioError> {
        let w = self.width() as usize;
        let h = self.height() as usize;
        let pixel_count = w * h;
        if pixel_count == 0 {
            return Err(OiioError::InvalidArgument("image has zero dimensions"));
        }

        let float_count = pixel_count * 4;
        let mut buf: Vec<[f32; 4]> = vec![[0.0, 0.0, 0.0, 1.0]; pixel_count];

        // SAFETY: buf is a contiguous [f32; 4] array with exactly float_count floats.
        let ok = unsafe {
            sys::oiio_image_input_read_rgba_f32(
                self.ptr.as_ptr(),
                buf.as_mut_ptr().cast::<f32>(),
                float_count as i32,
            )
        };

        if ok == 0 {
            return Err(ffi_error());
        }

        Ok(buf)
    }
}

impl Drop for OiioImageInput {
    fn drop(&mut self) {
        // SAFETY: pointer came from FFI constructor and is owned by this wrapper.
        unsafe { sys::oiio_image_input_destroy(self.ptr.as_ptr()) };
    }
}
