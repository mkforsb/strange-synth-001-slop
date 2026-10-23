//! Dattorro plate reverb.
//!
//! J. Dattorro, "Effect Design Part 1: Reverberator and Other Filters",
//! JAES 45(9), 1997. Delay lengths below are the paper's values, specified at
//! 29761 Hz, rescaled to the running sample rate and the Size parameter.

use crate::dsp::{Delay, TAU};

const PAPER_SR: f32 = 29761.0;
const MAX_SIZE: f32 = 2.0;
const MAX_PREDELAY_S: f32 = 0.5;

const INPUT_DIFF: [(f32, f32); 4] = [(142.0, 0.75), (107.0, 0.75), (379.0, 0.625), (277.0, 0.625)];
/// Per tank half: (mod allpass, delay 1, allpass 2, delay 2)
const TANK: [[f32; 4]; 2] = [[672.0, 4453.0, 1800.0, 3720.0], [908.0, 4217.0, 2656.0, 3163.0]];
const EXCURSION: f32 = 16.0;
const DECAY_DIFF_1: f32 = 0.70;

/// Output taps: (source, offset, sign). Sources: 0=delay1, 1=allpass2, 2=delay2
/// of tank half `h`.
const TAPS_L: [(usize, usize, f32, f32); 7] = [
    (1, 0, 266.0, 1.0),
    (1, 0, 2974.0, 1.0),
    (1, 1, 1913.0, -1.0),
    (1, 2, 1996.0, 1.0),
    (0, 0, 1990.0, -1.0),
    (0, 1, 187.0, -1.0),
    (0, 2, 1066.0, -1.0),
];
const TAPS_R: [(usize, usize, f32, f32); 7] = [
    (0, 0, 353.0, 1.0),
    (0, 0, 3627.0, 1.0),
    (0, 1, 1228.0, -1.0),
    (0, 2, 2673.0, 1.0),
    (1, 0, 2111.0, -1.0),
    (1, 1, 335.0, -1.0),
    (1, 2, 121.0, -1.0),
];

struct Half {
    ap1: Delay,
    d1: Delay,
    ap2: Delay,
    d2: Delay,
    damp: f32,
    out: f32,
}

pub struct Dattorro {
    sr_scale: f32,
    pre: Delay,
    bw_state: f32,
    diff: [Delay; 4],
    half: [Half; 2],
    lfo: f32,
    lfo_inc: f32,
    // parameters
    pub predelay: f32,
    pub size: f32,
    pub decay: f32,
    pub damping: f32,
    pub modulation: f32,
    pub bandwidth: f32,
    size_s: f32,
}

#[inline]
fn allpass(d: &mut Delay, x: f32, len: f32, g: f32) -> f32 {
    let z = d.read_frac(len);
    let w = x + g * z;
    d.write(w);
    z - g * w
}

impl Dattorro {
    pub fn new(sr: f32) -> Self {
        let sr_scale = sr / PAPER_SR;
        let cap = |n: f32| Delay::new((n * sr_scale * MAX_SIZE) as usize + 64);
        let half = |t: [f32; 4]| Half {
            ap1: cap(t[0] + EXCURSION * 2.0),
            d1: cap(t[1]),
            ap2: cap(t[2]),
            d2: cap(t[3]),
            damp: 0.0,
            out: 0.0,
        };
        Dattorro {
            sr_scale,
            pre: Delay::new((sr * MAX_PREDELAY_S) as usize + 4),
            bw_state: 0.0,
            diff: INPUT_DIFF.map(|(n, _)| cap(n)),
            half: [half(TANK[0]), half(TANK[1])],
            lfo: 0.0,
            lfo_inc: TAU * 0.9 / sr,
            predelay: 0.02 * sr,
            size: 1.0,
            decay: 0.6,
            damping: 0.3,
            modulation: 0.3,
            bandwidth: 0.9995,
            size_s: 1.0,
        }
    }

