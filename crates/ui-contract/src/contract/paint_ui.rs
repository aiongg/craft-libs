//! `PaintUi` (5.3): what `paint` and `paint_over` get instead of a `Ui`. It lives in the contract (layer 4)
//! because `child` and `face` show components; `Draw` and the painters stay in `primitives`. On craft-ui-rt each
//! `PaintUi` records one paint variant (docs/contract-rt.md 15.1, 15.6).

use super::label::{self, LeafLabel};
use super::rt::node::Pointer;
use super::run::{self, FaceSource, ShowAt};
use super::{ContentPainter, Cx, Show, TipFn, Ui};
use super::{InsideLabel, SideLabel};
use crate::primitives::sensing::PartResults;
use crate::primitives::text_adapter::Placed;
use crate::primitives::{Canvas, ContentPainterCx, DataMode, Draw, DrawEnv, Id, PaintRecord, PaintShape, PaintSlot, TextRun, Wrap};
use crate::runtime::access;
use crate::runtime::env::{Env, Label};
use crate::style::{Flags, Ink, Stateful, StyleColor, StyleCx};
use craft_behavior::PartRef;
use craft_theme::{ControlSize, Variant};
use craft_ui_rt::Rect;
use craft_ui_rt::text_edit::{EditConfig, EditLayout, TextEdit};
use std::cell::RefCell;
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::Arc;

/// The build a context that holds no `Ui` of its own reaches through: `PaintUi` and `OverlayUi` show children, and
/// `MeasureCx` measures them, on the `Ui` of the run they belong to. Object safe, so those contexts keep one lifetime.
pub(crate) trait UiHost {
    /// Runs `f` on the run's `Ui`.
    fn with_ui(&mut self, f: &mut dyn FnMut(&mut Ui<'_, '_>));
}

impl UiHost for Ui<'_, '_> {
    fn with_ui(&mut self, f: &mut dyn FnMut(&mut Ui<'_, '_>)) {
        f(self);
    }
}

/// Runs `f` on `host`'s `Ui` and returns what it returns.
pub(crate) fn host_ui<R>(host: &mut (dyn UiHost + '_), f: impl FnOnce(&mut Ui<'_, '_>) -> R) -> R {
    let mut f = Some(f);
    let mut out = None;
    host.with_ui(&mut |ui| {
        if let Some(f) = f.take() {
            out = Some(f(ui));
        }
    });
    out.expect("`UiHost::with_ui` runs its closure once")
}

/// What `paint` and `paint_over` get instead of a `Ui`: one paint variant being recorded (docs/contract-rt.md 15.1).
pub struct PaintUi<'a> {
    /// The run's `Ui`, for the children shown from `paint`.
    pub(crate) host: &'a mut (dyn UiHost + 'a),
    #[doc(hidden)]
    pub env: &'a Env,
    /// The ink of the component's own surface, which `draw` paints text and rules in.
    pub(crate) ink: Ink,
    /// How `draw` paints data colours: dimmed while the component paints its Disabled state.
    pub(crate) data: DataMode,
    pub(crate) id: Id,
    pub(crate) rect: Rect,
    pub(crate) record: &'a RefCell<PaintRecord>,
    /// The part results of this run; faces read their part's state from here.
    pub(crate) results: &'a PartResults,
    pub(crate) active: Option<(PartRef, bool)>,
    /// The composite's text colour, which its faces take as their host ink.
    pub(crate) text: Option<&'a Stateful<StyleColor>>,
    /// The composite's muted text colour (its second text look), which its faces take as their host's muted ink.
    pub(crate) muted: Option<&'a Stateful<StyleColor>>,
    /// The composite's validity flags, which its faces carry.
    pub(crate) flags: Flags,
    /// The composite's resolved enabled state and size, which the children it builds inherit.
    pub(crate) enabled: bool,
    pub(crate) size: ControlSize,
    /// The selection parts step 8b ran, with their selections.
    pub(crate) selections: &'a [crate::contract::text::SelectionShown],
    /// The fade `steps::fade` began (`DisabledLook::Faded`): the faces paint their enabled look, and the children
    /// shown from here lie under the component's veil.
    pub(crate) fade: Option<super::steps::Fade>,
    /// The composite has a node (`CxCore::node`): its step 16 fills its parts' sub-nodes from the faces' records.
    pub(crate) node: bool,
    /// The variant's canvas: what `draw` records, in the node's coordinates.
    pub(crate) canvas: &'a Canvas<'a>,
    /// The canvas of the component whose face this paints: what it records goes there, at the position of the
    /// `face` call.
    pub(crate) host_canvas: Option<&'a Canvas<'a>>,
    /// The pointer state this variant shows; the pointer facts a paint reads answer it.
    pub(crate) pointer: Pointer,
    /// The children this paint showed, in order (15.6 rule 5).
    pub(crate) children: RefCell<Vec<Shown>>,
    /// In a later look, the children the first look showed from here on: this look keeps them instead of
    /// reconciling (15.6 rule 5). `None` in the first look.
    pub(crate) keep: Option<&'a [Shown]>,
}

