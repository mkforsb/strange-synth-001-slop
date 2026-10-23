//! Canvas visuals, drawn from a requestAnimationFrame loop:
//!
//! * **Attractor** — one Lorenz trajectory per sounding voice, rotating in
//!   3D. The Chaos knob bends ρ from 20 to 32: below ~24.7 the system spirals
//!   into a fixed point (calm), above it the butterfly blooms. Attractor speeds it up.
//! * **Phosphor scope** — stereo XY with persistence.
//! * **Spectrogram** — scrolling log-frequency waterfall.
//!
//! Voice LEDs and the level meter are updated by poking the DOM directly so
//! Dioxus doesn't re-render at 60 fps.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use dioxus::prelude::*;
use sa_engine::P;
use sa_engine::engine::{NUM_VOICES, TELEMETRY_VOICE_STRIDE};
use sa_engine::params::real;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, HtmlElement};

use crate::audio;
use crate::state::AppState;

const TRAIL: usize = 900;

type FrameCallback = Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>>;

struct Canvas {
    el: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
    w: f64,
    h: f64,
}

impl Canvas {
    fn find(id: &str) -> Option<Canvas> {
        let el: HtmlCanvasElement = web_sys::window()?.document()?.get_element_by_id(id)?.dyn_into().ok()?;
        let ctx: CanvasRenderingContext2d = el.get_context("2d").ok()??.dyn_into().ok()?;
        Some(Canvas { el, ctx, w: 0.0, h: 0.0 })
    }

    /// Match the backing store to the CSS size. Returns true if it changed
    /// (which also clears the canvas).
    fn fit(&mut self) -> bool {
        let dpr = web_sys::window().map(|w| w.device_pixel_ratio()).unwrap_or(1.0).min(2.0);
        let w = (self.el.client_width() as f64 * dpr).round().max(1.0);
        let h = (self.el.client_height() as f64 * dpr).round().max(1.0);
        if w != self.w || h != self.h {
            self.el.set_width(w as u32);
            self.el.set_height(h as u32);
            self.w = w;
            self.h = h;
            return true;
        }
        false
    }
}

struct Trail {
    pts: VecDeque<[f32; 3]>,
    s: [f32; 3],
    note: i32,
    energy: f32,
    alive: bool,
}

impl Trail {
    fn new() -> Self {
        Trail { pts: VecDeque::with_capacity(TRAIL + 1), s: [1.0, 1.0, 20.0], note: -1, energy: 0.0, alive: false }
    }

    fn seed(&mut self, note: i32) {
        let h = ((note * 7919) % 97) as f32 / 97.0;
        self.s = [1.0 + h * 4.0, -2.0 + h * 3.0, 18.0 + h * 10.0];
        self.pts.clear();
        self.note = note;
        self.alive = true;
    }

    fn step(&mut self, rho: f32, dt: f32, n: usize) {
        for _ in 0..n {
            let [x, y, z] = self.s;
            self.s = [x + 10.0 * (y - x) * dt, y + (x * (rho - z) - y) * dt, z + (x * y - 8.0 / 3.0 * z) * dt];
            if !self.s.iter().all(|v| v.is_finite()) {
                self.s = [1.0, 1.0, 20.0];
            }
            self.pts.push_back(self.s);
            if self.pts.len() > TRAIL {
                self.pts.pop_front();
            }
        }
    }
}

struct Viz {
    st: AppState,
    attractor: Canvas,
    scope: Canvas,
    spectro: Canvas,
    trails: Vec<Trail>,
    ghost: Trail,
    rot: f32,
    time_l: Vec<f32>,
    time_r: Vec<f32>,
    freq: Vec<u8>,
    palette: Vec<String>,
    leds: Vec<Option<HtmlElement>>,
    meter: Option<HtmlElement>,
    root: Option<HtmlElement>,
    energy: f32,
    hue: f64,
}

