//! One synth voice.
//!
//! Signal flow:
//!
//! ```text
//!  strike ─┐                         ┌──────────── core / logic direct ───────────┐
//!  grit   ─┤ impulse ┐               │                                            ▼
//!  bow ◄───┼─────────┤  modal body ──┴─► rattle ─► mix ─► growl ─► throat ─► ring ─► screech ─► VCA ─► pan
//!  breath ─┤ contin. ┘  (12/24 partials)
//!  core   ─┘ (feed)
//! ```
//!
//! Heavy per-partial work (coefficients) runs at control rate every
//! [`CTRL`] samples; everything audible runs per sample.

use crate::dsp::*;
use crate::params::{NUM_PARAMS, P};

pub const MAX_MODES: usize = 12;
pub const CTRL: usize = 32;
const BODY_GAIN: f32 = 0.3;
const VOICE_GAIN: f32 = 1.0;

/// Partial ratio tables for the material morph (all ascending).
const MATERIAL_TABLES: [[f32; MAX_MODES]; 8] = [
    // String: harmonic
    [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0],
    // Tube: odd harmonics (closed pipe)
    [1.0, 3.0, 5.0, 7.0, 9.0, 11.0, 13.0, 15.0, 17.0, 19.0, 21.0, 23.0],
    // Drum: circular membrane (Bessel zeros)
    [1.0, 1.594, 2.136, 2.296, 2.653, 2.918, 3.156, 3.501, 3.600, 3.652, 4.060, 4.154],
    // Plate: rectangular plate, aspect 1:1.4
    [1.0, 2.013, 2.987, 3.702, 4.0, 5.689, 6.066, 6.298, 7.311, 8.053, 9.0, 10.934],
    // Bar: free-free beam
    [1.0, 2.756, 5.404, 8.933, 13.344, 18.638, 24.81, 31.87, 39.81, 48.63, 58.33, 68.92],
    // Bell: hum, prime, tierce, quint, nominal, ...
    [0.5, 1.0, 1.183, 1.506, 2.0, 2.514, 2.662, 3.011, 4.166, 5.433, 6.796, 8.215],
    // Glass: wine glass
    [1.0, 2.32, 4.25, 6.63, 9.38, 12.5, 16.0, 19.9, 24.1, 28.7, 33.6, 38.9],
    // Alien: nobody knows
    [1.0, 1.31, 1.97, 2.71, 3.17, 4.43, 5.07, 6.61, 7.23, 8.89, 10.3, 11.9],
];

/// Formant frequencies F1..F3 for the vowel morph.
const VOWEL_TABLE: [[f32; 3]; 6] = [
    [180.0, 420.0, 1100.0],  // Beast
    [300.0, 870.0, 2240.0],  // U
    [450.0, 800.0, 2830.0],  // O
    [730.0, 1090.0, 2440.0], // A
    [530.0, 1840.0, 2480.0], // E
    [270.0, 2290.0, 3010.0], // I
];

/// Parameter-derived state shared by every voice; recomputed when a
/// parameter changes rather than per voice per block.
pub struct Shared {
    pub v: [f32; NUM_PARAMS],
    pub sr: f32,
    ln_tables: [[f32; MAX_MODES]; 8],
    pub ratios: [f32; MAX_MODES],
    pub radius: [f32; MAX_MODES],
    pub cgain: [f32; MAX_MODES],
    pub amp: [f32; MAX_MODES],
    pub twin_ratio: [f32; MAX_MODES],
    pub n_modes: usize,
    pub twin: bool,
}

impl Shared {
    pub fn new(sr: f32) -> Self {
        let mut ln_tables = [[0.0; MAX_MODES]; 8];
        for (t, src) in ln_tables.iter_mut().zip(MATERIAL_TABLES.iter()) {
            for (d, s) in t.iter_mut().zip(src.iter()) {
                *d = s.ln();
            }
        }
        Shared {
            v: [0.0; NUM_PARAMS],
            sr,
            ln_tables,
            ratios: [1.0; MAX_MODES],
            radius: [0.0; MAX_MODES],
            cgain: [0.0; MAX_MODES],
            amp: [0.0; MAX_MODES],
            twin_ratio: [1.0; MAX_MODES],
            n_modes: MAX_MODES,
            twin: false,
        }
    }

