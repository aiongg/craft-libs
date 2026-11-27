//! Overlays on craft-ui-rt layers (docs/contract-rt.md sections 2.3 and 10): popups, menus,
//! dialogs, tooltips and toasts are inline layer children of the node whose build shows them
//! ([`BuildCx::inline_layer`]). A closed overlay is not reached, so its node goes and it costs
//! nothing. An open one is a scope: its body runs when it opens, when its owner ran, or when a
//! node under it is marked; otherwise it keeps its subtree and returns its kept value.
//!
//! The body is laid out in the build, at the place ui-rt gives it after layout (anchor, flip,
//! clamp to the window, [`LayerSpec::place`]), so the rects its children return are this frame's
//! window rects. A popup opening measures its body once first (a measuring run: no node runs);
//! later runs place it by its kept size and build again in the same frame when that size moved.
//!
//! Dismissal comes from ui-rt: a press outside, an Escape no focused node handled, focus leaving.
//! The layer child keeps the reason and asks its owner to build; the owner's overlay pass reads
//! it, closes the popup and reports it through [`OverlayUi::closing`]. A modal blocks the layers
//! below it in its window and, as an app-modal, every other window.

use std::any::Any;
use std::cell::RefCell;
use std::sync::Arc;

use craft_behavior::overlay::{Align, MinWidth, Side};
use craft_i18n::Direction;
use craft_theme::Tokens;
use craft_ui_rt::{
    AccessCx, BuildCx, Constraints, DismissReason, DisplayList, Element, Event, EventCx, Handled, HitShape, InlineState, Layer, LayerSpec, LayoutCx,
    NodeId, OutsidePress, PaintCx, Placement, Pos2, Rect, Sense, State, Vec2, hash_key,
};
use egui::Id;
use smallvec::SmallVec;

use super::intents::ScopeStep;
use super::node::Kept;
use super::sweep::Sweep;
use super::ui::{Flow, Ui};
use crate::contract::POPUP_PAD;
use crate::contract::overlay::{Anchor, Outside, Overlay, OverlayFit, Place};
use crate::primitives::{Canvas, Draw, DrawEnv, FrameServices, PaintRecord};
use crate::runtime::access::PopupKind;
use crate::runtime::focus::{FocusMove, FocusPolicy};
use crate::runtime::overlay::{CloseReason, Rank};
use crate::style::{Corners, Ink, SurfaceKind, UiTokens};

/// A popup a component shows from its `overlay` hook: its name among the component's popups.
/// Its open state is kept on the component's node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopupHandle {
    pub(crate) key: u64,
    /// The name it was made with, for diagnostics.
    pub name: &'static str,
}

impl PopupHandle {
    /// The popup named `name` among its component's popups.
    pub fn new(name: &'static str) -> PopupHandle {
        PopupHandle { key: hash_key(&name), name }
    }
}

/// The popups of one component and whether each is open, kept on its node.
pub(crate) type PopupStates = SmallVec<[(u64, bool); 1]>;

/// What a layer child paints under its content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Surface {
    /// Nothing: the content paints its own frame (a dialog's container). Presses inside its
    /// bounds still stop there.
    Plain,
    /// A popup frame; presses inside stop there.
    Popup,
    /// A tooltip frame; it never takes input.
    Tip,
}

/// The retained half of a layer child: a scope's kept facts plus what ui-rt told it since its
/// owner last built.
pub(crate) struct LayerKept {
    pub(crate) kept: Kept,
    /// ui-rt asked it to go; its owner's next overlay pass closes it.
    pub(crate) dismissed: Option<DismissReason>,
    /// The pointer is over it (a toast stack holds its timers while it is).
    pub(crate) hovered: bool,
    /// The frame under the content, in the node's coordinates, recorded in the build.
    pub(crate) frame: DisplayList,
    /// The size and tokens `frame` was recorded for.
    frame_for: Option<(Vec2, usize)>,
    /// The Enter and Escape its scope took for its body, and whether its body has content to clear.
    pub(crate) scope: super::intents::ScopeIntents,
}

/// A layer child: a popup, menu, dialog, tooltip, toast stack.
pub(crate) struct LayerScope {
    pub(crate) key: u64,
    pub(crate) surface: Surface,
    /// It reports the pointer entering and leaving to its owner.
    pub(crate) hover: bool,
}

impl Element for LayerScope {
    type Output = ();
    type Local = LayerKept;

    fn props_key(&self) -> u64 {
        self.key ^ ((self.surface as u64) << 1) ^ u64::from(self.hover)
    }

