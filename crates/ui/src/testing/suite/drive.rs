//! The drivers and the value cases: each way of changing a story's value runs in its own fixture and must end the
//! gesture once, Escape restores, a double-click resets with one `Set` and a mixed value commits a concrete one.

use super::{ANY, Case, Subject};
use crate::contract::Probe;
use crate::runtime::access::Role;
use crate::runtime::egui::access::{Declared, NodeKey};
use crate::runtime::egui::fixture::AccessActionKind;
use crate::testing::fixture::{Fixture, Target};
use crate::testing::story::{Drive, EditKind, Required, Story};
use craft_behavior::{Input, PartRef, Sense};
use egui::{Id, Key, Modifiers, Pos2, Vec2};

pub(super) const CASES: &[Case] = &[
    Case::new("values", ANY, a_drive_ends_each_gesture_once),
    Case::new("reset", ANY, a_double_click_resets_with_one_set),
    Case::new("mixed", ANY, mixed_commits_a_concrete_value),
];

/// A drive with the library's choice made.
#[derive(Clone, Copy)]
pub(super) enum Plan {
    Activate,
    Step,
    Drag(PartRef),
    DragChild(PartRef),
    Type(&'static str),
    Pick(usize),
    Custom(fn(&mut Fixture)),
    Never,
}

/// The drive a story names, or the library's choice from the type's facts and the settled fixture's shape record
/// (22.4 drivers).
pub(super) fn plan(drive: Drive, probe: &Probe, fixture: &Fixture) -> Plan {
    match drive {
        Drive::Auto if probe.carries_value() && !probe.takes_input() => Plan::Never,
        Drive::Auto if matches!(probe.input, Input::Text | Input::Editor) => Plan::Type("sample"),
        Drive::Auto if probe.overlay => Plan::Pick(1),
        Drive::Auto if probe.carries_value() => match fixture.drag_part() {
            Some(part) => Plan::Drag(part),
            None if probe.keys.activate != crate::runtime::focus::ActivateKeys::Neither => Plan::Activate,
            None => Plan::Step,
        },
        Drive::Auto | Drive::Activate => Plan::Activate,
        Drive::Step => Plan::Step,
        Drive::Drag(key) => Plan::Drag(key.into()),
        Drive::DragChild(part) => Plan::DragChild(part),
        Drive::Type(text) => Plan::Type(text),
        Drive::Pick { row } => Plan::Pick(row),
        Drive::Custom(custom) => Plan::Custom(custom),
        _ => Plan::Never,
    }
}

/// The ways `plan` runs, by variant number: the activation keys the type keeps, a step by AccessKit and by key,
/// a drag and a drag cancelled by Escape, a pick by keys and by a click on the row.
pub(super) fn variants(plan: Plan, probe: &Probe) -> Vec<usize> {
    match plan {
        Plan::Activate => {
            let mut runs = vec![0];
            if probe.keys.activate.enter() {
                runs.push(1);
            }
            if probe.keys.activate.space() {
                runs.push(2);
            }
            runs.push(3);
            runs
        }
        Plan::Step | Plan::Drag(_) | Plan::DragChild(_) | Plan::Pick(_) => vec![0, 1],
        Plan::Type(_) | Plan::Custom(_) => vec![0],
        Plan::Never => Vec::new(),
    }
}

/// Whether the variant ends by cancelling, so the value must be as it was.
pub(super) fn cancels(plan: Plan, variant: usize) -> bool {
    matches!(plan, Plan::Drag(_) | Plan::DragChild(_)) && variant == 1
}

pub(super) fn variant_name(plan: Plan, variant: usize) -> &'static str {
    match (plan, variant) {
        (Plan::Activate, 0) => "click",
        (Plan::Activate, 1) => "Enter",
        (Plan::Activate, 2) => "Space",
        (Plan::Activate, _) => "AccessKit Click",
        (Plan::Step, 0) => "AccessKit Increment",
        (Plan::Step, _) => "forward key",
        (Plan::Drag(_) | Plan::DragChild(_), 0) => "drag",
        (Plan::Drag(_) | Plan::DragChild(_), _) => "drag with Escape",
        (Plan::Pick(_), 0) => "pick by keys",
        (Plan::Pick(_), _) => "pick by click",
        (Plan::Type(_), _) => "typed text",
        (Plan::Custom(_), _) => "custom drive",
        (Plan::Never, _) => "no drive",
    }
}

/// The node of the shown component, or the whole shown area when it has none.
pub(super) fn root_target(fixture: &Fixture) -> Target {
    fixture.root_id().and_then(|id| fixture.node_of(id)).unwrap_or(Target { node: NodeKey::of(Id::NULL), rect: fixture.shown() })
}

