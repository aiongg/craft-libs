//! `Lazy`: a child of a scroll area that runs only while it is in view (`runtime::lazy`).

use crate::runtime::egui::alloc;
use crate::runtime::env::Env;
use crate::runtime::focus;
use crate::runtime::layout_memory;
use crate::runtime::lazy::{self, Decision};
use crate::runtime::memory::sub;
use egui::{Pos2, Rect, Ui, UiBuilder, Vec2};
use std::hash::Hash;

/// A child of a scroll area that runs only while it is in view. Out of view it takes the extent it had in the last
/// pass that showed it, at any room, and its content does not run. It runs when it was never shown, when it meets the view widened by the view's size on each side, while one of the stops of its last run holds keyboard
/// focus or is the target of a focus request, and when `keep` asks. Content that was never shown has no stops to
/// find: a child an app focuses or scrolls to before it was ever in reach needs `keep`. Outside a scroll area it
/// always runs.
///
/// The key names the child among every `Lazy` of its scroll area, nested containers included. The idiom is
/// `Lazy::new(key).show(ui, |ui| ..)` around each large child (a card, a row of cards) in the scroll body.
#[must_use]
pub struct Lazy {
    key: u64,
    keep: bool,
}

impl Lazy {
    /// The child keyed `key` in its scroll area.
    pub fn new(key: impl Hash) -> Lazy {
        use std::hash::Hasher;
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut hasher);
        Lazy { key: hasher.finish(), keep: false }
    }

    /// Runs the child out of view too, while `keep` holds: the target of a scroll the app asks for.
    pub fn keep(mut self, keep: bool) -> Lazy {
        self.keep = keep;
        self
    }

    /// Runs `add` in a child `Ui` that flows in `ui`'s layout, or takes the remembered extent; `None` when it did not
    /// run.
    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> Option<R> {
        let ctx = ui.ctx().clone();
        let view = Env::of(ui).view().lazy_view();
        let id = match view {
            Some(view) => sub(view.owner, ("lazy", self.key)),
            None => sub(ui.scope_id(), ("lazy", self.key)),
        };
        let available = ui.available_rect_before_wrap();
        let rtl = ui.layout().prefer_right_to_left();
        let sizing = ui.is_sizing_pass();
        // The child is not part of its body's transcript: in a real run it counts there by its extent, whether it
        // runs or not, so what scrolls into reach does not change the body's content.
        let real = !sizing && !crate::runtime::measure::active(&ctx);
        if let Some(view) = view {
            let at = |size: Vec2| {
                let x = if rtl { available.max.x - size.x } else { available.min.x };
                Rect::from_min_size(Pos2::new(x, available.min.y), size)
            };
            if let Decision::Skip(size) = lazy::decide(&ctx, id, &view, at, sizing, self.keep) {
                alloc::space(ui, size);
                if real {
                    layout_memory::backstop(&ctx, id, size);
                }
                return None;
            }
        }
        let from = focus::stop_count(&ctx);
        let layout = *ui.layout();
        let mut child = alloc::child(ui, UiBuilder::new().max_rect(available).layout(layout).scope_id(id));
        let out = layout_memory::boundary(&ctx, || add(&mut child));
        let rect = child.min_rect();
        if real {
            layout_memory::backstop(&ctx, id, rect.size());
        }
        alloc::advance_past(ui, rect);
        if view.is_some() {
            lazy::shown(&ctx, id, rect.size(), sizing, (!sizing).then(|| focus::stops_since(&ctx, from)));
        }
        Some(out)
    }
}
