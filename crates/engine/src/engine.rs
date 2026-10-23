//! The whole instrument: voices plus the master chain.
//!
//! ```text
//! voices ─► glitch ─► crush ─► drive ─► ladder (follower → cutoff) ─► EQ ─► Dattorro ─► level ─► DC ─► limiter
//! ```

use crate::dsp::*;
use crate::params::{NUM_PARAMS, P, Patch, default_patch, real};
use crate::reverb::Dattorro;
use crate::voice::{Shared, Voice};

pub const NUM_VOICES: usize = 8;
pub const MAX_BLOCK: usize = 4096;
pub const TELEMETRY_VOICE_STRIDE: usize = 6;
/// Per voice: active, note, env, lorenz x, y, z. Then: peak L, peak R,
/// follower, active voice count.
pub const TELEMETRY_LEN: usize = NUM_VOICES * TELEMETRY_VOICE_STRIDE + 4;

/// Stutter / reverse buffer repeater.
struct Glitch {
    buf: [Vec<f32>; 2],
    w: usize,
    countdown: u32,
    active: bool,
    seg_start: usize,
    seg_len: usize,
    pos: usize,
    repeats: u32,
    rev: bool,
    xf: f32,
}

impl Glitch {
    fn new(sr: f32) -> Self {
        let n = (sr * 2.0) as usize;
        Glitch {
            buf: [vec![0.0; n], vec![0.0; n]],
            w: 0,
            countdown: 0,
            active: false,
            seg_start: 0,
            seg_len: 1,
            pos: 0,
            repeats: 0,
            rev: false,
            xf: 0.0,
        }
    }

    #[inline]
    fn process(&mut self, l: f32, r: f32, amt: f32, rng: &mut Rng, sr: f32) -> (f32, f32) {
        let len = self.buf[0].len();
        self.buf[0][self.w] = l;
        self.buf[1][self.w] = r;
        self.w = (self.w + 1) % len;
        if amt < 0.001 && !self.active && self.xf < 1e-4 {
            return (l, r);
        }
        if self.countdown == 0 {
            self.countdown = (sr * (0.04 + 0.12 * rng.uni())) as u32;
            if !self.active && rng.uni() < amt * 0.7 {
                let u = rng.uni();
                let mut seg = (sr * (0.01 + 0.18 * u * u * (0.4 + amt))) as usize;
                if rng.uni() < 0.25 * amt {
                    seg /= 4;
                }
                self.seg_len = seg.max(64);
                self.seg_start = (self.w + len - self.seg_len - 1) % len;
                self.pos = 0;
                self.repeats = 2 + (rng.uni() * 7.0 * amt) as u32;
                self.rev = rng.uni() < 0.3 * amt;
                self.active = true;
            }
        } else {
            self.countdown -= 1;
        }
        let target = if self.active { 1.0 } else { 0.0 };
        self.xf += (target - self.xf) * 0.02;
        if self.xf < 1e-4 {
            return (l, r);
        }
        let idx = if self.rev { self.seg_len - 1 - self.pos } else { self.pos };
        let j = (self.seg_start + idx) % len;
        let edge = (self.pos.min(self.seg_len - self.pos) as f32 / 48.0).min(1.0);
        let (gl, gr) = (self.buf[0][j] * edge, self.buf[1][j] * edge);
        if self.active {
            self.pos += 1;
            if self.pos >= self.seg_len {
                self.pos = 0;
                self.repeats -= 1;
                if self.repeats == 0 {
                    self.active = false;
                }
            }
        }
        (lerp(l, gl, self.xf), lerp(r, gr, self.xf))
    }

    fn clear(&mut self) {
        self.buf.iter_mut().for_each(|b| b.iter_mut().for_each(|s| *s = 0.0));
        self.active = false;
        self.xf = 0.0;
    }
}

