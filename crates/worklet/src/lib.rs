//! C-ABI bridge between the AudioWorklet processor (JS) and the engine.
//!
//! The module has no imports, so the worklet can instantiate it with an
//! empty import object. All calls happen on the audio thread.

use sa_engine::Engine;
use sa_engine::engine::TELEMETRY_LEN;
use std::cell::UnsafeCell;

struct Global(UnsafeCell<Option<Engine>>);
// SAFETY: the AudioWorklet is single threaded; there is no concurrent access.
unsafe impl Sync for Global {}

static ENGINE: Global = Global(UnsafeCell::new(None));

#[allow(clippy::mut_from_ref)]
fn engine() -> Option<&'static mut Engine> {
    // SAFETY: see `Global`.
    unsafe { (*ENGINE.0.get()).as_mut() }
}

#[unsafe(no_mangle)]
pub extern "C" fn sa_init(sample_rate: f32) {
    // SAFETY: see `Global`.
    unsafe { *ENGINE.0.get() = Some(Engine::new(sample_rate)) };
}

#[unsafe(no_mangle)]
pub extern "C" fn sa_set_param(index: u32, value: f32) {
    if let Some(e) = engine() {
        e.set_param(index as usize, value);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sa_note_on(note: u32, velocity: f32) {
    if let Some(e) = engine() {
        e.note_on(note.min(127) as u8, velocity);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sa_note_off(note: u32) {
    if let Some(e) = engine() {
        e.note_off(note.min(127) as u8);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sa_all_notes_off() {
    if let Some(e) = engine() {
        e.all_notes_off();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sa_panic() {
    if let Some(e) = engine() {
        e.panic();
    }
}

/// Renders `frames` samples; returns a pointer to planar stereo output.
#[unsafe(no_mangle)]
pub extern "C" fn sa_process(frames: u32) -> *const f32 {
    match engine() {
        Some(e) => e.process(frames as usize).as_ptr(),
        None => std::ptr::null(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sa_telemetry() -> *const f32 {
    engine().map_or(std::ptr::null(), |e| e.telemetry().as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn sa_telemetry_len() -> u32 {
    TELEMETRY_LEN as u32
}
