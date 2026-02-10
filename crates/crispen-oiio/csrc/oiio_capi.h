#pragma once

#ifdef __cplusplus
extern "C" {
#endif

typedef struct OiioImageInput OiioImageInput;

// Error handling
const char * oiio_get_last_error(void);

// Open an image file for reading. Returns owned handle or NULL on error.
OiioImageInput * oiio_image_input_open(const char * path);

// Spec queries (handle must be non-NULL).
int oiio_image_input_width(const OiioImageInput * h);
int oiio_image_input_height(const OiioImageInput * h);
int oiio_image_input_nchannels(const OiioImageInput * h);

// Returns the OIIO TypeDesc basetype as an int (UINT8=1, INT8=2, ..., HALF=10, FLOAT=11, ...).
int oiio_image_input_format(const OiioImageInput * h);

// Returns the detected color space ("oiio:ColorSpace" attribute) or NULL.
const char * oiio_image_input_color_space(const OiioImageInput * h);

// Read entire image as RGBA f32 into caller-provided buffer.
// buf_len is the total number of floats (must be >= width * height * 4).
// Returns 1 on success, 0 on error (check oiio_get_last_error).
int oiio_image_input_read_rgba_f32(const OiioImageInput * h, float * buf, int buf_len);

// Destroy handle and free resources.
void oiio_image_input_destroy(OiioImageInput * h);

#ifdef __cplusplus
}
#endif
