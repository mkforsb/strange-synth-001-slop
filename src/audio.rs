//! Main-thread side of the audio engine: builds the WebAudio graph, boots
//! the DSP worklet and relays parameters/notes to it.
//!
//! ```text
//! AudioWorkletNode(strange-attractor) ─► master gain ─► destination
//!                                          ├─► splitter ─► analyser L / R  (vectorscope)
//!                                          └─► analyser mix               (spectrogram)
//! ```

use std::cell::RefCell;

use dioxus::prelude::*;
use js_sys::{Array, Float32Array, Object, Reflect, Uint8Array};
use sa_engine::engine::TELEMETRY_LEN;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    AnalyserNode, AudioContext, AudioWorkletNode, AudioWorkletNodeOptions, Blob, BlobPropertyBag, GainNode,
    MessageEvent, MessagePort, Url,
};

static WORKLET_JS: &str = include_str!("../assets/worklet.js");
static DSP_WASM: &[u8] = include_bytes!(env!("DSP_WASM"));

#[derive(Clone, Debug, PartialEq)]
pub enum AudioStatus {
    Off,
    Starting,
    Running { sample_rate: f32 },
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

/// Boot the audio graph. Must be called from a user gesture.
pub async fn start(patch: Vec<f32>, volume: f32, mut status: Signal<AudioStatus>) {
    if is_running() {
        return;
    }
    status.set(AudioStatus::Starting);
    if let Err(e) = start_inner(patch, volume, status).await {
        let text = e.as_string().unwrap_or_else(|| format!("{e:?}"));
        status.set(AudioStatus::Failed(text));
    }
}

async fn start_inner(patch: Vec<f32>, volume: f32, mut status: Signal<AudioStatus>) -> Result<(), JsValue> {
    let ctx = AudioContext::new()?;

    let props = BlobPropertyBag::new();
    props.set_type("application/javascript");
    let blob = Blob::new_with_str_sequence_and_options(&Array::of1(&WORKLET_JS.into()), &props)?;
    let url = Url::create_object_url_with_blob(&blob)?;
    JsFuture::from(ctx.audio_worklet()?.add_module(&url)?).await?;
    Url::revoke_object_url(&url)?;

    let opts = AudioWorkletNodeOptions::new();
    opts.set_number_of_inputs(0);
    opts.set_number_of_outputs(1);
    opts.set_output_channel_count(&Array::of1(&2.into()));
    let node = AudioWorkletNode::new_with_options(&ctx, "strange-attractor", &opts)?;
    let port = node.port()?;

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
                let sr = Reflect::get(&data, &"sampleRate".into()).ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
                status.set(AudioStatus::Running { sample_rate: sr as f32 });
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

    // Hand the DSP module to the worklet (a copy; the worklet compiles it).
    let bytes = Uint8Array::from(DSP_WASM);
    port.post_message(&msg(&[("t", "wasm".into()), ("bytes", bytes.buffer().into())]))?;
    port.post_message(&msg(&[("t", "patch".into()), ("values", Float32Array::from(patch.as_slice()).into())]))?;

    let _ = JsFuture::from(ctx.resume()?).await;

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
        })
    });
    Ok(())
}
