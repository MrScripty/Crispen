//! Crispen GPU — wgpu-based compute pipeline for LUT baking, application, and scopes.
//!
//! This crate owns all GPU resources. No Bevy dependency — it exposes a
//! plain wgpu API that `crispen-bevy` wraps into ECS resources and systems.

pub mod buffers;
pub mod lut_applicator;
pub mod lut_baker;
pub mod pipeline;
pub mod readback;
pub mod scope_dispatch;
pub mod vulkan_interop;