/// A child a paint showed (15.6 rule 5): its id, the rect the paint gave it and the rect it took (the control's rect
/// beside its label for `labelled`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Shown {
    pub(crate) id: Id,
    pub(crate) at: Rect,
    pub(crate) rect: Rect,
}

/// What a child shown from `paint` inherits from its composite.
#[derive(Clone, Copy)]
struct Kids {
    enabled: bool,
    size: ControlSize,
    veiled: bool,
}

impl Kids {
    /// Shows `child` at `rect` on `ui` under the composite's `enabled` and `size`, and `inside`, the inside label of the
    /// side label the child is the control of. The rect is physical: this site is unmirrored.
    fn show<C: Show>(self, ui: &mut Ui<'_, '_>, id: Id, rect: Rect, child: C, tip: Option<TipFn<'_>>, inside: Option<Arc<InsideLabel>>) -> C::Output {
        let env = self.env(ui, inside);
        run::show_leaf(&mut Some(super::Props::id(child, id)), ui, (ShowAt::Rect(rect), env), tip)
    }

    /// The keep path (15.6 rule 5): `child`'s output from the node the first look reconciled, at the rect it took
    /// there.
    fn keep<C: Show>(self, ui: &mut Ui<'_, '_>, id: Id, rect: Rect, child: C, inside: Option<Arc<InsideLabel>>) -> C::Output {
        let env = self.env(ui, inside);
        run::keep_leaf(super::Props::id(child, id), ui, env, rect)
    }

    /// The env a child shows under.
    fn env(self, ui: &mut Ui<'_, '_>, inside: Option<Arc<InsideLabel>>) -> Env {
        let outer = Env::of(ui);
        let env = super::cx::child_env(&outer, &super::cx::Under { enabled: self.enabled, size: self.size, inside });
        // A child shown while the owner's fade runs lies under the owner's veil, so a faded child adds none of its own.
        match self.veiled {
            true => env.derive_local(|scoped| {
                scoped.inherited_mut().values.set(super::steps::UnderVeil(true));
            }),
            false => env,
        }
    }
}

