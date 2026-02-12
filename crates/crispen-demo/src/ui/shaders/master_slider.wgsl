// Horizontal master-level slider for primary color wheel channels.
//
// Draws a thin horizontal track with center tick, value indicator,
// and subtle directional fill.

#import bevy_ui::ui_vertex_output::UiVertexOutput

struct MasterSliderUniforms {
    value_norm: f32,
    center_norm: f32,
    is_active: f32,
}

@group(1) @binding(0) var<uniform> material: MasterSliderUniforms;

const AA: f32 = 0.012;

fn rounded_rect_sdf(p: vec2<f32>, half_size: vec2<f32>, radius: f32) -> f32 {
    let d = abs(p) - half_size + radius;
    return length(max(d, vec2(0.0))) + min(max(d.x, d.y), 0.0) - radius;
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let p = uv - vec2(0.5);

    // Track shape: rounded rectangle filling most of the node.
    let half_size = vec2<f32>(0.48, 0.34);
    let d = rounded_rect_sdf(p, half_size, 0.18);
    let shape_alpha = 1.0 - smoothstep(-AA, AA, d);

    if shape_alpha < 0.001 {
        discard;
    }

    // Track background with subtle vertical gradient (inset look).
    let bg = mix(vec3<f32>(0.11, 0.11, 0.11), vec3<f32>(0.16, 0.16, 0.16), uv.y);

    // Center tick at default position.
    let center_x = material.center_norm;
    let center_dist = abs(uv.x - center_x);
    let center_tick = (1.0 - smoothstep(0.002, 0.006, center_dist)) * 0.5;

    // Directional fill between center and value.
    let value_x = material.value_norm;
    let fill_lo = min(center_x, value_x);
    let fill_hi = max(center_x, value_x);
    let in_fill = step(fill_lo, uv.x) * (1.0 - step(fill_hi, uv.x));
    let fill_strength = 0.12 + material.is_active * 0.06;

    // Value indicator line.
    let ind_dist = abs(uv.x - value_x);
    let indicator = 1.0 - smoothstep(0.004, 0.014, ind_dist);
    let ind_color = mix(
        vec3<f32>(0.78, 0.78, 0.78),
        vec3<f32>(0.95, 0.55, 0.094),
        material.is_active,
    );

    // Compose layers.
    var color = bg;
    color = mix(color, vec3<f32>(0.24, 0.24, 0.24), in_fill * fill_strength);
    color = mix(color, vec3<f32>(0.32, 0.32, 0.32), center_tick);
    color = mix(color, ind_color, indicator);

    return vec4<f32>(color, shape_alpha);
}