    fn init(&self, _: &mut BuildCx<'_>) -> LayerKept {
        LayerKept { kept: Kept::new(), dismissed: None, hovered: false, frame: DisplayList::default(), frame_for: None, scope: Default::default() }
    }

    fn layout(&self, lk: &mut LayerKept, cx: &mut LayoutCx<'_>, _: Constraints) -> Vec2 {
        lk.kept.place_children(cx, Vec2::ZERO);
        lk.kept.size
    }

    fn relayout_boundary(&self) -> bool {
        true
    }

    fn paint(&self, lk: &LayerKept, cx: &mut PaintCx<'_>) {
        cx.extend(&lk.frame);
        if self.surface != Surface::Tip {
            let bounds = cx.bounds();
            cx.hit(HitShape::Rect(bounds), Sense::HOVER | Sense::CLICK, 0);
        }
    }

    fn event(&self, lk: &mut LayerKept, cx: &mut EventCx<'_>, ev: &Event) -> Handled {
        match ev {
            Event::Dismiss(reason) => {
                if lk.dismissed.is_none() {
                    lk.dismissed = Some(*reason);
                    cx.request_owner_build();
                }
                Handled::Yes
            }
            Event::Hover(on) => {
                lk.hovered = *on;
                cx.request_owner_build();
                Handled::Yes
            }
            // The scope's rules for what its content left: Enter submits, Escape clears or closes.
            Event::Key(k) => match super::intents::scope_key(&mut lk.scope, lk.kept.focus.scope, k) {
                ScopeStep::Queued => {
                    cx.request_build();
                    Handled::Yes
                }
                ScopeStep::Close => {
                    if lk.dismissed.is_none() {
                        lk.dismissed = Some(DismissReason::Escape);
                        cx.request_owner_build();
                    }
                    Handled::Yes
                }
                ScopeStep::No => super::focus::trap_tab(&lk.kept, true, cx, k),
            },
            // A press on the frame between its rows ends there.
            Event::Pointer(_) if self.surface != Surface::Tip => Handled::Yes,
            _ => Handled::No,
        }
    }

    fn access(&self, lk: &LayerKept, _: &mut AccessCx<'_>) -> Option<accesskit::Node> {
        lk.kept.access.clone()
    }
}

/// How a layer child is shown: its layer, its placement and dismissal for ui-rt, and what its
/// body gets.
pub(crate) struct LayerShow {
    pub(crate) layer: Layer,
    pub(crate) spec: LayerSpec,
    pub(crate) surface: Surface,
    /// Space between the frame and the body.
    pub(crate) pad: f32,
    /// The share of the window the body may take.
    pub(crate) share: Vec2,
    /// The window rect `Placement::Anchor` hangs from, known in this build.
    pub(crate) anchor: Option<Rect>,
    pub(crate) role: accesskit::Role,
    /// It reports the pointer entering and leaving.
    pub(crate) hover: bool,
    /// The window's toggle sweep record, passed to the body's flow.
    pub(crate) sweep: Option<State<Sweep>>,
    /// Its focus scope: whether it traps Tab, takes focus when it appears and gives it back.
    pub(crate) focus: FocusPolicy,
}

/// The ui-rt layer of an overlay's rank.
fn layer_of(overlay: &Overlay) -> Layer {
    match overlay.rank {
        Rank::Window => Layer::Floating,
        Rank::Toast => Layer::Toast,
        Rank::Popup if overlay.popup == PopupKind::Menu => Layer::Menu,
        Rank::Popup => Layer::Popup,
        Rank::Modal { .. } | Rank::ModalChrome => Layer::Modal,
        Rank::Tooltip => Layer::Tooltip,
    }
}

/// The ui-rt side of a contract side, mirrored in a right-to-left layout.
fn side_of(side: Side, rtl: bool) -> craft_ui_rt::Side {
    match (side, rtl) {
        (Side::Above, _) => craft_ui_rt::Side::Above,
        (Side::Start, false) | (Side::End, true) => craft_ui_rt::Side::Left,
        (Side::End, false) | (Side::Start, true) => craft_ui_rt::Side::Right,
        _ => craft_ui_rt::Side::Below,
    }
}

/// The ui-rt alignment along the anchor's edge. Below or above, reading order runs along x and
/// mirrors; beside, it runs down.
fn align_of(align: Align, side: Side, rtl: bool) -> craft_ui_rt::Align {
    let mirrored = rtl && matches!(side, Side::Below | Side::Above);
    match (align, mirrored) {
        (Align::Center, _) => craft_ui_rt::Align::Center,
        (Align::End, false) | (Align::Start, true) => craft_ui_rt::Align::Max,
        _ => craft_ui_rt::Align::Min,
    }
}

