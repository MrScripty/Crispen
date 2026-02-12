// vectorscope.wgsl — Vectorscope scope via YCbCr chromaticity mapping.

@group(0) @binding(0) var<storage, read> pixels: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read_write> density: array<atomic<u32>>;
@group(0) @binding(2) var<uniform> pixel_count: u32;
@group(0) @binding(3) var<uniform> resolution: u32;
@group(0) @binding(4) var<storage, read> mask: array<u32>;
@group(0) @binding(5) var<uniform> mask_active: u32;

@compute @workgroup_size(256, 1, 1)
fn vectorscope(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x >= pixel_count) { return; }
    if (mask_active != 0u && mask[gid.x] == 0u) { return; }

    let pixel = pixels[gid.x];
    // BT.709 luma
    let y = 0.2126729 * pixel.x + 0.7151522 * pixel.y + 0.0721750 * pixel.z;
    // Cb, Cr scaled to roughly [-0.5, 0.5]
    let cb = (pixel.z - y) * 0.5389;
    let cr = (pixel.x - y) * 0.6350;

    // Map [-0.5, 0.5] to [0, resolution)
    let res_f = f32(resolution);
    let gx = u32(clamp((cb + 0.5) * res_f, 0.0, res_f - 1.0));
    let gy = u32(clamp((cr + 0.5) * res_f, 0.0, res_f - 1.0));

    atomicAdd(&density[gy * resolution + gx], 1u);
}