    #[inline]
    pub fn get(&self, p: P) -> f32 {
        self.v[p as usize]
    }

    pub fn update(&mut self) {
        let mat = self.get(P::Material).clamp(0.0, 7.0);
        let i0 = (mat.floor() as usize).min(6);
        let frac = mat - i0 as f32;
        let stretch = (self.get(P::Stretch) * 0.6).exp2();
        let count = self.get(P::Modes);
        let ring = self.get(P::RingTime);
        let mute = self.get(P::Mute);
        let d = self.get(P::Damping) + mute * 0.6;
        let damp_k = d * d * 4.0;
        let mute_f = 1.0 - 0.985 * mute.sqrt();
        let pos = self.get(P::Position);
        let shimmer = self.get(P::Shimmer);

        for i in 0..MAX_MODES {
            let ln = lerp(self.ln_tables[i0][i], self.ln_tables[i0 + 1][i], frac);
            let ratio = (ln * stretch).exp();
            self.ratios[i] = ratio;
            let t = ring * mute_f / (1.0 + damp_k * (ratio - 1.0).max(0.0));
            let r = (-6.9078 / (t * self.sr).max(1.0)).exp();
            self.radius[i] = r;
            self.cgain[i] = (1.0 - r).powf(0.75) * 0.35;
            let place = 0.08 + 0.92 * (PI * (i + 1) as f32 * pos).sin().abs();
            let fade = (count - i as f32).clamp(0.0, 1.0);
            self.amp[i] = place / (1.0 + 0.15 * i as f32) * fade;
            let sign = if i % 2 == 0 { 1.0 } else { -1.0 };
            self.twin_ratio[i] = (sign * shimmer * (4.0 + 26.0 * hash01(i as u32 + 17)) / 1200.0).exp2();
        }
        self.n_modes = (count.ceil() as usize).clamp(1, MAX_MODES);
        self.twin = shimmer > 0.001;
    }
}

/// One partial: a complex one-pole resonator (a decaying phasor). Unlike a
/// direct-form biquad it keeps its amplitude when the frequency is swept,
/// which matters for sweeps, gravity and glide through the body.
#[derive(Clone, Copy, Default)]
struct Mode {
    re: f32,
    im: f32,
    c: f32,
    s: f32,
    bi: f32,
    bc: f32,
}

