// bake_lut.wgsl — Bake GradingParams into a 3D LUT.
// Mirrors evaluate_transform() from crispen-core exactly.

struct GradingParamsGpu {
    lift: vec4<f32>,
    gamma: vec4<f32>,
    gain: vec4<f32>,
    offset_val: vec4<f32>,
    temperature: f32,
    tint: f32,
    contrast: f32,
    pivot: f32,
    shadows: f32,
    highlights: f32,
    saturation: f32,
    hue_deg: f32,
    luma_mix: f32,
    input_space: u32,
    working_space: u32,
    output_space: u32,
};

@group(0) @binding(0) var<storage, read_write> lut_data: array<vec4<f32>>;
@group(0) @binding(1) var<uniform> params: GradingParamsGpu;
@group(0) @binding(2) var<uniform> lut_size: u32;
@group(0) @binding(3) var curve_hue_vs_hue: texture_1d<f32>;
@group(0) @binding(4) var curve_hue_vs_sat: texture_1d<f32>;
@group(0) @binding(5) var curve_lum_vs_sat: texture_1d<f32>;
@group(0) @binding(6) var curve_sat_vs_sat: texture_1d<f32>;
@group(0) @binding(7) var curve_sampler: sampler;

// ── Color space matrices (to/from CIE XYZ D65) ─────────────────────

// sRGB / Rec.709 → XYZ
const SRGB_TO_XYZ_0: vec3<f32> = vec3<f32>(0.4124564, 0.3575761, 0.1804375);
const SRGB_TO_XYZ_1: vec3<f32> = vec3<f32>(0.2126729, 0.7151522, 0.0721750);
const SRGB_TO_XYZ_2: vec3<f32> = vec3<f32>(0.0193339, 0.1191920, 0.9503041);
// XYZ → sRGB / Rec.709
const XYZ_TO_SRGB_0: vec3<f32> = vec3<f32>( 3.2404542, -1.5371385, -0.4985314);
const XYZ_TO_SRGB_1: vec3<f32> = vec3<f32>(-0.9692660,  1.8760108,  0.0415560);
const XYZ_TO_SRGB_2: vec3<f32> = vec3<f32>( 0.0556434, -0.2040259,  1.0572252);

// ACEScg (AP1) → XYZ D65
const AP1_TO_XYZ_0: vec3<f32> = vec3<f32>( 0.6624542, 0.1340042, 0.1561877);
const AP1_TO_XYZ_1: vec3<f32> = vec3<f32>( 0.2722287, 0.6740818, 0.0536895);
const AP1_TO_XYZ_2: vec3<f32> = vec3<f32>(-0.0055746, 0.0040607, 1.0103391);
// XYZ D65 → ACEScg (AP1)
const XYZ_TO_AP1_0: vec3<f32> = vec3<f32>( 1.6410234, -0.3248033, -0.2364247);
const XYZ_TO_AP1_1: vec3<f32> = vec3<f32>(-0.6636629,  1.6153316,  0.0167563);
const XYZ_TO_AP1_2: vec3<f32> = vec3<f32>( 0.0117219, -0.0082844,  0.9883949);

// ACES 2065-1 (AP0) → XYZ D65
const AP0_TO_XYZ_0: vec3<f32> = vec3<f32>(0.9525524, 0.0000000, 0.0000937);
const AP0_TO_XYZ_1: vec3<f32> = vec3<f32>(0.3439664, 0.7281661, -0.0721325);
const AP0_TO_XYZ_2: vec3<f32> = vec3<f32>(0.0000000, 0.0000000, 1.0088252);
// XYZ D65 → ACES AP0
const XYZ_TO_AP0_0: vec3<f32> = vec3<f32>( 1.0498110, 0.0000000, -0.0000974);
const XYZ_TO_AP0_1: vec3<f32> = vec3<f32>(-0.4959030, 1.3733131,  0.0982400);
const XYZ_TO_AP0_2: vec3<f32> = vec3<f32>( 0.0000000, 0.0000000,  0.9912520);

