//! The parameter model.
//!
//! Every parameter is stored *normalized* (0..1) — that is what knobs, the
//! randomizer and the wire protocol deal in. A [`Curve`] maps the normalized
//! value to the real value the DSP uses, and back again (presets and defaults
//! are written in real units for readability).

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    Exciter,
    Core,
    Body,
    Creature,
    Motion,
    Mangle,
    Envelope,
    Master,
    Eq,
    Reverb,
}

impl Group {
    pub const ALL: [Group; 10] = [
        Group::Exciter,
        Group::Core,
        Group::Body,
        Group::Creature,
        Group::Motion,
        Group::Mangle,
        Group::Envelope,
        Group::Master,
        Group::Eq,
        Group::Reverb,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Group::Exciter => "Exciter",
            Group::Core => "Core",
            Group::Body => "Body",
            Group::Creature => "Creature",
            Group::Motion => "Motion",
            Group::Mangle => "Mangle",
            Group::Envelope => "Envelope",
            Group::Master => "Master Filter",
            Group::Eq => "EQ",
            Group::Reverb => "Dattorro Plate",
        }
    }

    /// CSS-friendly slug, also used to pick the panel colour.
    pub fn slug(self) -> &'static str {
        match self {
            Group::Exciter => "exciter",
            Group::Core => "core",
            Group::Body => "body",
            Group::Creature => "creature",
            Group::Motion => "motion",
            Group::Mangle => "mangle",
            Group::Envelope => "envelope",
            Group::Master => "master",
            Group::Eq => "eq",
            Group::Reverb => "reverb",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Curve {
    /// Linear between `min` and `max`.
    Lin(f32, f32),
    /// Exponential (log-scaled knob) between `min` and `max`, both > 0.
    Exp(f32, f32),
    /// Like `Exp`, but the bottom of the knob snaps to exactly zero.
    Exp0(f32, f32),
    /// A discrete choice; the real value is the option index.
    Choice(&'static [&'static str]),
}

impl Curve {
    pub fn map(self, n: f32) -> f32 {
        let n = n.clamp(0.0, 1.0);
        match self {
            Curve::Lin(a, b) => a + (b - a) * n,
            Curve::Exp(a, b) => a * (b / a).powf(n),
            Curve::Exp0(a, b) => {
                if n <= 0.005 {
                    0.0
                } else {
                    a * (b / a).powf(n)
                }
            }
            Curve::Choice(opts) => {
                let k = opts.len();
                ((n * k as f32) as usize).min(k - 1) as f32
            }
        }
    }

    pub fn unmap(self, v: f32) -> f32 {
        let n = match self {
            Curve::Lin(a, b) => (v - a) / (b - a),
            Curve::Exp(a, b) => (v.max(a) / a).ln() / (b / a).ln(),
            Curve::Exp0(a, b) => {
                if v <= 0.0 {
                    0.0
                } else {
                    (v.max(a) / a).ln() / (b / a).ln()
                }
            }
            Curve::Choice(opts) => (v.round() + 0.5) / opts.len() as f32,
        };
        n.clamp(0.0, 1.0)
    }

