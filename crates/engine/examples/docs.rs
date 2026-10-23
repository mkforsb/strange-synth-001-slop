//! Print the parameter and preset tables as Markdown (used for the README).
use sa_engine::params::{Group, PARAMS};
use sa_engine::presets::{CATEGORIES, factory};

fn main() {
    for g in Group::ALL {
        let names: Vec<String> = PARAMS
            .iter()
            .filter(|p| p.group == g)
            .map(|p| format!("**{}** — {}", p.name, p.desc))
            .collect();
        println!("- *{}*: {}", g.title(), names.join("; "));
    }
    println!();
    let f = factory();
    for cat in CATEGORIES {
        let names: Vec<&str> = f.iter().filter(|p| p.category == cat).map(|p| p.name).collect();
        println!("- **{cat}** ({}): {}", names.len(), names.join(" · "));
    }
}