pub struct Engine {
    sr: f32,
    norm: Patch,
    sh: Shared,
    dirty: bool,
    voices: Vec<Voice>,
    order: u64,
    last_pitch: Option<f32>,
    ladder: [Ladder; 2],
    cutoff_s: f32,
    follow_env: f32,
    follow_att: f32,
    follow_rel: f32,
    eq: [Biquad; 5],
    reverb: Dattorro,
    glitch: Glitch,
    crush_hold: [f32; 2],
    crush_acc: f32,
    dc: [f32; 4],
    lim_env: f32,
    level_s: f32,
    mix_s: f32,
    rng: Rng,
    out: Vec<f32>,
    telemetry: [f32; TELEMETRY_LEN],
    peak: [f32; 2],
}

impl Engine {
    pub fn new(sr: f32) -> Self {
        let mut e = Engine {
            sr,
            norm: default_patch(),
            sh: Shared::new(sr),
            dirty: true,
            voices: (0..NUM_VOICES).map(Voice::new).collect(),
            order: 0,
            last_pitch: None,
            ladder: [Ladder::default(); 2],
            cutoff_s: 20000.0,
            follow_env: 0.0,
            follow_att: tau_coef(0.005, sr),
            follow_rel: tau_coef(0.15, sr),
            eq: [Biquad::default(); 5],
            reverb: Dattorro::new(sr),
            glitch: Glitch::new(sr),
            crush_hold: [0.0; 2],
            crush_acc: 0.0,
            dc: [0.0; 4],
            lim_env: 0.0,
            level_s: 1.0,
            mix_s: 0.0,
            rng: Rng::new(0x5EED),
            out: vec![0.0; MAX_BLOCK * 2],
            telemetry: [0.0; TELEMETRY_LEN],
            peak: [0.0; 2],
        };
        e.apply_params();
        e.cutoff_s = e.sh.get(P::Cutoff);
        e.mix_s = e.sh.get(P::RvMix);
        e.level_s = db_to_gain(e.sh.get(P::Level));
        e
    }

    pub fn sample_rate(&self) -> f32 {
        self.sr
    }

    pub fn set_param(&mut self, i: usize, norm: f32) {
        if i < NUM_PARAMS && norm.is_finite() {
            self.norm[i] = norm.clamp(0.0, 1.0);
            self.dirty = true;
        }
    }

    pub fn set_patch(&mut self, patch: &Patch) {
        for (i, &v) in patch.iter().enumerate() {
            self.set_param(i, v);
        }
    }

    /// Jump smoothed parameters straight to their targets (offline renders,
    /// or anywhere a glide from the previous patch is unwanted).
    pub fn snap_smoothing(&mut self) {
        if self.dirty {
            self.apply_params();
        }
        self.cutoff_s = self.sh.get(P::Cutoff);
        self.mix_s = self.sh.get(P::RvMix);
        self.level_s = db_to_gain(self.sh.get(P::Level));
    }

    pub fn param(&self, i: usize) -> f32 {
        self.norm[i]
    }

    fn apply_params(&mut self) {
        for i in 0..NUM_PARAMS {
            self.sh.v[i] = real(i, self.norm[i]);
        }
        self.sh.update();
        let sr = self.sr;
        let g = |p: P| self.sh.get(p);
        use BiquadKind::*;
        self.eq[0].set(HighPass, g(P::LowCut), 0.707, sr);
        self.eq[1].set(LowShelf(g(P::EqLo)), 120.0, 0.707, sr);
        self.eq[2].set(Peak(g(P::EqMid)), g(P::EqMidFreq), 0.9, sr);
        self.eq[3].set(HighShelf(g(P::EqHi)), 6000.0, 0.707, sr);
        self.eq[4].set(LowPass, g(P::HiCut), 0.707, sr);
        let (pre, size, decay, damp, md) = (g(P::Predelay), g(P::Size), g(P::RvDecay), g(P::RvDamp), g(P::RvMod));
        self.reverb.set_predelay_seconds(pre, sr);
        self.reverb.size = size;
        self.reverb.decay = decay;
        self.reverb.damping = damp;
        self.reverb.modulation = md;
        self.dirty = false;
    }