    pub fn choices(self) -> Option<&'static [&'static str]> {
        match self {
            Curve::Choice(c) => Some(c),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    Pct,
    Bipolar,
    Hz,
    Sec,
    Ms,
    Db,
    St,
    StPerSec,
    Ratio,
    Count,
    Times,
    Material,
    Vowel,
    Choice,
}

#[derive(Debug)]
pub struct ParamInfo {
    pub key: &'static str,
    pub name: &'static str,
    pub group: Group,
    /// Default in *real* units.
    pub default: f32,
    pub curve: Curve,
    pub unit: Unit,
    pub desc: &'static str,
}

impl ParamInfo {
    pub fn default_norm(&self) -> f32 {
        self.curve.unmap(self.default)
    }
}

pub const MATERIALS: [&str; 8] = ["String", "Tube", "Drum", "Plate", "Bar", "Bell", "Glass", "Alien"];
pub const VOWELS: [&str; 6] = ["Beast", "U", "O", "A", "E", "I"];
pub const FILTER_TYPES: [&str; 4] = ["Moog 12", "Moog 24", "Band", "High"];
pub const FORMULAS: [&str; 8] = ["Sierp", "Melt", "Crypt", "Pulse", "Stairs", "Loom", "Oracle", "Swarm"];

macro_rules! define_params {
    ($( $id:ident : $name:literal, $group:ident, $def:expr, $curve:expr, $unit:ident, $desc:literal; )*) => {
        /// Parameter indices.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        #[repr(usize)]
        pub enum P { $($id,)* }

        pub const NUM_PARAMS: usize = [$(P::$id),*].len();

        pub static PARAMS: [ParamInfo; NUM_PARAMS] = [
            $( ParamInfo {
                key: stringify!($id),
                name: $name,
                group: Group::$group,
                default: $def,
                curve: $curve,
                unit: Unit::$unit,
                desc: $desc,
            }, )*
        ];
    };
}

use Curve::*;

define_params! {
    // ── Exciter: what sets the body in motion ───────────────────────────
    Strike:    "Strike",     Exciter, 0.8, Lin(0.0, 1.0), Pct, "Mallet impulse into the body at note-on";
    Hardness:  "Hardness",   Exciter, 0.5, Lin(0.0, 1.0), Pct, "Mallet material: soft rubber ↔ steel hammer";
    Position:  "Position",   Exciter, 0.23, Lin(0.02, 0.5), Pct, "Where the body is struck — carves notches into the partials";
    Bow:       "Bow",        Exciter, 0.0, Lin(0.0, 1.0), Pct, "Stick-slip friction bow; self-oscillates the body";
    Pressure:  "Pressure",   Exciter, 0.4, Lin(0.0, 1.0), Pct, "Bow pressure: silky glide ↔ grinding scrape";
    Breath:    "Breath",     Exciter, 0.0, Lin(0.0, 1.0), Pct, "Pitch-tracked noise blown into the body";
    Grit:      "Grit",       Exciter, 0.0, Lin(0.0, 1.0), Pct, "Rusty crackle impulses that keep pelting the body";

    // ── Core: the oscillator heart ──────────────────────────────────────
    Core:      "Core",       Core, 0.0, Lin(0.0, 1.0), Pct, "Direct level of the core oscillator";
    Feed:      "Feed",       Core, 0.0, Lin(0.0, 1.0), Pct, "Core + logic signal driven into the body resonator";
    Warp:      "Warp",       Core, 0.0, Lin(0.0, 1.0), Pct, "Phase-distortion bend (sine ↔ saw-ish)";
    SelfFm:    "Self-FM",    Core, 0.0, Lin(0.0, 1.0), Pct, "Oscillator modulates its own phase";
    FmRatio:   "FM Ratio",   Core, 1.0, Exp(0.25, 16.0), Ratio, "Modulator frequency ratio (inharmonic between integers)";
    FmDepth:   "FM Depth",   Core, 0.0, Lin(0.0, 1.0), Pct, "Modulator index";
    Fold:      "Fold",       Core, 0.0, Lin(0.0, 1.0), Pct, "Sine wavefolder";
    Chaos:     "Chaos",      Core, 0.0, Lin(0.0, 1.0), Pct, "Lorenz attractor bends pitch, screech and audio";
    Attractor: "Attractor",  Core, 6.0, Exp(0.1, 800.0), Hz, "Speed of the Lorenz attractor: slow drift ↔ audio-rate buzz";
    Undertone: "Undertone",  Core, 0.0, Lin(0.0, 1.0), Pct, "Subharmonics at 1/2 and 1/3 of the pitch";
    Logic:     "Logic",      Core, 0.0, Lin(0.0, 1.0), Pct, "Bytebeat machine-logic layer, pitch tracked";
    Formula:   "Formula",    Core, 0.0, Choice(&FORMULAS), Choice, "Which bytebeat expression the machine thinks";

    // ── Body: modal resonator ───────────────────────────────────────────
    Body:      "Body",       Body, 0.8, Lin(0.0, 1.0), Pct, "Resonator output level";
    Material:  "Material",   Body, 4.0, Lin(0.0, 7.0), Material, "Morph partial ratios: string → tube → drum → plate → bar → bell → glass → alien";
    Stretch:   "Stretch",    Body, 0.0, Lin(-1.0, 1.0), Bipolar, "Squash or stretch the partial series";
    RingTime:  "Ring Time",  Body, 1.5, Exp(0.02, 30.0), Sec, "How long the body rings";
    Damping:   "Damping",    Body, 0.4, Lin(0.0, 1.0), Pct, "High partials die faster (metal ↔ wood)";
    Shimmer:   "Shimmer",    Body, 0.0, Lin(0.0, 1.0), Pct, "Twin detuned partials that beat against each other";
    Modes:     "Modes",      Body, 12.0, Lin(1.0, 12.0), Count, "Number of active partials";
    BodyTune:  "Body Tune",  Body, 0.0, Lin(-24.0, 24.0), St, "Resonator pitch relative to the note";
    Mute:      "Mute",       Body, 0.0, Lin(0.0, 1.0), Pct, "Palm-mute the body: short, dull, choked";
    Rattle:    "Rattle",     Body, 0.0, Lin(0.0, 1.0), Pct, "Loose contact buzz that feeds back into the body";

    // ── Creature: throat & pitch gestures ───────────────────────────────
    Throat:    "Throat",     Creature, 0.0, Lin(0.0, 1.0), Pct, "Formant filter amount";
    Vowel:     "Vowel",      Creature, 3.0, Lin(0.0, 5.0), Vowel, "Formant morph: beast → U → O → A → E → I";
    Growl:     "Growl",      Creature, 0.0, Lin(0.0, 1.0), Pct, "Vocal-fry roughness, period doubling and saturation";
    Sweep:     "Sweep",      Creature, 0.0, Lin(-48.0, 48.0), St, "Pitch envelope depth (decays to the note)";
    SweepTime: "Sweep Time", Creature, 0.3, Exp(0.005, 8.0), Sec, "Pitch envelope time";
    Gravity:   "Gravity",    Creature, 0.0, Lin(-24.0, 24.0), StPerSec, "Pitch keeps falling (or rising) for as long as the note lives";

    // ── Motion ──────────────────────────────────────────────────────────
    Drift:     "Drift",      Motion, 0.0, Lin(0.0, 1.0), Pct, "Drunken random-walk pitch wander";
    WobRate:   "Wob Rate",   Motion, 4.0, Exp(0.05, 40.0), Hz, "Wobble LFO rate";
    Wobble:    "Wobble",     Motion, 0.0, Lin(0.0, 1.0), Pct, "Throbbing pitch + amplitude wobble";
    Spin:      "Spin",       Motion, 0.0, Lin(0.0, 1.0), Pct, "Each voice orbits the listener (pan + doppler)";
    Spread:    "Spread",     Motion, 0.3, Lin(0.0, 1.0), Pct, "Stereo spread of voices";
    Glide:     "Glide",      Motion, 0.0, Exp0(0.005, 4.0), Sec, "Portamento from the previous note";

    // ── Mangle ──────────────────────────────────────────────────────────
    RingMod:   "Ring Mod",   Mangle, 0.0, Lin(0.0, 1.0), Pct, "Ring modulator amount";
    RingFreq:  "Ring Freq",  Mangle, 120.0, Exp(0.5, 5000.0), Hz, "Ring modulator frequency (not key tracked)";
    Screech:   "Screech",    Mangle, 0.0, Lin(0.0, 1.0), Pct, "Feedback squeal resonator — self-oscillates past ~40%";
    Crush:     "Crush",      Mangle, 0.0, Lin(0.0, 1.0), Pct, "Bit & sample-rate destruction";
    Glitch:    "Glitch",     Mangle, 0.0, Lin(0.0, 1.0), Pct, "Probability of stutter / reverse buffer repeats";

    // ── Envelope ────────────────────────────────────────────────────────
    Attack:    "Attack",     Envelope, 0.002, Exp(0.001, 8.0), Sec, "Amplitude attack";
    Decay:     "Decay",      Envelope, 1.0, Exp(0.005, 20.0), Sec, "Amplitude decay";
    Sustain:   "Sustain",    Envelope, 1.0, Lin(0.0, 1.0), Pct, "Amplitude sustain";
    Release:   "Release",    Envelope, 0.6, Exp(0.005, 20.0), Sec, "Amplitude release";

    // ── Master filter ───────────────────────────────────────────────────
    FilterType:"Type",       Master, 1.0, Choice(&FILTER_TYPES), Choice, "Ladder filter mode";
    Cutoff:    "Cutoff",     Master, 20000.0, Exp(20.0, 20000.0), Hz, "Filter cutoff";
    Resonance: "Resonance",  Master, 0.1, Lin(0.0, 1.0), Pct, "Ladder feedback (self-oscillates at the top)";
    Drive:     "Drive",      Master, 0.0, Lin(0.0, 1.0), Pct, "Saturation into the ladder";
    Follow:    "Follow",     Master, 0.0, Lin(-1.0, 1.0), Bipolar, "Envelope follower → cutoff";
    Level:     "Level",      Master, 0.0, Lin(-24.0, 6.0), Db, "Patch output level";

    // ── EQ ──────────────────────────────────────────────────────────────
    LowCut:    "Low Cut",    Eq, 10.0, Exp(10.0, 2000.0), Hz, "12 dB/oct high-pass";
    EqLo:      "Lo",         Eq, 0.0, Lin(-15.0, 15.0), Db, "Low shelf @ 120 Hz";
    EqMid:     "Mid",        Eq, 0.0, Lin(-15.0, 15.0), Db, "Mid peak gain";
    EqMidFreq: "Mid Freq",   Eq, 1000.0, Exp(100.0, 10000.0), Hz, "Mid peak frequency";
    EqHi:      "Hi",         Eq, 0.0, Lin(-15.0, 15.0), Db, "High shelf @ 6 kHz";
    HiCut:     "Hi Cut",     Eq, 22000.0, Exp(500.0, 22000.0), Hz, "12 dB/oct low-pass";

    // ── Reverb ──────────────────────────────────────────────────────────
    Predelay:  "Predelay",   Reverb, 0.02, Lin(0.0, 0.5), Ms, "Time before the plate answers";
    Size:      "Size",       Reverb, 1.0, Lin(0.25, 2.0), Times, "Tank delay scale";
    RvDecay:   "Decay",      Reverb, 0.6, Lin(0.0, 0.999), Pct, "Tank feedback (near 100% ≈ freeze)";
    RvDamp:    "Damping",    Reverb, 0.3, Lin(0.0, 0.95), Pct, "High-frequency loss in the tank";
    RvMod:     "Mod",        Reverb, 0.3, Lin(0.0, 1.0), Pct, "Tank allpass modulation (chorusy shimmer)";
    RvMix:     "Mix",        Reverb, 0.25, Lin(0.0, 1.0), Pct, "Dry/wet";
}

impl P {
    pub fn info(self) -> &'static ParamInfo {
        &PARAMS[self as usize]
    }

    pub fn from_index(i: usize) -> Option<P> {
        if i < NUM_PARAMS {
            // SAFETY: `P` is a fieldless `repr(usize)` enum with variants 0..NUM_PARAMS.
            Some(unsafe { std::mem::transmute::<usize, P>(i) })
        } else {
            None
        }
    }
}

