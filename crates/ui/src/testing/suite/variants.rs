//! The variants the runner makes of every story, so authors write none of them: disabled, read-only, right-to-left,
//! each motion mode and a language change.

use super::drive::{cancels, run, settled, variants};
use super::{ANY, Case, Subject};
use crate::runtime::egui::access::Declared;
use crate::runtime::env::Direction;
use crate::testing::access::{is_interactive, violations};
use crate::testing::fixture::Fixture;
use crate::testing::relations::mirrored;
use crate::testing::story::Story;
use craft_behavior::{Input, MotionMode};
use egui::{Key, Modifiers};
use std::sync::Arc;

pub(super) const CASES: &[Case] = &[
    Case::new("disabled", ANY, disabled_changes_nothing),
    Case::new("read-only", ANY, read_only_changes_nothing),
    Case::new("rtl", ANY, rtl_mirrors),
    Case::new("motion", ANY, motion_modes_settle),
    Case::new("language", ANY, a_language_change_keeps_the_ids),
];

/// Whether the drive changed a value, reported an edit or reported a click.
fn acted(fixture: &mut Fixture, before: &crate::testing::story::StorySnapshot) -> bool {
    let changed = !fixture.data().changed_since(before).is_empty();
    let outputs = fixture.take_outputs();
    changed || outputs.iter().any(|output| output.clicked || !output.edits.is_empty())
}

/// A disabled component changes no value, reports no click and takes no focus unless it explains itself: a node
/// with a description stands for the tooltip of `disabled_text`.
fn disabled_changes_nothing(subject: &Subject) -> Result<(), String> {
    for story in &subject.stories {
        let (_, plan) = settled(story, subject.probe);
        for variant in variants(plan, subject.probe).into_iter().filter(|variant| !cancels(plan, *variant)) {
            let mut fixture = Fixture::story_scoped(story, |scope| scope.enabled = Some(false));
            fixture.settle();
            let before = fixture.data().snapshot();
            fixture.take_outputs();
            run(&mut fixture, plan, variant);
            if acted(&mut fixture, &before) {
                return Err(format!("{}: a disabled story acted on drive {variant}", story.name));
            }
        }
        let mut fixture = Fixture::story_scoped(story, |scope| scope.enabled = Some(false));
        fixture.settle();
        fixture.key(Key::Tab, Modifiers::NONE);
        let tree = fixture.tree();
        let explains = tree.keys().into_iter().any(|key| tree.data_of(key).and_then(|node| node.description()).is_some_and(|text| !text.is_empty()));
        if fixture.focused().is_some() && !explains {
            return Err(format!("{}: a disabled story took focus", story.name));
        }
    }
    Ok(())
}

fn is_text(subject: &Subject) -> bool {
    matches!(subject.probe.input, Input::Text | Input::Editor)
}

/// A read-only component keeps focus, changes no value, takes no typed text or deletion
/// and reports read-only.
fn read_only_changes_nothing(subject: &Subject) -> Result<(), String> {
    if !(subject.probe.carries_value() || is_text(subject)) {
        return Ok(());
    }
    for story in &subject.stories {
        let (_, plan) = settled(story, subject.probe);
        for variant in variants(plan, subject.probe).into_iter().filter(|variant| !cancels(plan, *variant)) {
            let mut fixture = Fixture::story_scoped(story, |scope| scope.read_only = Some(true));
            fixture.settle();
            let before = fixture.data().snapshot();
            fixture.take_outputs();
            run(&mut fixture, plan, variant);
            if acted(&mut fixture, &before) {
                return Err(format!("{}: a read-only story acted on drive {variant}", story.name));
            }
        }
        let mut fixture = Fixture::story_scoped(story, |scope| scope.read_only = Some(true));
        fixture.settle();
        let tree = fixture.tree();
        let reports = tree.keys().into_iter().any(|key| tree.data_of(key).is_some_and(|node| node.is_read_only()));
        let focusable = tree.keys().into_iter().any(|key| tree.declares_key(key, Declared::Focus) && is_interactive(tree, key));
        if !reports {
            return Err(format!("{}: no node reports read-only", story.name));
        }
        let before = fixture.data().snapshot();
        fixture.take_outputs();
        fixture.key(Key::Tab, Modifiers::NONE);
        if focusable && fixture.focused().is_none() {
            return Err(format!("{}: a read-only story cannot take focus", story.name));
        }
        if matches!(subject.probe.input, Input::Editor) {
            fixture.type_text("typed");
            fixture.key(Key::Backspace, Modifiers::NONE);
            fixture.key(Key::Delete, Modifiers::NONE);
            if acted(&mut fixture, &before) {
                return Err(format!("{}: a read-only editor took typing or deletion", story.name));
            }
        }
    }
    Ok(())
}