    pub fn note_on(&mut self, note: u8, vel: f32) {
        if self.dirty {
            self.apply_params();
        }
        self.order += 1;
        let vel = vel.clamp(0.0, 1.0);
        let idx = self
            .voices
            .iter()
            .position(|v| v.active && v.note == note)
            .or_else(|| self.voices.iter().position(|v| !v.active))
            .or_else(|| {
                // Steal the quietest released voice, else the oldest.
                self.voices
                    .iter()
                    .enumerate()
                    .filter(|(_, v)| !v.gate)
                    .min_by(|a, b| a.1.env.total_cmp(&b.1.env))
                    .map(|(i, _)| i)
            })
            .unwrap_or_else(|| {
                self.voices.iter().enumerate().min_by_key(|(_, v)| v.order).map(|(i, _)| i).unwrap_or(0)
            });
        let from = self.last_pitch;
        self.voices[idx].start(note, vel, from, &self.sh, self.order);
        self.last_pitch = Some(note as f32);
    }

    pub fn note_off(&mut self, note: u8) {
        for v in &mut self.voices {
            if v.active && v.gate && v.note == note {
                v.release();
            }
        }
    }

    pub fn all_notes_off(&mut self) {
        for v in &mut self.voices {
            if v.active {
                v.release();
            }
        }
    }

    pub fn panic(&mut self) {
        self.reset_state();
    }

    fn reset_state(&mut self) {
        self.voices.iter_mut().for_each(Voice::kill);
        self.ladder.iter_mut().for_each(Ladder::reset);
        self.eq.iter_mut().for_each(Biquad::reset);
        self.reverb.clear();
        self.glitch.clear();
        self.dc = [0.0; 4];
        self.lim_env = 0.0;
        self.follow_env = 0.0;
    }

    pub fn active_voices(&self) -> usize {
        self.voices.iter().filter(|v| v.active).count()
    }