impl Mode {
    #[inline]
    fn set(&mut self, hz: f32, r: f32, amp: f32, cgain: f32, sr: f32) {
        if hz >= sr * 0.45 || amp <= 0.0 {
            // Silence partials above Nyquist instead of letting them alias.
            *self = Mode::default();
            return;
        }
        let (sw, cw) = (TAU * hz / sr).sin_cos();
        self.c = r * cw;
        self.s = r * sw;
        self.bi = amp;
        self.bc = amp * cgain;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    Attack,
    Decay,
    Release,
}

/// Control-rate cache.
#[derive(Clone, Copy, Default)]
struct Ctl {
    inc: f32,
    fm_inc: f32,
    fm_index: f32,
    self_fm: f32,
    warp_d: f32,
    ring_inc: f32,
    bt_inc: f32,
    amp: f32,
    pan_l: f32,
    pan_r: f32,
    att: f32,
    dec: f32,
    rel: f32,
    gate_coef: f32,
    strike_coef: f32,
    strike_norm: f32,
    strike_lp: f32,
    breath_lp: f32,
    grit_p: f32,
    grit_amp: f32,
    growl_inc: f32,
    sc_fb: f32,
    lorenz_dt: f32,
}

pub struct Voice {
    pub active: bool,
    pub gate: bool,
    pub note: u8,
    pub vel: f32,
    pub order: u64,
    pub env: f32,
    pub lorenz: [f32; 3],
    stage: Stage,
    age: f32,
    pitch: f32,
    target: f32,
    gate_s: f32,
    // oscillators
    ph: f32,
    fm_ph: f32,
    fb: f32,
    sub2: f32,
    sub3: f32,
    bt: f32,
    ring_ph: f32,
    // exciters
    strike_env: f32,
    strike_lp: f32,
    breath_lp1: f32,
    breath_lp2: f32,
    // body
    modes: [Mode; MAX_MODES * 2],
    n_modes: usize,
    body_prev: f32,
    rattle_fb: f32,
    // creature
    formant: [Svf; 3],
    growl_ph: f32,
    growl_walk: f32,
    // motion
    drift: f32,
    drift_target: f32,
    wob_ph: f32,
    spin_ph: f32,
    // screech
    screech: Svf,
    sc_state: f32,
    sc_walk: f32,
    rng: Rng,
    ctrl: usize,
    c: Ctl,
}

#[inline]
fn fract(x: f32) -> f32 {
    x - x.floor()
}

#[inline]
fn bytebeat(t: u32, formula: usize) -> f32 {
    let v = match formula {
        0 => t.wrapping_mul((t >> 5) | (t >> 8)),
        1 => (t.wrapping_mul(5) & (t >> 7)) | (t.wrapping_mul(3) & (t >> 10)),
        2 => t.wrapping_mul(((t >> 12) | (t >> 8)) & 63 & (t >> 4)),
        3 => ((t >> 6) | t | (t >> ((t >> 16) & 15))).wrapping_mul(10).wrapping_add((t >> 11) & 7),
        4 => t.wrapping_mul((t >> 11) & (t >> 8) & 123 & (t >> 3)),
        5 => ((t >> 7) | t | (t >> 6)).wrapping_mul(10).wrapping_add(4 * ((t & (t >> 13)) | (t >> 6))),
        6 => (t.wrapping_mul(9) & (t >> 4)) | (t.wrapping_mul(5) & (t >> 7)) | (t.wrapping_mul(3) & (t >> 10)),
        _ => (t ^ (t >> 3)).wrapping_mul((t >> 9) | (t >> 12)),
    };
    (v & 255) as f32 / 127.5 - 1.0
}

impl Voice {
    pub fn new(index: usize) -> Self {
        Voice {
            active: false,
            gate: false,
            note: 60,
            vel: 1.0,
            order: 0,
            env: 0.0,
            lorenz: [0.1, 0.0, 20.0],
            stage: Stage::Release,
            age: 0.0,
            pitch: 60.0,
            target: 60.0,
            gate_s: 0.0,
            ph: 0.0,
            fm_ph: 0.0,
            fb: 0.0,
            sub2: 0.0,
            sub3: 0.0,
            bt: 0.0,
            ring_ph: 0.0,
            strike_env: 0.0,
            strike_lp: 0.0,
            breath_lp1: 0.0,
            breath_lp2: 0.0,
            modes: [Mode::default(); MAX_MODES * 2],
            n_modes: 0,
            body_prev: 0.0,
            rattle_fb: 0.0,
            formant: [Svf::default(); 3],
            growl_ph: 0.0,
            growl_walk: 0.0,
            drift: 0.0,
            drift_target: 0.0,
            wob_ph: 0.0,
            spin_ph: 0.0,
            screech: Svf::default(),
            sc_state: 0.0,
            sc_walk: 0.0,
            rng: Rng::new(0xBEEF + index as u32 * 7919),
            ctrl: 0,
            c: Ctl::default(),
        }
    }

