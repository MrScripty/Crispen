#pragma once

#ifdef __cplusplus
extern "C" {
#endif

typedef struct OcioConfig OcioConfig;
typedef struct OcioProcessor OcioProcessor;
typedef struct OcioCpuProcessor OcioCpuProcessor;

// Error handling
const char * ocio_get_last_error(void);

// Config lifecycle
OcioConfig * ocio_config_create_from_file(const char * path);
OcioConfig * ocio_config_create_from_env(void);
OcioConfig * ocio_config_create_builtin(const char * uri);
void ocio_config_destroy(OcioConfig * config);

// Config queries
int ocio_config_get_num_color_spaces(const OcioConfig * config);
const char * ocio_config_get_color_space_name(const OcioConfig * config, int index);
const char * ocio_config_get_role(const OcioConfig * config, const char * role);

// Display/view queries
int ocio_config_get_num_displays(const OcioConfig * config);
const char * ocio_config_get_display(const OcioConfig * config, int index);
const char * ocio_config_get_default_display(const OcioConfig * config);
int ocio_config_get_num_views(const OcioConfig * config, const char * display);
const char * ocio_config_get_view(const OcioConfig * config, const char * display, int index);
const char * ocio_config_get_default_view(const OcioConfig * config, const char * display);

// Processor creation
OcioProcessor * ocio_config_get_processor_by_names(
    const OcioConfig * config,
    const char * src,
    const char * dst
);
OcioProcessor * ocio_config_get_display_view_processor(
    const OcioConfig * config,
    const char * src,
    const char * display,
    const char * view
);
void ocio_processor_destroy(OcioProcessor * proc);

// CPU processor
OcioCpuProcessor * ocio_processor_get_cpu_f32(const OcioProcessor * proc);
void ocio_cpu_processor_destroy(OcioCpuProcessor * cpu);
void ocio_cpu_processor_apply_rgba(
    const OcioCpuProcessor * cpu,
    float * pixels,
    int width,
    int height
);
void ocio_cpu_processor_apply_rgb_pixel(const OcioCpuProcessor * cpu, float * pixel);
int ocio_cpu_processor_is_noop(const OcioCpuProcessor * cpu);

#ifdef __cplusplus
}
#endif
