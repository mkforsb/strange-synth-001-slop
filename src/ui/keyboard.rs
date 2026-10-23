//! Note input: on-screen piano, computer keyboard and Web MIDI.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use dioxus::prelude::*;
use dioxus::web::WebEventExt;
use js_sys::{Function, Promise, Reflect, Uint8Array};
use sa_engine::P;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use crate::state::AppState;

const OCTAVES: i32 = 3;
const WHITE: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];

/// Computer keyboard → semitone offset from the current octave's C.
fn key_offset(code: &str) -> Option<i32> {
    Some(match code {
        "KeyA" => 0,
        "KeyW" => 1,
        "KeyS" => 2,
        "KeyE" => 3,
        "KeyD" => 4,
        "KeyF" => 5,
        "KeyT" => 6,
        "KeyG" => 7,
        "KeyY" => 8,
        "KeyH" => 9,
        "KeyU" => 10,
        "KeyJ" => 11,
        "KeyK" => 12,
        "KeyO" => 13,
        "KeyL" => 14,
        "KeyP" => 15,
        "Semicolon" => 16,
        "Quote" => 17,
        _ => return None,
    })
}

pub fn base_note(octave: i32) -> i32 {
    (octave + 1) * 12
}

/// Install window-level keyboard handlers (once).
pub fn install_computer_keyboard(st: AppState) {
    let Some(window) = web_sys::window() else { return };
    let held: Rc<RefCell<HashMap<String, i32>>> = Rc::default();

    let h = held.clone();
    let mut oct = st.octave;
    let down = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
        if e.ctrl_key() || e.meta_key() || e.alt_key() {
            return;
        }
        let tag =
            e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()).map(|el| el.tag_name()).unwrap_or_default();
        if matches!(tag.as_str(), "INPUT" | "SELECT" | "TEXTAREA") {
            return;
        }
        let code = e.code();
        if e.repeat() {
            if key_offset(&code).is_some() {
                e.prevent_default();
            }
            return;
        }
        match code.as_str() {
            "KeyZ" => {
                let o = *oct.peek();
                oct.set((o - 1).max(0));
            }
            "KeyX" => {
                let o = *oct.peek();
                oct.set((o + 1).min(8));
            }
            "Escape" => st.panic(),
            "ArrowLeft" => st.step_preset(-1),
            "ArrowRight" => st.step_preset(1),
            _ => {
                if let Some(off) = key_offset(&code) {
                    e.prevent_default();
                    let n = base_note(*oct.peek()) + off;
                    h.borrow_mut().insert(code, n);
                    st.note_on(n, 0.85);
                }
            }
        }
    });
    let up = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
        if let Some(n) = held.borrow_mut().remove(&e.code()) {
            st.note_off(n);
        }
    });
    let _ = window.add_event_listener_with_callback("keydown", down.as_ref().unchecked_ref());
    let _ = window.add_event_listener_with_callback("keyup", up.as_ref().unchecked_ref());
    down.forget();
    up.forget();
}

