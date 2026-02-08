//! Crispen Core — domain layer for color grading.
//!
//! This crate contains all color science, grading math, LUT operations,
//! and scope computation. No GPU or framework dependencies.

pub mod color_management;
pub mod grading;
pub mod image;
pub mod scopes;
pub mod transform;

// Re-exports for convenience.
pub use image::{BitDepth, GradingImage};
pub use transform::evaluate::evaluate_transform;
pub use transform::lut::Lut3D;
pub use transform::params::{ColorManagementConfig, ColorSpaceId, GradingParams};
