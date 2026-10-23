//! Small DSP building blocks.

pub const TAU: f32 = std::f32::consts::TAU;
pub const PI: f32 = std::f32::consts::PI;

/// xorshift32 — cheap, deterministic, good enough for audio noise.
#[derive(Clone, Debug)]
pub struct Rng(u32);

impl Rng {
    pub fn new(seed: u32) -> Self {
        Rng(seed.wrapping_mul(0x9E37_79B9).max(1))
    }

    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }

    /// Uniform in [0, 1).
    #[inline]
    pub fn uni(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 * (1.0 / 16_777_216.0)
    }

    /// Uniform in [-1, 1).
    #[inline]
    pub fn bi(&mut self) -> f32 {
        self.uni() * 2.0 - 1.0
    }
}

/// Deterministic hash of an integer to [0, 1).
#[inline]
pub fn hash01(n: u32) -> f32 {
    let mut x = n.wrapping_mul(0x2C1B_3C6D) ^ 0x297A_2D39;
    x ^= x >> 15;
    x = x.wrapping_mul(0x85EB_CA6B);
    x ^= x >> 13;
    (x >> 8) as f32 * (1.0 / 16_777_216.0)
}

/// Padé tanh approximation, exact saturation at ±3.
#[inline]
pub fn tanh_fast(x: f32) -> f32 {
    let x = x.clamp(-3.0, 3.0);
    let x2 = x * x;
    x * (27.0 + x2) / (27.0 + 9.0 * x2)
}

#[inline]
pub fn mtof(note: f32) -> f32 {
    440.0 * ((note - 69.0) / 12.0).exp2()
}

#[inline]
pub fn db_to_gain(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// One-pole coefficient for a given time constant in seconds.
#[inline]
pub fn tau_coef(tau: f32, sr: f32) -> f32 {
    if tau <= 0.0 { 1.0 } else { 1.0 - (-1.0 / (tau * sr)).exp() }
}

/// One-pole coefficient for a cutoff frequency.
#[inline]
pub fn hz_coef(hz: f32, sr: f32) -> f32 {
    1.0 - (-TAU * hz.min(sr * 0.49) / sr).exp()
}

/// Topology-preserving-transform state variable filter (Simper).
#[derive(Clone, Copy, Debug, Default)]
pub struct Svf {
    ic1: f32,
    ic2: f32,
    a1: f32,
    a2: f32,
    a3: f32,
    k: f32,
}

impl Svf {
    pub fn set(&mut self, hz: f32, q: f32, sr: f32) {
        let g = (PI * hz.clamp(5.0, sr * 0.48) / sr).tan();
        self.k = 1.0 / q.max(0.05);
        self.a1 = 1.0 / (1.0 + g * (g + self.k));
        self.a2 = g * self.a1;
        self.a3 = g * self.a2;
    }

    /// Returns (low, band, high). `band` has peak gain Q; use [`Svf::band_norm`]
    /// for unity peak.
    #[inline]
    pub fn process(&mut self, v0: f32) -> (f32, f32, f32) {
        let v3 = v0 - self.ic2;
        let v1 = self.a1 * self.ic1 + self.a2 * v3;
        let v2 = self.ic2 + self.a2 * self.ic1 + self.a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1;
        self.ic2 = 2.0 * v2 - self.ic2;
        (v2, v1, v0 - self.k * v1 - v2)
    }

    #[inline]
    pub fn band_norm(&mut self, v0: f32) -> f32 {
        let (_, b, _) = self.process(v0);
        b * self.k
    }

    pub fn reset(&mut self) {
        self.ic1 = 0.0;
        self.ic2 = 0.0;
    }
}

/// Stereo RBJ biquad, transposed direct form II.
#[derive(Clone, Copy, Debug)]
pub struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: [f32; 2],
    z2: [f32; 2],
}

impl Default for Biquad {
    fn default() -> Self {
        Biquad { b0: 1.0, b1: 0.0, b2: 0.0, a1: 0.0, a2: 0.0, z1: [0.0; 2], z2: [0.0; 2] }
    }
}

pub enum BiquadKind {
    LowPass,
    HighPass,
    LowShelf(f32),
    HighShelf(f32),
    Peak(f32),
}

impl Biquad {
    pub fn set(&mut self, kind: BiquadKind, hz: f32, q: f32, sr: f32) {
        let w = TAU * hz.clamp(5.0, sr * 0.49) / sr;
        let (sw, cw) = w.sin_cos();
        let alpha = sw / (2.0 * q);
        let (b0, b1, b2, a0, a1, a2) = match kind {
            BiquadKind::LowPass => {
                let b = (1.0 - cw) / 2.0;
                (b, 1.0 - cw, b, 1.0 + alpha, -2.0 * cw, 1.0 - alpha)
            }
            BiquadKind::HighPass => {
                let b = (1.0 + cw) / 2.0;
                (b, -(1.0 + cw), b, 1.0 + alpha, -2.0 * cw, 1.0 - alpha)
            }
            BiquadKind::Peak(db) => {
                let a = 10f32.powf(db / 40.0);
                (1.0 + alpha * a, -2.0 * cw, 1.0 - alpha * a, 1.0 + alpha / a, -2.0 * cw, 1.0 - alpha / a)
            }
            BiquadKind::LowShelf(db) => {
                let a = 10f32.powf(db / 40.0);
                let s = 2.0 * a.sqrt() * alpha;
                (
                    a * ((a + 1.0) - (a - 1.0) * cw + s),
                    2.0 * a * ((a - 1.0) - (a + 1.0) * cw),
                    a * ((a + 1.0) - (a - 1.0) * cw - s),
                    (a + 1.0) + (a - 1.0) * cw + s,
                    -2.0 * ((a - 1.0) + (a + 1.0) * cw),
                    (a + 1.0) + (a - 1.0) * cw - s,
                )
            }
            BiquadKind::HighShelf(db) => {
                let a = 10f32.powf(db / 40.0);
                let s = 2.0 * a.sqrt() * alpha;
                (
                    a * ((a + 1.0) + (a - 1.0) * cw + s),
                    -2.0 * a * ((a - 1.0) + (a + 1.0) * cw),
                    a * ((a + 1.0) + (a - 1.0) * cw - s),
                    (a + 1.0) - (a - 1.0) * cw + s,
                    2.0 * ((a - 1.0) - (a + 1.0) * cw),
                    (a + 1.0) - (a - 1.0) * cw - s,
                )
            }
        };
        self.b0 = b0 / a0;
        self.b1 = b1 / a0;
        self.b2 = b2 / a0;
        self.a1 = a1 / a0;
        self.a2 = a2 / a0;
    }

