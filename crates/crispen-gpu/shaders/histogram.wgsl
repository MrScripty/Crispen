// histogram.wgsl — Compute RGB + luminance histogram (256 bins x 4 channels).

@group(0) @binding(0) var<storage, read> pixels: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read_write> bins: array<atomic<u32>>;
@group(0) @binding(2) var<uniform> pixel_count: u32;
@group(0) @binding(3) var<storage, read> mask: array<u32>;
@group(0) @binding(4) var<uniform> mask_active: u32;

// Layout: bins[0..255] = R, bins[256..511] = G, bins[512..767] = B, bins[768..1023] = Luma.
var<workgroup> local_bins: array<atomic<u32>, 1024>;

const LUMA: vec3<f32> = vec3<f32>(0.2126729, 0.7151522, 0.0721750);

@compute @workgroup_size(256, 1, 1)
fn histogram(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
) {
    // Zero local bins (each thread zeros 4 entries).
    for (var i = lid.x; i < 1024u; i = i + 256u) {
        atomicStore(&local_bins[i], 0u);
    }
    workgroupBarrier();

    let thread_id = gid.x;
    if (thread_id < pixel_count) {
        if (mask_active != 0u && mask[thread_id] == 0u) {
            // Skip pixels outside the scope mask.
        } else {
        let pixel = pixels[thread_id];
        let r_bin = min(u32(clamp(pixel.x, 0.0, 1.0) * 255.0), 255u);
        let g_bin = min(u32(clamp(pixel.y, 0.0, 1.0) * 255.0), 255u);
        let b_bin = min(u32(clamp(pixel.z, 0.0, 1.0) * 255.0), 255u);
        let luma = dot(pixel.xyz, LUMA);
        let l_bin = min(u32(clamp(luma, 0.0, 1.0) * 255.0), 255u);

        atomicAdd(&local_bins[r_bin], 1u);
        atomicAdd(&local_bins[256u + g_bin], 1u);
        atomicAdd(&local_bins[512u + b_bin], 1u);
        atomicAdd(&local_bins[768u + l_bin], 1u);
        }
    }

    workgroupBarrier();

    // Merge local bins to global.
    for (var i = lid.x; i < 1024u; i = i + 256u) {
        let val = atomicLoad(&local_bins[i]);
        if (val > 0u) {
            atomicAdd(&bins[i], val);
        }
    }
}
