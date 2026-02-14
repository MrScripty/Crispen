// Convert f32 linear-light RGBA pixels to packed sRGB u8 for viewer readback.
//
// Input:  array<vec4<f32>> — linear-light graded output
// Output: array<u32>       — packed RGBA8 sRGB (4 bytes per pixel)
//
// Eliminates the CPU-side powf(1/2.4) bottleneck by performing the
// sRGB transfer function on the GPU in parallel.

@group(0) @binding(0) var<storage, read> input: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read_write> output: array<u32>;
@group(0) @binding(2) var<uniform> pixel_count: u32;

/// IEC 61966-2-1 sRGB OETF (linear → sRGB).
fn linear_to_srgb(c: f32) -> f32 {
    let v = clamp(c, 0.0, 1.0);
    if v <= 0.0031308 {
        return v * 12.92;
    }
    return 1.055 * pow(v, 1.0 / 2.4) - 0.055;
}

@compute @workgroup_size(256, 1, 1)
fn convert_linear_to_srgb8(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if idx >= pixel_count {
        return;
    }

    let pixel = input[idx];
    let r = u32(linear_to_srgb(pixel.r) * 255.0 + 0.5);
    let g = u32(linear_to_srgb(pixel.g) * 255.0 + 0.5);
    let b = u32(linear_to_srgb(pixel.b) * 255.0 + 0.5);
    let a = u32(clamp(pixel.a, 0.0, 1.0) * 255.0 + 0.5);

    // Pack as RGBA8 little-endian: R in low byte, A in high byte.
    output[idx] = r | (g << 8u) | (b << 16u) | (a << 24u);
}