/// Where the overlay goes, for an owner `owner` whose window rect this frame is `owner_rect`.
/// Returns the placement and the anchor rect the build places it by.
fn placement_of(overlay: &Overlay, owner: NodeId, owner_rect: Rect, rtl: bool) -> (Placement, Option<Rect>) {
    if overlay.is_full_window() {
        return (Placement::Fill, None);
    }
    if let Some(rect) = overlay.placed {
        return (Placement::At(rect.min), None);
    }
    if let Some(pos) = overlay.placed_at {
        return (Placement::At(pos), None);
    }
    match &overlay.place {
        &Place::Anchored { ref anchor, side, align, offset, .. } => {
            let gap = match side {
                Side::Start | Side::End => offset.x.abs(),
                _ => offset.y.abs(),
            };
            let (rt_side, rt_align) = (side_of(side, rtl), align_of(align, side, rtl));
            let beside = |rect: Rect| (Placement::Beside { rect, side: rt_side, align: rt_align, gap }, Some(rect));
            match anchor {
                Anchor::Response => (Placement::Anchor { node: owner, side: rt_side, align: rt_align, gap }, Some(owner_rect)),
                &Anchor::Rect(rect) => beside(rect),
                &Anchor::Point(pos) => beside(Rect::from_min_size(pos, Vec2::ZERO)),
                &Anchor::Corner(body, corner) => beside(Rect::from_min_size(corner.pos_in_rect(&body), Vec2::ZERO)),
                // The pointer at opening and the parent layer's rect are not tracked on ui-rt yet.
                _ => (Placement::Center, None),
            }
        }
        Place::Float(float) => match float.pos {
            Some(pos) => (Placement::At(pos), None),
            None => (Placement::Center, None),
        },
    }
}

impl LayerShow {
    /// How `overlay` shows for its owner `owner` (window rect `owner_rect`) on ui-rt.
    pub(crate) fn of(overlay: &Overlay, owner: NodeId, owner_rect: Rect, rtl: bool, tokens: &Tokens) -> LayerShow {
        let layer = layer_of(overlay);
        let (placement, anchor) = placement_of(overlay, owner, owner_rect, rtl);
        let outside = match overlay.dismiss.outside {
            Outside::Keep => OutsidePress::Ignore,
            Outside::Absorb => OutsidePress::DismissAndConsume,
            _ => OutsidePress::Dismiss,
        };
        let mut spec = LayerSpec::new(placement).outside_press(outside);
        if overlay.min_width == MinWidth::Anchor {
            spec = spec.match_anchor_width();
        }
        if overlay.rank.takes_keys() {
            spec = spec.dismiss_on_escape();
        }
        if overlay.dismiss.focus_left {
            spec = spec.dismiss_on_focus_left();
        }
        if matches!(overlay.rank, Rank::Modal { .. }) {
            spec = spec.app_modal();
        }
        let surface = match overlay.rank {
            Rank::Tooltip => Surface::Tip,
            Rank::Popup | Rank::Toast => Surface::Popup,
            _ => Surface::Plain,
        };
        let share = match overlay.fit {
            OverlayFit::ContentCapped(share) => Vec2::new(share.x, share.y),
            _ => Vec2::splat(1.0),
        };
        let pad = if surface == Surface::Plain { 0.0 } else { POPUP_PAD.points(tokens) };
        let role = match overlay.popup {
            PopupKind::Menu => accesskit::Role::Menu,
            PopupKind::Listbox => accesskit::Role::ListBox,
            PopupKind::Tree => accesskit::Role::Tree,
            PopupKind::Grid => accesskit::Role::Grid,
            _ if surface == Surface::Tip => accesskit::Role::Tooltip,
            _ => accesskit::Role::Dialog,
        };
        LayerShow { layer, spec, surface, pad, share, anchor, role, hover: false, sweep: None, focus: overlay.focus }
    }
}

/// The close a ui-rt dismissal means.
pub(crate) fn close_reason(reason: DismissReason) -> CloseReason {
    match reason {
        DismissReason::Escape => CloseReason::Escape,
        DismissReason::FocusLeft => CloseReason::FocusLeft(FocusMove::Other),
        _ => CloseReason::Outside,
    }
}

/// The dismissal the layer child `key` on `layer` of the node being built got since its owner
/// last built.
pub(crate) fn dismissed(cx: &BuildCx<'_>, layer: Layer, key: u64) -> Option<DismissReason> {
    cx.peek_layer::<LayerScope>(layer, key).and_then(|p| p.local.dismissed)
}

