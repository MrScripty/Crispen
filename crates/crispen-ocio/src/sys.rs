use std::ffi::{c_char, c_int};

#[repr(C)]
pub struct OcioConfig {
    _private: [u8; 0],
}

#[repr(C)]
pub struct OcioProcessor {
    _private: [u8; 0],
}

#[repr(C)]
pub struct OcioCpuProcessor {
    _private: [u8; 0],
}

unsafe extern "C" {
    pub fn ocio_get_last_error() -> *const c_char;

    pub fn ocio_config_create_from_file(path: *const c_char) -> *mut OcioConfig;
    pub fn ocio_config_create_from_env() -> *mut OcioConfig;
    pub fn ocio_config_create_builtin(uri: *const c_char) -> *mut OcioConfig;
    pub fn ocio_config_destroy(config: *mut OcioConfig);

    pub fn ocio_config_get_num_color_spaces(config: *const OcioConfig) -> c_int;
    pub fn ocio_config_get_color_space_name(
        config: *const OcioConfig,
        index: c_int,
    ) -> *const c_char;
    pub fn ocio_config_get_role(config: *const OcioConfig, role: *const c_char) -> *const c_char;

    pub fn ocio_config_get_num_displays(config: *const OcioConfig) -> c_int;
    pub fn ocio_config_get_display(config: *const OcioConfig, index: c_int) -> *const c_char;
    pub fn ocio_config_get_default_display(config: *const OcioConfig) -> *const c_char;
    pub fn ocio_config_get_num_views(config: *const OcioConfig, display: *const c_char) -> c_int;
    pub fn ocio_config_get_view(
        config: *const OcioConfig,
        display: *const c_char,
        index: c_int,
    ) -> *const c_char;
    pub fn ocio_config_get_default_view(
        config: *const OcioConfig,
        display: *const c_char,
    ) -> *const c_char;

    pub fn ocio_config_get_processor_by_names(
        config: *const OcioConfig,
        src: *const c_char,
        dst: *const c_char,
    ) -> *mut OcioProcessor;
    pub fn ocio_config_get_display_view_processor(
        config: *const OcioConfig,
        src: *const c_char,
        display: *const c_char,
        view: *const c_char,
    ) -> *mut OcioProcessor;
    pub fn ocio_processor_destroy(proc: *mut OcioProcessor);

    pub fn ocio_processor_get_cpu_f32(proc: *const OcioProcessor) -> *mut OcioCpuProcessor;
    pub fn ocio_cpu_processor_destroy(cpu: *mut OcioCpuProcessor);
    pub fn ocio_cpu_processor_apply_rgba(
        cpu: *const OcioCpuProcessor,
        pixels: *mut f32,
        width: c_int,
        height: c_int,
    );
    pub fn ocio_cpu_processor_apply_rgb_pixel(cpu: *const OcioCpuProcessor, pixel: *mut f32);
    pub fn ocio_cpu_processor_is_noop(cpu: *const OcioCpuProcessor) -> c_int;
}
