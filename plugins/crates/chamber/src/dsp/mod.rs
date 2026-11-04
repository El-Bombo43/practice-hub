//! Chamber's DSP: three reverb tanks (Plate, Hall, Room) and the engine around them. Plain Rust, no plugin types.
pub mod engine;
pub mod tank;
pub mod util;

pub use engine::{Engine, Settings};
