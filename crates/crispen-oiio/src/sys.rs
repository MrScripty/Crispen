use std::ffi::{c_char, c_int};

#[repr(C)]
pub struct OiioImageInput {
    _private: [u8; 0],
}

unsafe extern "C" {
    pub fn oiio_get_last_error() -> *const c_char;

    pub fn oiio_image_input_open(path: *const c_char) -> *mut OiioImageInput;
    pub fn oiio_image_input_destroy(h: *mut OiioImageInput);

    pub fn oiio_image_input_width(h: *const OiioImageInput) -> c_int;
    pub fn oiio_image_input_height(h: *const OiioImageInput) -> c_int;
    pub fn oiio_image_input_nchannels(h: *const OiioImageInput) -> c_int;
    pub fn oiio_image_input_format(h: *const OiioImageInput) -> c_int;
    pub fn oiio_image_input_color_space(h: *const OiioImageInput) -> *const c_char;

    pub fn oiio_image_input_read_rgba_f32(
        h: *const OiioImageInput,
        buf: *mut f32,
        buf_len: c_int,
    ) -> c_int;
}