// DCI-P3 → XYZ
const P3_TO_XYZ_0: vec3<f32> = vec3<f32>(0.4865709, 0.2656677, 0.1982173);
const P3_TO_XYZ_1: vec3<f32> = vec3<f32>(0.2289746, 0.6917385, 0.0792869);
const P3_TO_XYZ_2: vec3<f32> = vec3<f32>(0.0000000, 0.0451134, 1.0439444);
// XYZ → DCI-P3
const XYZ_TO_P3_0: vec3<f32> = vec3<f32>( 2.4934969, -0.9313836, -0.4027108);
const XYZ_TO_P3_1: vec3<f32> = vec3<f32>(-0.8294890,  1.7626641,  0.0236247);
const XYZ_TO_P3_2: vec3<f32> = vec3<f32>( 0.0358458, -0.0761724,  0.9568845);

// Rec.2020 → XYZ
const R2020_TO_XYZ_0: vec3<f32> = vec3<f32>(0.6369580, 0.1446169, 0.1688810);
const R2020_TO_XYZ_1: vec3<f32> = vec3<f32>(0.2627002, 0.6779981, 0.0593017);
const R2020_TO_XYZ_2: vec3<f32> = vec3<f32>(0.0000000, 0.0280727, 1.0609851);
// XYZ → Rec.2020
const XYZ_TO_R2020_0: vec3<f32> = vec3<f32>( 1.7166512, -0.3556708, -0.2533663);
const XYZ_TO_R2020_1: vec3<f32> = vec3<f32>(-0.6666844,  1.6164812,  0.0157685);
const XYZ_TO_R2020_2: vec3<f32> = vec3<f32>( 0.0176399, -0.0427706,  0.9421031);

// BT.709 luma coefficients.
const LUMA_709: vec3<f32> = vec3<f32>(0.2126729, 0.7151522, 0.0721750);

// ── Matrix multiply helper ──────────────────────────────────────────

fn mat3_mul(r0: vec3<f32>, r1: vec3<f32>, r2: vec3<f32>, v: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(dot(r0, v), dot(r1, v), dot(r2, v));
}

// ── Transfer functions ──────────────────────────────────────────────

fn srgb_to_linear(v: f32) -> f32 {
    if (v <= 0.04045) { return v / 12.92; }
    return pow((v + 0.055) / 1.055, 2.4);
}

fn linear_to_srgb(v: f32) -> f32 {
    if (v <= 0.0031308) { return v * 12.92; }
    return 1.055 * pow(v, 1.0 / 2.4) - 0.055;
}

fn logc3_to_linear(v: f32) -> f32 {
    // ARRI LogC3 EI 800
    let cut_encoded = 0.1496582;
    if (v > cut_encoded) {
        return (pow(10.0, (v - 0.385537) / 0.247190) - 0.052272) / 5.555556;
    }
    return (v - 0.092809) / 5.367655;
}

fn linear_to_logc3(v: f32) -> f32 {
    let cut_linear = 0.01059148;
    if (v > cut_linear) {
        return 0.247190 * log(5.555556 * v + 0.052272) / log(10.0) + 0.385537;
    }
    return 5.367655 * v + 0.092809;
}

fn logc4_to_linear(v: f32) -> f32 {
    // ARRI LogC4 (ALEXA 35)
    let a = 2231.826309;
    let b = 64.0;
    let c = 0.0740718;
    let d = 1.0;
    let t = 0.01011722;
    let cut_encoded = c * log2(a * t + b) + d;
    if (v >= cut_encoded) {
        return (pow(2.0, (v - d) / c) - b) / a;
    }
    let lin_slope = a * c * log(2.0) / (a * t + b);
    return (v - cut_encoded) / lin_slope + t;
}

fn linear_to_logc4(v: f32) -> f32 {
    let a = 2231.826309;
    let b = 64.0;
    let c = 0.0740718;
    let d = 1.0;
    let t = 0.01011722;
    if (v >= t) {
        return c * log2(a * v + b) + d;
    }
    let cut_encoded = c * log2(a * t + b) + d;
    let lin_slope = a * c * log(2.0) / (a * t + b);
    return cut_encoded + lin_slope * (v - t);
}

fn slog3_to_linear(v: f32) -> f32 {
    let cut_encoded = 171.2102946929 / 1023.0;
    if (v >= cut_encoded) {
        return (pow(10.0, (v * 1023.0 - 420.0) / 261.5) * (0.18 + 0.01)) - 0.01;
    }
    return (v * 1023.0 - 95.0) * 0.01125000 / (cut_encoded * 1023.0 - 95.0);
}

