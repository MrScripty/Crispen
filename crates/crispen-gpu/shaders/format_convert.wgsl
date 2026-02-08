// Convert f32 RGBA pixels to packed f16 RGBA for viewer readback.
//
// Input:  array<vec4<f32>> — linear-light graded output
// Output: array<vec2<u32>> — packed f16 pairs (rg, ba) per pixel
//
// Each vec2<u32> encodes one RGBA pixel as 4 half-float channels (8 bytes).

@group(0) @binding(0) var<storage, read> input: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read_write> output: array<vec2<u32>>;
@group(0) @binding(2) var<uniform> pixel_count: u32;

@compute @workgroup_size(256, 1, 1)
fn convert_f32_to_f16(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if idx >= pixel_count {
        return;
    }

    let pixel = input[idx];
    let rg = pack2x16float(pixel.rg);
    let ba = pack2x16float(pixel.ba);
    output[idx] = vec2<u32>(rg, ba);
}
