//! Suggest a `Level` for every factory preset: render with the level pulled
//! down so the limiter stays idle, then aim for RMS ≈ 0.09 with peaks ≤ 0.7.
//! Prints `name<TAB>level_db`.
use sa_engine::P;
use sa_engine::presets::factory;
use sa_engine::render::render_phrase;

fn main() {
    for p in factory() {
        let mut patch = p.patch;
        let cur = P::Level.info().curve.map(patch[P::Level as usize]);
        let probe = -24.0f32;
        patch[P::Level as usize] = P::Level.info().curve.unmap(probe);
        let (_, s) = render_phrase(&patch, 48000.0, 4.0);
        let by_rms = 20.0 * (0.09 / s.rms.max(1e-6)).log10();
        let by_peak = 20.0 * (0.7 / s.peak.max(1e-6)).log10();
        let level = (probe + by_rms.min(by_peak)).clamp(-24.0, 6.0);
        println!("{}\t{:.1}\t(was {cur:.1}, rms-limited {})", p.name, level, by_rms < by_peak);
    }
}
