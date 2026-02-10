use crispen_ocio::OcioConfig;

/// Try to load a test config. Returns `None` when no config is available
/// (e.g. OCIO 2.1 has no built-in configs and `OCIO` env var is unset).
fn load_test_config() -> Option<OcioConfig> {
    let uris = [
        "ocio://studio-config-latest",
        "studio-config-v4.0.0_aces-v2.0_ocio-v2.5",
        "studio-config-v2.2.0_aces-v1.3_ocio-v2.4",
        "ocio://default",
    ];

    for uri in uris {
        if let Ok(config) = OcioConfig::builtin(uri) {
            return Some(config);
        }
    }

    // Fall back to OCIO env var if set.
    OcioConfig::from_env().ok()
}

fn pick_same_space(config: &OcioConfig) -> String {
    if let Some(name) = config.role("default") {
        return name;
    }

    config
        .color_space_names()
        .into_iter()
        .next()
        .expect("config should contain at least one color space")
}

fn pick_roundtrip_spaces(config: &OcioConfig) -> (String, String) {
    let names = config.color_space_names();
    let scene_linear = config
        .role("scene_linear")
        .unwrap_or_else(|| pick_same_space(config));

    let mut src_candidates = Vec::new();
    if let Some(default_role) = config.role("default") {
        src_candidates.push(default_role);
    }
    for name in names.iter().take(24) {
        src_candidates.push(name.clone());
    }

    src_candidates.retain(|src| !src.eq_ignore_ascii_case(&scene_linear));
    src_candidates.dedup_by(|a, b| a.eq_ignore_ascii_case(b));

    for src in src_candidates {
        if config.processor(&src, &scene_linear).is_ok()
            && config.processor(&scene_linear, &src).is_ok()
        {
            return (src, scene_linear.clone());
        }
    }

    panic!("failed to find a reversible source/scene_linear processor pair");
}

fn assert_close3(actual: [f32; 3], expected: [f32; 3], tol: f32) {
    for i in 0..3 {
        let diff = (actual[i] - expected[i]).abs();
        assert!(
            diff <= tol,
            "channel {} mismatch: got {}, expected {}, diff {} > {}",
            i,
            actual[i],
            expected[i],
            diff,
            tol
        );
    }
}

#[test]
fn builtin_config_loads_and_has_color_spaces() {
    let Some(config) = load_test_config() else {
        eprintln!("skipping: no OCIO config available (needs OCIO 2.2+ or OCIO env var)");
        return;
    };
    let spaces = config.color_space_names();
    assert!(
        !spaces.is_empty(),
        "builtin config returned no color spaces"
    );
}

#[test]
fn roundtrip_via_scene_linear_is_stable() {
    let Some(config) = load_test_config() else {
        eprintln!("skipping: no OCIO config available (needs OCIO 2.2+ or OCIO env var)");
        return;
    };
    let (src, scene_linear) = pick_roundtrip_spaces(&config);

    let to_working = config
        .processor(&src, &scene_linear)
        .and_then(|p| p.cpu_f32())
        .expect("source -> scene linear processor should be available");
    let to_srgb = config
        .processor(&scene_linear, &src)
        .and_then(|p| p.cpu_f32())
        .expect("scene linear -> source processor should be available");

    let samples = [
        [0.0_f32, 0.0, 0.0],
        [0.18_f32, 0.18, 0.18],
        [0.5_f32, 0.2, 0.8],
        [0.9_f32, 0.4, 0.1],
    ];

    for expected in samples {
        let mut px = expected;
        to_working.apply_pixel(&mut px);
        to_srgb.apply_pixel(&mut px);
        assert_close3(px, expected, 0.001);
    }
}

#[test]
fn lut_bake_has_expected_size_and_identity_corners() {
    let Some(config) = load_test_config() else {
        eprintln!("skipping: no OCIO config available (needs OCIO 2.2+ or OCIO env var)");
        return;
    };
    let same_space = pick_same_space(&config);
    let cpu = config
        .processor(&same_space, &same_space)
        .and_then(|p| p.cpu_f32())
        .expect("same-space processor should be available");

    let size = 17_u32;
    let lut = cpu.bake_3d_lut(size);
    assert_eq!(lut.len(), size as usize * size as usize * size as usize);

    assert_close3([lut[0][0], lut[0][1], lut[0][2]], [0.0, 0.0, 0.0], 0.0001);

    let mid = (size / 2) as usize;
    let mid_index = mid * size as usize * size as usize + mid * size as usize + mid;
    assert_close3(
        [lut[mid_index][0], lut[mid_index][1], lut[mid_index][2]],
        [0.5, 0.5, 0.5],
        0.0001,
    );

    let last = lut.len() - 1;
    assert_close3(
        [lut[last][0], lut[last][1], lut[last][2]],
        [1.0, 1.0, 1.0],
        0.0001,
    );
}

#[test]
fn default_display_and_view_are_non_empty() {
    let Some(config) = load_test_config() else {
        eprintln!("skipping: no OCIO config available (needs OCIO 2.2+ or OCIO env var)");
        return;
    };
    let display = config.default_display();
    assert!(!display.is_empty(), "default display should not be empty");

    let view = config.default_view(&display);
    assert!(!view.is_empty(), "default view should not be empty");
}

#[test]
fn same_space_processor_reports_noop() {
    let Some(config) = load_test_config() else {
        eprintln!("skipping: no OCIO config available (needs OCIO 2.2+ or OCIO env var)");
        return;
    };
    let same_space = pick_same_space(&config);
    let cpu = config
        .processor(&same_space, &same_space)
        .and_then(|p| p.cpu_f32())
        .expect("same-space processor should be available");
    assert!(cpu.is_noop(), "same-space processor should be no-op");
}
