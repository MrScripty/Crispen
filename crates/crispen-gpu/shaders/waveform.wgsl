// waveform.wgsl — Waveform scope via atomic scatter.
// Output layout: 3 channels (R,G,B) x image_width x waveform_height.

@group(0) @binding(0) var<storage, read> pixels: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read_write> waveform: array<atomic<u32>>;
@group(0) @binding(2) var<uniform> image_width: u32;
@group(0) @binding(3) var<uniform> image_height: u32;
@group(0) @binding(4) var<uniform> waveform_height: u32;

@compute @workgroup_size(256, 1, 1)
fn waveform_compute(@builtin(global_invocation_id) gid: vec3<u32>) {
    let total_pixels = image_width * image_height;
    if (gid.x >= total_pixels) { return; }

    let x = gid.x % image_width;
    let pixel = pixels[gid.x];
    let h = waveform_height;
    let hf = f32(h - 1u);

    let r_bin = min(u32(clamp(pixel.x, 0.0, 1.0) * hf), h - 1u);
    let g_bin = min(u32(clamp(pixel.y, 0.0, 1.0) * hf), h - 1u);
    let b_bin = min(u32(clamp(pixel.z, 0.0, 1.0) * hf), h - 1u);

    // Buffer layout: channel * (width * height) + x * height + bin
    let stride = image_width * h;
    atomicAdd(&waveform[0u * stride + x * h + r_bin], 1u);
    atomicAdd(&waveform[1u * stride + x * h + g_bin], 1u);
    atomicAdd(&waveform[2u * stride + x * h + b_bin], 1u);
}
