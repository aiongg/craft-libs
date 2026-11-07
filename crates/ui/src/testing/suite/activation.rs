//! Activation, declared actions, AccessKit focus and cancelled presses.

use super::drive::{Plan, root_target, run, settled, variant_name, variants};
use super::{ANY, Case, Subject};
use crate::runtime::egui::access::{AccessTree, Declared};
use crate::runtime::egui::fixture::AccessActionKind;
use crate::testing::pointer;
use crate::testing::story::{EditKind, Required};
use egui::{Key, Modifiers};

pub(super) const CASES: &[Case] = &[
    Case::new("activate-once", ANY, each_activation_clicks_once),
    Case::new("actions", ANY, declared_actions_are_handled),
    Case::new("access-focus", ANY, access_focus_focuses),
    Case::new("press-cancel", ANY, escape_and_blur_cancel_a_press),
];

/// Enter, Space, AccessKit `Click` and a primary click each give exactly one click on a component that reports
/// clicks and carries no value (value types are checked by the values case).
fn each_activation_clicks_once(subject: &Subject) -> Result<(), String> {
    if subject.probe.carries_value() {
        return Ok(());
    }
    for story in &subject.stories {
        let (fixture, plan) = settled(story, subject.probe);
        let root = root_target(&fixture);
        if !matches!(plan, Plan::Activate) || !fixture.tree().declares_key(root.node, Declared::Click) {
            continue;
        }
        for variant in variants(plan, subject.probe) {
            let (mut fixture, _) = settled(story, subject.probe);
            fixture.take_outputs();
            run(&mut fixture, plan, variant);
            let clicks = fixture.take_outputs().iter().filter(|output| output.clicked).count();
            if clicks != 1 {
                return Err(format!("{}: {} gave {clicks} clicks", story.name, variant_name(plan, variant)));
            }
        }
    }
    Ok(())
}

/// What a node tree says about values and states: any change of it is an effect.
fn signature(tree: &AccessTree) -> Vec<String> {
    tree.keys()
        .into_iter()
        .map(|key| {
            tree.data_of(key).map_or_else(String::new, |node| {
                format!("{:?}|{:?}|{:?}|{:?}|{:?}", node.toggled(), node.is_expanded(), node.is_selected(), node.numeric_value(), node.value())
            })
        })
        .collect()
}

/// Every action a node declares changes something: a click, an edit, a node state or the focus.
fn declared_actions_are_handled(subject: &Subject) -> Result<(), String> {
    for story in subject.stories.iter().filter(|story| !story.covers.contains(&Required::Invalid)) {
        let (fixture, _) = settled(story, subject.probe);
        let actions = [
            (Declared::Click, AccessActionKind::Click),
            (Declared::Increment, AccessActionKind::Increment),
            (Declared::Expand, AccessActionKind::Expand),
        ];
        let nodes: Vec<_> = fixture
            .tree()
            .keys()
            .into_iter()
            .filter(|key| fixture.tree().data_of(*key).is_some_and(|node| !node.is_disabled()))
            .flat_map(|key| {
                actions.iter().filter(move |(declared, _)| fixture.tree().declares_key(key, *declared)).map(move |(_, kind)| (key, *kind))
            })
            .collect();
        for (key, kind) in nodes {
            let (mut fixture, _) = settled(story, subject.probe);
            let Some(target) = fixture.target(key) else { continue };
            let (before, focus) = (signature(fixture.tree()), fixture.focused());
            fixture.take_outputs();
            fixture.access(kind, &target);
            fixture.idle(2);
            let outputs = fixture.take_outputs();
            let acted = outputs.iter().any(|output| output.clicked || !output.edits.is_empty())
                || signature(fixture.tree()) != before
                || fixture.focused() != focus;
            if !acted {
                return Err(format!("{}: {kind:?} on {:?} did nothing", story.name, fixture.tree().role_of(key)));
            }
        }
    }
    Ok(())
}

/// AccessKit `Focus` on a node that declares it gives the component key focus.
fn access_focus_focuses(subject: &Subject) -> Result<(), String> {
    for story in &subject.stories {
        let (fixture, _) = settled(story, subject.probe);
        let node =
            fixture.tree().keys().into_iter().find(|key| {
                fixture.tree().declares_key(*key, Declared::Focus) && fixture.tree().data_of(*key).is_some_and(|node| !node.is_disabled())
            });
        let Some(target) = node.and_then(|key| fixture.target(key)) else { continue };
        let mut fixture = fixture;
        fixture.access(AccessActionKind::Focus, &target);
        fixture.idle(1);
        if fixture.focused().is_none() {
            return Err(format!("{}: AccessKit Focus focused nothing", story.name));
        }
    }
    Ok(())
}

/// Escape and the window losing focus during a press end it without a click or a commit.
fn escape_and_blur_cancel_a_press(subject: &Subject) -> Result<(), String> {
    for story in &subject.stories {
        let (fixture, plan) = settled(story, subject.probe);
        if !matches!(plan, Plan::Activate | Plan::Drag(_)) {
            continue;
        }
        let root = root_target(&fixture);
        if !fixture.tree().declares_key(root.node, Declared::Click) && !matches!(plan, Plan::Drag(_)) {
            continue;
        }
        for escape in [true, false] {
            let (mut fixture, _) = settled(story, subject.probe);
            let target = root_target(&fixture);
            let before = fixture.data().snapshot();
            fixture.take_outputs();
            fixture.press(&target);
            if escape {
                fixture.key(Key::Escape, Modifiers::NONE);
            } else {
                fixture.frame(vec![pointer::window_blurred()]);
            }
            fixture.release(&target);
            fixture.idle(2);
            let outputs = fixture.take_outputs();
            let ended = outputs.iter().flat_map(|output| output.edits.iter()).any(|kind| matches!(kind, EditKind::Commit | EditKind::Set));
            if outputs.iter().any(|output| output.clicked) || ended || !fixture.data().changed_since(&before).is_empty() {
                return Err(format!("{}: a press cancelled by {} still acted", story.name, if escape { "Escape" } else { "blur" }));
            }
        }
    }
    Ok(())
}
