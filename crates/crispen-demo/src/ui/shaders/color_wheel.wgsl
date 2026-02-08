// Color wheel fragment shader for DaVinci Resolve-style Lift/Gamma/Gain/Offset wheels.
//
// Draws an outer HSV hue ring, a dark inner circle with a subtle color tint
// from the current cursor position, and a small white cursor dot.

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
const RING_INNER: f32 = 0.35;
// Inner circle radius.
const CIRCLE_R: f32 = 0.33;
// Cursor dot radius (UV space).
const DOT_R: f32 = 0.018;
// Anti-alias width.
const AA: f32 = 0.006;

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
        let hue = (angle + PI) / TWO_PI;
        let rgb = hsv_to_rgb(hue, 1.0, 1.0);
        // Anti-aliased edges on both sides of the ring.
        let outer_alpha = 1.0 - smoothstep(RING_OUTER - AA, RING_OUTER + AA, r);
        let inner_alpha = smoothstep(RING_INNER - AA, RING_INNER + AA, r);
        return vec4(rgb, outer_alpha * inner_alpha);
    }

    // Inner circle: dark background with subtle tint from cursor position.
    let inner_alpha = 1.0 - smoothstep(CIRCLE_R - AA, CIRCLE_R + AA, r);

    // Base dark gray, modulated by master brightness.
    let base_brightness = 0.12 + material.master * 0.04;
    let base = vec3(base_brightness);

    // Subtle color tint derived from cursor position.
    let tint_strength = length(vec2(material.cursor_x, material.cursor_y)) * 0.15;
    let tint_angle = atan2(material.cursor_y, material.cursor_x);
    let tint_hue = (tint_angle + PI) / TWO_PI;
    let tint_color = hsv_to_rgb(tint_hue, 0.6, 0.3);
    let inner_color = mix(base, tint_color, tint_strength);

    // Cursor dot: white circle at cursor position scaled to inner circle.
    let cursor_pos = vec2(material.cursor_x, material.cursor_y) * (CIRCLE_R - DOT_R - AA);
    let dot_dist = length(p - cursor_pos);
    let dot_alpha = 1.0 - smoothstep(DOT_R - AA, DOT_R + AA, dot_dist);

    let color = mix(inner_color, vec3(1.0), dot_alpha);
    return vec4(color, inner_alpha);
}
