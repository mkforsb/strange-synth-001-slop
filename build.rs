//! Builds `crates/worklet` to a bare wasm module and exposes its path as
//! `DSP_WASM` so the app can embed it with `include_bytes!`.
//!
//! The nested cargo uses its own target directory (avoiding a lock on the
//! outer build's) and a scrubbed environment so flags meant for the app
//! build (dx linker interception, RUSTFLAGS, ...) don't leak into it.

use std::{env, path::PathBuf, process::Command};

fn main() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    for dir in ["crates/engine/src", "crates/worklet/src", "crates/engine/Cargo.toml", "crates/worklet/Cargo.toml"] {
        println!("cargo:rerun-if-changed={dir}");
    }

    let target_dir = root.join("target").join("dsp");
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let mut cmd = Command::new(cargo);
    cmd.current_dir(&root)
        .args(["build", "-p", "sa-worklet", "--release", "--target", "wasm32-unknown-unknown", "--target-dir"])
        .arg(&target_dir);
    for (key, _) in env::vars() {
        let leaky = (key.starts_with("CARGO_") && key != "CARGO_HOME")
            || key.starts_with("DX_")
            || matches!(key.as_str(), "RUSTFLAGS" | "RUSTC_WRAPPER" | "RUSTC_WORKSPACE_WRAPPER" | "RUSTDOCFLAGS");
        if leaky {
            cmd.env_remove(key);
        }
    }
    let status = cmd.status().expect("failed to spawn cargo for the DSP worklet");
    assert!(status.success(), "building the DSP worklet (sa-worklet) failed");

    let wasm = target_dir.join("wasm32-unknown-unknown/release/sa_worklet.wasm");
    let dest = out_dir.join("dsp.wasm");
    std::fs::copy(&wasm, &dest).expect("copy dsp.wasm");
    println!("cargo:rustc-env=DSP_WASM={}", dest.display());
}
