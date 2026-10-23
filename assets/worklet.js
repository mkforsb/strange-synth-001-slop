// AudioWorklet processor: hosts the Rust DSP engine (a bare wasm module with
// no imports) on the audio thread. Loaded from a Blob URL by the app.
//
// main → worklet: {t:'wasm', bytes} {t:'p', i, v} {t:'patch', values}
//                 {t:'on', n, v} {t:'off', n} {t:'alloff'} {t:'panic'}
// worklet → main: {t:'ready'} {t:'tel', data} {t:'error', msg}

const TELEMETRY_EVERY = 1536; // samples between telemetry posts (~31 Hz @ 48k)

class StrangeAttractorProcessor extends AudioWorkletProcessor {
  constructor() {
    super();
    this.ex = null;
    this.queue = [];
    this.mem = null;
    this.telCountdown = 0;
    this.port.onmessage = (e) => this.onMessage(e.data);
  }

  async boot(bytes) {
    try {
      const { instance } = await WebAssembly.instantiate(bytes, {});
      this.ex = instance.exports;
      this.ex.sa_init(sampleRate);
      for (const m of this.queue) this.handle(m);
      this.queue = [];
      this.port.postMessage({ t: 'ready', sampleRate });
    } catch (err) {
      this.port.postMessage({ t: 'error', msg: String(err) });
    }
  }

  onMessage(m) {
    if (m.t === 'wasm') this.boot(m.bytes);
    else if (!this.ex) this.queue.push(m);
    else this.handle(m);
  }

  handle(m) {
    const ex = this.ex;
    switch (m.t) {
      case 'p': ex.sa_set_param(m.i, m.v); break;
      case 'patch': for (let i = 0; i < m.values.length; i++) ex.sa_set_param(i, m.values[i]); break;
      case 'on': ex.sa_note_on(m.n, m.v); break;
      case 'off': ex.sa_note_off(m.n); break;
      case 'alloff': ex.sa_all_notes_off(); break;
      case 'panic': ex.sa_panic(); break;
    }
  }

  heap() {
    // Views detach if wasm memory grows, so re-create on buffer change.
    const buf = this.ex.memory.buffer;
    if (!this.mem || this.mem.buffer !== buf) this.mem = new Float32Array(buf);
    return this.mem;
  }

  process(_inputs, outputs) {
    const out = outputs[0];
    if (!this.ex || !out || out.length === 0) return true;
    const n = out[0].length;
    const ptr = this.ex.sa_process(n) >> 2;
    const heap = this.heap();
    out[0].set(heap.subarray(ptr, ptr + n));
    if (out.length > 1) out[1].set(heap.subarray(ptr + n, ptr + 2 * n));

    this.telCountdown -= n;
    if (this.telCountdown <= 0) {
      this.telCountdown = TELEMETRY_EVERY;
      const tp = this.ex.sa_telemetry() >> 2;
      const len = this.ex.sa_telemetry_len();
      this.port.postMessage({ t: 'tel', data: heap.slice(tp, tp + len) });
    }
    return true;
  }
}

registerProcessor('strange-attractor', StrangeAttractorProcessor);