/// Shows a layer child of the node `cx` builds, keyed `key`, with `body` as its content: keeps it
/// when nothing reaches for it (`rerun` false, no mark, not new), otherwise runs the body at its
/// place. `id` is the craft `Id` its content is keyed under.
pub(crate) fn show_layer<R: Clone + 'static>(
    cx: &mut BuildCx<'_>,
    id: Id,
    key: u64,
    show: LayerShow,
    rerun: bool,
    body: &mut dyn FnMut(&mut Ui<'_, '_>) -> R,
) -> R {
    let viewport = cx.env().viewport.get(cx);
    let LayerShow { layer, spec, surface, pad, share, anchor, role, hover, sweep, focus } = show;
    let element = LayerScope { key, surface, hover };
    cx.inline_layer(layer, key, spec, element, |lk, state, cx| {
        lk.kept.focus.scope = focus;
        let kept_value = lk.kept.value.as_ref().and_then(|v| v.downcast_ref::<R>()).cloned();
        if let Some(value) = kept_value.filter(|_| !rerun && !state.must_run() && !state.path) {
            cx.keep_children();
            return value;
        }
        let out = run_layer(lk, state, cx, LayerRun { id, spec, surface, pad, share, anchor, viewport, role, hover, sweep }, body);
        if state.new {
            super::focus::enter_layer(&lk.kept, cx);
        }
        out
    })
}

struct LayerRun {
    id: Id,
    spec: LayerSpec,
    surface: Surface,
    pad: f32,
    share: Vec2,
    anchor: Option<Rect>,
    viewport: Vec2,
    role: accesskit::Role,
    hover: bool,
    sweep: Option<State<Sweep>>,
}

fn run_layer<R: Clone + 'static>(
    lk: &mut LayerKept,
    state: InlineState,
    cx: &mut BuildCx<'_>,
    run: LayerRun,
    body: &mut dyn FnMut(&mut Ui<'_, '_>) -> R,
) -> R {
    cx.count_scope();
    cx.hover_events(run.hover);
    lk.dismissed = None;
    let tokens: Arc<Tokens> = super::env::tokens(cx);
    let gap = tokens.space.gap.stack;
    let constraints = run.spec.constraints(run.viewport, run.anchor);
    let room = (constraints.max * run.share - Vec2::splat(2.0 * run.pad)).max(Vec2::ZERO);
    let frame = |content: Vec2| constraints.constrain(content + Vec2::splat(2.0 * run.pad));
    let current = lk.kept.size;
    // A new layer child (or one that never sized) measures its body before it is placed; a
    // kept one is placed by its kept size and builds again if that moved.
    let size = if state.new || current == Vec2::ZERO {
        let region = Rect::from_min_size(Pos2::new(run.pad, run.pad), room);
        let mut measuring = Ui::new(cx, Flow::new(run.id, Pos2::ZERO, region, gap, tokens.clone()).with_sweep(run.sweep).measuring(None));
        body(&mut measuring);
        frame(measuring.flow.content_size())
    } else {
        current
    };
    let at = run.spec.place(size, run.viewport, run.anchor, Vec2::ZERO).to_pos2();
    let region = Rect::from_min_size(at + Vec2::splat(run.pad), room);
    let mut ui = Ui::new(cx, Flow::new(run.id, at, region, gap, tokens.clone()).with_sweep(run.sweep));
    ui.flow.scope = lk.scope.take_for_body();
    let out = body(&mut ui);
    let Ui { cx, flow, .. } = ui;
    lk.scope.clearable = flow.scope.clearable;
    let real = frame(flow.content_size());
    if real != size && !state.new {
        cx.rebuild();
    }
    let frame_for = Some((real, Arc::as_ptr(&tokens) as usize));
    if lk.frame_for != frame_for {
        lk.frame = record_frame(cx, run.surface, real, &tokens);
        lk.frame_for = frame_for;
        cx.mark_changed();
    }
    if lk.kept.placed != flow.placed || lk.kept.size != real {
        lk.kept.placed = flow.placed;
        lk.kept.size = real;
        cx.request_layout();
    }
    lk.kept.id = run.id;
    if lk.kept.access.is_none() {
        lk.kept.access = Some(accesskit::Node::new(run.role));
    }
    lk.kept.value = Some(Box::new(out.clone()) as Box<dyn Any>);
    out
}

