//! The suite runner: one trial per registered type, running the shared cases of its shape over the stories the
//! crate registered for it (22.4).
//!
//! A crate's integration binary calls [`trials`] once with its package name and `testing::registered()`, and hands
//! the result to `libtest-mimic` beside its own `#[trial]` functions.

use super::access::violations;
use super::fixture::{Fixture, Target};
use super::relations::grows_with;
use super::story::{Drive, EditKind, Required, STORIES, Story};
use crate::contract::{Probe, Registration, ShapeKind};
use crate::runtime::egui::access::NodeKey;
use crate::runtime::egui::fixture::AccessActionKind;
use crate::runtime::env::Scale;
use craft_behavior::Input;
use craft_theme::{Size, TextSize};
use egui::{Id, Key, Modifiers, Pos2, Rect, Vec2};
use libtest_mimic::{Failed, Trial};
use std::panic::{AssertUnwindSafe, catch_unwind};

/// What a case reads: the registered type and the stories written for it.
struct Subject {
    probe: &'static Probe,
    stories: Vec<Story>,
}

type Case = fn(&Subject) -> Result<(), String>;

/// The per-type cases the runner implements, by the name an exemption uses.
const CASES: [(&str, Case); 7] = [
    ("stories", every_required_state_has_a_story),
    ("shows", stories_show_and_hold_still),
    ("access", stories_pass_the_access_rules),
    ("text-size", stories_grow_with_text_density_and_scale),
    ("values", a_drive_ends_each_gesture_once),
    ("tab", focusable_stories_are_tab_reachable),
    ("body-regions", body_children_lie_inside_the_container),
];

/// The cases the later steps add, with the step that provides what they need.
const PENDING: [(&str, &str); 10] = [
    ("items::navigation-and-selection", "F6 items"),
    ("items::virtualisation", "F6 items"),
    ("items::rename", "F6 items"),
    ("items::cell-selection-and-columns", "F6 items"),
    ("items::scroll-shows-rows-in-the-same-frame", "F6 items"),
    ("text::ime-composition", "F6 text"),
    ("text::editor-lines-reach-accesskit", "F6 text"),
    ("text::paste-once", "F6 text"),
    ("windows::viewports-and-modal-gates", "F7 windows"),
    ("windows::shell-os-matrix", "F7 windows"),
];

/// The states a registered type must show, from the facts of its probe.
fn required(probe: &Probe) -> Vec<Required> {
    let mut states = vec![Required::Default];
    if probe.carries_value() {
        states.extend([Required::Mixed, Required::Invalid]);
    }
    if matches!(probe.input, Input::Text | Input::Editor) {
        states.push(Required::LongText);
    }
    if probe.overlay {
        states.push(Required::Open);
    }
    if matches!(probe.shape, ShapeKind::Items) {
        states.extend([Required::Empty, Required::One, Required::Many]);
    }
    states
}

fn every_required_state_has_a_story(subject: &Subject) -> Result<(), String> {
    let missing: Vec<_> =
        required(subject.probe).into_iter().filter(|state| !subject.stories.iter().any(|story| story.covers.contains(state))).collect();
    if missing.is_empty() { Ok(()) } else { Err(format!("no story covers {missing:?}")) }
}

fn stories_show_and_hold_still(subject: &Subject) -> Result<(), String> {
    for story in &subject.stories {
        let mut fixture = Fixture::story(story);
        fixture.settle();
        let (size, id) = (fixture.shown().size(), fixture.root_id());
        fixture.idle(1);
        if matches!(subject.probe.shape, ShapeKind::Leaf) && !(size.x > 0.0 && size.y > 0.0) {
            return Err(format!("{}: a leaf story allocates nothing", story.name));
        }
        if fixture.root_id() != id {
            return Err(format!("{}: the id changed between frames", story.name));
        }
        if fixture.shown().size() != size {
            return Err(format!("{}: the size changed between settled frames", story.name));
        }
    }
    Ok(())
}

fn stories_pass_the_access_rules(subject: &Subject) -> Result<(), String> {
    for story in &subject.stories {
        let mut fixture = Fixture::story(story);
        fixture.settle();
        let found = violations(fixture.ctx(), fixture.tree());
        if !found.is_empty() {
            return Err(format!("{}: {found:?}", story.name));
        }
    }
    Ok(())
}

