//! IPC message contracts between the Bevy backend and the Svelte UI.
//!
//! These enums define the complete set of messages exchanged over the
//! CEF IPC bridge. They follow the `#[serde(tag = "type", content = "data")]`
//! pattern from Pentimento for consistent serialization.

use base64::Engine;
use serde::{Deserialize, Serialize};

use crispen_core::scopes::{CieData, HistogramData, VectorscopeData, WaveformData};
use crispen_core::transform::params::GradingParams;

/// Messages from the Bevy backend to the Svelte UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum BevyToUi {
    /// Initial state sync when UI connects.
    Initialize {
        /// Current grading parameters.
        params: GradingParams,
    },

    /// Grading parameters were updated (backend → UI sync).
    ParamsUpdated {
        /// Updated grading parameters.
        params: GradingParams,
    },

    /// Scope analysis data with binary-encoded arrays for fast serialization.
    ///
    /// Large `u32` density arrays are encoded as base64 little-endian binary
    /// strings instead of JSON number arrays. This reduces serialization time
    /// from ~100ms to ~2ms for typical scope data (~1.5M values).
    ScopeData {
        histogram: BinaryHistogram,
        waveform: BinaryWaveform,
        vectorscope: BinaryDensityGrid,
        cie: BinaryDensityGrid,
    },

    /// A new image was loaded successfully.
    ImageLoaded {
        /// File path of the loaded image.
        path: String,
        /// Image width in pixels.
        width: u32,
        /// Image height in pixels.
        height: u32,
        /// Human-readable bit depth description.
        bit_depth: String,
    },

    /// An error occurred in the backend.
    Error {
        /// Error description.
        message: String,
    },
}

/// Messages from the Svelte UI to the Bevy backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum UiToBevy {
    /// Request a fresh snapshot of backend state after UI connects.
    RequestState,

    /// Set new grading parameters (UI → backend).
    SetParams {
        /// The new grading parameters.
        params: GradingParams,
    },

    /// Request automatic white balance.
    AutoBalance,

    /// Reset all grading to identity (no-op) defaults.
    ResetGrade,

    /// Load a new source image.
    LoadImage {
        /// File path to the image.
        path: String,
    },

    /// Load a 3D LUT from a file.
    LoadLut {
        /// File path to the .cube LUT file.
        path: String,
        /// Which LUT slot to load into.
        slot: String,
    },

    /// Export the current grading as a 3D LUT.
    ExportLut {
        /// File path for the exported .cube file.
        path: String,
        /// LUT grid size (e.g., 33 or 65).
        size: u32,
    },

    /// Toggle scope visibility.
    ToggleScope {
        /// The scope type identifier.
        scope_type: String,
        /// Whether to show or hide the scope.
        visible: bool,
    },

    /// CEF dirty signal — triggers framebuffer recapture.
    UiDirty,

    /// Dockview panel layout changed.
    LayoutUpdate {
        /// Current panel positions and sizes.
        regions: Vec<LayoutRegion>,
    },

    /// Persist the dockview layout to disk.
    SaveLayout {
        /// Serialised dockview JSON.
        layout_json: String,
    },
}

// ── Binary-encoded scope types for fast JSON serialization ───────

/// Histogram with base64-encoded bin arrays.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryHistogram {
    /// Base64 LE u32 arrays for `[R, G, B, Luma]` channels (256 bins each).
    pub bins: [String; 4],
    pub peak: u32,
}

/// Waveform with base64-encoded channel data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryWaveform {
    pub width: u32,
    pub height: u32,
    /// Base64 LE u32 arrays for R, G, B channels.
    pub data: [String; 3],
}

/// Square density grid with base64-encoded data (vectorscope or CIE).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryDensityGrid {
    pub resolution: u32,
    /// Base64 LE u32 density values (length = resolution²).
    pub density: String,
}

/// Encode a `u32` slice as a base64 string of its little-endian byte representation.
fn encode_u32_slice(data: &[u32]) -> String {
    let bytes: &[u8] = bytemuck::cast_slice(data);
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// Build a `BevyToUi::ScopeData` message from core scope types.
pub fn scope_data_to_binary(
    histogram: &HistogramData,
    waveform: &WaveformData,
    vectorscope: &VectorscopeData,
    cie: &CieData,
) -> BevyToUi {
    BevyToUi::ScopeData {
        histogram: BinaryHistogram {
            bins: std::array::from_fn(|i| encode_u32_slice(&histogram.bins[i])),
            peak: histogram.peak,
        },
        waveform: BinaryWaveform {
            width: waveform.width,
            height: waveform.height,
            data: std::array::from_fn(|i| encode_u32_slice(&waveform.data[i])),
        },
        vectorscope: BinaryDensityGrid {
            resolution: vectorscope.resolution,
            density: encode_u32_slice(&vectorscope.density),
        },
        cie: BinaryDensityGrid {
            resolution: cie.resolution,
            density: encode_u32_slice(&cie.density),
        },
    }
}

/// A rectangular region where Bevy should render a widget.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutRegion {
    /// Unique panel identifier (e.g. `"viewer"`, `"color-wheels"`).
    pub id: String,
    /// X position in CSS pixels.
    pub x: f32,
    /// Y position in CSS pixels.
    pub y: f32,
    /// Width in CSS pixels.
    pub width: f32,
    /// Height in CSS pixels.
    pub height: f32,
    /// Whether the panel is visible.
    pub visible: bool,
}
