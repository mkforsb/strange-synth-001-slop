//! UI state: one signal per parameter (so a knob only re-renders when its
//! own value changes), locks, notes, and the actions that mutate them.

use dioxus::prelude::*;
use sa_engine::params::{Curve, NUM_PARAMS, P, PARAMS, Patch, default_patch};
use sa_engine::presets::{Preset, factory};

use crate::audio::{self, AudioStatus};

const STORAGE_KEY: &str = "strange-attractor/v1";

#[derive(Clone, Copy)]
pub struct AppState {
    pub params: [Signal<f32>; NUM_PARAMS],
    pub locks: [Signal<bool>; NUM_PARAMS],
    pub presets: CopyValue<Vec<Preset>>,
    pub preset: Signal<usize>,
    pub modified: Signal<bool>,
    pub mutation: Signal<f32>,
    pub rnd_gen: Signal<u32>,
    /// Bitmask of sounding notes (0..127).
    pub notes: Signal<u128>,
    /// Notes whose key was released while Hold is on.
    pub latched: Signal<u128>,
    pub hold: Signal<bool>,
    pub octave: Signal<i32>,
    pub volume: Signal<f32>,
    pub status: Signal<AudioStatus>,
    pub midi: Signal<String>,
}

struct Saved {
    patch: Patch,
    locks: [bool; NUM_PARAMS],
    preset: usize,
    modified: bool,
}

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

fn load_saved() -> Option<Saved> {
    let text = storage()?.get_item(STORAGE_KEY).ok()??;
    let mut patch = default_patch();
    let mut locks = [false; NUM_PARAMS];
    let (mut preset, mut modified) = (0, false);
    for part in text.split('|') {
        let (k, v) = part.split_once(':')?;
        match k {
            // Saved by parameter key so reordering parameters doesn't scramble patches.
            "p" => {
                for kv in v.split(',') {
                    let (key, val) = kv.split_once('=')?;
                    if let Some(i) = PARAMS.iter().position(|p| p.key == key) {
                        patch[i] = val.parse::<f32>().ok()?.clamp(0.0, 1.0);
                    }
                }
            }
            "l" => {
                for key in v.split(',').filter(|s| !s.is_empty()) {
                    if let Some(i) = PARAMS.iter().position(|p| p.key == key) {
                        locks[i] = true;
                    }
                }
            }
            "i" => preset = v.parse().ok()?,
            "m" => modified = v == "1",
            _ => {}
        }
    }
    Some(Saved { patch, locks, preset, modified })
}

impl AppState {
    pub fn new() -> Self {
        let presets = factory();
        let saved = load_saved();
        let (patch, locks, preset, modified) = match &saved {
            Some(s) if s.preset < presets.len() => (s.patch, s.locks, s.preset, s.modified),
            _ => (presets[0].patch, [false; NUM_PARAMS], 0, false),
        };
        AppState {
            params: std::array::from_fn(|i| Signal::new(patch[i])),
            locks: std::array::from_fn(|i| Signal::new(locks[i])),
            presets: CopyValue::new(presets),
            preset: Signal::new(preset),
            modified: Signal::new(modified),
            mutation: Signal::new(0.5),
            rnd_gen: Signal::new(0),
            notes: Signal::new(0),
            latched: Signal::new(0),
            hold: Signal::new(false),
            octave: Signal::new(4),
            volume: Signal::new(0.8),
            status: Signal::new(AudioStatus::Off),
            midi: Signal::new(String::new()),
        }
    }

    pub fn patch(&self) -> Patch {
        std::array::from_fn(|i| *self.params[i].peek())
    }

    pub fn save(&self) {
        let p = PARAMS
            .iter()
            .zip(self.params.iter())
            .map(|(info, s)| format!("{}={:.5}", info.key, s.peek()))
            .collect::<Vec<_>>()
            .join(",");
        let l = PARAMS
            .iter()
            .zip(self.locks.iter())
            .filter(|(_, s)| *s.peek())
            .map(|(info, _)| info.key)
            .collect::<Vec<_>>()
            .join(",");
        let text = format!("p:{p}|l:{l}|i:{}|m:{}", self.preset.peek(), if *self.modified.peek() { 1 } else { 0 });
        if let Some(s) = storage() {
            let _ = s.set_item(STORAGE_KEY, &text);
        }
    }

    pub fn set_param(mut self, i: usize, norm: f32) {
        let norm = norm.clamp(0.0, 1.0);
        if *self.params[i].peek() == norm {
            return;
        }
        self.params[i].set(norm);
        audio::set_param(i, norm);
        if !*self.modified.peek() {
            self.modified.set(true);
        }
    }

    pub fn toggle_lock(mut self, i: usize) {
        let v = !*self.locks[i].peek();
        self.locks[i].set(v);
        self.save();
    }

    pub fn unlock_all(mut self) {
        for l in self.locks.iter_mut() {
            if *l.peek() {
                l.set(false);
            }
        }
        self.save();
    }

    pub fn locked_count(&self) -> usize {
        self.locks.iter().filter(|l| *l.read()).count()
    }