/// The node a click or an AccessKit `Click` activates: the first clickable part, or the component itself.
fn activation_target(fixture: &Fixture) -> Target {
    let part = fixture.click_part().zip(fixture.root_id()).and_then(|(part, id)| fixture.part(id, part));
    part.unwrap_or_else(|| root_target(fixture))
}

/// Activation by the `variant`th way: click, Enter, Space or AccessKit `Click`.
pub(super) fn activate(fixture: &mut Fixture, variant: usize, target: &Target) {
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
            fixture.idle(2);
        }
    }
}

fn step(fixture: &mut Fixture, variant: usize, target: &Target) {
    if variant == 0 {
        // A component whose value lives in sub-nodes (thumbs, axes) steps through the first of them.
        let tree = fixture.tree();
        let part = fixture.by_role(Role::SLIDER).into_iter().find(|slider| tree.declares_key(slider.node, Declared::Increment));
        let target = if tree.declares_key(target.node, Declared::Increment) { *target } else { part.unwrap_or(*target) };
        fixture.access(AccessActionKind::Increment, &target);
        fixture.idle(2);
    } else {
        fixture.key(Key::Tab, Modifiers::NONE);
        fixture.key(Key::ArrowRight, Modifiers::NONE);
        fixture.idle(2);
    }
}

/// A drag from `from` by twice the slop along x, ended by a release or, with `escape`, by Escape before it.
fn drag(fixture: &mut Fixture, from: Pos2, escape: bool) {
    let travel = 2.0 * crate::testing::input::max_click_dist(fixture.ctx());
    if !escape {
        fixture.drag_at(from, Vec2::new(travel, 0.0));
        return;
    }
    fixture.frame(vec![crate::testing::pointer::moved(from)]);
    fixture.frame(vec![crate::testing::pointer::press(from)]);
    for at in 1..=4 {
        fixture.frame(vec![crate::testing::pointer::moved(from + Vec2::new(travel, 0.0) * (at as f32 / 4.0))]);
    }
    fixture.key(Key::Escape, Modifiers::NONE);
    fixture.frame(vec![crate::testing::pointer::release(from + Vec2::new(travel, 0.0))]);
    fixture.idle(2);
}

/// The rows a popup offers: the options, menu entries, tree items and list items (a `TileGrid` tile) on screen.
fn pickable(fixture: &Fixture) -> Vec<Target> {
    [Role::LIST_BOX_OPTION, Role::MENU_ITEM, Role::TREE_ITEM, Role::LIST_ITEM].into_iter().flat_map(|role| fixture.by_role(role)).collect()
}

fn pick(fixture: &mut Fixture, row: usize, by_click: bool) {
    let none = Modifiers::NONE;
    let target = root_target(fixture);
    // Rows the component shows before its popup opens are not the popup's.
    let before: Vec<NodeKey> = pickable(fixture).into_iter().map(|row| row.node).collect();
    fixture.click(&target);
    if by_click {
        let mut rows: Vec<Target> = pickable(fixture).into_iter().filter(|row| row.node != target.node && !before.contains(&row.node)).collect();
        rows.sort_by(|a, b| (a.rect.min.y, a.rect.min.x).partial_cmp(&(b.rect.min.y, b.rect.min.x)).unwrap_or(std::cmp::Ordering::Equal));
        if let Some(row) = rows.get(row).copied() {
            super::row_click::click(fixture, &row);
        }
    } else {
        for _ in 0..row {
            fixture.key(Key::ArrowDown, none);
        }
        fixture.key(Key::Enter, none);
    }
    fixture.idle(2);
}

/// Runs the `variant`th way of `plan` on a settled fixture.
pub(super) fn run(fixture: &mut Fixture, plan: Plan, variant: usize) {
    let target = root_target(fixture);
    match plan {
        Plan::Activate => activate(fixture, variant, &activation_target(fixture)),
        Plan::Step => step(fixture, variant, &target),
        Plan::Drag(part) => {
            let grip = fixture.root_id().and_then(|id| fixture.part(id, part)).unwrap_or(target);
            drag(fixture, grip.center(), variant == 1);
        }
        Plan::DragChild(part) => {
            let owner = fixture.root_id().unwrap_or(Id::NULL);
            let grip =
                crate::runtime::pointer::child_part_bounds(fixture.ctx(), owner, part, Sense::DRAG).map_or(target.center(), |rect| rect.center());
            drag(fixture, grip, variant == 1);
        }
        Plan::Type(text) => {
            fixture.key(Key::Tab, Modifiers::NONE);
            fixture.type_text(text);
            fixture.key(Key::Enter, Modifiers::NONE);
            fixture.idle(2);
        }
        Plan::Pick(row) => pick(fixture, row, variant == 1),
        Plan::Custom(custom) => custom(fixture),
        Plan::Never => {}
    }
}

