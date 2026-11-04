//! Contour's sound engine. Plain Rust (no plugin types) so every behaviour in the spec can be unit-tested offline.
pub mod band;
pub mod design;
pub mod engine;
pub mod response;

pub use design::{FilterType, BAND_COUNT};
pub use engine::{BandSettings, Engine, GlobalSettings};
