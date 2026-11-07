//! The container rows of the 22.4 table: title naming, bodies on `Mount::Layer` (Escape, focus trap, focus return,
//! placement memory) and scrolling. Each case runs over the stories of a type of the `Body` shape and looks only at
//! what the story shows: a layer case skips the stories whose container is not on a layer, a scroll case those whose
//! body does not overflow.

use super::Subject;
use super::nodes::{focus_within, root, subtree};
use crate::contract::{MOVE_BAND, ShapeKind};
use crate::runtime::access::Role;
use crate::runtime::egui::access::NodeKey;
use crate::runtime::overlay;
use crate::testing::access::is_interactive;
use crate::testing::fixture::Fixture;
use crate::testing::story::Story;
use craft_behavior::MotionMode;
use egui::{Key, Modifiers, Vec2};

fn is_body(subject: &Subject) -> bool {
    matches!(subject.probe.shape, ShapeKind::Body)
}

/// Whether the story's container is a layer of the overlay host that is not fading out.
fn on_layer(fixture: &Fixture) -> bool {
    fixture.root_id().is_some_and(|id| overlay::entry(fixture.ctx(), id).is_some_and(|entry| !entry.exiting))
}

/// A settled fixture of `story` with motion off, when its container is on a layer.
fn layer_fixture(story: &Story) -> Option<Fixture> {
    let mut fixture = Fixture::story(story);
    fixture.motion(MotionMode::Off);
    fixture.settle();
    on_layer(&fixture).then_some(fixture)
}

/// A container that is a dialog or a group has a role that needs a name: the title names it.
pub(super) fn title_names_the_container(subject: &Subject) -> Result<(), String> {
    if !is_body(subject) {
        return Ok(());
    }
    for story in &subject.stories {
        let mut fixture = Fixture::story(story);
        fixture.settle();
        let Some(container) = root(&fixture) else { continue };
        let role = fixture.tree().role_of(container.node);
        if (role == Some(Role::DIALOG) || role == Some(Role::GROUP)) && crate::testing::access::name(fixture.tree(), container.node).is_none() {
            return Err(format!("{}: the container's node has the role {role:?} and no name", story.name));
        }
    }
    Ok(())
}

/// Escape closes the layer: no layer that takes keys is left on top of it. A layer whose scope ignores Escape (a
/// blocking overlay nobody can cancel, or a pane such as a floating panel) is not asked to.
pub(super) fn layer_closes_on_escape(subject: &Subject) -> Result<(), String> {
    if !is_body(subject) {
        return Ok(());
    }
    for story in &subject.stories {
        let Some(mut fixture) = layer_fixture(story) else { continue };
        let Some(id) = fixture.root_id() else { continue };
        let ignores = crate::runtime::focus::with(fixture.ctx(), |state| {
            state.last_scopes.get(&id).is_some_and(|scope| scope.policy.cancel == craft_behavior::focus::CancelRule::Ignore)
        });
        if ignores {
            continue;
        }
        fixture.key(Key::Escape, Modifiers::NONE);
        fixture.idle(3);
        if on_layer(&fixture) {
            return Err(format!("{}: the layer is still open after Escape", story.name));
        }
        if overlay::top(fixture.ctx()) == Some(id) {
            return Err(format!("{}: the closed layer still receives Cancel", story.name));
        }
    }
    Ok(())
}

/// Tab and Shift+Tab stay inside the layer: a trap, since a layer owns its focus scope. A scope that does not trap
/// (a dropdown, whose focus stays on its opener) is not asked to.
pub(super) fn layer_traps_focus(subject: &Subject) -> Result<(), String> {
    if !is_body(subject) {
        return Ok(());
    }
    for story in &subject.stories {
        let Some(mut fixture) = layer_fixture(story) else { continue };
        let Some(id) = fixture.root_id() else { continue };
        let traps = crate::runtime::focus::with(fixture.ctx(), |state| state.last_scopes.get(&id).is_some_and(|scope| scope.policy.trap));
        if !traps {
            continue;
        }
        let Some(container) = root(&fixture) else { continue };
        let stops = subtree(fixture.tree(), container.node).into_iter().filter(|key| is_interactive(fixture.tree(), *key)).count();
        if stops == 0 {
            continue;
        }
        for step in 0..stops + 2 {
            fixture.key(Key::Tab, Modifiers::NONE);
            if !focus_within(&fixture, container.node) {
                return Err(format!("{}: Tab number {} moved focus out of the layer", story.name, step + 1));
            }
        }
        fixture.key(Key::Tab, Modifiers::SHIFT);
        if !focus_within(&fixture, container.node) {
            return Err(format!("{}: Shift+Tab moved focus out of the layer", story.name));
        }
    }
    Ok(())
}

