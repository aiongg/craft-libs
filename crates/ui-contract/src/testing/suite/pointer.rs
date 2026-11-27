//! The pointer rows of 22.4: slop and hit areas. Which shape wins an overlap is ui-rt's hit test, which its own tests cover.

use super::drive::{Plan, input_fixture, root_target};
use super::{ANY, Case, Subject, nodes};
use crate::contract::rt::harness::Target;
use crate::testing::story::{EditKind, Grip, Required};
use crate::testing::story_run::StoryHarness;
use accesskit::Action;
use craft_behavior::{PartRef, Sense};
use craft_ui_rt::{Pos2, Rect, Vec2};
use std::cell::Cell;

pub(super) const CASES: &[Case] =
    &[Case::new("pointer-slop", ANY, slop_separates_click_from_drag), Case::new("pointer-hit-areas", ANY, parts_take_presses_in_their_hit_areas)];

/// A press of `by` points along x, released where it ends.
fn gesture(fixture: &mut StoryHarness, target: &Target, by: f32) {
    fixture.drag(target, Vec2::new(by, 0.0));
}

/// A release within slop of a press on a clicking component is one click. A press that moves past slop starts one
/// drag on a component that drags and is no click on one that does not. A drag part that senses `SETS_ON_PRESS` or
/// `FOLLOWS_IN_SLOP` begins one gesture within slop too, ended by a Commit. The slop is the runtime's: it installs the
/// theme's `size.hit.slop` as egui's click distance.
fn slop_separates_click_from_drag(subject: &Subject) -> Result<(), String> {
    let probe = subject.probe;
    for story in &subject.stories {
        let (fixture, plan) = subject.base(story).parts();
        let slop = fixture.slop();
        match plan {
            Plan::Activate if !probe.carries_value() && fixture.tree().declares_key(root_target(fixture).id, Action::Click) => {
                let part = fixture.root_id().and_then(|id| fixture.click_part(id).map(|part| (part, id)));
                let target = |fixture: &StoryHarness| {
                    let target = part.and_then(|(part, id)| fixture.part(id, part)).unwrap_or_else(|| root_target(fixture));
                    fixture.visible(&target).unwrap_or(target)
                };
                for (by, expected) in [(0.5 * slop, 1), (2.0 * slop, 0)] {
                    let mut fixture = input_fixture(story);
                    let target = target(&fixture);
                    fixture.take_outputs();
                    gesture(&mut fixture, &target, by);
                    let clicks = fixture.take_outputs().iter().filter(|output| output.activated()).count();
                    if clicks != expected {
                        return Err(format!("{}: a press moved {by} points (slop {slop}) gave {clicks} clicks, expected {expected}", story.name));
                    }
                }
            }
            Plan::Drag(Grip { part, .. }) if probe.carries_value() => {
                // A part whose press sets the value begins its gesture at the press, and one that follows within slop at
                // the first move: either begins within slop.
                let senses = |sense: Sense| fixture.root_id().is_some_and(|id| fixture.part_senses(id, part, sense));
                let within = senses(Sense::SETS_ON_PRESS) || senses(Sense::FOLLOWS_IN_SLOP);
                for (by, begins) in [(0.5 * slop, usize::from(within)), (2.0 * slop, 1)] {
                    let mut fixture = input_fixture(story);
                    let Some(grip) = fixture.root_id().and_then(|id| fixture.part(id, part)).and_then(|grip| fixture.visible(&grip)) else {
                        continue;
                    };
                    fixture.take_outputs();
                    gesture(&mut fixture, &grip, by);
                    let kinds: Vec<EditKind> = fixture.take_outputs().into_iter().flat_map(|output| output.edits).collect();
                    let started = kinds.iter().filter(|kind| **kind == EditKind::Begin).count();
                    if started != begins {
                        return Err(format!(
                            "{}: a press moved {by} points (slop {slop}) began {started} drags, expected {begins}: {kinds:?}",
                            story.name
                        ));
                    }
                    if within && by < slop && kinds.last() != Some(&EditKind::Commit) {
                        return Err(format!("{}: a press within slop that begins a gesture ended {kinds:?}, not with a Commit", story.name));
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/// A point of `part` outside `root` when its hit area leaves the root's rect, else a point inside it off the middle.
fn probe_point(part: Rect, root: Rect) -> Pos2 {
    let across = |fx: f32, fy: f32| Pos2::new(part.min.x + part.width() * fx, part.min.y + part.height() * fy);
    let candidates = [(0.5, 0.5), (0.1, 0.5), (0.9, 0.5), (0.5, 0.1), (0.5, 0.9), (0.1, 0.1), (0.9, 0.9), (0.1, 0.9), (0.9, 0.1)];
    candidates.into_iter().map(|(fx, fy)| across(fx, fy)).find(|point| !root.contains(*point)).unwrap_or_else(|| across(0.75, 0.5))
}

/// A point target: input addresses the middle of its rect.
fn at(target: &Target, point: Pos2) -> Target {
    Target { bounds: Rect::from_center_size(point, Vec2::ZERO), ..target.clone() }
}

/// The visible piece of `part` of the shown component, where a press on it lands, when it is enabled and not empty: one
/// past the window's edge cannot be seen, so not pressed.
fn pressable(fixture: &StoryHarness, part: PartRef) -> Option<Target> {
    let target = fixture.root_id().and_then(|id| fixture.part(id, part)).and_then(|target| fixture.visible(&target))?;
    let enabled = fixture.tree().data_of(target.id).is_some_and(|node| !node.is_disabled());
    (enabled && target.bounds.area() > 0.0).then_some(target)
}

/// A press inside the visible piece of a part's hit area (inside the root's clip), also where it lies outside the root's
/// rect, is taken by the component; on the part a drag moves, a press off its middle moves the value. A repeated part
/// runs on a fixed sample of its repeats under the gate's fraction ([`nodes::sampled_parts`]).
fn parts_take_presses_in_their_hit_areas(subject: &Subject) -> Result<(), String> {
    let probe = subject.probe;
    for story in &subject.stories {
        let (fixture, plan) = subject.base(story).parts();
        let Some(id) = fixture.root_id() else { continue };
        let mut parts = fixture.sensing_parts(id, Sense::CLICK);
        for part in fixture.sensing_parts(id, Sense::DRAG) {
            if !parts.contains(&part) {
                parts.push(part);
            }
        }
        // A part the settled base shows no pressable piece of gets no fixture, and the sample is taken of the rest.
        parts.retain(|part| pressable(fixture, *part).is_some());
        for part in nodes::sampled_parts(parts) {
            let mut fixture = input_fixture(story);
            let Some(target) = pressable(&fixture, part) else { continue };
            let point = probe_point(target.bounds, root_target(&fixture).bounds);
            fixture.press(&at(&target, point));
            if !fixture.press_captured() {
                return Err(format!(
                    "{}: a press at {point:?}, inside the hit area {:?} of the part {part:?}, was not taken",
                    story.name, target.bounds
                ));
            }
        }
        if let (Plan::Drag(Grip { part, .. }), true) = (plan, probe.carries_value()) {
            let Some(grip) = fixture.root_id().and_then(|id| fixture.part(id, part)).and_then(|grip| fixture.visible(&grip)) else { continue };
            let off_middle = Pos2::new(grip.bounds.center().x + 0.25 * grip.bounds.width(), grip.bounds.center().y);
            let mut fixture = input_fixture(story);
            let before = fixture.data().snapshot();
            let travel = 2.0 * fixture.slop();
            // A refused value is moved while the pointer drags and put back at the release, so it is read before the release.
            let refused = story.covers.contains(&Required::Invalid);
            let moved_live = Cell::new(false);
            fixture.drag_at_pausing(off_middle, Vec2::new(travel, 0.0), |fixture| {
                moved_live.set(!fixture.data().changed_since(&before).is_empty());
            });
            let moved = if refused { moved_live.get() } else { !fixture.data().changed_since(&before).is_empty() };
            if !moved {
                return Err(format!("{}: a drag from {off_middle:?}, inside the part {part:?}, moved no value", story.name));
            }
        }
    }
    Ok(())
}