fn linear_to_slog3(v: f32) -> f32 {
    let cut_linear = 0.01125;
    if (v >= cut_linear) {
        return (420.0 + log(((v + 0.01) / (0.18 + 0.01))) / log(10.0) * 261.5) / 1023.0;
    }
    return (v * (171.2102946929 - 95.0) / 0.01125000 + 95.0) / 1023.0;
}

fn redlog3g10_to_linear(v: f32) -> f32 {
    let a = 0.224282;
    let b = 155.975327;
    let c = 0.01;
    if (v < 0.0) { return v / a; }
    return (pow(10.0, v / a) - 1.0) / b - c;
}

fn linear_to_redlog3g10(v: f32) -> f32 {
    let a = 0.224282;
    let b = 155.975327;
    let c = 0.01;
    if (v < -c) { return v * a; }
    return a * log(b * (v + c) + 1.0) / log(10.0);
}

fn vlog_to_linear(v: f32) -> f32 {
    let d = 0.598206;
    let c = 0.241514;
    let b = 0.00873;
    if (v < d) {
        return (v - 0.125) / 5.6;
    }
    return pow(10.0, (v - d) / c) - b;
}

fn linear_to_vlog(v: f32) -> f32 {
    let d = 0.598206;
    let c = 0.241514;
    let b = 0.00873;
    let cut_linear = 0.01;
    if (v < cut_linear) {
        return 5.6 * v + 0.125;
    }
    return c * log(v + b) / log(10.0) + d;
}

// ACEScc/ACEScct use AP1 primaries with log encoding.
fn acescc_to_linear(v: f32) -> f32 {
    if (v < -0.3013699) {
        return (pow(2.0, v * 17.52 - 9.72) - 0.000030518) * 2.0;
    }
    return pow(2.0, v * 17.52 - 9.72);
}

fn linear_to_acescc(v: f32) -> f32 {
    if (v <= 0.0) { return -0.3584475; }
    if (v < 0.000030518) {
        return (log2(0.000030518 + v * 0.5) + 9.72) / 17.52;
    }
    return (log2(v) + 9.72) / 17.52;
}

fn acescct_to_linear(v: f32) -> f32 {
    let cut = 0.155251141552511;
    if (v <= cut) {
        return (v - 0.0729055341958355) / 10.5402377416545;
    }
    return pow(2.0, v * 17.52 - 9.72);
}

fn linear_to_acescct(v: f32) -> f32 {
    let cut_linear = 0.0078125;
    if (v <= cut_linear) {
        return 10.5402377416545 * v + 0.0729055341958355;
    }
    return (log2(v) + 9.72) / 17.52;
}

// ── Linearize / encode per color space ──────────────────────────────

fn linearize_channel(v: f32, space: u32) -> f32 {
    switch (space) {
        case 2u: { return acescc_to_linear(v); }
        case 3u: { return acescct_to_linear(v); }
        case 4u: { return srgb_to_linear(v); }
        case 8u: { return logc3_to_linear(v); }
        case 9u: { return logc4_to_linear(v); }
        case 10u: { return slog3_to_linear(v); }
        case 11u: { return redlog3g10_to_linear(v); }
        case 12u: { return vlog_to_linear(v); }
        default: { return v; } // Linear spaces (0,1,5,6,7)
    }
}

fn encode_channel(v: f32, space: u32) -> f32 {
    switch (space) {
        case 2u: { return linear_to_acescc(v); }
        case 3u: { return linear_to_acescct(v); }
        case 4u: { return linear_to_srgb(v); }
        case 8u: { return linear_to_logc3(v); }
        case 9u: { return linear_to_logc4(v); }
        case 10u: { return linear_to_slog3(v); }
        case 11u: { return linear_to_redlog3g10(v); }
        case 12u: { return linear_to_vlog(v); }
        default: { return v; }
    }
}

fn linearize(v: vec3<f32>, space: u32) -> vec3<f32> {
    return vec3<f32>(
        linearize_channel(v.x, space),
        linearize_channel(v.y, space),
        linearize_channel(v.z, space),
    );
}

