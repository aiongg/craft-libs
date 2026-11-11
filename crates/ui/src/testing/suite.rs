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
mod purity;
mod row_click;
mod shape;
mod size;
mod states;
mod style;
mod text;
mod variants;

pub use body::title_case;
pub use drive::values_case;
pub use purity::measure_pure_case;
pub use size::{reflowed, text_size_case};
pub use states::required_states;
pub use variants::rtl_case;

use super::fixture::Fixture;
use super::story::{Story, stories};
use crate::contract::{PatternProbe, Probe, Registration, ShapeKind};
use libtest_mimic::{Failed, Trial};
use std::cell::OnceCell;
use std::panic::{AssertUnwindSafe, catch_unwind};

/// What a case reads: the registered type, the stories written for it, and what the cases share of them.
struct Subject {
    probe: &'static Probe,
    stories: Vec<Story>,
    /// Each story's [`Base`], built when a case first asks for it.
    bases: Vec<OnceCell<Base>>,
    /// The style sources the style cases measure, read once for all of them.
    style_sources: OnceCell<Result<Vec<style::Source>, String>>,
    /// The context the style cases resolve looks in.
    style_ctx: OnceCell<egui::Context>,
}

/// A story shown in the default room and env and settled (`Fixture::story` then `settle`), with no input, and the
/// drive the runner resolves from it. Cases that only read a settled story, or need only its drive, share it, so a
/// story settles once per type for them rather than once per case. A case that sends input builds its own fixture.
struct Base {
    fixture: Fixture,
    plan: drive::Plan,
}

impl Subject {
    fn new(probe: &'static Probe, stories: Vec<Story>) -> Self {
        let bases = stories.iter().map(|_| OnceCell::new()).collect();
        Subject { probe, stories, bases, style_sources: OnceCell::new(), style_ctx: OnceCell::new() }
    }

    /// The shared [`Base`] of `story`, which is one of `self.stories`.
    fn base(&self, story: &Story) -> &Base {
        let index = self.stories.iter().position(|known| std::ptr::eq(known, story));
        assert!(index.is_some(), "the story `{}` is not one of the subject's", story.name);
        let index = index.unwrap_or_default();
        self.bases[index].get_or_init(|| Base::of(story, self.probe))
    }

    /// Whether the case `name` runs for this type: its shapes take the type and the type is not exempt from it.
    fn runs(&self, name: &str) -> bool {
        cases().any(|case| case.name == name && runs_for(case, self.probe))
    }
}

impl Base {
    fn of(story: &Story, probe: &Probe) -> Base {
        let (fixture, plan) = drive::settled(story, probe);
        Base { fixture, plan }
    }

    /// The settled fixture and the drive.
    fn parts(&self) -> (&Fixture, drive::Plan) {
        (&self.fixture, self.plan)
    }
}

fn runs_for(case: &Case, probe: &Probe) -> bool {
    case.shapes.contains(&probe.shape) && !probe.exempt.iter().any(|exemption| exemption.case == case.name)
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
    Case::new("items-virtualisation", ANY, collection::long_collection_builds_rows_within_the_reach_only),
    Case::new("items-scroll-frame", ANY, collection::scroll_shows_rows_in_the_same_frame),
    Case::new("items-cells", ANY, collection::cells_and_columns_are_reported),
];

/// The case modules, in the order their cases run.
const GROUPS: &[&[Case]] = &[
    states::CASES,
    size::CASES,
    drive::CASES,
    activation::CASES,
    variants::CASES,
    text::CASES,
    pointer::CASES,
    purity::CASES,
    SHAPES,
    shape::CASES,
    style::CASES,
];

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
    let subject = Subject::new(probe, stories);
    let mut failures = Vec::new();
    for exemption in probe.exempt {
        if !states::exemption_is_known(exemption.case) {
            failures.push(format!("the exemption names the unknown case `{}`", exemption.case));
        }
    }
    for case in cases() {
        if !runs_for(case, probe) {
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
    let own: Vec<_> = stories().into_iter().filter(|entry| entry.crate_name == package && entry.crate_target == library).collect();
    let probes: Vec<&'static Probe> = registered.iter().copied().filter_map(Registration::probe).collect();
    let mut trials = Vec::new();
    for probe in probes.iter().copied() {
        let mut stories: Vec<Story> = own.iter().filter(|entry| (entry.type_name)() == probe.type_name()).map(|entry| entry.story).collect();
        if stories.is_empty() {
            stories.extend(probe.sample_story.map(|story| story()));
        }
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
    for entry in stories().into_iter().filter(|entry| entry.crate_name == package && entry.crate_target != library) {
        let message = format!(
            "the story `{}` is declared in the target `{}`; stories live in the library crate's `stories` module",
            entry.story.name, entry.crate_target
        );
        trials.push(Trial::test(format!("suite::stories::{}", entry.story.name), move || Err(Failed::from(message))));
    }
    trials.extend(shape::trials(package));
    trials
}
