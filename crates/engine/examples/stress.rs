//! Worst case CPU: 8 held voices with every expensive feature engaged.
use sa_engine::{Engine, P};
fn main() {
    let sr = 48000.0;
    let mut e = Engine::new(sr);
    for (p, v) in [
        (P::Shimmer, 1.0),
        (P::Bow, 1.0),
        (P::Breath, 1.0),
        (P::Throat, 1.0),
        (P::Screech, 1.0),
        (P::Growl, 1.0),
        (P::Logic, 1.0),
        (P::Feed, 1.0),
        (P::FmDepth, 1.0),
        (P::Fold, 1.0),
        (P::Undertone, 1.0),
        (P::RingMod, 1.0),
        (P::Rattle, 1.0),
        (P::Glitch, 1.0),
        (P::Crush, 0.5),
        (P::Grit, 1.0),
        (P::Chaos, 1.0),
    ] {
        e.set_param(p as usize, v);
    }
    for n in 0..8 {
        e.note_on(40 + n * 3, 1.0);
    }
    let t = std::time::Instant::now();
    let secs = 10.0;
    for _ in 0..(sr * secs / 128.0) as usize {
        e.process(128);
    }
    println!("worst case: {:.3}x realtime ({} voices)", t.elapsed().as_secs_f64() / secs as f64, e.active_voices());
}
