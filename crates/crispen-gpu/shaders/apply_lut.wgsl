// apply_lut.wgsl — Apply a baked 3D LUT to a source image via trilinear sampling.

@group(0) @binding(0) var<storage, read> source: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read_write> output: array<vec4<f32>>;
@group(0) @binding(2) var lut_texture: texture_3d<f32>;
@group(0) @binding(3) var lut_sampler: sampler;
@group(0) @binding(4) var<uniform> dimensions: vec2<u32>;

@compute @workgroup_size(16, 16, 1)
fn apply_lut(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x >= dimensions.x || gid.y >= dimensions.y) { return; }

    let idx = gid.y * dimensions.x + gid.x;
    let pixel = source[idx];

    // Clamp RGB to [0,1] for LUT lookup.
    let rgb = clamp(pixel.xyz, vec3<f32>(0.0), vec3<f32>(1.0));
    let graded = textureSampleLevel(lut_texture, lut_sampler, rgb, 0.0);

    // Preserve alpha from source.
    output[idx] = vec4<f32>(graded.xyz, pixel.w);
}