impl PaintUi<'_> {
    /// The content-paint API: text, rules, surfaces and shapes, recorded for the later steps. While the component
    /// paints its Disabled state (`DisabledLook::States`), its data colours paint at `Opacity::DISABLED`.
    pub fn draw(&self) -> Draw<'_> {
        Draw::new(self.canvas, self.record, DrawEnv { tokens: &self.env.tokens, direction: self.env.direction, ink: self.ink })
            .text_direction(self.env.text_direction)
            .data(self.data)
    }

    /// Shows the selection part `part` (`PartsCx::selectable_text`): `run` in `rect`, painted as `Draw::text`
    /// paints it, with the selection behind its characters while the part holds focus; a line wider than `rect` is
    /// cut at its edge. The text's place becomes the part's shape, which the pointer selects in. A part step 7 did
    /// not declare (a face, an inert component) paints as plain text.
    pub fn selectable_text(&mut self, part: impl Into<PartRef>, rect: Rect, run: &TextRun<'_>) {
        let part = part.into();
        let Some(selection) = self.selections.iter().find(|shown| shown.part == part).cloned() else {
            self.draw().text(rect, run);
            return;
        };
        let (layout, at) = self.draw().place_text(rect, run);
        self.record.borrow_mut().baseline.get_or_insert(at.y + layout.first_baseline);
        let config = EditConfig { multiline: run.wrap == Wrap::Wrap, mask: false, limit: None, read_only: true, os: self.env.os };
        let shown = TextEdit::default().shown(layout.full(), &config);
        let placed = Arc::new(EditLayout::new(layout.rt().clone(), layout.metrics(), &shown));
        let draw = self.draw();
        draw.clipped(rect, |draw| {
            if selection.focused && !selection.range.is_empty() {
                let fill = crate::style::StyleColor(crate::style::rgba_color(self.env.tokens.color.accent.soft));
                for row in placed.range_rects(selection.range.clone()) {
                    draw.fill(&PaintShape::Rect(row.translate(at.to_vec2()), Default::default()), fill);
                }
            }
            draw.galley(at, &layout);
        });
        if self.enabled {
            let hit = Rect::from_min_size(at, layout.size()).intersect(rect);
            self.record.borrow_mut().selections.push((part, Placed { layout: placed, origin: at, view: rect }, hit));
        }
    }

    /// The content-paint API for painting `part`: as [`draw`](Self::draw), and when the part was declared
    /// [`inactive`](super::PartBuilder::inactive) its data colours use the theme's disabled fill.
    pub fn draw_part(&self, part: impl Into<PartRef>) -> Draw<'_> {
        let part = part.into();
        let inactive = self.results.get(&(part.key, part.index)).is_some_and(|result| result.inactive);
        let draw = self.draw();
        if inactive { draw.data(DataMode::Inactive) } else { draw }
    }

    /// Runs `paint` with this `PaintUi` cut to `rect` (inside the clip already in force), as `Draw::clipped` does for a
    /// `Draw`: content painted whole over one pane of the component, such as a sheet's merged cell under a frozen row.
    /// The children shown inside take the cut too; the clip is restored afterwards.
    pub fn clipped<R>(&mut self, rect: Rect, paint: impl FnOnce(&mut Self) -> R) -> R {
        let canvas = self.canvas;
        let kept = host_ui(self.host, |ui| {
            let kept = ui.clip_rect();
            ui.set_clip_rect(rect.intersect(kept));
            kept
        });
        canvas.push_clip(rect);
        let out = paint(self);
        canvas.pop_clip();
        host_ui(self.host, |ui| ui.set_clip_rect(kept));
        out
    }

    /// Runs `f` with the style context of this component's env, so app hooks called from `paint` pick colours and
    /// sizes through tokens as `style` does.
    pub fn with_style<R>(&self, variant: Variant, f: impl FnOnce(&StyleCx<'_>) -> R) -> R {
        let env = self.env;
        let cx = StyleCx::new(&env.tokens, &env.values, self.size, variant, env.direction, env.scale, env.os, env.active, env.key, env.line_ratio)
            .with_caches(env.ctx.styles());
        f(&cx)
    }

    /// Shows a child component at a physical rect, with id `cx.key(key)`. The child `Ui` is pinned left to
    /// right: this site is unmirrored.
    pub fn child<C: Show>(&mut self, key: impl Hash + Debug, rect: Rect, child: C) -> C::Output {
        self.child_with(self.id.with(key), rect, child, None)
    }

    /// Shows `child` as the owner's own control, such as a picker the owner draws its field with. The child's node
    /// folds into the owner's: the owner's node stands for the child's focus stop, carries the child's value text when
    /// the owner's `access` sets none, and takes the actions and requests aimed at it, which reach the child. The
    /// owner's name names the control.
    pub fn inner<C: Show>(&mut self, key: impl Hash + Debug, rect: Rect, child: C) -> C::Output {
        let (id, owner) = (self.id.with(key), self.id);
        // Only the access report reads the fold, and turning the access tree on runs every node again. A later look
        // keeps the fold its first look asked for.
        let first = self.keep.is_none();
        host_ui(self.host, |ui| {
            if first && ui.build_cx().access_enabled() {
                access::fold::request(ui.ctx(), id, owner);
            }
        });
        self.child_with(id, rect, child, None)
    }

    /// Makes the child shown with `key` the one a click on the composite's label focuses, in place of the composite's
    /// first stop, such as the number field of a row whose slider comes first in Tab order. A child with no stop this
    /// pass leaves the first stop.
    pub fn label_target(&mut self, key: impl Hash + Debug) {
        access::scope::note_label_target(self.env.view().field_label(), self.id.with(key));
    }

    /// Shows a child with rich tooltip content.
    pub fn child_with_tip<C: Show>(&mut self, key: impl Hash + Debug, rect: Rect, child: C, tip: impl FnOnce(&mut Ui)) -> C::Output {
        self.child_with(self.id.with(key), rect, child, Some(Box::new(tip)))
    }

    /// Runs the label step (B2) for one leaf child at `rect`: the label's placement beside or above the child, the
    /// label scope, scrub and the reset dot. The child gets the width the label leaves and its own height at the top
    /// of that rect, as in a labelled container's body, so the label lines up with its baseline on either path.
    pub fn labelled<C: Show>(&mut self, key: impl Hash + Debug, rect: Rect, label: Label, side: SideLabel, child: C) -> C::Output {
        let id = self.id.with(key);
        let kids = self.kids();
        let (enabled, size) = (self.enabled, self.size);
        // Without a node of its own the composite leaves the label to the node its `Ui` sits under.
        let parent = self.node.then_some(self.id);
        let owner = self.id;
        if let Some(control) = self.kept_rect(id, rect) {
            return host_ui(self.host, |ui| {
                let inside = label::leaf_under(&Env::of(ui), &side, &child, (enabled, size)).inside;
                kids.keep(ui, id, control, child, inside)
            });
        }
        let (out, control) = host_ui(self.host, |ui| {
            let env = Env::of(ui);
            let mut child = child;
            let under = label::leaf_under(&env, &side, &child, (enabled, size));
            let at = LeafLabel { id, owner, parent, row: rect, egui_cell: false, text: &label, side: &side, under: &under };
            let placed = crate::layout::ColumnGroup::field(ui, |ui| label::leaf(ui, &env, &at, &mut child, None));
            let mut control = placed.control;
            if child.fills() != Some(craft_behavior::Axis::Y) {
                let ideal = run::measure_under(&mut child, ui, Some(id), craft_behavior::Proposal::Ideal, Some(&under));
                control.max.y = control.max.y.min(control.min.y + ideal.y);
            }
            // The child shows under its label scope.
            (Env::run_in(ui, placed.env_for(&env), |ui| kids.show(ui, id, control, child, None, under.inside.clone())), control)
        });
        self.children.borrow_mut().push(Shown { id, at: rect, rect: control });
        out
    }

    /// What the children shown from here inherit.
    fn kids(&self) -> Kids {
        Kids { enabled: self.enabled, size: self.size, veiled: self.fade.is_some() }
    }

    /// Shows `child` at `rect` under the composite's `enabled` and `size`.
    fn child_with<C: Show>(&mut self, id: Id, rect: Rect, child: C, tip: Option<TipFn<'_>>) -> C::Output {
        let kids = self.kids();
        if let Some(rect) = self.kept_rect(id, rect) {
            return host_ui(self.host, |ui| kids.keep(ui, id, rect, child, None));
        }
        self.children.borrow_mut().push(Shown { id, at: rect, rect });
        host_ui(self.host, |ui| kids.show(ui, id, rect, child, tip, None))
    }

    /// In a later look (15.6 rule 5): records the child `id` shown at `at` and gives the rect it took in the first
    /// look, or `at` when the first look showed another child in this place. `None` in the first look.
    fn kept_rect(&self, id: Id, at: Rect) -> Option<Rect> {
        let keep = self.keep?;
        let mut children = self.children.borrow_mut();
        let rect = keep.get(children.len()).filter(|first| first.id == id && first.at == at).map_or(at, |first| first.rect);
        children.push(Shown { id, at, rect });
        Some(rect)
    }

    /// Paints `face` as the look of `part`: its style, surface and state fill from the part's state merged with the
    /// instance's `flags()` and the host's validity flags, then its own `paint`. The face reads the host's text
    /// colour through `StyleCx::host_ink` and its muted colour through `StyleCx::host_muted_ink`. It registers no id, senses nothing, opens no memory stage and
    /// fires nothing.
    pub fn face<C: Show>(&mut self, part: impl Into<PartRef>, face: C) {
        self.face_with(part.into(), face, self.text, true, 1.0);
    }

    /// `face`, for a face on a fill the composite draws for that part alone (a plate over a picture): the face reads
    /// `ink` through `StyleCx::host_ink` instead of the composite's text colour.
    pub fn face_in<C: Show>(&mut self, part: impl Into<PartRef>, face: C, ink: &Stateful<StyleColor>) {
        self.face_with(part.into(), face, Some(ink), true, 1.0);
    }

    /// [`face`](Self::face) painted `share` as opaque as it would be (1 paints as `face` does, 0 paints nothing): a
    /// component fading in or out as the one component that draws it. A share outside 0 to 1 is clamped and a
    /// non-finite one leaves the opacity as it was; fades stack with the opacity already in force. A faded face is a
    /// look only: it reports no sub-node, so a component fading out has no node while it fades.
    pub fn face_faded<C: Show>(&mut self, part: impl Into<PartRef>, face: C, share: f32) {
        self.face_with(part.into(), face, self.text, false, share);
    }

    /// Paints `face` for `part`; a `reported` face gives the part's sub-node its facts (`steps::report_parts`).
    fn face_with<C: Show>(&mut self, part: PartRef, face: C, ink: Option<&Stateful<StyleColor>>, reported: bool, share: f32) {
        let Some(result) = self.results.get(&(part.key, part.index)) else {
            debug_assert!(false, "face for a part `parts` did not declare");
            return;
        };
        let focus = self.active.filter(|(active, _)| *active == part).map(|(_, visible)| visible);
        let source = FaceSource {
            id: self.id,
            result,
            focus,
            ink,
            muted: self.muted,
            flags: self.flags,
            enabled: self.enabled,
            faded: self.fade.is_some(),
            reported: reported && self.node,
        };
        let (canvas, env) = (self.canvas, self.env);
        let host = &mut *self.host;
        let (baseline, record) = canvas.faded(share, || run::paint_face(host, env, canvas, &source, part, face));
        let mut own = self.record.borrow_mut();
        if let Some(baseline) = baseline {
            own.baseline.get_or_insert(baseline);
        }
        own.faces.extend(record);
    }

    /// Reserves an empty place at the current end of the variant's paint order, clipped to `clip`, for items added
    /// later by code that holds no painter (`PaintSlot::fill`).
    pub fn reserve_slot(&self, clip: Rect) -> PaintSlot {
        PaintSlot::reserve_in(self.canvas, self.record, self.id, self.rect.min, clip)
    }

    /// The raw painter for an allowlisted content painter, clipped to the component's own rect. Its data paths paint
    /// data colours as [`draw`](Self::draw) does.
    pub fn content_painter<C: ContentPainter>(&self, cx: &Cx<'_, C>) -> ContentPainterCx<'_> {
        let _ = cx;
        ContentPainterCx::new(self.canvas, self.env, self.ink).data(self.data).within(self.rect)
    }
}