    /// Render `frames` samples. Returns planar stereo: `frames` left samples
    /// followed by `frames` right samples.
    pub fn process(&mut self, frames: usize) -> &[f32] {
        let frames = frames.min(MAX_BLOCK);
        if self.dirty {
            self.apply_params();
        }
        let sr = self.sr;
        let glitch_amt = self.sh.get(P::Glitch);
        let crush = self.sh.get(P::Crush);
        let hold_n = 1.0 + crush * crush * 30.0;
        let q = (10.0 - crush * 7.0).exp2();
        let cutoff = self.sh.get(P::Cutoff);
        let follow = self.sh.get(P::Follow);
        let k = self.sh.get(P::Resonance) * 4.1;
        let drive = 1.0 + self.sh.get(P::Drive) * 11.0;
        let drive_out = 2.0 / drive.sqrt();
        let mode = LadderMode::from_index(self.sh.get(P::FilterType) as usize);
        let mix = self.sh.get(P::RvMix);
        let level = db_to_gain(self.sh.get(P::Level));
        let lim_rel = tau_coef(0.25, sr);

        let mut peak = [0.0f32; 2];
        for i in 0..frames {
            let (mut l, mut r) = (0.0, 0.0);
            for v in self.voices.iter_mut() {
                if v.active {
                    let (a, b) = v.tick(&self.sh);
                    l += a;
                    r += b;
                }
            }

            (l, r) = self.glitch.process(l, r, glitch_amt, &mut self.rng, sr);

            if crush > 0.001 {
                self.crush_acc += 1.0;
                if self.crush_acc >= hold_n {
                    self.crush_acc -= hold_n;
                    // Quantize in a companded domain so quiet signals turn
                    // to grit instead of silence.
                    let cq = |x: f32| {
                        let y = (x.abs().sqrt() * q).round() / q;
                        x.signum() * y * y
                    };
                    self.crush_hold = [cq(l), cq(r)];
                }
                [l, r] = self.crush_hold;
            }

            let m = 0.5 * (l.abs() + r.abs());
            let fc = if m > self.follow_env { self.follow_att } else { self.follow_rel };
            self.follow_env += (m - self.follow_env) * fc;

            self.cutoff_s += (cutoff - self.cutoff_s) * 0.002;
            let hz = (self.cutoff_s * (follow * 6.0 * (self.follow_env * 3.0).min(1.5)).exp2()).clamp(20.0, 20000.0);
            let g = hz_coef(hz, sr * 2.0);
            l = self.ladder[0].process(l * drive * 0.5, g, k, mode) * drive_out;
            r = self.ladder[1].process(r * drive * 0.5, g, k, mode) * drive_out;

            for b in &mut self.eq {
                l = b.process(0, l);
                r = b.process(1, r);
            }

            let (wl, wr) = self.reverb.process(l, r);
            self.mix_s += (mix - self.mix_s) * 0.001;
            let dry = 1.0 - self.mix_s * self.mix_s;
            let wet = self.mix_s * (2.0 - self.mix_s);
            l = l * dry + wl * wet;
            r = r * dry + wr * wet;

            self.level_s += (level - self.level_s) * 0.002;
            l *= self.level_s;
            r *= self.level_s;

            // DC blocker
            let yl = l - self.dc[0] + 0.995 * self.dc[1];
            self.dc[0] = l;
            self.dc[1] = yl;
            let yr = r - self.dc[2] + 0.995 * self.dc[3];
            self.dc[2] = r;
            self.dc[3] = yr;
            (l, r) = (yl, yr);

            // Peak limiter with instant attack, then a soft safety clip.
            let pk = l.abs().max(r.abs());
            if pk > self.lim_env {
                self.lim_env = pk;
            } else {
                self.lim_env += (pk - self.lim_env) * lim_rel;
            }
            if self.lim_env > 0.9 {
                let gr = 0.9 / self.lim_env;
                l *= gr;
                r *= gr;
            }
            l = soft_clip(l);
            r = soft_clip(r);

            if !(l.is_finite() && r.is_finite()) {
                self.reset_state();
                l = 0.0;
                r = 0.0;
            }
            peak[0] = peak[0].max(l.abs());
            peak[1] = peak[1].max(r.abs());
            self.out[i] = l;
            self.out[frames + i] = r;
        }

        self.peak = [peak[0].max(self.peak[0] * 0.9), peak[1].max(self.peak[1] * 0.9)];
        self.write_telemetry();
        &self.out[..frames * 2]
    }

    fn write_telemetry(&mut self) {
        let t = &mut self.telemetry;
        for (i, v) in self.voices.iter().enumerate() {
            let o = i * TELEMETRY_VOICE_STRIDE;
            t[o] = if v.active { 1.0 } else { 0.0 };
            t[o + 1] = v.note as f32;
            t[o + 2] = v.env;
            t[o + 3] = v.lorenz[0];
            t[o + 4] = v.lorenz[1];
            t[o + 5] = v.lorenz[2];
        }
        let o = NUM_VOICES * TELEMETRY_VOICE_STRIDE;
        t[o] = self.peak[0];
        t[o + 1] = self.peak[1];
        t[o + 2] = self.follow_env;
        t[o + 3] = self.voices.iter().filter(|v| v.active).count() as f32;
    }

    pub fn telemetry(&self) -> &[f32; TELEMETRY_LEN] {
        &self.telemetry
    }

    /// Pointer to the output buffer (for the wasm worklet bridge).
    pub fn out_ptr(&self) -> *const f32 {
        self.out.as_ptr()
    }
}

#[inline]
fn soft_clip(x: f32) -> f32 {
    let a = x.abs();
    if a < 0.7 { x } else { x.signum() * (0.7 + 0.3 * tanh_fast((a - 0.7) / 0.3)) }
}