/// Under right-to-left the shown area keeps its size and every interactive node lies where the left-to-right
/// layout reflected across the shown area puts it.
fn rtl_mirrors(subject: &Subject) -> Result<(), String> {
    for story in &subject.stories {
        let show = |direction: Direction| {
            let mut fixture = Fixture::story(story);
            fixture.direction(direction);
            fixture.settle();
            fixture
        };
        let (ltr, rtl) = (show(Direction::Ltr), show(Direction::Rtl));
        let within = ltr.shown();
        if (ltr.shown().width() - rtl.shown().width()).abs() > 0.5 {
            return Err(format!("{}: the width differs between directions", story.name));
        }
        let tree = ltr.tree();
        for key in tree.keys().into_iter().filter(|key| is_interactive(tree, *key)) {
            let (Some(a), Some(b)) = (tree.bounds_of(key), rtl.tree().bounds_of(key)) else { continue };
            if !mirrored(a, b, within) {
                return Err(format!("{}: a node at {a:?} is at {b:?} under right-to-left, not its reflection", story.name));
            }
        }
    }
    Ok(())
}

fn bounds_after_drive(story: &Story, subject: &Subject, motion: MotionMode) -> Vec<Vec<egui::Rect>> {
    let (_, plan) = settled(story, subject.probe);
    let mut fixture = Fixture::story(story);
    fixture.motion(motion);
    fixture.settle();
    if let Some(variant) = variants(plan, subject.probe).into_iter().find(|variant| !cancels(plan, *variant)) {
        run(&mut fixture, plan, variant);
    }
    let mut frames = Vec::new();
    for _ in 0..4 {
        fixture.idle(1);
        frames.push(fixture.shape_bounds().to_vec());
    }
    frames
}

/// Under `Off` a driven story is at rest one frame after the drive; under `Reduced` its nodes do not travel.
fn motion_modes_settle(subject: &Subject) -> Result<(), String> {
    for story in &subject.stories {
        let off = bounds_after_drive(story, subject, MotionMode::Off);
        if off.windows(2).any(|pair| pair[0] != pair[1]) {
            return Err(format!("{}: under Off the painted shapes still change after the drive", story.name));
        }
        let (_, plan) = settled(story, subject.probe);
        let mut fixture = Fixture::story(story);
        fixture.motion(MotionMode::Reduced);
        fixture.settle();
        if let Some(variant) = variants(plan, subject.probe).into_iter().find(|variant| !cancels(plan, *variant)) {
            run(&mut fixture, plan, variant);
        }
        let first = fixture.shown();
        fixture.idle(4);
        if fixture.shown().min != first.min {
            return Err(format!("{}: under Reduced the shown area travelled from {first:?} to {:?}", story.name, fixture.shown()));
        }
    }
    Ok(())
}

/// A host whose catalogs offer `ja`, in English.
fn hosts() -> Option<(Arc<craft_host::Host>, Arc<craft_host::Host>)> {
    use craft_host::{AppInfo, Host};
    let (i18n, problems) = craft_i18n::I18n::from_owned([("ja".to_owned(), String::new(), String::new())], &["ja"]);
    if !problems.is_empty() {
        return None;
    }
    let i18n = Arc::new(i18n);
    let japanese = i18n.from_code("ja")?;
    let english = Arc::new(Host::builder(AppInfo { id: "suite", name: "Suite", version: "0" }).i18n(i18n).build());
    Some((english.clone(), Arc::new(english.with_lang(japanese))))
}

/// After a language change to a non-Latin language the ids are as before, a settled frame runs one pass (no
/// font-coverage discard) and the access rules still hold.
fn a_language_change_keeps_the_ids(subject: &Subject) -> Result<(), String> {
    let Some((english, japanese)) = hosts() else { return Err("the suite's language catalogs did not build".to_owned()) };
    for story in &subject.stories {
        let mut fixture = Fixture::story(story);
        fixture.host(english.clone());
        fixture.settle();
        let id = fixture.root_id();
        fixture.host(japanese.clone());
        fixture.idle(4);
        if fixture.root_id() != id {
            return Err(format!("{}: the id changed with the language", story.name));
        }
        if fixture.passes() != 1 {
            return Err(format!("{}: a settled frame ran {} passes after the language changed", story.name, fixture.passes()));
        }
        let found = violations(fixture.ctx(), fixture.tree());
        if !found.is_empty() {
            return Err(format!("{}: {found:?}", story.name));
        }
    }
    Ok(())
}
