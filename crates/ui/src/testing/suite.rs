//! The suite runner: one trial per registered type, running the shared cases of its shape over the stories the
//! crate registered for it (22.4). A registered pattern gets one trial that shows its stories (24.4).
//!
//! A crate's integration binary calls [`trials`] once with its package name and `testing::registered()`, and hands
//! the result to `libtest-mimic` beside its own `#[trial]` functions.
//!
//! The cases live in modules by family. A module exports `CASES`, its list of [`Case`]s; [`GROUPS`] lists the
//! modules. A new family of cases adds a module and one line there.

mod activation;
mod body;
mod collection;
mod drive;
mod nodes;
mod pattern;
mod pointer;
mod shape;
mod size;
mod states;
mod text;
mod variants;

use super::story::{STORIES, Story};
use crate::contract::{PatternProbe, Probe, Registration, ShapeKind};
use libtest_mimic::{Failed, Trial};
use std::panic::{AssertUnwindSafe, catch_unwind};

/// What a case reads: the registered type and the stories written for it.
struct Subject {
    probe: &'static Probe,
    stories: Vec<Story>,
}

/// One shared case: its name (the name an exemption uses), the shapes it runs on and the check.
struct Case {
    name: &'static str,
    shapes: &'static [ShapeKind],
    run: fn(&Subject) -> Result<(), String>,
}

impl Case {
    const fn new(name: &'static str, shapes: &'static [ShapeKind], run: fn(&Subject) -> Result<(), String>) -> Case {
        Case { name, shapes, run }
    }
}

/// Every shape.
const ANY: &[ShapeKind] = &[ShapeKind::Leaf, ShapeKind::Body, ShapeKind::Items];

/// The container and collection rows. Each case returns `Ok` for a type of another shape.
const SHAPES: &[Case] = &[
    Case::new("container-title", ANY, body::title_names_the_container),
    Case::new("layer-escape", ANY, body::layer_closes_on_escape),
    Case::new("layer-trap", ANY, body::layer_traps_focus),
    Case::new("layer-focus-return", ANY, body::layer_leaves_no_dangling_focus),
    Case::new("layer-placement", ANY, body::layer_remembers_its_placement),
    Case::new("container-scroll", ANY, body::overflowing_body_scrolls_to_its_end),
    Case::new("items-keys", ANY, collection::keys_move_the_lead),
    Case::new("items-click-select", ANY, collection::click_selects_one_row),
    Case::new("items-type-ahead", ANY, collection::type_ahead_moves_the_lead),
    Case::new("items-rename", ANY, collection::rename_commits_and_cancels_once),
    Case::new("items-reorder", ANY, collection::keyboard_reorder_ends_once),
    Case::new("items-empty", ANY, collection::empty_collection_shows_no_rows),
    Case::new("items-virtualisation", ANY, collection::long_collection_builds_only_visible_rows),
    Case::new("items-scroll-frame", ANY, collection::scroll_shows_rows_in_the_same_frame),
    Case::new("items-cells", ANY, collection::cells_and_columns_are_reported),
];

/// The case modules, in the order their cases run.
const GROUPS: &[&[Case]] = &[states::CASES, size::CASES, drive::CASES, activation::CASES, variants::CASES, text::CASES, pointer::CASES, SHAPES];

fn cases() -> impl Iterator<Item = &'static Case> {
    GROUPS.iter().flat_map(|group| group.iter())
}

fn panic_text(payload: Box<dyn std::any::Any + Send>) -> String {
    match payload.downcast::<String>() {
        Ok(text) => *text,
        Err(payload) => payload.downcast_ref::<&str>().map_or_else(|| "a panic".to_owned(), |text| (*text).to_owned()),
    }
}

fn run_type(probe: &'static Probe, stories: Vec<Story>) -> Result<(), Failed> {
    let subject = Subject { probe, stories };
    let mut failures = Vec::new();
    for exemption in probe.exempt {
        if !states::exemption_is_known(exemption.case) {
            failures.push(format!("the exemption names the unknown case `{}`", exemption.case));
        }
    }
    for case in cases() {
        if !case.shapes.contains(&probe.shape) || probe.exempt.iter().any(|exemption| exemption.case == case.name) {
            continue;
        }
        match catch_unwind(AssertUnwindSafe(|| (case.run)(&subject))) {
            Ok(Ok(())) => {}
            Ok(Err(message)) => failures.push(format!("{}: {message}", case.name)),
            Err(payload) => failures.push(format!("{}: panicked: {}", case.name, panic_text(payload))),
        }
    }
    if failures.is_empty() { Ok(()) } else { Err(Failed::from(failures.join("\n"))) }
}

/// The trials of one package: a trial per registered type and one per story naming a type that is not linked. The
/// per-shape cases belong to `craft-ui`'s own package and run once there.
pub fn trials(package: &'static str, registered: &[&'static Registration]) -> Vec<Trial> {
    let library = package.replace('-', "_");
    let own: Vec<_> = STORIES.iter().filter(|entry| entry.crate_name == package && entry.crate_target == library).collect();
    let probes: Vec<&'static Probe> = registered.iter().copied().filter_map(Registration::probe).collect();
    let mut trials = Vec::new();
    for probe in probes.iter().copied() {
        let stories: Vec<Story> = own.iter().filter(|entry| (entry.type_name)() == probe.type_name()).map(|entry| entry.story).collect();
        trials.push(Trial::test(format!("suite::{}", probe.type_name()), move || run_type(probe, stories)));
    }
    let patterns: Vec<&'static PatternProbe> = registered.iter().copied().filter_map(Registration::pattern_probe).collect();
    for pattern in &patterns {
        let stories: Vec<Story> = own.iter().filter(|entry| (entry.type_name)() == pattern.type_name()).map(|entry| entry.story).collect();
        let name = pattern.type_name();
        trials.push(Trial::test(format!("suite::{name}"), move || pattern::run(name, &stories).map_err(Failed::from)));
    }
    for entry in &own {
        let name = (entry.type_name)();
        if !probes.iter().any(|probe| probe.type_name() == name) && !patterns.iter().any(|pattern| pattern.type_name() == name) {
            let message = format!("the story `{}` shows `{name}`, which is not in the linked registrations", entry.story.name);
            trials.push(Trial::test(format!("suite::stories::{}", entry.story.name), move || Err(Failed::from(message))));
        }
    }
    for entry in STORIES.iter().filter(|entry| entry.crate_name == package && entry.crate_target != library) {
        let message = format!(
            "the story `{}` is declared in the target `{}`; stories live in the library crate's `stories` module",
            entry.story.name, entry.crate_target
        );
        trials.push(Trial::test(format!("suite::stories::{}", entry.story.name), move || Err(Failed::from(message))));
    }
    trials.extend(shape::trials(package));
    trials
}
