use std::ptr::NonNull;

use crate::error::{OcioError, ffi_error};
use crate::sys;

pub struct OcioProcessor {
    ptr: NonNull<sys::OcioProcessor>,
}

pub struct OcioCpuProcessor {
    ptr: NonNull<sys::OcioCpuProcessor>,
}

impl OcioProcessor {
    pub(crate) fn from_raw(raw: *mut sys::OcioProcessor) -> Result<Self, OcioError> {
        NonNull::new(raw)
            .map(|ptr| Self { ptr })
            .ok_or_else(ffi_error)
    }

    pub fn cpu_f32(&self) -> Result<OcioCpuProcessor, OcioError> {
        // SAFETY: `self.ptr` is valid while `self` is alive.
        let raw = unsafe { sys::ocio_processor_get_cpu_f32(self.ptr.as_ptr()) };
        OcioCpuProcessor::from_raw(raw)
    }
}

impl Drop for OcioProcessor {
    fn drop(&mut self) {
        // SAFETY: pointer came from FFI constructor and is owned by this wrapper.
        unsafe { sys::ocio_processor_destroy(self.ptr.as_ptr()) };
    }
}

impl OcioCpuProcessor {
    fn from_raw(raw: *mut sys::OcioCpuProcessor) -> Result<Self, OcioError> {
        NonNull::new(raw)
            .map(|ptr| Self { ptr })
            .ok_or_else(ffi_error)
    }

    pub fn is_noop(&self) -> bool {
        // SAFETY: `self.ptr` is valid while `self` is alive.
        unsafe { sys::ocio_cpu_processor_is_noop(self.ptr.as_ptr()) != 0 }
    }

    pub fn apply_rgba(&self, pixels: &mut [[f32; 4]], width: u32, height: u32) {
        let expected_len = width as usize * height as usize;
        if pixels.len() != expected_len {
            return;
        }

        // SAFETY: pixel slice is contiguous f32 RGBA memory, dimensions validated.
        unsafe {
            sys::ocio_cpu_processor_apply_rgba(
                self.ptr.as_ptr(),
                pixels.as_mut_ptr().cast::<f32>(),
                width as i32,
                height as i32,
            );
        }
    }

    pub fn apply_pixel(&self, rgb: &mut [f32; 3]) {
        // SAFETY: pointer references exactly 3 contiguous f32 values.
        unsafe { sys::ocio_cpu_processor_apply_rgb_pixel(self.ptr.as_ptr(), rgb.as_mut_ptr()) };
    }

    pub fn bake_3d_lut(&self, size: u32) -> Vec<[f32; 4]> {
        if size < 2 {
            return vec![[0.0, 0.0, 0.0, 1.0]];
        }

        let total = size as usize * size as usize * size as usize;
        let mut lut = Vec::with_capacity(total);
        let denom = (size - 1) as f32;

        for z in 0..size {
            for y in 0..size {
                for x in 0..size {
                    let mut px = [x as f32 / denom, y as f32 / denom, z as f32 / denom];
                    self.apply_pixel(&mut px);
                    lut.push([px[0], px[1], px[2], 1.0]);
                }
            }
        }

        lut
    }
}

impl Drop for OcioCpuProcessor {
    fn drop(&mut self) {
        // SAFETY: pointer came from FFI constructor and is owned by this wrapper.
        unsafe { sys::ocio_cpu_processor_destroy(self.ptr.as_ptr()) };
    }
}