fn palette() -> Vec<String> {
    // black → indigo → magenta → amber → white
    let stops: [(f32, [f32; 3]); 6] = [
        (0.0, [4.0, 3.0, 10.0]),
        (0.25, [30.0, 10.0, 90.0]),
        (0.5, [130.0, 20.0, 160.0]),
        (0.7, [247.0, 37.0, 133.0]),
        (0.87, [255.0, 183.0, 3.0]),
        (1.0, [255.0, 255.0, 235.0]),
    ];
    (0..256)
        .map(|i| {
            let t = (i as f32 / 255.0).powf(1.3);
            let k = stops.windows(2).position(|w| t <= w[1].0).unwrap_or(stops.len() - 2);
            let (t0, a) = stops[k];
            let (t1, b) = stops[k + 1];
            let f = ((t - t0) / (t1 - t0)).clamp(0.0, 1.0);
            let c: Vec<u8> = (0..3).map(|j| (a[j] + (b[j] - a[j]) * f) as u8).collect();
            format!("rgb({},{},{})", c[0], c[1], c[2])
        })
        .collect()
}

fn element(id: &str) -> Option<HtmlElement> {
    web_sys::window()?.document()?.get_element_by_id(id)?.dyn_into().ok()
}

impl Viz {
    fn new(st: AppState) -> Option<Viz> {
        Some(Viz {
            st,
            attractor: Canvas::find("viz-attractor")?,
            scope: Canvas::find("viz-scope")?,
            spectro: Canvas::find("viz-spectro")?,
            trails: (0..NUM_VOICES).map(|_| Trail::new()).collect(),
            ghost: {
                let mut g = Trail::new();
                g.seed(0);
                g
            },
            rot: 0.0,
            time_l: vec![0.0; 2048],
            time_r: vec![0.0; 2048],
            freq: vec![0; 2048],
            palette: palette(),
            leds: (0..NUM_VOICES).map(|i| element(&format!("led-{i}"))).collect(),
            meter: element("meter-fill"),
            root: web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.document_element())
                .and_then(|e| e.dyn_into().ok()),
            energy: 0.0,
            hue: 0.0,
        })
    }

    fn param(&self, p: P) -> f32 {
        real(p as usize, *self.st.params[p as usize].peek())
    }

    fn frame(&mut self) {
        let running = audio::is_running();
        let (voices, peak) = audio::telemetry(|t| {
            let v: Vec<(bool, i32, f32)> = (0..NUM_VOICES)
                .map(|i| {
                    let o = i * TELEMETRY_VOICE_STRIDE;
                    (t[o] > 0.5, t[o + 1] as i32, t[o + 2])
                })
                .collect();
            let o = NUM_VOICES * TELEMETRY_VOICE_STRIDE;
            (v, t[o].max(t[o + 1]))
        });
        self.hue = (self.hue + 0.15) % 360.0;
        self.draw_attractor(&voices);
        if running {
            audio::with_audio(|a| {
                a.analyser_l.get_float_time_domain_data(&mut self.time_l);
                a.analyser_r.get_float_time_domain_data(&mut self.time_r);
                a.analyser_mix.get_byte_frequency_data(&mut self.freq);
            });
        }
        self.draw_scope(running);
        let sr = audio::with_audio(|a| a.ctx.sample_rate()).unwrap_or(48000.0);
        self.draw_spectro(running, sr);

        for (i, led) in self.leds.iter().enumerate() {
            if let Some(led) = led {
                let (on, note, env) = voices[i];
                let a = if on { (0.25 + env * 0.75).min(1.0) } else { 0.08 };
                let hue = (note * 29).rem_euclid(360);
                let _ = led.style().set_property("background", &format!("hsla({hue},100%,62%,{a:.2})"));
                let _ = led
                    .style()
                    .set_property("box-shadow", &format!("0 0 {:.0}px hsla({hue},100%,60%,{a:.2})", 2.0 + 10.0 * a));
            }
        }
        let db = 20.0 * peak.max(1e-5).log10();
        let level = ((db + 48.0) / 48.0).clamp(0.0, 1.0);
        if let Some(m) = &self.meter {
            let _ = m.style().set_property("width", &format!("{:.1}%", level * 100.0));
        }
        // The background nebula breathes with the output.
        self.energy += (level - self.energy) * if level > self.energy { 0.3 } else { 0.04 };
        if let Some(root) = &self.root {
            let _ = root.style().set_property("--energy", &format!("{:.3}", self.energy));
        }
    }

    fn draw_attractor(&mut self, voices: &[(bool, i32, f32)]) {
        let chaos = self.param(P::Chaos);
        let speed = self.param(P::Attractor);
        let rho = 20.0 + 12.0 * chaos;
        let steps = ((speed.ln() + 3.0) * 1.5).clamp(3.0, 16.0) as usize;
        let dt = 0.0045;

        for (t, &(on, note, env)) in self.trails.iter_mut().zip(voices) {
            if on {
                if !t.alive || t.note != note {
                    t.seed(note);
                }
                t.energy = t.energy.max(env.min(1.0));
                t.energy += (env.min(1.0) - t.energy) * 0.2;
                t.step(rho, dt, steps);
            } else {
                t.alive = false;
                t.energy *= 0.94;
                for _ in 0..3 {
                    t.pts.pop_front();
                }
            }
        }
        self.ghost.step(rho, 0.003, 1);
        self.rot += 0.0035 + 0.004 * chaos;

        let c = &mut self.attractor;
        c.fit();
        let (w, h) = (c.w, c.h);
        let ctx = &c.ctx;
        ctx.set_global_composite_operation("source-over").ok();
        ctx.set_fill_style_str("rgba(5,4,12,0.55)");
        ctx.fill_rect(0.0, 0.0, w, h);
        ctx.set_global_composite_operation("lighter").ok();
        let scale = (h / 62.0).min(w / 70.0);
        let (sr, cr) = self.rot.sin_cos();
        let project = |p: &[f32; 3]| -> (f64, f64) {
            let xr = p[0] * cr - p[1] * sr;
            let depth = p[0] * sr + p[1] * cr;
            let persp = 1.0 / (1.0 + depth as f64 * 0.008);
            (w / 2.0 + xr as f64 * scale * persp, h * 0.52 - (p[2] - rho.max(20.0)) as f64 * scale * persp)
        };

        let draw = |t: &Trail, hue: i32, gain: f32, width: f64| {
            let n = t.pts.len();
            if n < 2 {
                return;
            }
            let chunk = 30;
            let mut i = 0;
            while i + 1 < n {
                let end = (i + chunk).min(n - 1);
                let age = end as f32 / n as f32;
                let a = (age * age * gain).clamp(0.0, 1.0);
                ctx.set_stroke_style_str(&format!("hsla({hue},100%,{}%,{a:.3})", 45 + (age * 25.0) as i32));
                ctx.set_line_width(width * (0.4 + age as f64));
                ctx.begin_path();
                let (x, y) = project(&t.pts[i]);
                ctx.move_to(x, y);
                for p in t.pts.range(i + 1..=end) {
                    let (x, y) = project(p);
                    ctx.line_to(x, y);
                }
                ctx.stroke();
                i = end;
            }
        };

        let any = self.trails.iter().any(|t| t.energy > 0.02);
        draw(&self.ghost, 200, if any { 0.12 } else { 0.35 }, 1.0);
        let dpr = (w / c.el.client_width().max(1) as f64).max(1.0);
        for t in &self.trails {
            if t.energy > 0.01 {
                let hue = (t.note * 29).rem_euclid(360);
                draw(t, hue, 0.35 + t.energy, 1.6 * dpr);
                if let Some(head) = t.pts.back() {
                    let (x, y) = project(head);
                    ctx.set_fill_style_str(&format!("hsla({hue},100%,85%,{:.2})", t.energy.min(1.0)));
                    ctx.begin_path();
                    let _ = ctx.arc(x, y, 2.5 * dpr * (1.0 + t.energy as f64), 0.0, std::f64::consts::TAU);
                    ctx.fill();
                }
            }
        }
    }

    fn draw_scope(&mut self, running: bool) {
        let c = &mut self.scope;
        c.fit();
        let (w, h) = (c.w, c.h);
        let ctx = &c.ctx;
        ctx.set_global_composite_operation("source-over").ok();
        ctx.set_fill_style_str("rgba(3,6,8,0.30)");
        ctx.fill_rect(0.0, 0.0, w, h);
        ctx.set_stroke_style_str("rgba(0,255,200,0.06)");
        ctx.set_line_width(1.0);
        ctx.begin_path();
        ctx.move_to(w / 2.0, 0.0);
        ctx.line_to(w / 2.0, h);
        ctx.move_to(0.0, h / 2.0);
        ctx.line_to(w, h / 2.0);
        ctx.stroke();
        if !running {
            return;
        }
        ctx.set_global_composite_operation("lighter").ok();
        let k = h.min(w) * 0.42 / 0.6;
        let hue = 160.0 + 40.0 * (self.hue.to_radians() * 3.0).sin();
        ctx.set_stroke_style_str(&format!("hsla({hue:.0},100%,60%,0.55)"));
        ctx.set_line_width((w / 400.0).max(1.0));
        ctx.begin_path();
        for (i, (l, r)) in self.time_l.iter().zip(self.time_r.iter()).enumerate() {
            let (l, r) = (*l as f64, *r as f64);
            // Mid/side: mono is vertical, wide stereo spreads sideways.
            let x = w / 2.0 + (r - l) * std::f64::consts::FRAC_1_SQRT_2 * k;
            let y = h / 2.0 - (l + r) * std::f64::consts::FRAC_1_SQRT_2 * k;
            if i == 0 {
                ctx.move_to(x, y);
            } else {
                ctx.line_to(x, y);
            }
        }
        ctx.stroke();
    }

    fn draw_spectro(&mut self, running: bool, sr: f32) {
        let c = &mut self.spectro;
        let resized = c.fit();
        let (w, h) = (c.w, c.h);
        let ctx = &c.ctx;
        if resized {
            ctx.set_fill_style_str("#05030a");
            ctx.fill_rect(0.0, 0.0, w, h);
        }
        let col = (w / 300.0).max(1.0).round();
        let _ = ctx.draw_image_with_html_canvas_element(&c.el, -col, 0.0);
        let bins = self.freq.len() as f32;
        let nyq = sr / 2.0;
        let row = (h / 160.0).max(1.0).round();
        let mut y = 0.0;
        while y < h {
            let t = 1.0 - (y / h) as f32;
            let lo = 25.0 * (18000.0f32 / 25.0).powf(t);
            let hi = 25.0 * (18000.0f32 / 25.0).powf(t + (row / h) as f32);
            let b0 = ((lo / nyq) * bins) as usize;
            let b1 = (((hi / nyq) * bins) as usize).max(b0 + 1).min(self.freq.len());
            let mag =
                if running { self.freq[b0.min(self.freq.len() - 1)..b1].iter().copied().max().unwrap_or(0) } else { 0 };
            ctx.set_fill_style_str(&self.palette[mag as usize]);
            ctx.fill_rect(w - col, y, col, row);
            y += row;
        }
    }
}

