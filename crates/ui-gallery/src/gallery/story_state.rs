//! What a story keeps beside its args: the state its widgets hold in egui memory (an edited curve,
//! a slider's value, a field's text), which a reset has to drop too, and whether the user has
//! worked the component in its preview, which Reset offers to undo.
use std::collections::{HashMap, HashSet};

use egui::{Context, Event, Id, Key, Rect};

/// Per story id: how many times it was reset, and whether its preview was used since.
#[derive(Default)]
pub(crate) struct StoryStates {
    generation: HashMap<String, u32>,
    touched: HashSet<String>,
}

impl StoryStates {
    /// Mixed into the ids under a story's tile: a reset gives every widget and memory entry the
    /// story keeps new ids, so the story starts over.
    pub(crate) fn generation(&self, story: &str) -> u32 {
        self.generation.get(story).copied().unwrap_or(0)
    }

    pub(crate) fn touched(&self, story: &str) -> bool {
        self.touched.contains(story)
    }

    /// Drops the story's state and its touched flag.
    pub(crate) fn reset(&mut self, story: &str) {
        *self.generation.entry(story.to_owned()).or_insert(0) += 1;
        self.touched.remove(story);
    }

    /// Notes the story as touched when the user worked a widget inside `body`, the rect the story
    /// was drawn in, this frame. `own` is the id of the surface around the story, whose click selects
    /// the tile.
    pub(crate) fn observe(&mut self, ctx: &Context, story: &str, body: Rect, own: Id) {
        if !self.touched.contains(story) && used(ctx, body, own) {
            self.touched.insert(story.to_owned());
            ctx.request_repaint();
        }
    }
}

/// Whether a click or drag landed on a widget inside `body` this frame, or a key went to a
/// focused widget inside it.
fn used(ctx: &Context, body: Rect, own: Id) -> bool {
    let inside = |id: Id| id != own && ctx.read_response(id).is_some_and(|r| body.expand(1.0).contains_rect(r.rect));
    let pointed = ctx.interaction_snapshot(|s| [s.clicked, s.drag_started, s.dragged, s.drag_stopped]);
    let focused = ctx.memory(|m| m.focused());
    pointed.into_iter().flatten().any(inside)
        || focused.is_some_and(inside)
            && ctx.input(|i| {
                i.events.iter().any(|e| match e {
                    Event::Text(_) => true,
                    Event::Key { pressed, key, .. } => *pressed && !matches!(key, Key::Tab | Key::Escape),
                    _ => false,
                })
            })
}