fn stories_grow_with_text_density_and_scale(subject: &Subject) -> Result<(), String> {
    for story in &subject.stories {
        let size_with = |change: &dyn Fn(&mut Fixture)| {
            let mut fixture = Fixture::story(story);
            change(&mut fixture);
            fixture.settle();
            fixture.shown().size()
        };
        let base = size_with(&|_| {});
        let variants: [(&str, Vec2); 3] = [
            (
                "text size",
                size_with(&|f| {
                    f.text_size(TextSize::Large);
                }),
            ),
            (
                "density",
                size_with(&|f| {
                    f.density(Size::Comfortable);
                }),
            ),
            (
                "scale 2",
                size_with(&|f| {
                    f.scale(Scale(2.0));
                }),
            ),
        ];
        for (name, grown) in variants {
            if !(grows_with(base.x, grown.x) && grows_with(base.y, grown.y)) {
                return Err(format!("{}: under {name} the size went from {base:?} to {grown:?}", story.name));
            }
        }
    }
    Ok(())
}

/// The drive a story names, or the library's choice from the type's facts.
fn resolve(drive: Drive, probe: &Probe) -> Drive {
    match drive {
        Drive::Auto if matches!(probe.input, Input::Text | Input::Editor) => Drive::Type("sample"),
        Drive::Auto if probe.overlay => Drive::Pick(1),
        Drive::Auto => Drive::Activate,
        other => other,
    }
}

fn root_target(fixture: &Fixture) -> Target {
    fixture.root_id().and_then(|id| fixture.node_of(id)).unwrap_or(Target { node: NodeKey::of(Id::NULL), rect: fixture.shown() })
}

/// Activation by the `variant`th way: click, Enter, Space or AccessKit `Click`.
fn activate(fixture: &mut Fixture, variant: usize, target: &Target) {
    let none = Modifiers::NONE;
    match variant {
        0 => {
            fixture.click(target);
        }
        1 | 2 => {
            fixture.key(Key::Tab, none);
            fixture.key(if variant == 1 { Key::Enter } else { Key::Space }, none);
            fixture.idle(2);
        }
        _ => {
            fixture.access(AccessActionKind::Click, target);
        }
    }
}

/// A step by the `variant`th way: AccessKit `Increment`, or the forward key.
fn step(fixture: &mut Fixture, variant: usize, target: &Target) {
    if variant == 0 {
        fixture.access(AccessActionKind::Increment, target);
    } else {
        fixture.key(Key::Tab, Modifiers::NONE);
        fixture.key(Key::ArrowRight, Modifiers::NONE);
        fixture.idle(2);
    }
}

/// Runs the `variant`th way of `drive`; `Activate` has four ways and `Step` two.
fn run_drive(fixture: &mut Fixture, drive: Drive, variant: usize) {
    let target = root_target(fixture);
    let none = Modifiers::NONE;
    match drive {
        Drive::Activate => activate(fixture, variant, &target),
        Drive::Step => step(fixture, variant, &target),
        Drive::Drag(part) => {
            let grip = fixture.root_id().and_then(|id| fixture.part(id, part)).unwrap_or(target);
            let travel = 2.0 * super::input::max_click_dist(fixture.ctx());
            fixture.drag(&grip, Vec2::new(travel, 0.0));
        }
        Drive::Type(text) => {
            fixture.key(Key::Tab, none);
            fixture.type_text(text);
            fixture.key(Key::Enter, none);
            fixture.idle(2);
        }
        Drive::Pick(row) => {
            fixture.click(&target);
            for _ in 0..row {
                fixture.key(Key::ArrowDown, none);
            }
            fixture.key(Key::Enter, none);
            fixture.idle(2);
        }
        Drive::Custom(custom) => custom(fixture),
        _ => {}
    }
}

fn a_drive_ends_each_gesture_once(subject: &Subject) -> Result<(), String> {
    if !subject.probe.carries_value() {
        return Ok(());
    }
    let mut changed_any = false;
    let mut failures = Vec::new();
    for story in &subject.stories {
        let drive = resolve(story.drive, subject.probe);
        let runs = match drive {
            Drive::Never => continue,
            Drive::Activate => 4,
            Drive::Step => 2,
            _ => 1,
        };
        for variant in 0..runs {
            let mut fixture = Fixture::story(story);
            fixture.settle();
            let before = fixture.data().snapshot();
            fixture.take_outputs();
            run_drive(&mut fixture, drive, variant);
            if fixture.data().changed_since(&before).is_empty() {
                continue;
            }
            changed_any = true;
            let kinds: Vec<EditKind> = fixture.take_outputs().into_iter().flat_map(|output| output.edits).collect();
            let ends = kinds.iter().filter(|kind| matches!(kind, EditKind::Commit | EditKind::Set)).count();
            if ends != 1 || kinds.contains(&EditKind::Cancel) {
                failures.push(format!("{}: drive {variant} gave {kinds:?}", story.name));
            }
        }
    }
    if !changed_any {
        failures.push("no drive changed the value".to_owned());
    }
    if failures.is_empty() { Ok(()) } else { Err(failures.join("; ")) }
}

