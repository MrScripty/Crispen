//! Scope computation — histogram, waveform, vectorscope, parade, and CIE diagram.

pub mod cie;
pub mod histogram;
pub mod parade;
pub mod vectorscope;
pub mod waveform;

pub use cie::CieData;
pub use histogram::HistogramData;
pub use parade::ParadeData;
pub use vectorscope::VectorscopeData;
pub use waveform::WaveformData;