fn encode(v: vec3<f32>, space: u32) -> vec3<f32> {
    return vec3<f32>(
        encode_channel(v.x, space),
        encode_channel(v.y, space),
        encode_channel(v.z, space),
    );
}

// ── Gamut conversion via XYZ ────────────────────────────────────────

fn gamut_to_xyz(v: vec3<f32>, space: u32) -> vec3<f32> {
    switch (space) {
        case 0u: { return mat3_mul(AP0_TO_XYZ_0, AP0_TO_XYZ_1, AP0_TO_XYZ_2, v); }
        case 1u, 2u, 3u: { return mat3_mul(AP1_TO_XYZ_0, AP1_TO_XYZ_1, AP1_TO_XYZ_2, v); }
        case 6u: { return mat3_mul(R2020_TO_XYZ_0, R2020_TO_XYZ_1, R2020_TO_XYZ_2, v); }
        case 7u: { return mat3_mul(P3_TO_XYZ_0, P3_TO_XYZ_1, P3_TO_XYZ_2, v); }
        default: { return mat3_mul(SRGB_TO_XYZ_0, SRGB_TO_XYZ_1, SRGB_TO_XYZ_2, v); }
    }
}

fn xyz_to_gamut(v: vec3<f32>, space: u32) -> vec3<f32> {
    switch (space) {
        case 0u: { return mat3_mul(XYZ_TO_AP0_0, XYZ_TO_AP0_1, XYZ_TO_AP0_2, v); }
        case 1u, 2u, 3u: { return mat3_mul(XYZ_TO_AP1_0, XYZ_TO_AP1_1, XYZ_TO_AP1_2, v); }
        case 6u: { return mat3_mul(XYZ_TO_R2020_0, XYZ_TO_R2020_1, XYZ_TO_R2020_2, v); }
        case 7u: { return mat3_mul(XYZ_TO_P3_0, XYZ_TO_P3_1, XYZ_TO_P3_2, v); }
        default: { return mat3_mul(XYZ_TO_SRGB_0, XYZ_TO_SRGB_1, XYZ_TO_SRGB_2, v); }
    }
}

fn input_transform(v: vec3<f32>, from_space: u32, to_space: u32) -> vec3<f32> {
    var lin = linearize(v, from_space);
    if (from_space == to_space) { return lin; }
    let xyz = gamut_to_xyz(lin, from_space);
    return xyz_to_gamut(xyz, to_space);
}

fn output_transform(v: vec3<f32>, from_space: u32, to_space: u32) -> vec3<f32> {
    var out = v;
    if (from_space != to_space) {
        let xyz = gamut_to_xyz(out, from_space);
        out = xyz_to_gamut(xyz, to_space);
    }
    return encode(out, to_space);
}

// ── White balance (simplified Planckian shift via Bradford) ──────────

// Bradford cone response matrix.
const BRAD_0: vec3<f32> = vec3<f32>( 0.8951,  0.2664, -0.1614);
const BRAD_1: vec3<f32> = vec3<f32>(-0.7502,  1.7135,  0.0367);
const BRAD_2: vec3<f32> = vec3<f32>( 0.0389, -0.0685,  1.0296);
const BRAD_INV_0: vec3<f32> = vec3<f32>( 0.9870, -0.1471,  0.1600);
const BRAD_INV_1: vec3<f32> = vec3<f32>( 0.4323,  0.5184,  0.0493);
const BRAD_INV_2: vec3<f32> = vec3<f32>(-0.0085,  0.0400,  0.9685);

fn planckian_xy(kelvin: f32) -> vec2<f32> {
    // CIE daylight locus approximation.
    let t = kelvin;
    let t2 = t * t;
    let t3 = t2 * t;
    var x: f32;
    if (t <= 7000.0) {
        x = -4.607e9 / t3 + 2.9678e6 / t2 + 0.09911e3 / t + 0.244063;
    } else {
        x = -2.0064e9 / t3 + 1.9018e6 / t2 + 0.24748e3 / t + 0.237040;
    }
    let y = -3.0 * x * x + 2.87 * x - 0.275;
    return vec2<f32>(x, y);
}

fn xy_to_xyz(xy: vec2<f32>) -> vec3<f32> {
    return vec3<f32>(xy.x / xy.y, 1.0, (1.0 - xy.x - xy.y) / xy.y);
}