    pub fn set_predelay_seconds(&mut self, s: f32, sr: f32) {
        self.predelay = (s * sr).clamp(1.0, self.pre.capacity() as f32 - 2.0);
    }

    pub fn clear(&mut self) {
        self.pre.clear();
        self.diff.iter_mut().for_each(Delay::clear);
        for h in &mut self.half {
            h.ap1.clear();
            h.d1.clear();
            h.ap2.clear();
            h.d2.clear();
            h.damp = 0.0;
            h.out = 0.0;
        }
        self.bw_state = 0.0;
    }

    #[inline]
    pub fn process(&mut self, l: f32, r: f32) -> (f32, f32) {
        // Glide size changes so tank lengths don't jump.
        self.size_s += (self.size - self.size_s) * 0.0005;
        let sc = self.sr_scale * self.size_s;
        let isc = self.sr_scale * self.size_s.min(1.0);

        self.pre.write((l + r) * 0.5);
        let x = self.pre.read_frac(self.predelay);
        self.bw_state += self.bandwidth * (x - self.bw_state);

        let mut s = self.bw_state;
        for (d, &(len, g)) in self.diff.iter_mut().zip(INPUT_DIFF.iter()) {
            s = allpass(d, s, len * isc, g);
        }

        self.lfo += self.lfo_inc;
        if self.lfo > TAU {
            self.lfo -= TAU;
        }
        let exc = EXCURSION * self.sr_scale * self.modulation;
        let mods = [self.lfo.sin() * exc, (self.lfo * 1.37 + 1.3).cos() * exc];
        let decay_diff_2 = (self.decay + 0.15).clamp(0.25, 0.5);
        let fb = [self.half[1].out, self.half[0].out];

        for (i, h) in self.half.iter_mut().enumerate() {
            let t = TANK[i];
            let inp = s + self.decay * fb[i];
            let a = allpass(&mut h.ap1, inp, t[0] * sc + EXCURSION * self.sr_scale + mods[i], -DECAY_DIFF_1);
            h.d1.write(a);
            let b = h.d1.read_frac(t[1] * sc);
            h.damp = b * (1.0 - self.damping) + h.damp * self.damping;
            let c = allpass(&mut h.ap2, h.damp * self.decay, t[2] * sc, decay_diff_2);
            h.d2.write(c);
            h.out = h.d2.read_frac(t[3] * sc);
        }

        let tap = |taps: &[(usize, usize, f32, f32); 7]| {
            let mut acc = 0.0;
            for &(hi, src, off, sign) in taps {
                let h = &self.half[hi];
                let line = match src {
                    0 => &h.d1,
                    1 => &h.ap2,
                    _ => &h.d2,
                };
                acc += sign * line.read((off * sc) as usize);
            }
            acc * 0.6
        };
        (tap(&TAPS_L), tap(&TAPS_R))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn impulse_decays_and_stays_finite() {
        let sr = 48000.0;
        let mut rv = Dattorro::new(sr);
        rv.decay = 0.7;
        let mut early = 0.0f32;
        let mut late = 0.0f32;
        for i in 0..(sr as usize * 6) {
            let x = if i == 0 { 1.0 } else { 0.0 };
            let (l, r) = rv.process(x, x);
            assert!(l.is_finite() && r.is_finite());
            if i < sr as usize {
                early = early.max(l.abs());
            }
            if i > sr as usize * 5 {
                late = late.max(l.abs());
            }
        }
        assert!(early > 0.01, "reverb produced no output");
        assert!(late < early * 0.05, "reverb did not decay: early {early} late {late}");
    }

    #[test]
    fn freeze_is_bounded() {
        let sr = 44100.0;
        let mut rv = Dattorro::new(sr);
        rv.decay = 0.999;
        rv.damping = 0.0;
        rv.size = 2.0;
        let mut rng = crate::dsp::Rng::new(9);
        for _ in 0..(sr as usize * 8) {
            let x = rng.bi() * 0.5;
            let (l, r) = rv.process(x, x);
            assert!(l.abs() < 50.0 && r.abs() < 50.0);
        }
    }
}
