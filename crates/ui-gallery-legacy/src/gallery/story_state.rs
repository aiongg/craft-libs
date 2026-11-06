//! What a story keeps beside its args: the state its widgets hold in egui memory (an edited curve,
//! a slider's value, a field's text), which a reset has to drop too, and whether the user has
//! worked the component in its preview, which Reset offers to undo.
use std::collections::{HashMap, HashSet};

use egui::{Context, Event, Id, Key, LayerId, Pos2, Rect, Vec2};

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
    /// was drawn in on `layer`, this frame. `own` is the id of the surface around the story, whose
    /// click selects the tile.
    pub(crate) fn observe(&mut self, ctx: &Context, story: &str, body: Rect, layer: LayerId, own: Id) {
        if !self.touched.contains(story) && used(ctx, body, layer, own) {
            self.touched.insert(story.to_owned());
            ctx.request_repaint();
        }
    }
}

/// Where the gesture of this frame's pointer interaction began: the press origin, or this frame's
/// press event when egui has already cleared the origin (a click that releases in the frame it
/// presses), or the pointer.
fn gesture_origin(ctx: &Context) -> Option<Pos2> {
    ctx.input(|i| {
        i.pointer
            .press_origin()
            .or_else(|| {
                i.events.iter().find_map(|e| match e {
                    Event::PointerButton { pos, pressed: true, .. } => Some(*pos),
                    _ => None,
                })
            })
            .or_else(|| i.pointer.latest_pos())
    })
}

/// Whether a click or drag began on `layer` inside `body` on a widget other than `own`, a wheel
/// turn over `body` was taken by a widget (the story's widgets zero the scroll delta they use;
/// the gallery's own scrolling reads what is left later), or a key went to a focused widget
/// inside `body`.
///
/// The press origin decides for the pointer, not the widget's rect: a row half scrolled out of
/// `body` and a control clipped at the tile's edge count as the story's.
fn used(ctx: &Context, body: Rect, layer: LayerId, own: Id) -> bool {
    let at = |pos: Pos2| body.contains(pos) && ctx.layer_id_at(pos) == Some(layer);
    let pointed = ctx.interaction_snapshot(|s| [s.clicked, s.drag_started, s.dragged, s.drag_stopped]);
    let worked = pointed.into_iter().flatten().any(|id| id != own && ctx.read_response(id).is_some()) && gesture_origin(ctx).is_some_and(at);
    // `at` reads the context, so it runs after `input` has released the context's lock.
    let wheel = ctx
        .input(|i| {
            (i.events.iter().any(|e| matches!(e, Event::MouseWheel { .. })) && i.smooth_scroll_delta == Vec2::ZERO)
                .then(|| i.pointer.latest_pos())
                .flatten()
        })
        .is_some_and(at);
    let inside = |id: Id| id != own && ctx.read_response(id).is_some_and(|r| body.expand(1.0).contains_rect(r.rect));
    let focused = ctx.memory(|m| m.focused());
    worked
        || wheel
        || focused.is_some_and(inside)
            && ctx.input(|i| {
                i.events.iter().any(|e| match e {
                    Event::Text(_) => true,
                    Event::Key { pressed, key, .. } => *pressed && !matches!(key, Key::Tab | Key::Escape),
                    _ => false,
                })
            })
}

#[cfg(test)]
mod tests {
    use egui::{Context, Event, Id, LayerId, Modifiers, MouseWheelSource, MouseWheelUnit, RawInput, Rect, Vec2, pos2, vec2};

    /// A wheel turn a widget took, with the pointer over the story, checks the layer under the
    /// pointer; that check must not run while the input is read (the context's lock would deadlock).
    #[test]
    fn a_taken_wheel_turn_over_the_story_returns() {
        let ctx = Context::default();
        let pos = pos2(50.0, 50.0);
        let events = vec![
            Event::PointerMoved(pos),
            Event::MouseWheel {
                unit: MouseWheelUnit::Line,
                delta: vec2(0.0, 1.0),
                modifiers: Modifiers::NONE,
                phase: egui::TouchPhase::Move,
                source: MouseWheelSource::Wheel,
            },
        ];
        let mut out = ctx.run_ui(RawInput { events, ..Default::default() }, |ui| {
            ui.input_mut(|i| i.smooth_scroll_delta = Vec2::ZERO);
            let body = Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 100.0));
            let _ = super::used(ui.ctx(), body, LayerId::background(), Id::unique("own"));
        });
        out.textures_delta.clear();
    }
}