/// A full set of normalized parameter values.
pub type Patch = [f32; NUM_PARAMS];

pub fn default_patch() -> Patch {
    let mut p = [0.0; NUM_PARAMS];
    for (v, info) in p.iter_mut().zip(PARAMS.iter()) {
        *v = info.default_norm();
    }
    p
}

/// Map a normalized value to the real value for parameter `i`.
pub fn real(i: usize, norm: f32) -> f32 {
    PARAMS[i].curve.map(norm)
}

/// Human-readable value for display under a knob.
pub fn display(i: usize, norm: f32) -> String {
    let info = &PARAMS[i];
    let v = info.curve.map(norm);
    match info.unit {
        Unit::Pct => format!("{:.0}%", v * 100.0),
        Unit::Bipolar => format!("{:+.0}%", v * 100.0),
        Unit::Hz => {
            if v >= 1000.0 {
                format!("{:.1}k", v / 1000.0)
            } else if v >= 10.0 {
                format!("{v:.0}Hz")
            } else {
                format!("{v:.2}Hz")
            }
        }
        Unit::Sec => {
            if v == 0.0 {
                "off".into()
            } else if v < 1.0 {
                format!("{:.0}ms", v * 1000.0)
            } else {
                format!("{v:.2}s")
            }
        }
        Unit::Ms => format!("{:.0}ms", v * 1000.0),
        Unit::Db => format!("{v:+.1}dB"),
        Unit::St => format!("{v:+.1}st"),
        Unit::StPerSec => format!("{v:+.1}st/s"),
        Unit::Ratio => format!("×{v:.2}"),
        Unit::Count => format!("{v:.1}"),
        Unit::Times => format!("×{v:.2}"),
        Unit::Material => morph_label(&MATERIALS, v),
        Unit::Vowel => morph_label(&VOWELS, v),
        Unit::Choice => info.curve.choices().map(|c| c[v as usize]).unwrap_or("?").into(),
    }
}

