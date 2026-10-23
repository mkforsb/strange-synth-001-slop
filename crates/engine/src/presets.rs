//! Factory presets, written in real units (seconds, Hz, semitones, 0..1
//! amounts) on top of the default patch.

use crate::params::{P, Patch, default_patch};

#[derive(Clone, Debug)]
pub struct Preset {
    pub name: &'static str,
    pub category: &'static str,
    pub patch: Patch,
}

fn mk(name: &'static str, category: &'static str, overrides: &[(P, f32)]) -> Preset {
    let mut patch = default_patch();
    for &(p, v) in overrides {
        patch[p as usize] = p.info().curve.unmap(v);
    }
    Preset { name, category, patch }
}

macro_rules! pr {
    ($name:literal, $cat:literal $(, $k:ident = $v:expr)* $(,)?) => {
        mk($name, $cat, &[$((P::$k, $v as f32)),*])
    };
}

#[rustfmt::skip]
pub const CATEGORIES: [&str; 11] = [
    "Init", "Metal", "Creature", "Machine", "Space", "Glass & Air", "Strings & Sticks", "Percussion", "Drones", "Chaos", "Keys & Leads",
];

#[rustfmt::skip]
pub fn factory() -> Vec<Preset> {
    vec![
        pr!("Init — Struck Bar", "Init", Level = 0.4),
        // ── Metal ───────────────────────────────────────────────────────
        pr!("Metal-on-Metal Screeching Industrial Elevator", "Metal",
            Strike = 0.3, Hardness = 0.9, Bow = 0.8, Pressure = 0.85, Grit = 0.4,
            Material = 3.6, Stretch = 0.2, RingTime = 3.0, Damping = 0.15, Shimmer = 0.4, Rattle = 0.35,
            Screech = 0.65, Chaos = 0.35, Attractor = 2.0, Gravity = -1.5, Drift = 0.4, Wobble = 0.2, WobRate = 0.7,
            Attack = 0.3, Release = 2.5, Cutoff = 9000, Resonance = 0.3, LowCut = 60, EqMid = 4, EqMidFreq = 2500,
            RvMix = 0.35, RvDecay = 0.8, Size = 1.6, Level = -21.4),
        pr!("Struck Bowed Metal Sheet", "Metal",
            Strike = 1.0, Hardness = 0.8, Position = 0.17, Bow = 0.45, Pressure = 0.35,
            Material = 3.0, Stretch = 0.05, RingTime = 8.0, Damping = 0.2, Shimmer = 0.55,
            Wobble = 0.15, WobRate = 0.3, Spread = 0.6, Decay = 6.0, Sustain = 0.7, Release = 4.0,
            RvMix = 0.35, RvDecay = 0.85, Size = 1.5, Level = -16.7),
        pr!("Resonating Metal Clothes Rack", "Metal",
            Strike = 1.0, Hardness = 0.95, Position = 0.11, Grit = 0.25,
            Material = 4.0, Stretch = 0.1, RingTime = 5.0, Damping = 0.25, Shimmer = 0.6, Rattle = 0.6,
            Drift = 0.15, Spread = 0.8, Decay = 6.0, Release = 4.0, RvMix = 0.3, Size = 1.2, Level = -13.1),
        pr!("Rusted Ship Hull", "Metal",
            Strike = 1.0, Hardness = 0.6, Grit = 0.5, Material = 3.3, Stretch = -0.2, BodyTune = -24,
            RingTime = 9.0, Damping = 0.3, Shimmer = 0.35, Rattle = 0.3, Gravity = -0.4,
            Release = 5.0, RvMix = 0.4, Size = 2.0, RvDecay = 0.85, RvDamp = 0.6, Level = -20.9),
        pr!("Anvil Choir", "Metal",
            Strike = 1.0, Hardness = 1.0, Material = 4.4, RingTime = 4.0, Damping = 0.2, Shimmer = 0.3, Modes = 10,
            Throat = 0.6, Vowel = 3.0, Release = 2.5, RvMix = 0.35, RvDecay = 0.8, Level = -9.4),
        pr!("Gong of the Deep", "Metal",
            Strike = 1.0, Hardness = 0.25, Material = 3.2, BodyTune = -12, RingTime = 20.0, Damping = 0.15, Shimmer = 0.7,
            Wobble = 0.15, WobRate = 0.4, Sweep = -1.5, SweepTime = 2.0, Decay = 20.0, Release = 8.0,
            RvMix = 0.4, Size = 2.0, RvDecay = 0.9, Level = -18.4),
        pr!("Bent Saw Blade", "Metal",
            Strike = 0.0, Bow = 0.6, Pressure = 0.3, Material = 3.8, Stretch = 0.3, RingTime = 3.0, Shimmer = 0.2,
            Wobble = 0.45, WobRate = 5.0, Glide = 0.3, Throat = 0.2, Vowel = 4.0,
            Attack = 0.2, Release = 1.5, RvMix = 0.35, Level = -10.7),
        pr!("Spring Tank Accident", "Metal",
            Strike = 1.0, Hardness = 0.9, Material = 1.5, Stretch = 0.7, RingTime = 2.5, Rattle = 0.8, Shimmer = 0.5,
            Sweep = -10, SweepTime = 0.15, RvMix = 0.5, RvDecay = 0.75, Level = -13.5),
        pr!("Bridge Cable Twang", "Metal",
            Strike = 1.0, Hardness = 0.8, Position = 0.05, Material = 0.3, Stretch = 0.5, BodyTune = -12,
            RingTime = 6.0, Damping = 0.1, Sweep = -6, SweepTime = 0.04, Release = 3.0, RvMix = 0.35, Size = 2.0, Level = -5.7),
        pr!("Scrap Yard Wind Chimes", "Metal",
            Strike = 0.9, Hardness = 1.0, Position = 0.3, Grit = 0.35, Material = 4.6, RingTime = 6.0, Shimmer = 0.2,
            Drift = 0.2, Spread = 1.0, Release = 6.0, RvMix = 0.4, RvDecay = 0.8, Level = -15.2),
        // ── Creature ────────────────────────────────────────────────────
        pr!("T-Rex Roar", "Creature",
            Strike = 0.0, Breath = 0.7, Core = 0.7, Feed = 0.3, Warp = 0.6, Fold = 0.35, Undertone = 0.8,
            Body = 0.4, Material = 1.0, RingTime = 0.3,
            Throat = 0.85, Vowel = 2.6, Growl = 0.85, Sweep = 7, SweepTime = 0.6, Gravity = -4, Drift = 0.5,
            Attack = 0.15, Decay = 2.5, Sustain = 0.5, Release = 1.2,
            Cutoff = 3500, Resonance = 0.25, Drive = 0.6, EqLo = 6,
            RvMix = 0.3, RvDecay = 0.75, Size = 1.8, Predelay = 0.04, Level = -11.7),
        pr!("Godzilla Laser Beam", "Creature",
            Strike = 0.0, Core = 0.8, Feed = 0.5, Warp = 0.8, SelfFm = 0.5, FmRatio = 2.0, FmDepth = 0.5, Fold = 0.6,
            Chaos = 0.3, Attractor = 120, Body = 0.3, Material = 6.0, RingTime = 0.6,
            Throat = 0.3, Vowel = 5.0, Growl = 0.3, Sweep = 36, SweepTime = 0.08, Gravity = -10,
            Spin = 0.3, RingMod = 0.4, RingFreq = 900, Screech = 0.45, Glitch = 0.15,
            Attack = 0.01, Decay = 1.5, Sustain = 0.8, Release = 1.0,
            FilterType = 0, Cutoff = 12000, Resonance = 0.4, Drive = 0.5, RvMix = 0.35, RvDecay = 0.8, Level = -10.2),
        pr!("Swamp Throat", "Creature",
            Strike = 0.0, Breath = 0.4, Core = 0.5, Warp = 0.4, Undertone = 0.4, Body = 0.2,
            Throat = 0.9, Vowel = 0.5, Growl = 0.6, Drift = 0.4, Wobble = 0.3, WobRate = 6.0,
            Attack = 0.1, Release = 0.8, Cutoff = 2500, Drive = 0.3, Level = -10.3),
        pr!("Whale Lament", "Creature",
            Strike = 0.0, Breath = 0.15, Core = 0.6, Feed = 0.3, Warp = 0.1, Body = 0.4, Material = 1.0, RingTime = 4.0,
            Throat = 0.6, Vowel = 1.5, Sweep = 5, SweepTime = 1.5, Gravity = -1, Drift = 0.3, Glide = 0.8,
            Attack = 0.8, Release = 3.0, Cutoff = 3000, RvMix = 0.55, RvDecay = 0.92, Size = 2.0, Level = -10.3),
        pr!("Kaiju Footstep", "Creature",
            Strike = 1.0, Hardness = 0.1, Material = 2.0, BodyTune = -24, RingTime = 2.0, Damping = 0.6, Rattle = 0.4,
            Growl = 0.6, Sweep = -12, SweepTime = 0.3, Decay = 2.5, Sustain = 0.0, Release = 2.0,
            Cutoff = 900, Drive = 0.7, EqLo = 8, RvMix = 0.35, Size = 2.0, RvDecay = 0.8, Level = -17.3),
        pr!("Banshee Wail", "Creature",
            Strike = 0.0, Breath = 0.6, Core = 0.4, Body = 0.3,
            Throat = 0.8, Vowel = 4.6, Sweep = 12, SweepTime = 1.2, Gravity = -3, Wobble = 0.4, WobRate = 6.5,
            Screech = 0.25, Attack = 0.6, Release = 2.5, RvMix = 0.5, RvDecay = 0.88, Level = -6.9),
        pr!("Goblin Chatter", "Creature",
            Strike = 0.0, Core = 0.6, Logic = 0.8, Formula = 1, Body = 0.2,
            Throat = 0.9, Vowel = 2.5, Growl = 0.3, Drift = 0.6, Wobble = 0.7, WobRate = 11, Glitch = 0.4,
            Attack = 0.005, Decay = 0.25, Sustain = 0.6, Release = 0.2, Level = 6.0),
        pr!("Insect Swarm", "Creature",
            Strike = 0.0, Breath = 0.3, Core = 0.4, FmRatio = 7.3, FmDepth = 0.5, Body = 0.2,
            Throat = 0.4, Vowel = 5.0, Drift = 0.8, Wobble = 0.5, WobRate = 28, Spin = 0.6, Spread = 1.0,
            RingMod = 0.3, RingFreq = 230, Attack = 0.5, Release = 1.5, Cutoff = 6000, Level = 3.4),
        pr!("Dragon Hatchling", "Creature",
            Strike = 0.0, Breath = 0.4, Core = 0.5, Warp = 0.5, Body = 0.2,
            Throat = 0.7, Vowel = 3.2, Growl = 0.45, Sweep = 9, SweepTime = 0.25, Gravity = -6,
            Attack = 0.02, Decay = 0.8, Sustain = 0.3, Release = 0.6, Drive = 0.3, Level = -4.5),
        // ── Machine ─────────────────────────────────────────────────────
        pr!("Thoughts of the Machine", "Machine",
            Strike = 0.4, Hardness = 1.0, Core = 0.2, Feed = 0.5, FmRatio = 3.5, FmDepth = 0.2,
            Chaos = 0.5, Attractor = 0.4, Logic = 0.55, Formula = 6,
            Body = 0.6, Material = 7.0, RingTime = 2.0,
            Throat = 0.5, Vowel = 3.5, Wobble = 0.3, WobRate = 0.25, Spin = 0.4, Crush = 0.25, Glitch = 0.55,
            Attack = 0.4, Release = 3.0, FilterType = 2, Cutoff = 1600, Resonance = 0.5, Follow = 0.5,
            RvMix = 0.5, RvDecay = 0.88, Size = 1.7, Level = 6.0),
        pr!("Server Farm at 3AM", "Machine",
            Strike = 0.0, Core = 0.2, Feed = 0.3, FmRatio = 5.0, Logic = 0.35, Formula = 0, Breath = 0.1,
            Body = 0.5, Material = 3.0, RingTime = 0.8, Crush = 0.15, Glitch = 0.2,
            Attack = 0.3, FilterType = 0, Cutoff = 3000, Resonance = 0.3, RvMix = 0.3, Level = -2.9),
        pr!("Dial-Up Prophecy", "Machine",
            Strike = 0.0, Body = 0.0, Core = 0.3, FmRatio = 2.7, FmDepth = 0.4, Logic = 0.6, Formula = 3,
            RingMod = 0.5, RingFreq = 1800, Crush = 0.4, Glitch = 0.5,
            FilterType = 2, Cutoff = 2200, Resonance = 0.4, Level = -7.5),
        pr!("Broken Teletype", "Machine",
            Strike = 1.0, Hardness = 1.0, Logic = 0.2, Formula = 4, Material = 4.2, RingTime = 0.2, Mute = 0.4, Rattle = 0.7,
            Crush = 0.3, Glitch = 0.6, Decay = 0.12, Sustain = 0.0, Release = 0.1, Level = 2.3),
        pr!("Factory Floor Loop", "Machine",
            Strike = 0.5, Hardness = 0.9, Grit = 0.3, Logic = 0.5, Formula = 5, Feed = 0.8,
            Body = 0.8, Material = 4.0, RingTime = 0.6, Rattle = 0.3,
            Cutoff = 5000, Resonance = 0.35, Drive = 0.4, RvMix = 0.25, Size = 1.3, Level = -11.1),
        pr!("Robot Lullaby", "Machine",
            Strike = 0.4, Hardness = 0.8, Core = 0.5, Warp = 0.2, FmRatio = 2.0, FmDepth = 0.2, Logic = 0.15, Formula = 7,
            Material = 5.0, RingTime = 2.5, Throat = 0.4, Vowel = 2.0, Wobble = 0.25, WobRate = 3.0, Crush = 0.2,
            Attack = 0.01, Decay = 1.5, Sustain = 0.4, Release = 1.5, RvMix = 0.4, Level = -2.0),
        pr!("Tape Machine Seizure", "Machine",
            Strike = 0.5, Core = 0.4, Warp = 0.5, Body = 0.5, Material = 0.0, RingTime = 2.0,
            Drift = 0.7, Wobble = 0.5, WobRate = 0.6, Gravity = -2, Crush = 0.3, Glitch = 0.85, RvMix = 0.2, Level = -2.3),
        // ── Space ───────────────────────────────────────────────────────
        pr!("UFO Engine Room", "Space",
            Strike = 0.0, Breath = 0.25, Core = 0.6, Feed = 0.4, Warp = 0.3, SelfFm = 0.3, FmRatio = 1.41, FmDepth = 0.35,
            Chaos = 0.25, Attractor = 0.6, Undertone = 0.6,
            Body = 0.6, Material = 7.0, RingTime = 4.0, Damping = 0.3,
            Wobble = 0.65, WobRate = 5.5, Spin = 0.55, RingMod = 0.3, RingFreq = 37,
            Attack = 1.5, Release = 3.0, Cutoff = 2400, Resonance = 0.55, Follow = 0.3, EqLo = 5,
            RvMix = 0.45, RvDecay = 0.9, Size = 2.0, RvMod = 0.8, Level = -2.4),
        pr!("Event Horizon", "Space",
            Strike = 0.0, Breath = 0.3, Core = 0.5, Feed = 0.4, Undertone = 0.7,
            Material = 6.5, RingTime = 15.0, Shimmer = 0.6, Gravity = -2.5, Spin = 0.4,
            Attack = 3.0, Release = 10.0, Cutoff = 4000, Follow = -0.3,
            RvMix = 0.7, RvDecay = 0.97, Size = 2.0, RvMod = 0.9, Level = -6.5),
        pr!("Pulsar Beacon", "Space",
            Strike = 1.0, Hardness = 1.0, Material = 5.5, RingTime = 1.5, Wobble = 1.0, WobRate = 2.0, Spin = 0.35,
            RingMod = 0.3, RingFreq = 2.0, Release = 2.0, RvMix = 0.5, RvDecay = 0.9, Predelay = 0.25, Level = -0.1),
        pr!("Alien Radio", "Space",
            Strike = 0.0, Breath = 0.4, Core = 0.4, FmRatio = 3.3, FmDepth = 0.6, Chaos = 0.6, Attractor = 3.0,
            Logic = 0.2, Formula = 2, Body = 0.2, Wobble = 0.3, RingMod = 0.6, RingFreq = 333, Glitch = 0.3,
            FilterType = 2, Cutoff = 1500, Resonance = 0.6, Follow = 0.4, Level = -1.4),
        pr!("Tractor Beam", "Space",
            Strike = 0.0, Core = 0.6, Warp = 0.6, Undertone = 0.5, Body = 0.2,
            Sweep = -24, SweepTime = 3.0, Wobble = 0.6, WobRate = 12, Spin = 0.5, Screech = 0.2,
            Attack = 0.4, Release = 2.0, Cutoff = 1500, Resonance = 0.75, Follow = 0.6, RvMix = 0.45, Level = -1.9),
        pr!("Wormhole Choir", "Space",
            Strike = 0.0, Breath = 0.25, Core = 0.4, Feed = 0.4, Material = 7.0, RingTime = 6.0, Shimmer = 0.6,
            Throat = 0.75, Vowel = 2.0, Gravity = 0.8, Drift = 0.25, Spin = 0.25,
            Attack = 2.0, Release = 6.0, RvMix = 0.65, RvDecay = 0.95, Size = 2.0, Level = -2.9),
        pr!("Satellite Tumble", "Space",
            Strike = 1.0, Hardness = 0.9, Core = 0.2, FmRatio = 11, FmDepth = 0.3,
            Material = 3.5, RingTime = 6.0, Shimmer = 0.4, Spin = 0.85, Glitch = 0.2, Release = 4.0, RvMix = 0.45, Level = -7.7),
        // ── Glass & Air ─────────────────────────────────────────────────
        pr!("Haunting Singing Glass", "Glass & Air",
            Strike = 0.0, Bow = 0.35, Pressure = 0.15, Breath = 0.12,
            Material = 6.0, RingTime = 12.0, Damping = 0.55, Shimmer = 0.25, Modes = 6,
            Throat = 0.25, Vowel = 1.2, Wobble = 0.2, WobRate = 4.5, Drift = 0.2, Spread = 0.6,
            Attack = 1.2, Release = 5.0, Cutoff = 9000,
            RvMix = 0.5, RvDecay = 0.9, Size = 1.8, Predelay = 0.08, RvMod = 0.6, Level = -6.8),
        pr!("Frozen Lake Ping", "Glass & Air",
            Strike = 1.0, Hardness = 1.0, Material = 6.0, Stretch = 0.25, RingTime = 3.0, Damping = 0.3,
            Sweep = -18, SweepTime = 0.03, Release = 3.0, RvMix = 0.5, RvDecay = 0.85, Predelay = 0.1, Size = 2.0, Level = -5.6),
        pr!("Breath in a Bottle", "Glass & Air",
            Strike = 0.0, Breath = 0.9, Material = 1.0, RingTime = 1.5, Damping = 0.4, Modes = 4,
            Throat = 0.3, Vowel = 1.0, Attack = 0.2, Release = 0.6, RvMix = 0.3, Level = 3.0),
        pr!("Crystal Lattice", "Glass & Air",
            Strike = 1.0, Hardness = 0.9, Position = 0.37, Material = 6.4, RingTime = 8.0, Shimmer = 0.8, Spread = 1.0,
            Decay = 8.0, Release = 6.0, RvMix = 0.45, RvDecay = 0.9, RvMod = 0.7, Level = -13.5),
        pr!("Ghost Flute", "Glass & Air",
            Strike = 0.0, Breath = 0.5, Bow = 0.15, Core = 0.25, Material = 1.0, RingTime = 2.5, Damping = 0.6, Modes = 6,
            Throat = 0.2, Drift = 0.25, Wobble = 0.15, WobRate = 5.0,
            Attack = 0.25, Release = 1.5, RvMix = 0.5, RvDecay = 0.9, Level = -5.3),
        pr!("Wet Finger on the Rim", "Glass & Air",
            Strike = 0.0, Bow = 0.25, Pressure = 0.1, Material = 6.0, Modes = 3, RingTime = 15.0, Damping = 0.7, Shimmer = 0.15,
            Attack = 0.8, Release = 3.0, RvMix = 0.4, Level = -7.7),
        // ── Strings & Sticks ────────────────────────────────────────────
        pr!("Hitting Muted Strings with a Plastic Pipe", "Strings & Sticks",
            Strike = 1.0, Hardness = 0.35, Position = 0.13, Body = 1.0,
            Material = 0.0, Stretch = 0.04, RingTime = 1.2, Damping = 0.5, Mute = 0.75, Rattle = 0.45,
            Throat = 0.25, Vowel = 1.0, Decay = 0.4, Sustain = 0.0, Release = 0.3,
            LowCut = 50, EqLo = 3, EqMid = -3, EqMidFreq = 600, RvMix = 0.15, RvDecay = 0.4, Size = 0.6, Level = 1.0),
        pr!("Piano Wire Cathedral", "Strings & Sticks",
            Strike = 1.0, Hardness = 0.7, Position = 0.12, Material = 0.0, Stretch = 0.08, RingTime = 10.0, Damping = 0.35,
            Shimmer = 0.12, Decay = 10.0, Release = 3.0, RvMix = 0.5, RvDecay = 0.9, Size = 2.0, Level = -14.1),
        pr!("Prepared Kalimba", "Strings & Sticks",
            Strike = 1.0, Hardness = 0.75, Position = 0.3, Material = 4.0, RingTime = 1.8, Mute = 0.2, Rattle = 0.5, Modes = 6,
            Decay = 2.0, Sustain = 0.0, Release = 1.0, RvMix = 0.2, Level = -5.3),
        pr!("Cello From Hell", "Strings & Sticks",
            Strike = 0.0, Bow = 0.75, Pressure = 0.65, Material = 0.0, RingTime = 2.0, Damping = 0.45,
            Throat = 0.35, Vowel = 2.5, Growl = 0.3, Drift = 0.15,
            Attack = 0.15, Release = 0.8, Cutoff = 6000, Drive = 0.3, Level = -17.5),
        pr!("Rubber Band Bass", "Strings & Sticks",
            Strike = 1.0, Hardness = 0.2, Core = 0.3, Undertone = 0.3, Material = 0.0, RingTime = 0.8, Mute = 0.3,
            Sweep = 5, SweepTime = 0.05, Glide = 0.05, FilterType = 1, Cutoff = 1800, Resonance = 0.4, EqLo = 4, Level = -2.3),
        pr!("Bowed Drum Skin", "Strings & Sticks",
            Strike = 0.3, Bow = 0.6, Pressure = 0.5, Material = 2.0, BodyTune = -12, RingTime = 3.0, Damping = 0.5,
            Attack = 0.4, Release = 2.0, RvMix = 0.35, Level = -11.4),
        pr!("Sitar of Static", "Strings & Sticks",
            Strike = 1.0, Hardness = 0.85, Position = 0.06, Grit = 0.15, Material = 0.0, RingTime = 4.0, Rattle = 0.55,
            Drift = 0.15, Crush = 0.15, Release = 2.0, RvMix = 0.3, Level = -6.7),
        // ── Percussion ──────────────────────────────────────────────────
        pr!("Junkyard Kick", "Percussion",
            Strike = 1.0, Hardness = 0.4, Material = 2.0, BodyTune = -24, RingTime = 0.5, Damping = 0.7,
            Sweep = 24, SweepTime = 0.04, Decay = 0.5, Sustain = 0.0, Release = 0.3,
            Cutoff = 3000, Drive = 0.6, EqLo = 6, RvMix = 0.1, Level = -8.2),
        pr!("Oil Drum Snare", "Percussion",
            Strike = 1.0, Hardness = 0.85, Breath = 0.4, Material = 2.4, RingTime = 0.4, Rattle = 0.9,
            Decay = 0.18, Sustain = 0.0, Release = 0.15, RvMix = 0.2, Level = -3.0),
        pr!("Pipe Organ Tom", "Percussion",
            Strike = 1.0, Hardness = 0.5, Material = 1.2, RingTime = 0.9, Sweep = -7, SweepTime = 0.12,
            Decay = 0.8, Sustain = 0.0, Release = 0.5, RvMix = 0.25, Level = -5.8),
        pr!("Glitched Hi-Hat", "Percussion",
            Strike = 1.0, Hardness = 1.0, Material = 7.0, BodyTune = 24, RingTime = 0.15, Shimmer = 0.8, Rattle = 0.5,
            Crush = 0.2, Glitch = 0.5, Decay = 0.08, Sustain = 0.0, Release = 0.08, LowCut = 3000, Level = 2.4),
        pr!("Thunder Sheet", "Percussion",
            Strike = 1.0, Hardness = 0.3, Grit = 0.6, Material = 3.0, BodyTune = -24, RingTime = 6.0, Damping = 0.2,
            Shimmer = 0.9, Rattle = 0.7, Release = 5.0, Drive = 0.3, RvMix = 0.5, Size = 2.0, RvDecay = 0.88, Level = -24.0),
        // ── Drones ──────────────────────────────────────────────────────
        pr!("Cathedral of Rust", "Drones",
            Strike = 0.0, Bow = 0.4, Pressure = 0.6, Grit = 0.2, Material = 4.8, RingTime = 12.0, Shimmer = 0.5, Drift = 0.3,
            Attack = 3.0, Release = 10.0, Cutoff = 5000, RvMix = 0.7, RvDecay = 0.96, Size = 2.0, Level = -11.6),
        pr!("Subterranean Hum", "Drones",
            Strike = 0.0, Core = 0.7, Feed = 0.3, Warp = 0.15, Undertone = 1.0, Material = 1.0, RingTime = 5.0,
            Throat = 0.3, Vowel = 0.5, Wobble = 0.3, WobRate = 0.15,
            Attack = 2.0, Release = 5.0, Cutoff = 900, Resonance = 0.4, Follow = 0.2, EqLo = 5, RvMix = 0.4, Level = -8.6),
        pr!("Lorenz Garden", "Drones",
            Strike = 0.0, Core = 0.55, Feed = 0.4, Chaos = 0.75, Attractor = 0.8, Material = 5.5, RingTime = 6.0,
            Throat = 0.3, Vowel = 3.5, Spin = 0.25, Attack = 1.5, Release = 4.0, RvMix = 0.55, RvDecay = 0.92, Level = -5.8),
        pr!("Neon Fog", "Drones",
            Strike = 0.0, Breath = 0.6, Core = 0.3, Feed = 0.3, Warp = 0.4, Material = 3.5, RingTime = 8.0, Shimmer = 0.7,
            Spin = 0.4, Drift = 0.3, Attack = 2.5, Release = 6.0,
            FilterType = 0, Cutoff = 3500, Resonance = 0.35, RvMix = 0.6, RvDecay = 0.94, Level = 0.1),
        // ── Chaos ───────────────────────────────────────────────────────
        pr!("Strange Attractor", "Chaos",
            Strike = 0.0, Core = 0.5, Feed = 0.5, SelfFm = 0.4, Fold = 0.4, Chaos = 1.0, Attractor = 40,
            Material = 7.0, RingTime = 3.0, Throat = 0.3, Vowel = 2.0, Spin = 0.3, Screech = 0.35,
            Attack = 0.05, Release = 2.0, Cutoff = 8000, Resonance = 0.3, Follow = 0.4, RvMix = 0.4, Level = -3.6),
        pr!("Butterfly Effect", "Chaos",
            Strike = 0.3, Core = 0.45, Feed = 0.3, Chaos = 0.5, Attractor = 0.3, Logic = 0.2, Formula = 6,
            Material = 6.0, Wobble = 0.2, Glitch = 0.3, Release = 3.0, RvMix = 0.5, Level = -5.9),
        pr!("Feedback Cathedral", "Chaos",
            Strike = 0.6, Bow = 0.2, Chaos = 0.4, Attractor = 1.0, Material = 5.0, RingTime = 5.0, Screech = 0.55,
            Release = 3.0, Cutoff = 7000, RvMix = 0.6, RvDecay = 0.93, Level = -15.0),
        pr!("Fractal Accordion", "Chaos",
            Strike = 0.0, Core = 0.5, Warp = 0.7, Fold = 0.5, Undertone = 0.4, Body = 0.2,
            Throat = 0.5, Vowel = 3.8, Wobble = 0.4, WobRate = 7.0,
            Attack = 0.08, Release = 0.4, Cutoff = 4000, Level = -1.1),
        // ── Keys & Leads ────────────────────────────────────────────────
        pr!("Haunted Rhodes", "Keys & Leads",
            Strike = 0.9, Hardness = 0.55, Core = 0.3, Material = 4.3, RingTime = 2.5, Damping = 0.5,
            Wobble = 0.2, WobRate = 4.5, Drift = 0.15, Crush = 0.08,
            Decay = 2.0, Sustain = 0.3, Release = 1.2, RvMix = 0.3, Level = -3.8),
        pr!("Mutant Lead", "Keys & Leads",
            Strike = 0.0, Core = 0.7, Warp = 0.7, SelfFm = 0.25, Fold = 0.3, Body = 0.2, Glide = 0.08,
            Wobble = 0.15, WobRate = 5.5, Attack = 0.005, Sustain = 0.9, Release = 0.3,
            Cutoff = 2500, Resonance = 0.55, Drive = 0.5, Follow = 0.5, Level = -11.1),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enough_presets_with_unique_names() {
        let f = factory();
        assert!(f.len() >= 50, "only {} presets", f.len());
        let mut names: Vec<_> = f.iter().map(|p| p.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), f.len(), "duplicate preset names");
        for p in &f {
            assert!(CATEGORIES.contains(&p.category), "unknown category {}", p.category);
        }
    }
}
