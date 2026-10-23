//! Main-thread side of the audio engine: builds the WebAudio graph, boots
//! the DSP worklet and relays parameters/notes to it.
//!
//! ```text
//! AudioWorkletNode(strange-attractor) ─► master gain ─► destination
//!                                          ├─► splitter ─► analyser L / R  (vectorscope)
//!                                          └─► analyser mix               (spectrogram)
//! ```

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use dioxus::prelude::*;
use js_sys::{Array, Float32Array, Object, Reflect, Uint8Array, WebAssembly};
use sa_engine::engine::TELEMETRY_LEN;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    AnalyserNode, AudioContext, AudioContextState, AudioWorkletNode, AudioWorkletNodeOptions, Blob, BlobPropertyBag,
    GainNode, MessageEvent, MessagePort, Url,
};

static WORKLET_JS: &str = include_str!("../assets/worklet.js");
static DSP_WASM: &[u8] = include_bytes!(env!("DSP_WASM"));

#[derive(Clone, Debug, PartialEq)]
pub enum AudioStatus {
    Off,
    Starting,
    /// Worklet is up but the browser holds the context suspended until a
    /// gesture it accepts.
    Suspended,
    Running {
        sample_rate: f32,
    },
    Failed(String),
}

pub struct Audio {
    pub ctx: AudioContext,
    port: MessagePort,
    pub master: GainNode,
    pub analyser_l: AnalyserNode,
    pub analyser_r: AnalyserNode,
    pub analyser_mix: AnalyserNode,
    _node: AudioWorkletNode,
    _on_message: Closure<dyn FnMut(MessageEvent)>,
    _on_state: Closure<dyn FnMut()>,
}

thread_local! {
    static AUDIO: RefCell<Option<Audio>> = const { RefCell::new(None) };
    static TELEMETRY: RefCell<[f32; TELEMETRY_LEN]> = const { RefCell::new([0.0; TELEMETRY_LEN]) };
}

fn msg(fields: &[(&str, JsValue)]) -> Object {
    let o = Object::new();
    for (k, v) in fields {
        let _ = Reflect::set(&o, &JsValue::from_str(k), v);
    }
    o
}

fn post(fields: &[(&str, JsValue)]) {
    AUDIO.with_borrow(|a| {
        if let Some(a) = a {
            let _ = a.port.post_message(&msg(fields));
        }
    });
}

pub fn is_running() -> bool {
    AUDIO.with_borrow(|a| a.is_some())
}

pub fn with_audio<R>(f: impl FnOnce(&Audio) -> R) -> Option<R> {
    AUDIO.with_borrow(|a| a.as_ref().map(f))
}

pub fn telemetry<R>(f: impl FnOnce(&[f32; TELEMETRY_LEN]) -> R) -> R {
    TELEMETRY.with_borrow(|t| f(t))
}

pub fn set_param(index: usize, norm: f32) {
    post(&[("t", "p".into()), ("i", (index as u32).into()), ("v", norm.into())]);
}

pub fn send_patch(values: &[f32]) {
    post(&[("t", "patch".into()), ("values", Float32Array::from(values).into())]);
}

pub fn note_on(note: u8, velocity: f32) {
    post(&[("t", "on".into()), ("n", (note as u32).into()), ("v", velocity.into())]);
}

pub fn note_off(note: u8) {
    post(&[("t", "off".into()), ("n", (note as u32).into())]);
}

pub fn panic() {
    post(&[("t", "panic".into())]);
}

pub fn set_volume(gain: f32) {
    with_audio(|a| a.master.gain().set_target_at_time(gain, a.ctx.current_time(), 0.02));
}

/// Boot the audio graph, or resume it if it is already up.
///
/// Must be called synchronously from a user-gesture handler (not from a task
/// spawned by one): mobile browsers, Firefox on Android in particular, only let
/// an `AudioContext` start if it is created/resumed inside the gesture itself.
pub fn start(patch: Vec<f32>, volume: f32, mut status: Signal<AudioStatus>) {
    if with_audio(|a| a.ctx.resume()).is_some() || *status.peek() == AudioStatus::Starting {
        return;
    }
    let ctx = match AudioContext::new() {
        Ok(ctx) => ctx,
        Err(e) => return status.set(AudioStatus::Failed(js_error(&e))),
    };
    let _ = ctx.resume();
    status.set(AudioStatus::Starting);
    spawn(async move {
        if let Err(e) = start_inner(ctx, patch, volume, status).await {
            status.set(AudioStatus::Failed(js_error(&e)));
        }
    });
}

/// Resume a context the browser suspended (autoplay policy, app backgrounded,
/// ...). Installed on window-level gesture events by [`install_gesture_resume`].
fn resume_if_suspended() {
    with_audio(|a| {
        if a.ctx.state() != AudioContextState::Running {
            let _ = a.ctx.resume();
        }
    });
}

/// Every later tap or key press gets a chance to (re)start a suspended context.
pub fn install_gesture_resume() {
    let Some(window) = web_sys::window() else { return };
    let cb = Closure::<dyn FnMut()>::new(resume_if_suspended);
    for ev in ["pointerup", "touchend", "click", "keydown"] {
        let _ = window.add_event_listener_with_callback(ev, cb.as_ref().unchecked_ref());
    }
    cb.forget();
}