    pub fn start(&mut self, note: u8, vel: f32, glide_from: Option<f32>, sh: &Shared, order: u64) {
        let was_active = self.active;
        self.active = true;
        self.gate = true;
        self.note = note;
        self.vel = vel;
        self.order = order;
        self.stage = Stage::Attack;
        self.age = 0.0;
        self.target = note as f32;
        self.pitch = match glide_from {
            Some(p) if sh.get(P::Glide) > 0.0 => p,
            _ => note as f32,
        };
        self.strike_env = 1.0;
        if !was_active {
            self.modes.iter_mut().for_each(|m| {
                m.re = 0.0;
                m.im = 0.0;
            });
            let h = hash01(note as u32 * 31 + order as u32);
            self.lorenz = [0.1 + h, h - 0.5, 20.0 + h * 5.0];
            self.ph = 0.0;
            self.fm_ph = 0.0;
            self.fb = 0.0;
            self.sub2 = 0.0;
            self.sub3 = 0.0;
            self.bt = 0.0;
            self.env = 0.0;
            self.gate_s = 0.0;
            self.body_prev = 0.0;
            self.rattle_fb = 0.0;
            self.sc_state = 0.0;
            self.strike_lp = 0.0;
            self.drift = 0.0;
            self.spin_ph = h;
            self.wob_ph = 0.0;
            self.screech.reset();
            self.formant.iter_mut().for_each(Svf::reset);
        }
        self.ctrl = 0;
    }

    pub fn release(&mut self) {
        self.gate = false;
        self.stage = Stage::Release;
    }

    pub fn kill(&mut self) {
        self.active = false;
        self.gate = false;
        self.env = 0.0;
    }

    /// Current pitch in semitones (for glide continuity).
    pub fn pitch(&self) -> f32 {
        self.pitch
    }

