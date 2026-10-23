//! Strange Attractor — a very strange synthesizer.
//!
//! UI in Dioxus; all DSP in Rust running inside an AudioWorklet (see
//! `crates/engine` and `crates/worklet`).

use dioxus::prelude::*;
use sa_engine::NUM_PARAMS;

mod audio;
mod state;
mod ui;

use audio::AudioStatus;
use state::AppState;
use ui::keyboard::{Piano, install_computer_keyboard};
use ui::panels::Panels;
use ui::topbar::TopBar;
use ui::visuals::Visuals;

const CSS: &str = include_str!("../assets/main.css");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let st = use_context_provider(AppState::new);
    use_hook(move || install_computer_keyboard(st));
    use_hook(audio::install_gesture_resume);
    let status = (st.status)();
    let started = matches!(status, AudioStatus::Running { .. });

    rsx! {
        document::Title { "Strange Attractor" }
        style { {CSS} }
        div { class: "app",
            TopBar {}
            Visuals {}
            Panels {}
            div { class: "keys-wrap",
                Piano {}
                div { class: "help",
                    "Play: A W S E D F T G Y H U J K O L P · Z/X octave · ←/→ presets · Esc panic · "
                    "Shift-click a knob to lock it from RND · double-click resets · Ctrl-drag for fine"
                }
            }
        }
        if !started {
            Splash { status }
        }
    }
}

#[component]
fn Splash(status: AudioStatus) -> Element {
    let st = use_context::<AppState>();
    let n_presets = st.presets.read().len();
    let (label, detail) = match &status {
        AudioStatus::Starting => ("WAKING…", String::new()),
        AudioStatus::Suspended => ("TAP TO START AUDIO", "the browser is holding audio paused".into()),
        AudioStatus::Failed(e) => ("TRY AGAIN", format!("audio failed: {e}")),
        _ => ("AWAKEN THE MACHINE", String::new()),
    };
    rsx! {
        div { class: "splash",
            div { class: "splash-inner",
                h1 { class: "glitch", "data-text": "STRANGE ATTRACTOR", "STRANGE ATTRACTOR" }
                p { "a very strange synthesizer · {NUM_PARAMS} parameters · {n_presets} abominations" }
                button {
                    id: "start",
                    onclick: move |_| {
                        let patch = st.patch().to_vec();
                        let vol = *st.volume.peek();
                        // Synchronously, inside the gesture: see `audio::start`.
                        audio::start(patch, vol * vol, st.status);
                    },
                    "{label}"
                }
                if !detail.is_empty() { p { class: "err", "{detail}" } }
            }
        }
    }
}