fn js_error(e: &JsValue) -> String {
    if let Some(err) = e.dyn_ref::<js_sys::Error>() {
        return String::from(err.message());
    }
    e.as_string().unwrap_or_else(|| format!("{e:?}"))
}

/// Status to show once the worklet is up: the context may still be held
/// suspended by the browser until a gesture it accepts.
fn live_status(ctx: &AudioContext) -> AudioStatus {
    match ctx.state() {
        AudioContextState::Running => AudioStatus::Running { sample_rate: ctx.sample_rate() },
        _ => AudioStatus::Suspended,
    }
}

async fn start_inner(
    ctx: AudioContext,
    patch: Vec<f32>,
    volume: f32,
    mut status: Signal<AudioStatus>,
) -> Result<(), JsValue> {
    let props = BlobPropertyBag::new();
    props.set_type("application/javascript");
    let blob = Blob::new_with_str_sequence_and_options(&Array::of1(&WORKLET_JS.into()), &props)?;
    let url = Url::create_object_url_with_blob(&blob)?;
    JsFuture::from(ctx.audio_worklet()?.add_module(&url)?).await?;
    Url::revoke_object_url(&url)?;

    // Compile the DSP here and hand the module to the worklet, which then
    // instantiates it synchronously in its constructor; no promise jobs needed
    // on the audio thread (which may not run them while the context is suspended).
    let bytes = Uint8Array::from(DSP_WASM);
    let module = JsFuture::from(WebAssembly::compile(&bytes.buffer().into())).await?;
    let processor_options = Object::new();
    Reflect::set(&processor_options, &"module".into(), &module)?;

    let opts = AudioWorkletNodeOptions::new();
    opts.set_number_of_inputs(0);
    opts.set_number_of_outputs(1);
    opts.set_output_channel_count(&Array::of1(&2.into()));
    opts.set_processor_options(Some(&processor_options));
    let node = AudioWorkletNode::new_with_options(&ctx, "strange-attractor", &opts)?;
    let port = node.port()?;

    let ready = Rc::new(Cell::new(false));
    let on_state = {
        let (ctx, ready) = (ctx.clone(), ready.clone());
        Closure::<dyn FnMut()>::new(move || {
            if ready.get() {
                status.set(live_status(&ctx));
            }
        })
    };
    ctx.set_onstatechange(Some(on_state.as_ref().unchecked_ref()));

    let ctx_msg = ctx.clone();
    let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |ev: MessageEvent| {
        let data = ev.data();
        let kind = Reflect::get(&data, &"t".into()).ok().and_then(|t| t.as_string()).unwrap_or_default();
        match kind.as_str() {
            "tel" => {
                if let Ok(arr) = Reflect::get(&data, &"data".into()) {
                    let arr: Float32Array = arr.unchecked_into();
                    TELEMETRY.with_borrow_mut(|t| {
                        if arr.length() as usize == t.len() {
                            arr.copy_to(t);
                        }
                    });
                }
            }
            "ready" => {
                ready.set(true);
                status.set(live_status(&ctx_msg));
            }
            "error" => {
                let m = Reflect::get(&data, &"msg".into()).ok().and_then(|v| v.as_string()).unwrap_or_default();
                status.set(AudioStatus::Failed(m));
            }
            _ => {}
        }
    });
    port.set_onmessage(Some(on_message.as_ref().unchecked_ref()));

    let master = ctx.create_gain()?;
    master.gain().set_value(volume);
    node.connect_with_audio_node(&master)?;
    master.connect_with_audio_node(&ctx.destination())?;

    let splitter = ctx.create_channel_splitter_with_number_of_outputs(2)?;
    master.connect_with_audio_node(&splitter)?;
    let analyser_l = ctx.create_analyser()?;
    let analyser_r = ctx.create_analyser()?;
    for a in [&analyser_l, &analyser_r] {
        a.set_fft_size(2048);
    }
    splitter.connect_with_audio_node_and_output(&analyser_l, 0)?;
    splitter.connect_with_audio_node_and_output(&analyser_r, 1)?;
    let analyser_mix = ctx.create_analyser()?;
    analyser_mix.set_fft_size(4096);
    analyser_mix.set_smoothing_time_constant(0.5);
    analyser_mix.set_min_decibels(-100.0);
    analyser_mix.set_max_decibels(-10.0);
    master.connect_with_audio_node(&analyser_mix)?;

    port.post_message(&msg(&[("t", "patch".into()), ("values", Float32Array::from(patch.as_slice()).into())]))?;

    // Covers browsers that let the context start but not before the first
    // `await` above; if this is refused too, the splash asks for another tap.
    let _ = ctx.resume();

    AUDIO.with_borrow_mut(|a| {
        *a = Some(Audio {
            ctx,
            port,
            master,
            analyser_l,
            analyser_r,
            analyser_mix,
            _node: node,
            _on_message: on_message,
            _on_state: on_state,
        })
    });
    Ok(())
}