/// The frame of a layer child of `size` with `surface`, through the content-paint API. A tooltip
/// takes the popup look, as the contract's tooltip does.
fn record_frame(cx: &mut BuildCx<'_>, surface: Surface, size: Vec2, tokens: &Arc<Tokens>) -> DisplayList {
    if surface == Surface::Plain {
        return DisplayList::default();
    }
    let scale = cx.scale();
    let (text, images) = cx.paint_services();
    let services = FrameServices::new(text, images, scale);
    let ui_tokens = UiTokens::new(tokens.clone(), craft_theme::Contrast::Standard, craft_theme::Size::Compact);
    let canvas = Canvas::new(&services, Pos2::ZERO, Rect::EVERYTHING);
    let record = RefCell::new(PaintRecord::default());
    let draw = Draw::new(&canvas, &record, DrawEnv { tokens: &ui_tokens, direction: Direction::Ltr, ink: Ink::OnPanel });
    draw.surface(Rect::from_min_size(Pos2::ZERO, size), SurfaceKind::Popup, Corners::All);
    canvas.finish().0
}

/// What the `overlay` hook gets: the component's popups and their open state, shown as layer
/// children of its node.
pub struct OverlayUi<'u, 'c> {
    pub(crate) cx: &'u mut BuildCx<'c>,
    /// The craft `Id` of the component.
    pub(crate) owner: Id,
    /// The component's window rect in this frame.
    pub(crate) anchor: Rect,
    pub(crate) popups: &'u mut PopupStates,
    /// The component's own hooks ran in this build: its popups' bodies run too, since they show
    /// what its props hold. On a pass that only reaches a marked node under a popup, they don't.
    pub(crate) owner_ran: bool,
    pub(crate) rtl: bool,
    pub(crate) tokens: Arc<Tokens>,
    /// The window's toggle sweep record, passed to the popups' flows.
    pub(crate) sweep: Option<State<Sweep>>,
    closed: SmallVec<[(u64, CloseReason); 1]>,
    /// A popup opened or closed in a pass where the component kept its outputs: it runs again in
    /// this frame to show its new state.
    pub(crate) changed: bool,
}

impl<'u, 'c> OverlayUi<'u, 'c> {
    pub(crate) fn new(
        cx: &'u mut BuildCx<'c>,
        owner: Id,
        anchor: Rect,
        popups: &'u mut PopupStates,
        owner_ran: bool,
        tokens: Arc<Tokens>,
        sweep: Option<State<Sweep>>,
    ) -> Self {
        OverlayUi { cx, owner, anchor, popups, owner_ran, rtl: false, tokens, sweep, closed: SmallVec::new(), changed: false }
    }

    /// Whether `popup` is open.
    pub fn is_open(&self, popup: PopupHandle) -> bool {
        self.popups.iter().any(|&(key, open)| key == popup.key && open)
    }

    /// Opens or closes `popup`.
    pub fn set_open(&mut self, popup: PopupHandle, open: bool) {
        if self.is_open(popup) == open {
            return;
        }
        match self.popups.iter_mut().find(|(key, _)| *key == popup.key) {
            Some(entry) => entry.1 = open,
            None => self.popups.push((popup.key, open)),
        }
        self.changed = true;
    }

    /// Opens a closed `popup`, closes an open one.
    pub fn toggle(&mut self, popup: PopupHandle) {
        let open = self.is_open(popup);
        self.set_open(popup, !open);
    }

    /// Why `popup` closed in this pass (its dismissal, or a close its body asked for); `None`
    /// while it stays open.
    pub fn closing(&self, popup: PopupHandle) -> Option<CloseReason> {
        self.closed.iter().find(|(key, _)| *key == popup.key).map(|(_, reason)| *reason)
    }

    /// Closes `popup` from its body's result, reporting `reason` through [`closing`](Self::closing).
    pub fn close(&mut self, popup: PopupHandle, reason: CloseReason) {
        if self.is_open(popup) {
            self.set_open(popup, false);
            self.closed.push((popup.key, reason));
        }
    }

    /// Shows `popup` while it is open, placed and dismissed as `overlay` says, with `body` as its
    /// content; `None` while it is closed (its node is gone). A popup ui-rt dismissed since the
    /// last pass closes here and shows nothing.
    pub fn show<R: Clone + 'static>(&mut self, popup: PopupHandle, overlay: &Overlay, mut body: impl FnMut(&mut Ui<'_, '_>) -> R) -> Option<R> {
        if !self.is_open(popup) {
            return None;
        }
        let show = LayerShow { sweep: self.sweep, ..LayerShow::of(overlay, self.cx.node(), self.anchor, self.rtl, &self.tokens) };
        if let Some(reason) = dismissed(self.cx, show.layer, popup.key) {
            self.close(popup, close_reason(reason));
            return None;
        }
        let id = self.owner.with(("craft-popup", popup.key));
        Some(show_layer(self.cx, id, popup.key, show, self.owner_ran, &mut body))
    }
}
