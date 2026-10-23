//! Render every factory preset and print loudness stats; set WAV_DIR to
//! also write WAV files. `cargo run --release -p sa-engine --example presets`
use sa_engine::presets::factory;
use sa_engine::render::{render_phrase, wav_bytes};

fn main() {
    let dir = std::env::var("WAV_DIR").ok();
    for (i, p) in factory().iter().enumerate() {
        let (buf, s) = render_phrase(&p.patch, 48000.0, 4.0);
        println!(
            "{i:>2} {:<46} peak {:.3} rms {:.4} tail {:.4} cpu {:.3}x{}",
            p.name,
            s.peak,
            s.rms,
            s.tail_rms,
            s.seconds_per_second,
            if s.nan { " NAN" } else { "" }
        );
        if let Some(d) = &dir {
            std::fs::write(format!("{d}/{i:02}.wav"), wav_bytes(&buf, 48000)).unwrap();
        }
    }
}
