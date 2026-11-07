//! `PaintUi` (5.3): what `paint` and `paint_over` get instead of a `Ui`. It lives in the contract (layer 4)
//! because `child` and `face` show components; `Draw` and the painters stay in `primitives`.

use super::SideLabel;
use super::label::{self, LeafLabel};
use super::run::{self, FaceSource, ShowAt};
use super::{ContentPainter, Cx, Show, TipFn};
use crate::primitives::sensing::PartResults;
use crate::primitives::{ContentPainterCx, Draw, DrawEnv, PaintRecord};
use crate::runtime::access;
use crate::runtime::egui::{alloc, child};
use crate::runtime::env::{Env, Label};
use crate::runtime::memory::sub;
use crate::style::{Flags, Ink, Stateful, StyleColor, StyleCx};
use craft_behavior::PartRef;
use craft_theme::{ControlSize, Variant};
use egui::{Id, Rect, Response, Ui};
use std::cell::RefCell;
use std::fmt::Debug;
use std::hash::Hash;

/// What `paint` and `paint_over` get instead of a `Ui`.
pub struct PaintUi<'a> {
    pub(crate) ui: &'a mut Ui,
    pub(crate) env: &'a Env,
    /// The ink of the component's own surface, which `draw` paints text and rules in.
    pub(crate) ink: Ink,
    pub(crate) id: Id,
    pub(crate) rect: Rect,
    pub(crate) record: &'a RefCell<PaintRecord>,
    /// The part results of this pass; faces read their part's state from here.
    pub(crate) results: &'a PartResults,
    pub(crate) response: &'a Response,
    pub(crate) active: Option<(PartRef, bool)>,
    /// The composite's text colour, which its faces take as their host ink.
    pub(crate) text: Option<&'a Stateful<StyleColor>>,
    /// The composite's validity flags, which its faces carry.
    pub(crate) flags: Flags,
    /// The composite's resolved enabled state and size, which the children it builds inherit.
    pub(crate) enabled: bool,
    pub(crate) size: ControlSize,
    /// The children's `Ui`, built on the first child under `sub(id, "children")`.
    pub(crate) kids: Option<Ui>,
}

impl PaintUi<'_> {
    /// The content-paint API: text, rules, surfaces and shapes, recorded for the later steps.
    pub fn draw(&self) -> Draw<'_> {
        Draw::new(self.ui.painter(), self.record, DrawEnv { tokens: &self.env.tokens, direction: self.env.direction, ink: self.ink })
    }

    /// Runs `f` with the style context of this component's env, so app hooks called from `paint` pick colours and
    /// sizes through tokens as `style` does.
    pub fn with_style<R>(&self, variant: Variant, f: impl FnOnce(&StyleCx<'_>) -> R) -> R {
        let env = self.env;
        let cx = StyleCx::new(self.ui.ctx(), &env.tokens, &env.values, self.size, variant, env.direction, env.scale, env.os, env.active, env.key);
        f(&cx)
    }

    /// Shows a child component at a physical rect, with id `cx.key(key)`. The child `Ui` is pinned left to
    /// right: this site is unmirrored.
    pub fn child<C: Show>(&mut self, key: impl Hash + Debug, rect: Rect, child: C) -> C::Output {
        self.child_with(self.id.with(key), rect, child, None)
    }

    /// Shows a child with rich tooltip content.
    pub fn child_with_tip<C: Show>(&mut self, key: impl Hash + Debug, rect: Rect, child: C, tip: impl FnOnce(&mut Ui)) -> C::Output {
        self.child_with(self.id.with(key), rect, child, Some(Box::new(tip)))
    }

    /// Runs the label step (B2) for one leaf child at `rect`: the label's placement beside or above the child, the
    /// label scope, scrub and the reset dot. The child gets the rect the label leaves.
    pub fn labelled<C: Show>(&mut self, key: impl Hash + Debug, rect: Rect, label: Label, side: SideLabel, child: C) -> C::Output {
        let id = self.id.with(key);
        let env = Env::of(self.ui);
        let mut child = child;
        let at =
            LeafLabel { id, owner: self.id, parent: Some(self.id), row: rect, text: &label, side: &side, enabled: self.enabled, size: self.size };
        let placed = crate::layout::ColumnGroup::barrier(&mut *self.ui, |ui| label::leaf(ui, &env, &at, &mut child));
        let output = self.child_with(id, placed.control, child, None);
        placed.finish(self.ui.ctx());
        output
    }

    fn child_with<C: Show>(&mut self, id: Id, rect: Rect, child: C, tip: Option<TipFn<'_>>) -> C::Output {
        let scope = sub(self.id, "children");
        let parent = &mut *self.ui;
        let kids = self.kids.get_or_insert_with(|| {
            let builder = egui::UiBuilder::new().layout(child::pinned_layout()).scope_id(scope).max_rect(self.rect);
            alloc::child(parent, access::with_parent(builder, self.id))
        });
        let ctx = kids.ctx().clone();
        let env = super::cx::child_env(&Env::of(kids), self.enabled, self.size);
        let installed = Env::install_for(kids, env);
        let output = run::show_leaf(super::Props::id(child, id), kids, ShowAt::Rect(rect), tip);
        Env::restore(&ctx, installed);
        output
    }

    /// Paints `face` as the look of `part`: its style, surface and state fill from the part's state merged with the
    /// instance's `flags()` and the host's validity flags, then its own `paint`. The face reads the host's text
    /// colour through `StyleCx::host_ink`. It registers no id, senses nothing, opens no memory stage and
    /// fires nothing.
    pub fn face<C: Show>(&mut self, part: impl Into<PartRef>, face: C) {
        let part = part.into();
        let Some(result) = self.results.get(&(part.key, part.index)) else {
            debug_assert!(false, "face for a part `parts` did not declare");
            return;
        };
        let focus = self.active.filter(|(active, _)| *active == part).map(|(_, visible)| visible);
        let source = FaceSource { id: self.id, result, focus, response: self.response, ink: self.text, flags: self.flags };
        let record = run::paint_face(self.ui, &source, part, face);
        self.record.borrow_mut().faces.push(record);
    }

    /// The raw painter for an allowlisted content painter, clipped to the component's own rect.
    pub fn content_painter<C: ContentPainter>(&self, cx: &Cx<'_, C>) -> ContentPainterCx<'_> {
        ContentPainterCx::new(self.ui.painter().with_clip_rect(cx.core.rect.intersect(self.ui.clip_rect())), &self.env.tokens, self.env.direction)
    }
}