#[component]
pub fn Visuals() -> Element {
    let st = use_context::<AppState>();
    use_effect(move || {
        let Some(viz) = Viz::new(st) else { return };
        let viz = Rc::new(RefCell::new(viz));
        let cb: FrameCallback = Rc::default();
        let cb2 = cb.clone();
        *cb.borrow_mut() = Some(Closure::new(move |_t: f64| {
            viz.borrow_mut().frame();
            if let (Some(w), Some(f)) = (web_sys::window(), cb2.borrow().as_ref()) {
                let _ = w.request_animation_frame(f.as_ref().unchecked_ref());
            }
        }));
        if let (Some(w), Some(f)) = (web_sys::window(), cb.borrow().as_ref()) {
            let _ = w.request_animation_frame(f.as_ref().unchecked_ref());
        }
    });

    rsx! {
        div { class: "visuals",
            div { class: "viz viz-main",
                canvas { id: "viz-attractor" }
                div { class: "viz-label", "LORENZ · one trajectory per voice" }
                div { class: "leds",
                    for i in 0..NUM_VOICES {
                        span { key: "{i}", id: "led-{i}", class: "led" }
                    }
                }
            }
            div { class: "viz",
                canvas { id: "viz-scope" }
                div { class: "viz-label", "PHOSPHOR · stereo XY" }
            }
            div { class: "viz",
                canvas { id: "viz-spectro" }
                div { class: "viz-label", "SPECTRUM · 25 Hz – 18 kHz" }
            }
        }
    }
}
