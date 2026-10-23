//! Probe: render the default patch with parameters overridden (real units),
//! printing level statistics.
//!
//! `cargo run --release --example probe -- Strike=0 Bow=1`
//! With no arguments, sweeps every parameter to both extremes.
use sa_engine::params::{NUM_PARAMS, PARAMS, default_patch};
use sa_engine::render::{render_phrase, wav_bytes};

fn report(label: &str, p: &sa_engine::Patch) {
    let (buf, s) = render_phrase(p, 48000.0, 3.0);
    println!(
        "{label:<28} peak {:.3} rms {:.4} tail {:.4} cpu {:.3}x{}",
        s.peak,
        s.rms,
        s.tail_rms,
        s.seconds_per_second,
        if s.nan { " NAN" } else { "" }
    );
    if let Ok(dir) = std::env::var("WAV_DIR") {
        let _ = std::fs::write(format!("{dir}/{}.wav", label.replace(['=', ' ', '/'], "_")), wav_bytes(&buf, 48000));
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut p = default_patch();
    if args.is_empty() {
        report("DEFAULT", &p);
        for i in 0..NUM_PARAMS {
            for v in [0.0f32, 1.0] {
                let mut q = p;
                q[i] = v;
                report(&format!("{}={v}", PARAMS[i].key), &q);
            }
        }
        return;
    }
    for a in &args {
        let (k, v) = a.split_once('=').expect("KEY=VALUE");
        let i = PARAMS.iter().position(|x| x.key == k).unwrap_or_else(|| panic!("unknown param {k}"));
        p[i] = PARAMS[i].curve.unmap(v.parse().unwrap());
    }
    report(&args.join(" "), &p);
}
