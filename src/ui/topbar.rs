//! Title, preset browser, RND and transport-ish controls.

use dioxus::prelude::*;
use sa_engine::presets::CATEGORIES;

use super::keyboard;
use super::knob::LockIcon;
use crate::audio::{self, AudioStatus};
use crate::state::AppState;

#[component]
pub fn TopBar() -> Element {
    let mut st = use_context::<AppState>();
    let current = (st.preset)();
    let modified = (st.modified)();
    let presets = st.presets.read();
    let name = presets[current].name;
    let category = presets[current].category;
    let locked = st.locked_count();
    let mutation = (st.mutation)();
    let hold = (st.hold)();
    let volume = (st.volume)();
    let midi = (st.midi)();
    let octave = (st.octave)();
    let mut shaking = use_signal(|| false);

    let status = match (st.status)() {
        AudioStatus::Running { sample_rate } => format!("{:.1} kHz", sample_rate / 1000.0),
        AudioStatus::Starting => "booting…".into(),
        AudioStatus::Failed(e) => format!("audio failed: {e}"),
        AudioStatus::Off => "audio off".into(),
    };

    rsx! {
        header { class: "topbar",
            div { class: "logo",
                h1 { "data-text": "STRANGE ATTRACTOR", "STRANGE ATTRACTOR" }
                div { class: "tagline", "modal · chaotic · bytebeat · bowed · screeching" }
            }

            div { class: "preset-box",
                button { class: "nav", title: "Previous preset (←)", onclick: move |_| st.step_preset(-1), "◀" }
                div { class: "preset-main",
                    select {
                        value: "{current}",
                        onchange: move |e| {
                            if let Ok(i) = e.value().parse::<usize>() {
                                st.load_preset(i);
                            }
                        },
                        for cat in CATEGORIES {
                            optgroup { key: "{cat}", label: "{cat}",
                                for (i, p) in presets.iter().enumerate().filter(|(_, p)| p.category == cat) {
                                    option { key: "{i}", value: "{i}", selected: i == current, "{p.name}" }
                                }
                            }
                        }
                    }
                    div { class: "preset-meta",
                        span { class: "cat", "{category}" }
                        span { class: "num", "{current + 1:02} / {presets.len()}" }
                        if modified { span { class: "mod", "● edited" } }
                    }
                    div { class: "preset-name", title: "{name}", "{name}" }
                }
                button { class: "nav", title: "Next preset (→)", onclick: move |_| st.step_preset(1), "▶" }
            }

            div { class: "rnd-box",
                button {
                    class: "rnd",
                    class: if shaking() { "shake" },
                    title: "Mutate every unlocked parameter. Shift-click any knob to lock it.",
                    onclick: move |_| {
                        st.randomize();
                        shaking.set(true);
                        spawn(async move {
                            gloo_sleep(260).await;
                            shaking.set(false);
                        });
                    },
                    "RND"
                }
                div { class: "rnd-side",
                    label { title: "How far RND pulls each parameter toward a random value",
                        "mutate {mutation * 100.0:.0}%"
                        input {
                            r#type: "range", min: "0.02", max: "1", step: "0.01", value: "{mutation}",
                            oninput: move |e| st.mutation.set(e.value().parse().unwrap_or(0.5)),
                        }
                    }
                    if locked > 0 {
                        button { class: "unlock", title: "Unlock all parameters", onclick: move |_| st.unlock_all(), LockIcon {} " {locked} locked · clear" }
                    } else {
                        span { class: "hint", "shift-click knobs to lock" }
                    }
                }
            }

            div { class: "tools",
                button { title: "Reset unlocked parameters to the init patch", onclick: move |_| st.init_patch(), "INIT" }
                button {
                    class: if hold { "on" },
                    title: "Latch notes (MIDI sustain pedal also works)",
                    onclick: move |_| st.set_hold(!hold),
                    "HOLD"
                }
                button { title: "Silence everything (Esc)", onclick: move |_| st.panic(), "PANIC" }
                button { title: "Enable Web MIDI input (CC1 → Chaos, CC74 → Cutoff, CC64 → Hold)", onclick: move |_| keyboard::enable_midi(st), if midi.is_empty() { "MIDI" } else { "{midi}" } }
                div { class: "oct",
                    button { title: "Octave down (Z)", onclick: move |_| st.octave.set((octave - 1).max(0)), "−" }
                    span { "C{octave}" }
                    button { title: "Octave up (X)", onclick: move |_| st.octave.set((octave + 1).min(8)), "+" }
                }
                label { class: "vol", title: "Master volume",
                    "VOL"
                    input {
                        r#type: "range", min: "0", max: "1", step: "0.01", value: "{volume}",
                        oninput: move |e| {
                            let v: f32 = e.value().parse().unwrap_or(0.8);
                            st.volume.set(v);
                            audio::set_volume(v * v);
                        },
                    }
                }
                div { class: "meter", title: "{status}",
                    div { id: "meter-fill" }
                }
                span { class: "status", "{status}" }
            }
        }
    }
}

/// Await a timeout without pulling in a timer crate.
async fn gloo_sleep(ms: i32) {
    let p = js_sys::Promise::new(&mut |resolve, _| {
        if let Some(w) = web_sys::window() {
            let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms);
        }
    });
    let _ = wasm_bindgen_futures::JsFuture::from(p).await;
}