    fn control(&mut self, sh: &Shared) {
        let sr = sh.sr;
        let dt = CTRL as f32 / sr;
        let ctl_rate = sr / CTRL as f32;
        self.age += dt;

        // ── pitch gestures ────────────────────────────────────────────
        let glide = sh.get(P::Glide);
        if glide > 0.0 {
            self.pitch += (self.target - self.pitch) * tau_coef(glide * 0.5, ctl_rate);
        } else {
            self.pitch = self.target;
        }
        let sweep = sh.get(P::Sweep) * (-self.age / sh.get(P::SweepTime)).exp();
        let gravity = (sh.get(P::Gravity) * self.age).clamp(-72.0, 72.0);

        let drift_amt = sh.get(P::Drift);
        let mut drift_off = 0.0;
        if drift_amt > 0.0 {
            if self.rng.uni() < dt * 3.0 {
                self.drift_target = self.rng.bi();
            }
            self.drift += (self.drift_target - self.drift) * tau_coef(0.25, ctl_rate);
            drift_off = drift_amt * (self.drift * 1.5 + self.rng.bi() * 0.04);
        }

        let wobble = sh.get(P::Wobble);
        self.wob_ph = fract(self.wob_ph + sh.get(P::WobRate) * dt);
        let w = (TAU * self.wob_ph).sin();
        let wob_pitch = wobble * wobble * w * 1.5;
        let wob_amp = 1.0 - wobble * 0.6 * (0.5 - 0.5 * w);

        let spin = sh.get(P::Spin);
        let depth = (spin * 3.0).min(1.0);
        self.spin_ph = fract(self.spin_ph + 0.15 * 80f32.powf(spin) * dt);
        let (ss, sc) = (TAU * self.spin_ph).sin_cos();
        let doppler = -ss * depth * 0.35 * spin;
        let spin_amp = 1.0 - depth * 0.25 * (0.5 - 0.5 * sc);

        let spread = sh.get(P::Spread);
        let pan = ((hash01(self.note as u32) * 2.0 - 1.0) * spread + sc * depth * 0.9).clamp(-1.0, 1.0);
        let angle = (pan + 1.0) * PI * 0.25;
        self.c.pan_l = angle.cos() * std::f32::consts::SQRT_2;
        self.c.pan_r = angle.sin() * std::f32::consts::SQRT_2;

        let pitch = self.pitch + sweep + gravity + drift_off + wob_pitch + doppler;
        let f = mtof(pitch).clamp(4.0, sr * 0.45);

        // ── core ──────────────────────────────────────────────────────
        let c = &mut self.c;
        c.inc = f / sr;
        c.fm_inc = f * sh.get(P::FmRatio) / sr;
        let fmd = sh.get(P::FmDepth);
        c.fm_index = fmd * fmd * 1.0;
        let sfm = sh.get(P::SelfFm);
        c.self_fm = sfm * sfm * 0.35;
        c.warp_d = 0.5 - 0.48 * sh.get(P::Warp);
        c.ring_inc = sh.get(P::RingFreq) / sr;
        c.bt_inc = 8000.0 / sr * f / 261.6256;
        c.amp = wob_amp * spin_amp * (0.35 + 0.65 * self.vel) * VOICE_GAIN;
        c.lorenz_dt = (sh.get(P::Attractor) / 1.4 / sr).min(0.012);

        // ── envelopes ─────────────────────────────────────────────────
        let attack = sh.get(P::Attack);
        c.att = tau_coef(attack / 1.466, sr);
        c.dec = tau_coef(sh.get(P::Decay) / 4.6, sr);
        c.rel = tau_coef(sh.get(P::Release) / 4.6, sr);
        c.gate_coef = tau_coef(if self.gate { attack.max(0.004) / 3.0 } else { 0.03 }, sr);

        // ── exciters ──────────────────────────────────────────────────
        let h = sh.get(P::Hardness);
        // Contact time and mallet brightness; soft mallets lose energy to
        // cancellation, so they get a level boost.
        let dur = 0.00015 + 0.0025 * (1.0 - h) * (1.0 - h);
        c.strike_coef = (-1.0 / (dur * sr)).exp();
        c.strike_norm = (1.0 - c.strike_coef) * (1.0 + 2.5 * (1.0 - h)) * 2.5;
        c.strike_lp = hz_coef(400.0 * (h * 5.6).exp2(), sr);
        c.breath_lp = hz_coef((f * 3.0).clamp(200.0, 9000.0), sr);
        let grit = sh.get(P::Grit);
        let density = 3.0 + 3000.0 * grit * grit;
        c.grit_p = density / sr;
        c.grit_amp = 0.4 / (1.0 + density / 50.0).sqrt();

        // ── creature ──────────────────────────────────────────────────
        self.growl_walk = (self.growl_walk + self.rng.bi() * 0.25).clamp(-1.0, 1.0);
        c.growl_inc = 38.0 * (1.0 + 0.45 * self.growl_walk) / sr;
        if sh.get(P::Throat) > 0.001 {
            let vow = (sh.get(P::Vowel) + wobble * w * 0.4).clamp(0.0, 5.0);
            let i0 = (vow.floor() as usize).min(4);
            let fr = vow - i0 as f32;
            for (k, svf) in self.formant.iter_mut().enumerate() {
                let hz = lerp(VOWEL_TABLE[i0][k], VOWEL_TABLE[i0 + 1][k], fr);
                svf.set(hz, 6.0 + 3.0 * k as f32, sr);
            }
        }

        // ── screech ───────────────────────────────────────────────────
        let screech = sh.get(P::Screech);
        if screech > 0.001 {
            self.sc_walk = (self.sc_walk + self.rng.bi() * 0.15).clamp(-1.0, 1.0);
            let base = 1100.0 + 2800.0 * hash01(self.note as u32 * 7 + 1);
            let chaos = sh.get(P::Chaos) * self.lorenz[0] * 0.05;
            let hz = base * (self.sc_walk * 0.25 + chaos * 0.6).exp2();
            self.screech.set(hz, 18.0, sr);
            c.sc_fb = 0.38 + 0.3 * screech;
        }

        // ── body partials ─────────────────────────────────────────────
        let fb = f * (sh.get(P::BodyTune) / 12.0).exp2();
        let n = sh.n_modes;
        for i in 0..n {
            let hz = fb * sh.ratios[i];
            self.modes[i].set(hz, sh.radius[i], sh.amp[i], sh.cgain[i], sr);
            if sh.twin {
                self.modes[n + i].set(hz * sh.twin_ratio[i], sh.radius[i], sh.amp[i] * 0.8, sh.cgain[i], sr);
            }
        }
        self.n_modes = if sh.twin { n * 2 } else { n };
    }