/// After Escape closes the layer, the AccessKit focus is not on a node of the closed container.
pub(super) fn layer_leaves_no_dangling_focus(subject: &Subject) -> Result<(), String> {
    if !is_body(subject) {
        return Ok(());
    }
    for story in &subject.stories {
        let Some(mut fixture) = layer_fixture(story) else { continue };
        let Some(container) = root(&fixture) else { continue };
        let was: Vec<NodeKey> = subtree(fixture.tree(), container.node);
        fixture.key(Key::Escape, Modifiers::NONE);
        fixture.idle(3);
        if was.contains(&fixture.tree().focused_key()) {
            return Err(format!("{}: focus stays on a node of the closed layer", story.name));
        }
    }
    Ok(())
}

/// A layer with a move band follows a drag of the band and keeps the place it was dragged to.
pub(super) fn layer_remembers_its_placement(subject: &Subject) -> Result<(), String> {
    if !is_body(subject) {
        return Ok(());
    }
    for story in &subject.stories {
        let Some(mut fixture) = layer_fixture(story) else { continue };
        let Some(id) = fixture.root_id() else { continue };
        let Some(band) = fixture.part(id, MOVE_BAND) else { continue };
        let Some(before) = root(&fixture).map(|container| container.rect) else { continue };
        fixture.drag(&band, Vec2::new(40.0, 30.0));
        fixture.settle();
        let Some(moved) = root(&fixture).map(|container| container.rect) else { continue };
        if !(moved.min.x > before.min.x || moved.min.y > before.min.y) {
            return Err(format!("{}: a drag of the move band left the layer at {before:?}", story.name));
        }
        fixture.idle(3);
        let Some(later) = root(&fixture).map(|container| container.rect) else { continue };
        if later.min != moved.min {
            return Err(format!("{}: the layer went from {moved:?} back to {later:?} with no input", story.name));
        }
    }
    Ok(())
}

/// How far below the container's bottom edge its lowest node reaches.
fn overflow(fixture: &Fixture, container: NodeKey, bottom: f32) -> f32 {
    subtree(fixture.tree(), container)
        .into_iter()
        .filter_map(|key| fixture.tree().bounds_of(key))
        .map(|bounds| bounds.max.y - bottom)
        .fold(0.0, f32::max)
}

/// A body whose content overflows the container scrolls to its end under the wheel, with the pointer over it.
pub(super) fn overflowing_body_scrolls_to_its_end(subject: &Subject) -> Result<(), String> {
    if !is_body(subject) {
        return Ok(());
    }
    for story in &subject.stories {
        let mut fixture = Fixture::story(story);
        fixture.motion(MotionMode::Off);
        fixture.settle();
        let Some(container) = root(&fixture) else { continue };
        let start = overflow(&fixture, container.node, container.rect.max.y);
        if start <= 1.0 {
            continue;
        }
        let at = container.rect.center();
        fixture.frame(vec![crate::testing::pointer::moved(at)]);
        for _ in 0..40 {
            fixture.frame(vec![crate::testing::pointer::wheel(-120.0)]);
        }
        fixture.idle(2);
        let Some(after) = root(&fixture) else { continue };
        let left = overflow(&fixture, after.node, after.rect.max.y);
        if left > 1.0 {
            return Err(format!("{}: after scrolling to the end the content still reaches {left} points below the container", story.name));
        }
    }
    Ok(())
}
