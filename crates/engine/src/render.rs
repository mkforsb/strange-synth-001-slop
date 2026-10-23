//! Offline rendering helpers, used by tests and the `render` example.

use crate::Engine;
use crate::params::Patch;

pub struct RenderStats {
    pub peak: f32,
    pub rms: f32,
    pub tail_rms: f32,
    pub nan: bool,
    pub seconds_per_second: f64,
}

/// Plays a short phrase (single note, then a chord) and returns the
/// interleaved stereo output.
pub fn render_phrase(patch: &Patch, sr: f32, seconds: f32) -> (Vec<f32>, RenderStats) {
    let mut e = Engine::new(sr);
    e.set_patch(patch);
    e.snap_smoothing();
    let total = (sr * seconds) as usize;
    let block = 128;
    let mut out = Vec::with_capacity(total * 2);
    let events: [(f32, bool, u8); 8] = [
        (0.0, true, 48),
        (seconds * 0.30, false, 48),
        (seconds * 0.40, true, 60),
        (seconds * 0.40, true, 63),
        (seconds * 0.40, true, 67),
        (seconds * 0.75, false, 60),
        (seconds * 0.75, false, 63),
        (seconds * 0.75, false, 67),
    ];
    let mut next = 0;
    let start = std::time::Instant::now();
    let mut nan = false;
    let mut n = 0;
    while n < total {
        let t = n as f32 / sr;
        while next < events.len() && events[next].0 <= t {
            let (_, on, note) = events[next];
            if on {
                e.note_on(note, 0.9)
            } else {
                e.note_off(note)
            }
            next += 1;
        }
        let frames = block.min(total - n);
        let buf = e.process(frames);
        for i in 0..frames {
            let (l, r) = (buf[i], buf[frames + i]);
            nan |= !l.is_finite() || !r.is_finite();
            out.push(l);
            out.push(r);
        }
        n += frames;
    }
    let elapsed = start.elapsed().as_secs_f64();
    let peak = out.iter().fold(0.0f32, |a, &s| a.max(s.abs()));
    let rms = (out.iter().map(|s| s * s).sum::<f32>() / out.len().max(1) as f32).sqrt();
    let tail = &out[out.len() * 9 / 10..];
    let tail_rms = (tail.iter().map(|s| s * s).sum::<f32>() / tail.len().max(1) as f32).sqrt();
    (out, RenderStats { peak, rms, tail_rms, nan, seconds_per_second: elapsed / seconds as f64 })
}

/// Minimal 16-bit stereo WAV encoder.
pub fn wav_bytes(interleaved: &[f32], sr: u32) -> Vec<u8> {
    let data_len = (interleaved.len() * 2) as u32;
    let mut b = Vec::with_capacity(44 + data_len as usize);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data_len).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&sr.to_le_bytes());
    b.extend_from_slice(&(sr * 4).to_le_bytes());
    b.extend_from_slice(&4u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data_len.to_le_bytes());
    for s in interleaved {
        b.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::Rng;
    use crate::params::{NUM_PARAMS, default_patch};

    #[test]
    fn default_patch_makes_sound() {
        let (_, s) = render_phrase(&default_patch(), 48000.0, 2.0);
        assert!(!s.nan);
        assert!(s.rms > 0.005, "too quiet: {}", s.rms);
        assert!(s.peak <= 1.0);
    }

    #[test]
    fn random_patches_are_finite_and_bounded() {
        let mut rng = Rng::new(1234);
        for _ in 0..40 {
            let mut p = [0.0; NUM_PARAMS];
            p.iter_mut().for_each(|v| *v = rng.uni());
            let (_, s) = render_phrase(&p, 44100.0, 1.0);
            assert!(!s.nan, "NaN for patch {p:?}");
            assert!(s.peak <= 1.0, "peak {} for patch {p:?}", s.peak);
        }
    }
}