#[component]
pub fn Piano() -> Element {
    let st = use_context::<AppState>();
    let mut pressing = use_signal(|| None::<i32>);
    let notes = (st.notes)();
    let base = base_note((st.octave)());

    use_hook(move || {
        // Release the pointer-held note wherever the pointer is let go.
        let up = Closure::<dyn FnMut(web_sys::PointerEvent)>::new(move |_| {
            let held = *pressing.peek();
            if let Some(n) = held {
                st.note_off(n);
                pressing.set(None);
            }
        });
        if let Some(w) = web_sys::window() {
            let _ = w.add_event_listener_with_callback("pointerup", up.as_ref().unchecked_ref());
            let _ = w.add_event_listener_with_callback("pointercancel", up.as_ref().unchecked_ref());
        }
        up.forget();
    });

    let whites: Vec<i32> =
        (0..OCTAVES).flat_map(|o| WHITE.iter().map(move |w| o * 12 + w)).chain([OCTAVES * 12]).collect();
    let n_white = whites.len() as f32;
    let blacks: Vec<(i32, f32)> = (0..OCTAVES)
        .flat_map(|o| {
            [(1, 1.0), (3, 2.0), (6, 4.0), (8, 5.0), (10, 6.0)]
                .map(move |(s, pos)| (o * 12 + s, (o as f32 * 7.0 + pos) / n_white))
        })
        .collect();

    let velocity = |e: &Event<PointerData>| -> f32 {
        let ev: web_sys::PointerEvent = e.as_web_event();
        let h = ev
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            .map(|el| el.client_height() as f64)
            .unwrap_or(100.0);
        (0.35 + 0.65 * (e.element_coordinates().y / h.max(1.0))).clamp(0.2, 1.0) as f32
    };

    let mut press = move |n: i32, vel: f32| {
        if let Some(old) = *pressing.peek() {
            if old == n {
                return;
            }
            st.note_off(old);
        }
        st.note_on(n, vel);
        pressing.set(Some(n));
    };

    let key = move |n: i32, class: &'static str, style: String| {
        let note = base + n;
        let lit = (0..128).contains(&note) && notes & (1u128 << note) != 0;
        let label = if n % 12 == 0 { format!("C{}", note / 12 - 1) } else { String::new() };
        rsx! {
            div {
                key: "{n}",
                class: "{class}",
                class: if lit { "lit" },
                style: "{style}",
                onpointerdown: move |e: Event<PointerData>| {
                    e.prevent_default();
                    // Touch pointers are implicitly captured; release so
                    // sliding across keys plays them.
                    let ev: web_sys::PointerEvent = e.as_web_event();
                    if let Some(el) = ev.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) {
                        let _ = el.release_pointer_capture(ev.pointer_id());
                    }
                    press(note, velocity(&e));
                },
                onpointerenter: move |e: Event<PointerData>| {
                    if pressing.peek().is_some() {
                        press(note, velocity(&e));
                    }
                },
                span { "{label}" }
            }
        }
    };

    rsx! {
        div { class: "piano",
            for n in whites.iter().copied() {
                {key(n, "white", String::new())}
            }
            for (n, left) in blacks.iter().copied() {
                {key(n, "black", format!("left: calc({:.4}% - var(--bw) / 2)", left * 100.0))}
            }
        }
    }
}

// ── Web MIDI ─────────────────────────────────────────────────────────────

fn handle_midi(st: AppState, data: &[u8]) {
    let [status, d1, d2] = match data {
        [s, a, b, ..] => [*s, *a, *b],
        [s, a] => [*s, *a, 0],
        _ => return,
    };
    match status & 0xF0 {
        0x90 if d2 > 0 => st.note_on(d1 as i32, d2 as f32 / 127.0),
        0x80 | 0x90 => st.note_off(d1 as i32),
        0xB0 => match d1 {
            1 => st.set_param(P::Chaos as usize, d2 as f32 / 127.0),
            74 => st.set_param(P::Cutoff as usize, d2 as f32 / 127.0),
            64 => st.set_hold(d2 >= 64),
            120 | 123 => st.panic(),
            _ => {}
        },
        _ => {}
    }
}

fn attach_inputs(st: AppState, access: &JsValue) -> usize {
    let Ok(inputs) = Reflect::get(access, &"inputs".into()) else { return 0 };
    let Ok(values) = Reflect::get(&inputs, &"values".into()).and_then(|f| f.dyn_into::<Function>()?.call0(&inputs))
    else {
        return 0;
    };
    let mut count = 0;
    if let Ok(Some(iter)) = js_sys::try_iter(&values) {
        for input in iter.flatten() {
            let cb = Closure::<dyn FnMut(JsValue)>::new(move |ev: JsValue| {
                if let Ok(d) = Reflect::get(&ev, &"data".into()) {
                    handle_midi(st, &Uint8Array::new(&d).to_vec());
                }
            });
            let _ = Reflect::set(&input, &"onmidimessage".into(), cb.as_ref());
            cb.forget();
            count += 1;
        }
    }
    count
}

pub fn enable_midi(st: AppState) {
    let mut label = st.midi;
    let Some(nav) = web_sys::window().map(|w| w.navigator()) else { return };
    let request = Reflect::get(&nav, &"requestMIDIAccess".into()).ok().and_then(|f| f.dyn_into::<Function>().ok());
    let Some(request) = request else {
        label.set("no Web MIDI".into());
        return;
    };
    let Ok(promise) = request.call0(&nav).and_then(|p| p.dyn_into::<Promise>()) else {
        label.set("MIDI error".into());
        return;
    };
    label.set("MIDI…".into());
    wasm_bindgen_futures::spawn_local(async move {
        match JsFuture::from(promise).await {
            Ok(access) => {
                let n = attach_inputs(st, &access);
                label.set(format!("MIDI ×{n}"));
                let acc = access.clone();
                let on_change = Closure::<dyn FnMut(JsValue)>::new(move |_| {
                    let n = attach_inputs(st, &acc);
                    label.set(format!("MIDI ×{n}"));
                });
                let _ = Reflect::set(&access, &"onstatechange".into(), on_change.as_ref());
                on_change.forget();
            }
            Err(_) => label.set("MIDI denied".into()),
        }
    });
}