fn white_balance(v: vec3<f32>, temp: f32, tint_val: f32) -> vec3<f32> {
    if (temp == 0.0 && tint_val == 0.0) { return v; }

    // D65 reference (6500K)
    let src_xy = planckian_xy(6500.0);
    let dst_xy = planckian_xy(6500.0 + temp * 100.0);
    // Apply tint as green-magenta shift on y axis.
    let dst_xy_tinted = vec2<f32>(dst_xy.x, dst_xy.y + tint_val * 0.02);

    let src_wp = xy_to_xyz(src_xy);
    let dst_wp = xy_to_xyz(dst_xy_tinted);

    let src_cone = mat3_mul(BRAD_0, BRAD_1, BRAD_2, src_wp);
    let dst_cone = mat3_mul(BRAD_0, BRAD_1, BRAD_2, dst_wp);

    let scale = dst_cone / src_cone;

    // Apply: Bradford → scale → inverse Bradford
    let cone = mat3_mul(BRAD_0, BRAD_1, BRAD_2, v);
    let adapted = cone * scale;
    return mat3_mul(BRAD_INV_0, BRAD_INV_1, BRAD_INV_2, adapted);
}

// ── CDL (lift/gamma/gain/offset) ────────────────────────────────────

fn apply_cdl(v: vec3<f32>) -> vec3<f32> {
    var r = v;
    let lift = params.lift;
    let gm = params.gamma;
    let gn = params.gain;
    let ov = params.offset_val;
    // Per-channel: out = pow(max(in * gain_ch * gain_master + off_ch + off_master, 0), 1 / (gamma_ch * gamma_master)) + lift_ch + lift_master
    for (var c = 0u; c < 3u; c = c + 1u) {
        let g = gn[c] * gn[3u];
        let o = ov[c] + ov[3u];
        let gamma_val = gm[c] * gm[3u];
        let l = lift[c] + lift[3u];
        r[c] = pow(max(r[c] * g + o, 0.0), 1.0 / max(gamma_val, 0.0001)) + l;
    }
    return r;
}

// ── Contrast with pivot ─────────────────────────────────────────────

fn apply_contrast(v: vec3<f32>) -> vec3<f32> {
    let c = params.contrast;
    let p = params.pivot;
    if (c == 1.0) { return v; }
    return vec3<f32>(
        p * pow(max(v.x / p, 0.0001), c),
        p * pow(max(v.y / p, 0.0001), c),
        p * pow(max(v.z / p, 0.0001), c),
    );
}

// ── Shadows / Highlights ────────────────────────────────────────────

fn apply_shadows_highlights(v: vec3<f32>) -> vec3<f32> {
    let sh = params.shadows;
    let hi = params.highlights;
    if (sh == 0.0 && hi == 0.0) { return v; }

    let luma = dot(v, LUMA_709);
    // Shadow weight: high near black, falls off toward mid.
    let shadow_w = 1.0 - smoothstep(0.0, 0.5, luma);
    // Highlight weight: rises toward white from mid.
    let highlight_w = smoothstep(0.5, 1.0, luma);

    return v + v * (sh * shadow_w + hi * highlight_w);
}

// ── Saturation + Hue rotation + Luma mix ────────────────────────────

fn apply_saturation_hue(v: vec3<f32>) -> vec3<f32> {
    let sat = params.saturation;
    let hue_d = params.hue_deg;
    let lm = params.luma_mix;

    let luma = dot(v, LUMA_709);
    // Saturation: lerp toward monochrome.
    var r = mix(vec3<f32>(luma, luma, luma), v, sat);

    // Hue rotation around the (1,1,1) axis via Rodrigues' formula.
    if (hue_d != 0.0) {
        let rad = hue_d * 3.14159265358979 / 180.0;
        let cos_a = cos(rad);
        let sin_a = sin(rad);
        let k = vec3<f32>(0.57735027, 0.57735027, 0.57735027); // (1,1,1)/sqrt(3)
        let d = dot(k, r);
        let cross_kr = cross(k, r);
        r = r * cos_a + cross_kr * sin_a + k * d * (1.0 - cos_a);
    }

    // Luma mix: blend between chroma-preserving and luma-preserving versions.
    if (lm != 0.0) {
        let new_luma = dot(r, LUMA_709);
        let luma_preserved = r * (luma / max(new_luma, 0.0001));
        r = mix(r, luma_preserved, lm);
    }

    return r;
}