    #[inline]
    pub fn process(&mut self, ch: usize, x: f32) -> f32 {
        let y = self.b0 * x + self.z1[ch];
        self.z1[ch] = self.b1 * x - self.a1 * y + self.z2[ch];
        self.z2[ch] = self.b2 * x - self.a2 * y;
        y
    }

    pub fn reset(&mut self) {
        self.z1 = [0.0; 2];
        self.z2 = [0.0; 2];
    }
}

/// Which tap of the ladder to listen to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LadderMode {
    Lp12,
    Lp24,
    Band,
    High,
}

impl LadderMode {
    pub fn from_index(i: usize) -> Self {
        match i {
            0 => LadderMode::Lp12,
            1 => LadderMode::Lp24,
            2 => LadderMode::Band,
            _ => LadderMode::High,
        }
    }
}

/// Non-linear Moog transistor ladder (Huovilainen-style, 2× oversampled).
/// Feedback always comes from the 4th stage; the other responses are
/// Oberheim-Xpander-style mixes of the stage taps, so every mode keeps the
/// characteristic ladder resonance.
#[derive(Clone, Copy, Debug, Default)]
pub struct Ladder {
    s: [f32; 4],
    t: [f32; 4],
}

impl Ladder {
    /// `g`: per-oversampled-step coefficient, `k`: feedback 0..~4.2
    #[inline]
    pub fn process(&mut self, x: f32, g: f32, k: f32, mode: LadderMode) -> f32 {
        let mut a = 0.0;
        // Passband loss compensation as resonance rises.
        let x = x * (1.0 + 0.5 * k);
        for _ in 0..2 {
            a = tanh_fast(x - k * self.s[3]);
            self.s[0] += g * (a - self.t[0]);
            self.t[0] = tanh_fast(self.s[0]);
            self.s[1] += g * (self.t[0] - self.t[1]);
            self.t[1] = tanh_fast(self.s[1]);
            self.s[2] += g * (self.t[1] - self.t[2]);
            self.t[2] = tanh_fast(self.s[2]);
            self.s[3] += g * (self.t[2] - self.t[3]);
            self.t[3] = tanh_fast(self.s[3]);
        }
        let [y1, y2, y3, y4] = self.s;
        match mode {
            LadderMode::Lp12 => y2,
            LadderMode::Lp24 => y4,
            LadderMode::Band => 4.0 * (y2 - 2.0 * y3 + y4),
            LadderMode::High => a - 2.0 * y1 + y2,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Power-of-two ring buffer delay line.
#[derive(Clone, Debug)]
pub struct Delay {
    buf: Vec<f32>,
    mask: usize,
    w: usize,
}

impl Delay {
    pub fn new(max_len: usize) -> Self {
        let n = (max_len + 2).next_power_of_two();
        Delay { buf: vec![0.0; n], mask: n - 1, w: 0 }
    }

    #[inline]
    pub fn write(&mut self, x: f32) {
        self.buf[self.w] = x;
        self.w = (self.w + 1) & self.mask;
    }

    /// Sample written `d` writes ago (d >= 1 is the most recent).
    #[inline]
    pub fn read(&self, d: usize) -> f32 {
        self.buf[(self.w.wrapping_sub(d.max(1))) & self.mask]
    }

    #[inline]
    pub fn read_frac(&self, d: f32) -> f32 {
        let d = d.max(1.0);
        let i = d as usize;
        let f = d - i as f32;
        lerp(self.read(i), self.read(i + 1), f)
    }

    pub fn capacity(&self) -> usize {
        self.mask
    }

    pub fn clear(&mut self) {
        self.buf.iter_mut().for_each(|s| *s = 0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tanh_fast_is_close() {
        for i in -40..=40 {
            let x = i as f32 * 0.1;
            assert!((tanh_fast(x) - x.tanh()).abs() < 0.03, "x={x}");
        }
    }

    #[test]
    fn ladder_is_stable_at_full_resonance() {
        let sr = 48000.0;
        for mode in [LadderMode::Lp12, LadderMode::Lp24, LadderMode::Band, LadderMode::High] {
            let mut l = Ladder::default();
            let g = hz_coef(2000.0, sr * 2.0);
            let mut rng = Rng::new(3);
            let mut peak: f32 = 0.0;
            for _ in 0..48000 {
                let y = l.process(rng.bi() * 2.0, g, 4.2, mode);
                assert!(y.is_finite());
                peak = peak.max(y.abs());
            }
            assert!(peak < 20.0, "{mode:?} peak {peak}");
        }
    }

    #[test]
    fn delay_reads_back() {
        let mut d = Delay::new(10);
        for i in 0..5 {
            d.write(i as f32);
        }
        assert_eq!(d.read(1), 4.0);
        assert_eq!(d.read(3), 2.0);
        assert!((d.read_frac(1.5) - 3.5).abs() < 1e-6);
    }
}
