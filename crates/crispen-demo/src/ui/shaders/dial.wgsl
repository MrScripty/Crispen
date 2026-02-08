// Dial / rotary knob fragment shader for parameter controls.
//
// Draws a dark circular knob with a 270° arc track (gap at bottom),
// an indicator dot at the current value angle, and tick marks at
// min / center / max positions.

#import bevy_ui::ui_vertex_output::UiVertexOutput

struct DialUniforms {
    // Normalized value 0..1 mapped to arc sweep.
    value_norm: f32,
    // 1.0 when the user is actively dragging.
    is_active: f32,
}

@group(1) @binding(0) var<uniform> material: DialUniforms;

const PI: f32 = 3.14159265;
const TWO_PI: f32 = 6.28318530;

// Arc sweep in radians (270°).
const SWEEP: f32 = 4.71238898; // 270 * PI / 180
// Start angle: 135° (7-o'clock position), measured CCW from +X.
const ARC_START: f32 = 2.35619449; // 135 * PI / 180

// Geometry (UV space, 0.5 = full radius).
const OUTER_R: f32 = 0.48;
const INNER_R: f32 = 0.38;
const TRACK_R: f32 = 0.43;       // Radius of the arc track center line.
const TRACK_HALF_W: f32 = 0.012; // Half-width of the arc track line.
const DOT_R: f32 = 0.035;        // Indicator dot radius.
const TICK_LEN: f32 = 0.06;      // Tick mark length.
const TICK_HALF_W: f32 = 0.004;  // Tick mark half-width.
const AA: f32 = 0.008;           // Anti-alias width.

// Colors.
const BG_COLOR: vec3<f32> = vec3(0.18, 0.18, 0.18);
const BORDER_COLOR: vec3<f32> = vec3(0.30, 0.30, 0.30);
const TRACK_COLOR: vec3<f32> = vec3(0.30, 0.30, 0.30);
const TICK_COLOR: vec3<f32> = vec3(0.40, 0.40, 0.40);
const INDICATOR_COLOR: vec3<f32> = vec3(0.85, 0.85, 0.85);
const ACTIVE_COLOR: vec3<f32> = vec3(0.95, 0.55, 0.094);

/// Map an angle to the 0..1 arc parameter.
/// `angle` is measured CCW from +X in [0, TWO_PI).
/// Returns < 0 or > 1 if the angle is outside the arc.
fn angle_to_arc_param(angle: f32) -> f32 {
    // Shift so ARC_START becomes 0.
    var a = angle - ARC_START;
    if a < -PI { a += TWO_PI; }
    if a > PI { a -= TWO_PI; }
    // The arc proceeds clockwise (negative direction in math coords)
    // so we negate and normalize.
    return -a / SWEEP;
}

/// Convert a normalized 0..1 value to the angle on the arc.
fn value_to_angle(v: f32) -> f32 {
    return ARC_START - v * SWEEP;
}

/// Test whether `angle` (0..TWO_PI) falls within the arc sweep.
fn in_arc(angle: f32) -> bool {
    return angle_to_arc_param(angle) >= -0.01 && angle_to_arc_param(angle) <= 1.01;
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let p = in.uv - vec2(0.5);
    let r = length(p);

    // Outside the knob: transparent.
    if r > OUTER_R + AA {
        return vec4(0.0, 0.0, 0.0, 0.0);
    }

    // Angle from center (0..TWO_PI, with 0 = +X, CCW positive).
    let raw_angle = atan2(p.y, p.x);
    let angle = select(raw_angle + TWO_PI, raw_angle, raw_angle >= 0.0);

    // Start with the background fill.
    let bg_alpha = 1.0 - smoothstep(OUTER_R - AA, OUTER_R + AA, r);
    var color = BG_COLOR;

    // Border ring at the outer edge.
    let border_inner = OUTER_R - 0.015;
    if r > border_inner - AA {
        let ring_blend = smoothstep(border_inner - AA, border_inner + AA, r);
        color = mix(color, BORDER_COLOR, ring_blend * 0.6);
    }

    // Arc track: thin line at TRACK_R within the arc sweep.
    let track_dist = abs(r - TRACK_R);
    if track_dist < TRACK_HALF_W + AA && in_arc(angle) {
        let track_alpha = 1.0 - smoothstep(TRACK_HALF_W - AA, TRACK_HALF_W + AA, track_dist);
        color = mix(color, TRACK_COLOR, track_alpha);
    }

    // Tick marks at 0%, 50%, 100% of the arc.
    for (var i = 0u; i < 3u; i++) {
        let tick_val = f32(i) * 0.5;
        let tick_angle = value_to_angle(tick_val);
        let tick_dir = vec2(cos(tick_angle), sin(tick_angle));

        // Project p onto tick direction to get along-tick and cross-tick distances.
        let along = dot(p, tick_dir);
        let cross = abs(dot(p, vec2(-tick_dir.y, tick_dir.x)));

        let tick_start = TRACK_R - TICK_LEN * 0.5;
        let tick_end = TRACK_R + TICK_LEN * 0.5;

        if along > tick_start - AA && along < tick_end + AA && cross < TICK_HALF_W + AA {
            let along_alpha = smoothstep(tick_start - AA, tick_start + AA, along)
                            * (1.0 - smoothstep(tick_end - AA, tick_end + AA, along));
            let cross_alpha = 1.0 - smoothstep(TICK_HALF_W - AA, TICK_HALF_W + AA, cross);
            color = mix(color, TICK_COLOR, along_alpha * cross_alpha * 0.7);
        }
    }

    // Value indicator dot.
    let val_angle = value_to_angle(material.value_norm);
    let dot_center = vec2(cos(val_angle), sin(val_angle)) * TRACK_R;
    let dot_dist = length(p - dot_center);
    if dot_dist < DOT_R + AA {
        let dot_alpha = 1.0 - smoothstep(DOT_R - AA, DOT_R + AA, dot_dist);
        let ind_color = select(INDICATOR_COLOR, ACTIVE_COLOR, material.is_active > 0.5);
        color = mix(color, ind_color, dot_alpha);
    }

    return vec4(color, bg_alpha);
}
