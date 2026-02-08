// midtone_detail.wgsl — Separable Gaussian blur + unsharp mask for local contrast.
//
// Dispatched twice: pass=0 for horizontal blur, pass=1 for vertical blur + combine.
// Only applied to luminance channel; chroma is preserved.

@group(0) @binding(0) var<storage, read> input_img: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read_write> output_img: array<vec4<f32>>;
@group(0) @binding(2) var<uniform> width: u32;
@group(0) @binding(3) var<uniform> height: u32;
@group(0) @binding(4) var<uniform> strength: f32;
@group(0) @binding(5) var<uniform> pass: u32;

const LUMA: vec3<f32> = vec3<f32>(0.2126729, 0.7151522, 0.0721750);
const RADIUS: i32 = 8;
const TILE_SIZE: u32 = 256u;
const SHARED_SIZE: u32 = 272u; // TILE_SIZE + 2 * RADIUS

var<workgroup> shared_luma: array<f32, 272>;

// Gaussian weights for radius 8 (sigma ~3.0), normalized.
fn gauss_weight(d: i32) -> f32 {
    let sigma = 3.0;
    let fd = f32(d);
    return exp(-0.5 * fd * fd / (sigma * sigma));
}

@compute @workgroup_size(256, 1, 1)
fn midtone_detail(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
    @builtin(workgroup_id) wid: vec3<u32>,
) {
    let total_pixels = width * height;

    if (pass == 0u) {
        // Horizontal blur pass. Each workgroup processes one row of tiles.
        let row = wid.y;
        if (row >= height) { return; }
        let tile_start = wid.x * TILE_SIZE;
        let local_col = lid.x;
        let global_col = tile_start + local_col;

        // Load tile + halo into shared memory.
        let halo_idx = i32(tile_start) + i32(local_col) - RADIUS;
        let clamped_col = clamp(halo_idx, 0, i32(width) - 1);
        let pixel_idx = row * width + u32(clamped_col);
        if (pixel_idx < total_pixels) {
            shared_luma[local_col] = dot(input_img[pixel_idx].xyz, LUMA);
        }
        // Load right halo.
        if (local_col < u32(RADIUS * 2)) {
            let extra_col = i32(tile_start) + i32(TILE_SIZE) + i32(local_col) - RADIUS;
            let ec = clamp(extra_col, 0, i32(width) - 1);
            let ei = row * width + u32(ec);
            if (ei < total_pixels) {
                shared_luma[TILE_SIZE + local_col] = dot(input_img[ei].xyz, LUMA);
            }
        }
        workgroupBarrier();

        if (global_col >= width) { return; }

        // Convolve.
        var sum = 0.0;
        var weight_sum = 0.0;
        for (var d = -RADIUS; d <= RADIUS; d = d + 1) {
            let w = gauss_weight(d);
            let si = i32(local_col) + RADIUS + d;
            sum = sum + shared_luma[u32(si)] * w;
            weight_sum = weight_sum + w;
        }
        let blurred_luma = sum / weight_sum;

        // Write blurred luma to output; store original RGB for pass 1.
        let out_idx = row * width + global_col;
        let orig = input_img[out_idx];
        output_img[out_idx] = vec4<f32>(orig.xyz, blurred_luma);
    } else {
        // Vertical blur + combine pass.
        let col = wid.y;
        if (col >= width) { return; }
        let tile_start = wid.x * TILE_SIZE;
        let local_row = lid.x;
        let global_row = tile_start + local_row;

        // Load vertical tile (read blurred luma from .w channel written by pass 0).
        let halo_row = i32(tile_start) + i32(local_row) - RADIUS;
        let cr = u32(clamp(halo_row, 0, i32(height) - 1));
        let pi = cr * width + col;
        if (pi < total_pixels) {
            shared_luma[local_row] = output_img[pi].w;
        }
        if (local_row < u32(RADIUS * 2)) {
            let extra_row = i32(tile_start) + i32(TILE_SIZE) + i32(local_row) - RADIUS;
            let er = u32(clamp(extra_row, 0, i32(height) - 1));
            let ei = er * width + col;
            if (ei < total_pixels) {
                shared_luma[TILE_SIZE + local_row] = output_img[ei].w;
            }
        }
        workgroupBarrier();

        if (global_row >= height) { return; }

        var sum = 0.0;
        var weight_sum = 0.0;
        for (var d = -RADIUS; d <= RADIUS; d = d + 1) {
            let w = gauss_weight(d);
            sum = sum + shared_luma[u32(i32(local_row) + RADIUS + d)] * w;
            weight_sum = weight_sum + w;
        }
        let blurred = sum / weight_sum;

        let out_idx = global_row * width + col;
        let orig = input_img[out_idx];
        let orig_luma = dot(orig.xyz, LUMA);
        let detail = orig_luma - blurred;
        // Apply detail to luminance only, preserving chroma ratio.
        let enhanced_luma = orig_luma + strength * detail;
        let scale = select(enhanced_luma / max(orig_luma, 0.0001), 1.0, orig_luma < 0.0001);
        output_img[out_idx] = vec4<f32>(orig.xyz * scale, orig.w);
    }
}
