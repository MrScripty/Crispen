// cie.wgsl — CIE xy chromaticity diagram scope.

@group(0) @binding(0) var<storage, read> pixels: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read_write> density: array<atomic<u32>>;
@group(0) @binding(2) var<uniform> pixel_count: u32;
@group(0) @binding(3) var<uniform> resolution: u32;
@group(0) @binding(4) var<storage, read> mask: array<u32>;
@group(0) @binding(5) var<uniform> mask_active: u32;

// sRGB to XYZ matrix rows.
const TO_XYZ_0: vec3<f32> = vec3<f32>(0.4124564, 0.3575761, 0.1804375);
const TO_XYZ_1: vec3<f32> = vec3<f32>(0.2126729, 0.7151522, 0.0721750);
const TO_XYZ_2: vec3<f32> = vec3<f32>(0.0193339, 0.1191920, 0.9503041);

@compute @workgroup_size(256, 1, 1)
fn cie_compute(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x >= pixel_count) { return; }
    if (mask_active != 0u && mask[gid.x] == 0u) { return; }

    let pixel = pixels[gid.x];
    let x_val = dot(TO_XYZ_0, pixel.xyz);
    let y_val = dot(TO_XYZ_1, pixel.xyz);
    let z_val = dot(TO_XYZ_2, pixel.xyz);
    let sum = x_val + y_val + z_val;

    // Skip near-black pixels to avoid division by zero.
    if (sum < 0.0001) { return; }

    let cx = x_val / sum; // CIE x
    let cy = y_val / sum; // CIE y

    // Map chromaticity [0, 0.8] to grid [0, resolution).
    let res_f = f32(resolution);
    let gx = u32(clamp(cx / 0.8 * res_f, 0.0, res_f - 1.0));
    let gy = u32(clamp(cy / 0.8 * res_f, 0.0, res_f - 1.0));

    atomicAdd(&density[gy * resolution + gx], 1u);
}
