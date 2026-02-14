// Color wheel fragment shader for DaVinci Resolve-style Lift/Gamma/Gain/Offset wheels.
//
// Draws an outer HSV hue ring and a neutral inner circle.

#import bevy_ui::ui_vertex_output::UiVertexOutput

struct ColorWheelUniforms {
    cursor_x: f32,
    cursor_y: f32,
    master: f32,
}

@group(1) @binding(0) var<uniform> material: ColorWheelUniforms;

const PI: f32 = 3.14159265;
const TWO_PI: f32 = 6.28318530;

// Outer ring radii (in UV space, where 0.5 = full radius).
const RING_OUTER: f32 = 0.48;
const RING_INNER: f32 = 0.43;
// Inner circle radius (just inside the ring, with a small gap).
const CIRCLE_R: f32 = 0.405;
// Anti-alias width.
const AA: f32 = 0.005;

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> vec3<f32> {
    let c = v * s;
    let hp = h * 6.0;
    let x = c * (1.0 - abs(hp % 2.0 - 1.0));
    let m = v - c;
    var rgb: vec3<f32>;
    if hp < 1.0 {
        rgb = vec3(c, x, 0.0);
    } else if hp < 2.0 {
        rgb = vec3(x, c, 0.0);
    } else if hp < 3.0 {
        rgb = vec3(0.0, c, x);
    } else if hp < 4.0 {
        rgb = vec3(0.0, x, c);
    } else if hp < 5.0 {
        rgb = vec3(x, 0.0, c);
    } else {
        rgb = vec3(c, 0.0, x);
    }
    return rgb + vec3(m);
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let p = in.uv - vec2(0.5);
    let r = length(p);

    // Outside everything: fully transparent.
    if r > RING_OUTER + AA {
        return vec4(0.0, 0.0, 0.0, 0.0);
    }

    // Outer ring: HSV hue wheel.
    if r > RING_INNER - AA {
        let angle = atan2(p.y, p.x);
        let hue = fract(angle / TWO_PI);
        let rgb = hsv_to_rgb(hue, 1.0, 0.98);
        // Anti-aliased edges on both sides of the ring.
        let outer_alpha = 1.0 - smoothstep(RING_OUTER - AA, RING_OUTER + AA, r);
        let inner_alpha = smoothstep(RING_INNER - AA, RING_INNER + AA, r);
        return vec4(rgb, outer_alpha * inner_alpha);
    }

    // Inner circle: static color field (independent of thumb position).
    let inner_alpha = 1.0 - smoothstep(CIRCLE_R - AA, CIRCLE_R + AA, r);
    let radial = clamp(r / CIRCLE_R, 0.0, 1.0);
    let angle = atan2(p.y, p.x);
    let hue = fract(angle / TWO_PI);
    let sat = smoothstep(0.08, 1.0, radial) * 0.9;
    let val = 0.22 + (1.0 - radial) * 0.18;
    var color = hsv_to_rgb(hue, sat, val);

    // Resolve-like faint cross guides.
    let line_w = 0.0035;
    let cross_x = 1.0 - smoothstep(line_w, line_w + AA, abs(p.x));
    let cross_y = 1.0 - smoothstep(line_w, line_w + AA, abs(p.y));
    let cross = max(cross_x, cross_y) * 0.22;
    color = mix(color, vec3(0.78), cross);

    // Slight center lift.
    let center_lift = 1.0 - smoothstep(0.0, 0.11, radial);
    color = mix(color, vec3(0.6), center_lift * 0.2);

    // Keep `master` bound for compatibility without thumb-driven hue shifts.
    color *= 0.92 + (material.master - 0.5) * 0.02;
    return vec4(color, inner_alpha);
}
