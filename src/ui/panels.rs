//! The parameter sections.

use dioxus::prelude::*;
use sa_engine::params::{Curve, Group, PARAMS};

use super::knob::{ChoiceWidget, Knob, LockIcon};
use crate::state::AppState;

const ROWS: [&[Group]; 4] = [
    &[Group::Exciter, Group::Body],
    &[Group::Core, Group::Creature],
    &[Group::Motion, Group::Mangle, Group::Envelope],
    &[Group::Master, Group::Eq, Group::Reverb],
];

#[component]
fn Panel(group: Group) -> Element {
    let st = use_context::<AppState>();
    let indices: Vec<usize> = (0..PARAMS.len()).filter(|&i| PARAMS[i].group == group).collect();
    let locked = indices.iter().filter(|&&i| (st.locks[i])()).count();
    // Grow panels in proportion to their widget count so rows stay balanced.
    let weight: usize = indices.iter().map(|&i| if matches!(PARAMS[i].curve, Curve::Choice(_)) { 3 } else { 1 }).sum();
    rsx! {
        section { class: "panel {group.slug()}", style: "flex-grow: {weight}",
            header {
                h2 { "{group.title()}" }
                if locked > 0 {
                    span { class: "panel-locks", LockIcon {} "{locked}" }
                }
            }
            div { class: "panel-body",
                for i in indices {
                    if matches!(PARAMS[i].curve, Curve::Choice(_)) {
                        ChoiceWidget { key: "{i}", index: i }
                    } else {
                        Knob { key: "{i}", index: i }
                    }
                }
            }
        }
    }
}

#[component]
pub fn Panels() -> Element {
    let st = use_context::<AppState>();
    let generation = (st.rnd_gen)();
    rsx! {
        div { class: "panels", key: "{generation}",
            for (r, row) in ROWS.iter().enumerate() {
                div { key: "{r}", class: "row",
                    for g in row.iter() {
                        Panel { key: "{g.slug()}", group: *g }
                    }
                }
            }
        }
    }
}