fn focusable_stories_are_tab_reachable(subject: &Subject) -> Result<(), String> {
    use super::access::is_interactive;
    use crate::runtime::egui::access::Declared;
    for story in &subject.stories {
        let mut fixture = Fixture::story(story);
        fixture.settle();
        let focusable =
            fixture.tree().keys().into_iter().any(|key| fixture.tree().declares_key(key, Declared::Focus) && is_interactive(fixture.tree(), key));
        if !focusable {
            continue;
        }
        fixture.key(Key::Tab, Modifiers::NONE);
        if fixture.focused().is_none() {
            return Err(format!("{}: Tab reached nothing", story.name));
        }
    }
    Ok(())
}

fn body_children_lie_inside_the_container(subject: &Subject) -> Result<(), String> {
    if !matches!(subject.probe.shape, ShapeKind::Body) {
        return Ok(());
    }
    for story in &subject.stories {
        let mut fixture = Fixture::story(story);
        fixture.settle();
        let Some(root) = fixture.root_id().and_then(|id| fixture.node_of(id)) else { continue };
        let mut pending = fixture.tree().children_of(root.node);
        while let Some(node) = pending.pop() {
            if let Some(bounds) = fixture.tree().bounds_of(node)
                && !super::relations::inside(bounds, root.rect)
            {
                return Err(format!("{}: a body node at {bounds:?} lies outside the container at {:?}", story.name, root.rect));
            }
            pending.extend(fixture.tree().children_of(node));
        }
    }
    Ok(())
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
        if !CASES.iter().any(|(name, _)| *name == exemption.case) {
            failures.push(format!("the exemption names the unknown case `{}`", exemption.case));
        }
    }
    for (name, case) in CASES {
        if probe.exempt.iter().any(|exemption| exemption.case == name) {
            continue;
        }
        match catch_unwind(AssertUnwindSafe(|| case(&subject))) {
            Ok(Ok(())) => {}
            Ok(Err(message)) => failures.push(format!("{name}: {message}")),
            Err(payload) => failures.push(format!("{name}: panicked: {}", panic_text(payload))),
        }
    }
    if failures.is_empty() { Ok(()) } else { Err(Failed::from(failures.join("\n"))) }
}

/// A leaf shown at a rect that is empty, not a number, infinite, over the extent cap, or one point wide does not
/// panic. This case runs on one sample leaf, because only shared code can break it.
fn leaf_degenerate_rects() -> Result<(), Failed> {
    use crate::contract::Show as _;
    use crate::testing::layout::Block;
    let nan = f32::NAN;
    let rects = [
        Rect::ZERO,
        Rect::from_min_max(Pos2::new(nan, nan), Pos2::new(nan, nan)),
        Rect::from_min_max(Pos2::ZERO, Pos2::new(f32::INFINITY, f32::INFINITY)),
        Rect::from_min_size(Pos2::ZERO, Vec2::from([1.0e9; 2])),
        Rect::from_min_size(Pos2::new(5.0, 5.0), Vec2::from([1.0; 2])),
    ];
    for rect in rects {
        let mut fixture = Fixture::new(move |ui| {
            Block::new(Vec2::from([40.0, 20.0])).show_at(ui, rect);
        });
        fixture.idle(3);
    }
    Ok(())
}

/// The trials of one package: a trial per registered type, one per story naming a type that is not linked, the
/// per-shape cases and the pending cases (ignored, with their step as the kind).
pub fn trials(package: &'static str, registered: &[&'static Registration]) -> Vec<Trial> {
    let library = package.replace('-', "_");
    let own: Vec<_> = STORIES.iter().filter(|entry| entry.crate_name == package && entry.crate_target == library).collect();
    let probes: Vec<&'static Probe> = registered.iter().copied().filter_map(Registration::probe).collect();
    let mut trials = Vec::new();
    for probe in probes.iter().copied() {
        let stories: Vec<Story> = own.iter().filter(|entry| (entry.type_name)() == probe.type_name()).map(|entry| entry.story).collect();
        trials.push(Trial::test(format!("suite::{}", probe.type_name()), move || run_type(probe, stories)));
    }
    for entry in &own {
        let name = (entry.type_name)();
        if !probes.iter().any(|probe| probe.type_name() == name) {
            let message = format!("the story `{}` shows `{name}`, which is not in the linked registrations", entry.story.name);
            trials.push(Trial::test(format!("suite::stories::{}", entry.story.name), move || Err(Failed::from(message))));
        }
    }
    trials.push(Trial::test(format!("suite::{package}::leaf-degenerate-rects"), leaf_degenerate_rects));
    for (name, step) in PENDING {
        trials.push(Trial::test(format!("suite::pending::{name}"), || Ok(())).with_ignored_flag(true).with_kind(step));
    }
    trials
}
