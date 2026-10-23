//! Knobs and choice widgets. Shift-click toggles a parameter's lock.

use dioxus::prelude::*;
use dioxus::web::WebEventExt;
use sa_engine::params::{Curve, PARAMS, display};
use wasm_bindgen::JsCast;

use crate::state::AppState;

const SWEEP: f32 = 270.0;
const R: f32 = 19.0;
const C: f32 = 26.0;

fn polar(deg: f32) -> (f32, f32) {
    let a = deg.to_radians();
    (C + R * a.sin(), C - R * a.cos())
}

fn arc(from: f32, to: f32) -> String {
    let (a, b) = if from <= to { (from, to) } else { (to, from) };
    if (b - a).abs() < 0.01 {
        return String::new();
    }
    let (x0, y0) = polar(a);
    let (x1, y1) = polar(b);
    let large = if b - a > 180.0 { 1 } else { 0 };
    format!("M {x0:.2} {y0:.2} A {R} {R} 0 {large} 1 {x1:.2} {y1:.2}")
}

/// Small padlock glyph (emoji fonts are not universal).
#[component]
pub fn LockIcon() -> Element {
    rsx! {
        svg { class: "lock-icon", view_box: "0 0 10 12",
            path { d: "M2.5 5.5 V3.8 a2.5 2.5 0 0 1 5 0 V5.5", fill: "none", stroke: "currentColor", stroke_width: "1.4" }
            rect { x: "1", y: "5.5", width: "8", height: "6", rx: "1.2", fill: "currentColor" }
        }
    }
}

/// Bipolar parameters draw their arc from the centre.
fn is_bipolar(i: usize) -> bool {
    matches!(PARAMS[i].curve, Curve::Lin(a, b) if a < 0.0 && b > 0.0)
}

fn capture_pointer(e: &Event<PointerData>) {
    let ev: web_sys::PointerEvent = e.as_web_event();
    if let Some(el) = ev.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) {
        let _ = el.set_pointer_capture(ev.pointer_id());
    }
}

#[component]
pub fn Knob(index: usize) -> Element {
    let st = use_context::<AppState>();
    let value = st.params[index];
    let locked = st.locks[index];
    let mut drag = use_signal(|| None::<(f64, f32)>);
    let info = &PARAMS[index];

    let v = value();
    let start = -SWEEP / 2.0;
    let angle = start + SWEEP * v;
    let origin = if is_bipolar(index) { 0.0 } else { start };
    let track = arc(start, start + SWEEP);
    let fill = arc(origin, angle);
    let (px, py) = polar(angle);
    let (ix, iy) = (C + (px - C) * 0.55, C + (py - C) * 0.55);
    let text = display(index, v);
    let title =
        format!("{}\n\nDrag (Ctrl = fine) · wheel · double-click resets · Shift-click locks for RND", info.desc);
    let dragging = drag().is_some();

    rsx! {
        div {
            class: "knob",
            class: if locked() { "locked" },
            class: if dragging { "dragging" },
            title: "{title}",
            onpointerdown: move |e: Event<PointerData>| {
                if e.modifiers().shift() {
                    st.toggle_lock(index);
                    return;
                }
                e.prevent_default();
                capture_pointer(&e);
                drag.set(Some((e.client_coordinates().y, *value.peek())));
            },
            onpointermove: move |e: Event<PointerData>| {
                if let Some((y0, v0)) = drag() {
                    let fine = e.modifiers().ctrl() || e.modifiers().meta();
                    let scale = if fine { 900.0 } else { 180.0 };
                    let dv = (y0 - e.client_coordinates().y) as f32 / scale;
                    st.set_param(index, v0 + dv);
                }
            },
            onpointerup: move |_| {
                if drag().is_some() {
                    drag.set(None);
                    st.save();
                }
            },
            onpointercancel: move |_| drag.set(None),
            ondoubleclick: move |_| {
                st.set_param(index, PARAMS[index].default_norm());
                st.save();
            },
            onwheel: move |e: Event<WheelData>| {
                e.prevent_default();
                let dy = e.delta().strip_units().y as f32;
                st.set_param(index, *value.peek() - dy.signum() * 0.01);
            },
            svg {
                class: "knob-svg",
                view_box: "0 0 52 52",
                path { class: "knob-track", d: "{track}" }
                path { class: "knob-fill", d: "{fill}" }
                circle { class: "knob-cap", cx: "{C}", cy: "{C}", r: "13" }
                line { class: "knob-pointer", x1: "{ix:.2}", y1: "{iy:.2}", x2: "{px:.2}", y2: "{py:.2}" }
                if locked() {
                    circle { class: "knob-lock-ring", cx: "{C}", cy: "{C}", r: "24" }
                }
            }
            if locked() {
                span { class: "lock-badge", LockIcon {} }
            }
            div { class: "knob-label", "{info.name}" }
            div { class: "knob-value", "{text}" }
        }
    }
}

/// Segmented selector for `Curve::Choice` parameters.
#[component]
pub fn ChoiceWidget(index: usize) -> Element {
    let st = use_context::<AppState>();
    let value = st.params[index];
    let locked = st.locks[index];
    let info = &PARAMS[index];
    let opts = info.curve.choices().unwrap_or(&[]);
    let current = info.curve.map(value()) as usize;
    let n = opts.len() as f32;
    let title = format!("{}\n\nShift-click locks for RND", info.desc);

    rsx! {
        div {
            class: "choice",
            class: if locked() { "locked" },
            title: "{title}",
            div { class: "knob-label",
                "{info.name}"
                if locked() { span { class: "lock-inline", LockIcon {} } }
            }
            div { class: "choice-options",
                for (k, name) in opts.iter().enumerate() {
                    button {
                        key: "{k}",
                        class: if k == current { "on" },
                        onclick: move |e: Event<MouseData>| {
                            if e.modifiers().shift() {
                                st.toggle_lock(index);
                            } else {
                                st.set_param(index, (k as f32 + 0.5) / n);
                                st.save();
                            }
                        },
                        "{name}"
                    }
                }
            }
        }
    }
}