/// A settled fixture of `story` with its drive resolved.
pub(super) fn settled(story: &Story, probe: &Probe) -> (Fixture, Plan) {
    let mut fixture = Fixture::story(story);
    fixture.settle();
    let plan = plan(story.drive, probe, &fixture);
    (fixture, plan)
}

fn a_drive_ends_each_gesture_once(subject: &Subject) -> Result<(), String> {
    if !subject.probe.carries_value() {
        return Ok(());
    }
    let mut changed_any = false;
    let mut failures = Vec::new();
    for story in &subject.stories {
        let (_, plan) = settled(story, subject.probe);
        let mut changed_by = Vec::new();
        let mut unchanged_by = Vec::new();
        for variant in variants(plan, subject.probe) {
            let (mut fixture, _) = settled(story, subject.probe);
            let before = fixture.data().snapshot();
            fixture.take_outputs();
            run(&mut fixture, plan, variant);
            let changed = !fixture.data().changed_since(&before).is_empty();
            let kinds: Vec<EditKind> = fixture.take_outputs().into_iter().flat_map(|output| output.edits).collect();
            let name = variant_name(plan, variant);
            if cancels(plan, variant) {
                if changed || kinds.iter().any(|kind| matches!(kind, EditKind::Commit | EditKind::Set)) {
                    failures.push(format!("{}: {name} left a changed value or a commit: {kinds:?}", story.name));
                }
                continue;
            }
            if !changed {
                unchanged_by.push(name);
                continue;
            }
            changed_by.push(name);
            let ends = kinds.iter().filter(|kind| matches!(kind, EditKind::Commit | EditKind::Set)).count();
            if ends != 1 || kinds.contains(&EditKind::Cancel) {
                failures.push(format!("{}: {name} gave {kinds:?}", story.name));
            }
        }
        if !changed_by.is_empty() {
            changed_any = true;
            if !unchanged_by.is_empty() {
                failures.push(format!("{}: {unchanged_by:?} changed nothing while {changed_by:?} did", story.name));
            }
        }
    }
    if !changed_any {
        failures.push("no drive changed the value".to_owned());
    }
    if failures.is_empty() { Ok(()) } else { Err(failures.join("; ")) }
}

/// A double-click on the thumb of a dragged control that resets its value gives one `Set`, with no `Begin`,
/// `Update` or `Commit` beside it. Controls with no reset value change nothing and pass.
fn a_double_click_resets_with_one_set(subject: &Subject) -> Result<(), String> {
    if !subject.probe.carries_value() {
        return Ok(());
    }
    for story in &subject.stories {
        let (mut fixture, plan) = settled(story, subject.probe);
        let Plan::Drag(part) = plan else { continue };
        run(&mut fixture, plan, 0);
        let Some(grip) = fixture.root_id().and_then(|id| fixture.part(id, part)) else { continue };
        let before = fixture.data().snapshot();
        fixture.take_outputs();
        fixture.click(&grip);
        if !fixture.data().changed_since(&before).is_empty() {
            continue;
        }
        fixture.click(&grip);
        if fixture.data().changed_since(&before).is_empty() {
            continue;
        }
        let kinds: Vec<EditKind> = fixture.take_outputs().into_iter().flat_map(|output| output.edits).collect();
        if kinds != [EditKind::Set] {
            return Err(format!("{}: a double-click reset gave {kinds:?}", story.name));
        }
    }
    Ok(())
}

/// A story that shows a mixed value reports it as mixed, and after its drive commits a value the shown value is
/// concrete.
fn mixed_commits_a_concrete_value(subject: &Subject) -> Result<(), String> {
    for story in subject.stories.iter().filter(|story| story.covers.contains(&Required::Mixed)) {
        let is_mixed = |fixture: &Fixture| {
            fixture
                .tree()
                .keys()
                .into_iter()
                .any(|key| fixture.tree().data_of(key).is_some_and(|node| node.toggled() == Some(crate::runtime::egui::access::Toggled::Mixed)))
        };
        let (mut fixture, plan) = settled(story, subject.probe);
        if !is_mixed(&fixture) {
            return Err(format!("{}: the mixed story reports no mixed node", story.name));
        }
        let Some(variant) = variants(plan, subject.probe).into_iter().find(|variant| !cancels(plan, *variant)) else { continue };
        let before = fixture.data().snapshot();
        run(&mut fixture, plan, variant);
        if !fixture.data().changed_since(&before).is_empty() && is_mixed(&fixture) {
            return Err(format!("{}: the value is still mixed after the {} committed", story.name, variant_name(plan, variant)));
        }
    }
    Ok(())
}