// ── Curves (sample 1D textures) ─────────────────────────────────────

fn rgb_to_hsl(rgb: vec3<f32>) -> vec3<f32> {
    let mx = max(rgb.x, max(rgb.y, rgb.z));
    let mn = min(rgb.x, min(rgb.y, rgb.z));
    let l = (mx + mn) * 0.5;
    if (mx == mn) { return vec3<f32>(0.0, 0.0, l); }
    let d = mx - mn;
    let s = select(d / (2.0 - mx - mn), d / (mx + mn), l < 0.5);
    var h: f32;
    if (rgb.x == mx) {
        h = (rgb.y - rgb.z) / d + select(0.0, 6.0, rgb.y < rgb.z);
    } else if (rgb.y == mx) {
        h = (rgb.z - rgb.x) / d + 2.0;
    } else {
        h = (rgb.x - rgb.y) / d + 4.0;
    }
    h = h / 6.0;
    return vec3<f32>(h, s, l);
}

fn apply_curves(v: vec3<f32>) -> vec3<f32> {
    let hsl = rgb_to_hsl(clamp(v, vec3<f32>(0.0), vec3<f32>(1.0)));
    let h = hsl.x;
    var sat_mult = 1.0;
    // Hue-vs-hue: offset hue.
    let hue_offset = textureSampleLevel(curve_hue_vs_hue, curve_sampler, h, 0.0).r;
    // Hue-vs-sat: multiply saturation by hue-dependent factor.
    let hvs_factor = textureSampleLevel(curve_hue_vs_sat, curve_sampler, h, 0.0).r;
    sat_mult = sat_mult * hvs_factor;
    // Lum-vs-sat: multiply saturation by luminance-dependent factor.
    let lvs_factor = textureSampleLevel(curve_lum_vs_sat, curve_sampler, hsl.z, 0.0).r;
    sat_mult = sat_mult * lvs_factor;
    // Sat-vs-sat: multiply saturation by saturation-dependent factor.
    let svs_factor = textureSampleLevel(curve_sat_vs_sat, curve_sampler, hsl.y, 0.0).r;
    sat_mult = sat_mult * svs_factor;

    // Apply hue offset (rotate hue).
    let luma = dot(v, LUMA_709);
    var r = v;
    if (hue_offset != 0.0) {
        let rad = hue_offset * 2.0 * 3.14159265358979;
        let cos_a = cos(rad);
        let sin_a = sin(rad);
        let k = vec3<f32>(0.57735027, 0.57735027, 0.57735027);
        let d = dot(k, r);
        let cross_kr = cross(k, r);
        r = r * cos_a + cross_kr * sin_a + k * d * (1.0 - cos_a);
    }
    // Apply saturation adjustment from curves.
    if (sat_mult != 1.0) {
        let l = dot(r, LUMA_709);
        r = mix(vec3<f32>(l, l, l), r, sat_mult);
    }
    return r;
}

// ── Main entry point ────────────────────────────────────────────────

@compute @workgroup_size(8, 8, 4)
fn bake_lut(@builtin(global_invocation_id) gid: vec3<u32>) {
    let size = lut_size;
    if (gid.x >= size || gid.y >= size || gid.z >= size) { return; }

    let idx = gid.z * size * size + gid.y * size + gid.x;
    let r = f32(gid.x) / f32(size - 1u);
    let g = f32(gid.y) / f32(size - 1u);
    let b = f32(gid.z) / f32(size - 1u);
    var c = vec3<f32>(r, g, b);

    // Full grading chain — mirrors evaluate_transform() exactly.
    c = input_transform(c, params.input_space, params.working_space);
    c = white_balance(c, params.temperature, params.tint);
    c = apply_cdl(c);
    c = apply_contrast(c);
    c = apply_shadows_highlights(c);
    c = apply_saturation_hue(c);
    c = apply_curves(c);
    c = output_transform(c, params.working_space, params.output_space);

    lut_data[idx] = vec4<f32>(c, 1.0);
}