    #[inline]
    pub fn tick(&mut self, sh: &Shared) -> (f32, f32) {
        if self.ctrl == 0 {
            self.control(sh);
            self.ctrl = CTRL;
        }
        self.ctrl -= 1;
        let c = self.c;

        // ── amplitude envelope ────────────────────────────────────────
        match self.stage {
            Stage::Attack => {
                self.env += (1.3 - self.env) * c.att;
                if self.env >= 1.0 {
                    self.env = 1.0;
                    self.stage = Stage::Decay;
                }
            }
            Stage::Decay => {
                let sus = sh.get(P::Sustain);
                self.env += (sus - self.env) * c.dec;
                if sus <= 0.0 && self.env < 1e-4 {
                    self.kill();
                    return (0.0, 0.0);
                }
            }
            Stage::Release => {
                self.env -= self.env * c.rel;
                if self.env < 1e-4 {
                    self.kill();
                    return (0.0, 0.0);
                }
            }
        }
        let g = if self.gate { 1.0 } else { 0.0 };
        self.gate_s += (g - self.gate_s) * c.gate_coef;

        // ── Lorenz attractor ──────────────────────────────────────────
        let [x, y, z] = self.lorenz;
        let dt = c.lorenz_dt;
        let nx = x + 10.0 * (y - x) * dt;
        let ny = y + (x * (28.0 - z) - y) * dt;
        let nz = z + (x * y - 8.0 / 3.0 * z) * dt;
        self.lorenz = if nx.is_finite() && ny.is_finite() && nz.is_finite() { [nx, ny, nz] } else { [0.1, 0.0, 20.0] };
        let lx = self.lorenz[0] * 0.05;

        // ── core oscillator ───────────────────────────────────────────
        let chaos = sh.get(P::Chaos);
        let inc = c.inc * (1.0 + chaos * lx * 0.5);
        self.fm_ph = fract(self.fm_ph + c.fm_inc);
        let fm = if c.fm_index > 0.0 { (TAU * self.fm_ph).sin() * c.fm_index } else { 0.0 };
        self.ph = fract(self.ph + inc);
        let p = fract(self.ph + fm + self.fb * c.self_fm);
        let pd = if p < c.warp_d { 0.5 * p / c.warp_d } else { 0.5 + 0.5 * (p - c.warp_d) / (1.0 - c.warp_d) };
        let mut s = (TAU * pd).sin();
        self.fb = 0.5 * (self.fb + s);
        let fold = sh.get(P::Fold);
        if fold > 0.001 {
            s = (s * (1.0 + fold * 5.0) * PI * 0.5).sin();
        }
        self.sub2 = fract(self.sub2 + inc * 0.5);
        self.sub3 = fract(self.sub3 + inc * (1.0 / 3.0));
        let under = sh.get(P::Undertone);
        if under > 0.001 {
            let sub = tanh_fast(3.0 * (TAU * self.sub2).sin()) * 0.6 + tanh_fast(3.0 * (TAU * self.sub3).sin()) * 0.4;
            s += under * sub * 1.2;
        }
        let core = s + chaos * lx * 0.6;

        let logic_amt = sh.get(P::Logic);
        let logic = if logic_amt > 0.001 {
            self.bt += c.bt_inc;
            if self.bt >= 16_777_216.0 {
                self.bt -= 16_777_216.0;
            }
            bytebeat(self.bt as u32, sh.get(P::Formula) as usize) * logic_amt
        } else {
            0.0
        };

        // ── exciters ──────────────────────────────────────────────────
        let mut ximp = 0.0;
        let mut xcont = 0.0;
        let mut direct = 0.0;

        let mut raw = 0.0;
        if self.strike_env > 1e-4 {
            raw = self.strike_env * (0.7 + 0.3 * self.rng.bi()) * c.strike_norm;
            self.strike_env *= c.strike_coef;
        }
        self.strike_lp += c.strike_lp * (raw - self.strike_lp);
        ximp += self.strike_lp * sh.get(P::Strike) * (0.3 + 0.7 * self.vel);

        let grit = sh.get(P::Grit);
        if grit > 0.001 && self.rng.uni() < c.grit_p {
            let k = self.rng.bi() * c.grit_amp * self.gate_s.max(0.15);
            ximp += k;
            direct += k * 0.2 * grit;
        }

        let bow = sh.get(P::Bow);
        if bow > 0.001 {
            let pressure = sh.get(P::Pressure);
            let vb = (0.3 + 0.5 * bow) * self.gate_s * (1.0 + pressure * 0.25 * self.rng.bi());
            let vrel = vb - self.body_prev * 0.9;
            let slope = 5.0 - 4.0 * pressure;
            let t = (vrel * slope).abs() + 0.75;
            let t2 = t * t;
            let table = (1.0 / (t2 * t2)).min(1.0);
            ximp += vrel * table * bow * 0.8 * self.gate_s;
        }

        let breath = sh.get(P::Breath);
        if breath > 0.001 {
            let nz = self.rng.bi();
            self.breath_lp1 += c.breath_lp * (nz - self.breath_lp1);
            self.breath_lp2 += c.breath_lp * (self.breath_lp1 - self.breath_lp2);
            let b = self.breath_lp2 * self.gate_s * breath * 2.5;
            xcont += b * 4.0;
            direct += b * 0.35;
        }

        let feed = sh.get(P::Feed);
        if feed > 0.001 {
            xcont += (core + logic) * feed * self.gate_s;
        }

        // ── modal body ────────────────────────────────────────────────
        let xi = ximp + self.rattle_fb;
        let mut sum = 0.0;
        for m in &mut self.modes[..self.n_modes] {
            let re = m.c * m.re - m.s * m.im + m.bi * xi + m.bc * xcont;
            m.im = m.s * m.re + m.c * m.im;
            m.re = re;
            sum += m.im;
        }
        let mut body = sum;
        let rattle = sh.get(P::Rattle);
        if rattle > 0.001 {
            let th = 0.02 + 0.5 * (1.0 - rattle) * (1.0 - rattle);
            let e = sum.abs() - th;
            if e > 0.0 {
                let sign = if self.rng.next_u32() & 1 == 0 { 1.0 } else { -1.0 };
                let buzz = tanh_fast(e * 3.0) * sign * 0.25 * (1.0 + rattle);
                body += buzz;
                self.rattle_fb = buzz * 0.03;
            } else {
                self.rattle_fb = 0.0;
            }
        }
        self.body_prev = sum;

        // ── mix ───────────────────────────────────────────────────────
        let mut x = core * sh.get(P::Core) * 0.5 + body * sh.get(P::Body) * BODY_GAIN + logic * 0.5 + direct;

        let growl = sh.get(P::Growl);
        if growl > 0.001 {
            self.growl_ph = fract(self.growl_ph + c.growl_inc);
            let gm = 0.5 + 0.5 * (TAU * self.growl_ph).sin();
            let sub = if self.sub2 < 0.5 { 1.0 } else { 0.0 };
            let am = 1.0 - growl * (0.45 * gm + 0.35 * sub);
            let drive = 1.0 + growl * 4.0;
            x = tanh_fast(x * am * drive) / drive.sqrt();
        }

        let throat = sh.get(P::Throat);
        if throat > 0.001 {
            let f =
                self.formant[0].band_norm(x) + self.formant[1].band_norm(x) * 0.7 + self.formant[2].band_norm(x) * 0.45;
            x = x * (1.0 - throat) + f * throat * 2.2;
        }

        let rm = sh.get(P::RingMod);
        if rm > 0.001 {
            self.ring_ph = fract(self.ring_ph + c.ring_inc);
            x = x * (1.0 - rm) + x * (TAU * self.ring_ph).sin() * rm;
        }

        let screech = sh.get(P::Screech);
        if screech > 0.001 {
            let inp = x * 0.6 + self.sc_state * c.sc_fb;
            let b = self.screech.band_norm(inp);
            let s = tanh_fast(b * 2.0);
            self.sc_state = s;
            x += s * screech * 0.3;
        }

        let out = x * self.env * c.amp;
        (out * c.pan_l, out * c.pan_r)
    }
}