fn morph_label(names: &[&str], v: f32) -> String {
    let i = v.floor() as usize;
    let f = v - v.floor();
    if f < 0.08 || i + 1 >= names.len() {
        names[i.min(names.len() - 1)].to_string()
    } else if f > 0.92 {
        names[i + 1].to_string()
    } else {
        format!("{}›{}", names[i], names[i + 1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curves_round_trip() {
        for (i, info) in PARAMS.iter().enumerate() {
            let n = info.default_norm();
            let back = info.curve.map(n);
            let tol = (info.default.abs() * 1e-3).max(1e-3);
            assert!(
                (back - info.default).abs() <= tol,
                "param {} ({}) default {} -> {} -> {}",
                i,
                info.key,
                info.default,
                n,
                back
            );
        }
    }

    #[test]
    fn from_index_matches() {
        assert_eq!(P::from_index(P::Glitch as usize), Some(P::Glitch));
        assert_eq!(P::from_index(NUM_PARAMS), None);
    }

    #[test]
    fn plenty_of_strange_params() {
        let strange = PARAMS
            .iter()
            .filter(|p| {
                matches!(
                    p.group,
                    Group::Exciter | Group::Core | Group::Body | Group::Creature | Group::Motion | Group::Mangle
                )
            })
            .count();
        assert!(strange >= 30, "only {strange} strange params");
    }
}