    fn apply_patch(mut self, patch: &Patch) {
        for (i, &v) in patch.iter().enumerate() {
            if *self.params[i].peek() != v {
                self.params[i].set(v);
            }
        }
        audio::send_patch(patch);
    }

    pub fn load_preset(mut self, idx: usize) {
        let n = self.presets.read().len();
        let idx = idx % n;
        let patch = self.presets.read()[idx].patch;
        self.apply_patch(&patch);
        self.preset.set(idx);
        self.modified.set(false);
        self.save();
    }

    pub fn step_preset(self, delta: i32) {
        let n = self.presets.read().len() as i32;
        let cur = *self.preset.peek() as i32;
        self.load_preset((cur + delta).rem_euclid(n) as usize);
    }

    /// RND: pull every unlocked parameter toward a random value by the
    /// mutation amount (100% = fully random), then keep the result playable.
    pub fn randomize(mut self) {
        let amt = *self.mutation.peek();
        let locks: [bool; NUM_PARAMS] = std::array::from_fn(|i| *self.locks[i].peek());
        let mut patch = self.patch();
        let rnd = || js_sys::Math::random() as f32;
        for i in 0..NUM_PARAMS {
            if locks[i] {
                continue;
            }
            patch[i] = match PARAMS[i].curve {
                Curve::Choice(opts) => {
                    if rnd() < amt {
                        ((rnd() * opts.len() as f32).floor() + 0.5) / opts.len() as f32
                    } else {
                        patch[i]
                    }
                }
                _ => patch[i] + (rnd() - patch[i]) * amt,
            };
        }
        keep_playable(&mut patch, &locks);
        self.apply_patch(&patch);
        self.modified.set(true);
        self.rnd_gen += 1;
        self.save();
    }

    pub fn init_patch(mut self) {
        let mut patch = default_patch();
        for (i, l) in self.locks.iter().enumerate() {
            if *l.peek() {
                patch[i] = *self.params[i].peek();
            }
        }
        self.apply_patch(&patch);
        self.modified.set(true);
        self.save();
    }

    // ── notes ──────────────────────────────────────────────────────────

    pub fn note_on(mut self, note: i32, velocity: f32) {
        if !(0..128).contains(&note) {
            return;
        }
        let bit = 1u128 << note;
        if *self.latched.peek() & bit != 0 {
            self.latched.with_mut(|l| *l &= !bit);
        }
        audio::note_on(note as u8, velocity);
        self.notes.with_mut(|n| *n |= bit);
    }

    pub fn note_off(mut self, note: i32) {
        if !(0..128).contains(&note) {
            return;
        }
        let bit = 1u128 << note;
        if *self.notes.peek() & bit == 0 {
            return;
        }
        if *self.hold.peek() {
            self.latched.with_mut(|l| *l |= bit);
            return;
        }
        audio::note_off(note as u8);
        self.notes.with_mut(|n| *n &= !bit);
    }

    pub fn set_hold(mut self, on: bool) {
        self.hold.set(on);
        if !on {
            let latched = *self.latched.peek();
            for n in 0..128 {
                if latched & (1u128 << n) != 0 {
                    audio::note_off(n as u8);
                }
            }
            self.notes.with_mut(|n| *n &= !latched);
            self.latched.set(0);
        }
    }

    pub fn panic(mut self) {
        audio::panic();
        self.notes.set(0);
        self.latched.set(0);
    }
}

/// Nudge a randomized patch away from silence and ear-splitting extremes.
/// Only unlocked parameters are touched.
fn keep_playable(p: &mut Patch, locks: &[bool; NUM_PARAMS]) {
    let get = |p: &Patch, k: P| PARAMS[k as usize].curve.map(p[k as usize]);
    let set = |p: &mut Patch, k: P, v: f32| {
        if !locks[k as usize] {
            p[k as usize] = PARAMS[k as usize].curve.unmap(v);
        }
    };

    // Something must make sound.
    let excite = get(p, P::Strike) + get(p, P::Bow) + get(p, P::Breath) + get(p, P::Grit) + get(p, P::Feed);
    let audible = get(p, P::Body) * excite + get(p, P::Core) + get(p, P::Logic);
    if audible < 0.35 {
        if !locks[P::Core as usize] {
            set(p, P::Core, 0.6);
        } else {
            set(p, P::Strike, 1.0);
            set(p, P::Body, 0.9);
        }
    }

    // Keep the master filter open enough to hear.
    let mode = get(p, P::FilterType) as usize;
    let cutoff = get(p, P::Cutoff);
    let (lo, hi) = match mode {
        0 | 1 => (250.0, 20000.0),
        2 => (300.0, 6000.0),
        _ => (20.0, 1500.0),
    };
    set(p, P::Cutoff, cutoff.clamp(lo, hi));
    set(p, P::LowCut, get(p, P::LowCut).min(600.0));
    set(p, P::HiCut, get(p, P::HiCut).max(2500.0));
    set(p, P::Level, get(p, P::Level).clamp(-12.0, 0.0));
    set(p, P::Attack, get(p, P::Attack).min(3.0));
    set(p, P::RvMix, get(p, P::RvMix).min(0.75));
}
