//! Strange Attractor — DSP engine, parameter model and factory presets.
//!
//! This crate is dependency-free so it can run natively (tests, offline
//! rendering) and inside an AudioWorklet as a bare wasm module.

pub mod dsp;
pub mod engine;
pub mod params;
pub mod reverb;
pub mod voice;

pub use engine::Engine;
pub use params::{NUM_PARAMS, P, PARAMS, Patch};
pub mod presets;
pub mod render;
