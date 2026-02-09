//! GPU integration tests. Requires a real wgpu device.
//!
//! Run with: `cargo test -p crispen-gpu`

use std::sync::Arc;
use std::sync::{Mutex, OnceLock};

use crispen_core::image::{BitDepth, GradingImage};
use crispen_core::transform::params::GradingParams;
use crispen_gpu::GpuGradingPipeline;

/// Create a test wgpu device. Panics if no adapter is available.
fn create_test_device() -> (Arc<wgpu::Device>, Arc<wgpu::Queue>) {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        ..Default::default()
    }))
    .expect("No GPU adapter found — GPU tests require a GPU");

    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("crispen_test_device"),
        required_features: crispen_gpu::required_features(),
        required_limits: adapter.limits(),
        ..Default::default()
    }))
    .expect("Failed to create test device");

    (Arc::new(device), Arc::new(queue))
}

fn gpu_test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Create a small test gradient image (4x4).
fn create_test_gradient(width: u32, height: u32) -> GradingImage {
    let mut pixels = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        for x in 0..width {
            let r = x as f32 / (width - 1) as f32;
            let g = y as f32 / (height - 1) as f32;
            let b = 0.5;
            pixels.push([r, g, b, 1.0]);
        }
    }
    GradingImage {
        width,
        height,
        pixels,
        source_bit_depth: BitDepth::F32,
    }
}

#[test]
fn test_gpu_lut_bake_identity() {
    let _lock = gpu_test_lock().lock().expect("gpu test lock poisoned");
    let (device, queue) = create_test_device();
    let mut pipeline = GpuGradingPipeline::new(device.clone(), queue.clone());

    // Default params = identity transform (when input_space == working_space == output_space
    // are both linear).
    let mut params = GradingParams::default();
    // Force all spaces to LinearSrgb so no transfer function or matrix is applied.
    params.color_management.input_space = crispen_core::transform::params::ColorSpaceId::LinearSrgb;
    params.color_management.working_space =
        crispen_core::transform::params::ColorSpaceId::LinearSrgb;
    params.color_management.output_space =
        crispen_core::transform::params::ColorSpaceId::LinearSrgb;

    let lut_size = 17u32; // Small for fast test.
    pipeline.bake_lut(&params, lut_size);

    // Verify that applying the identity LUT to an image is a no-op.
    let image = create_test_gradient(4, 4);
    let source = pipeline.upload_image(&image);
    pipeline.apply_lut(&source);
    let result = pipeline
        .download_current_output()
        .expect("output should exist");

    // Compare each pixel.
    let mut max_error: f32 = 0.0;
    for (i, (src, dst)) in image.pixels.iter().zip(result.pixels.iter()).enumerate() {
        for c in 0..3 {
            let err = (src[c] - dst[c]).abs();
            max_error = max_error.max(err);
            assert!(
                err < 0.02,
                "Pixel {i} channel {c}: src={} dst={} err={err}",
                src[c],
                dst[c],
            );
        }
    }
    eprintln!("Identity LUT max error: {max_error}");
}

#[test]
fn test_apply_lut_preserves_alpha() {
    let _lock = gpu_test_lock().lock().expect("gpu test lock poisoned");
    let (device, queue) = create_test_device();
    let mut pipeline = GpuGradingPipeline::new(device.clone(), queue.clone());

    let mut params = GradingParams::default();
    params.color_management.input_space = crispen_core::transform::params::ColorSpaceId::LinearSrgb;
    params.color_management.working_space =
        crispen_core::transform::params::ColorSpaceId::LinearSrgb;
    params.color_management.output_space =
        crispen_core::transform::params::ColorSpaceId::LinearSrgb;

    pipeline.bake_lut(&params, 17);

    // Create image with varying alpha.
    let mut image = create_test_gradient(4, 4);
    for (i, px) in image.pixels.iter_mut().enumerate() {
        px[3] = i as f32 / 15.0; // Alpha from 0 to 1.
    }

    let source = pipeline.upload_image(&image);
    pipeline.apply_lut(&source);
    let result = pipeline
        .download_current_output()
        .expect("output should exist");

    for (i, (src, dst)) in image.pixels.iter().zip(result.pixels.iter()).enumerate() {
        let alpha_err = (src[3] - dst[3]).abs();
        assert!(
            alpha_err < 1e-4,
            "Pixel {i}: alpha src={} dst={} err={alpha_err}",
            src[3],
            dst[3],
        );
    }
}

#[test]
fn test_histogram_bins_sum_to_pixel_count() {
    let _lock = gpu_test_lock().lock().expect("gpu test lock poisoned");
    let (device, queue) = create_test_device();
    let mut pipeline = GpuGradingPipeline::new(device.clone(), queue.clone());

    let image = create_test_gradient(16, 16);
    let source = pipeline.upload_image(&image);

    // Bake identity LUT and apply first.
    let mut params = GradingParams::default();
    params.color_management.input_space = crispen_core::transform::params::ColorSpaceId::LinearSrgb;
    params.color_management.working_space =
        crispen_core::transform::params::ColorSpaceId::LinearSrgb;
    params.color_management.output_space =
        crispen_core::transform::params::ColorSpaceId::LinearSrgb;
    let frame = pipeline.submit_frame(&source, &params, 17);
    let results = frame
        .scopes
        .expect("submit_frame should include scope readback");

    let pixel_count = 16u32 * 16;

    // Each channel's histogram bins should sum to pixel_count.
    for (ch, bins) in results.histogram.bins.iter().enumerate() {
        let sum: u32 = bins.iter().sum();
        assert_eq!(
            sum, pixel_count,
            "Histogram channel {ch} sum {sum} != pixel_count {pixel_count}",
        );
    }
}

#[test]
fn test_bake_lut_workgroup_coverage() {
    let _lock = gpu_test_lock().lock().expect("gpu test lock poisoned");
    let (device, queue) = create_test_device();
    let mut pipeline = GpuGradingPipeline::new(device.clone(), queue.clone());

    // Use size=33 which is not a multiple of workgroup size 8.
    let mut params = GradingParams::default();
    params.color_management.input_space = crispen_core::transform::params::ColorSpaceId::LinearSrgb;
    params.color_management.working_space =
        crispen_core::transform::params::ColorSpaceId::LinearSrgb;
    params.color_management.output_space =
        crispen_core::transform::params::ColorSpaceId::LinearSrgb;
    // Set gain > 1 to produce non-zero output everywhere.
    params.gain = [1.5, 1.5, 1.5, 1.0];

    let lut_size = 33u32;
    pipeline.bake_lut(&params, lut_size);

    // Apply to a small image and verify the output is not all zeros.
    let image = create_test_gradient(4, 4);
    let source = pipeline.upload_image(&image);
    pipeline.apply_lut(&source);
    let result = pipeline
        .download_current_output()
        .expect("output should exist");

    // Verify that at least some pixels were actually modified (gain > 1).
    let mut any_different = false;
    for (src, dst) in image.pixels.iter().zip(result.pixels.iter()) {
        for c in 0..3 {
            if (src[c] - dst[c]).abs() > 0.01 {
                any_different = true;
                break;
            }
        }
    }
    assert!(
        any_different,
        "Gain 1.5 should produce visibly different output"
    );
}
