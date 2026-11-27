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
//! Dismissal comes from ui-rt: a press outside, an Escape no focused node handled, focus leaving,
//! the user turning to another window or app. The layer child keeps the reason and asks its owner
//! to build; the owner's overlay pass reads it, closes the popup and reports it through
//! `OverlayUi::closing`. A close the body asks for takes the same path. A modal blocks the layers
//! below it in its window and, as an app-modal, every other window; the one input it lets through
//! is its modal reach (a canvas a dialog samples, `runtime::sampler`).

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use craft_behavior::overlay::{Align, MinWidth, Side};
use craft_i18n::Direction;
use craft_theme::Tokens;
use craft_ui_rt::{
    AccessCx, BuildCx, Constraints, CornerRadii, DismissReason, DisplayList, Element, Event, EventCx, Handled, HitShape, InlineState, Layer,
    LayerSpec, LayoutCx, ModalReach, NodeId, OutsidePress, PaintCx, Placement, Pos2, Rect, Sense, State, Vec2,
};
use smallvec::SmallVec;

use super::env::{EnvCells, LayerFrame};
use super::intents::ScopeStep;
use super::memory::Memory;
use super::motion::Level;
use super::node::Kept;
use super::sweep::Sweep;
use super::ui::{Flow, Ui};
use crate::contract::overlay::{ActivationChain, Anchor, Outside, Overlay, OverlayFit, Place};
use crate::contract::{Context, Id, POPUP_PAD};
use crate::primitives::text_adapter::ScopeEscape;
use crate::primitives::{Canvas, Draw, DrawEnv, FrameServices, PaintRecord};
use crate::runtime::access::PopupKind;
use crate::runtime::focus::{FocusMove, FocusPolicy};
use crate::runtime::motion::spec as motion;
use crate::runtime::overlay::{CloseReason, OpenEntry, Rank};
use crate::style::{Corners, Ink, SurfaceKind, UiTokens};

/// One popup of a component: its open state and what the run did to it.
#[derive(Clone, Copy, Debug)]
struct PopupEntry {
    id: Id,
    open: bool,
    /// The open state the run began with (`PopupHandle::changed`).
    began: bool,
    /// The value its open binding held after the last run (`PopupHandle::bind_open`).
    bound: Option<bool>,
    /// Why it closed in this run (its dismissal, a close its body asked for).
    closed: Option<CloseReason>,
    /// A `Trigger::Hover` opener runs it: its layer reports the pointer entering and leaving.
    watch_hover: bool,
    /// The pointer is over its layer, as of the start of the run.
    hovered: bool,
    /// The ui-rt layer it showed on last.
    layer: Option<Layer>,
    /// The frame time it last opened at (`Popup::opening`).
    opened_at: Option<f64>,
}

/// The popups of one component and whether each is open, kept on its node (`Kept::popups`). The
/// run takes them into its `CxCore` (`PopupHandle` reads and changes them in `update`), the
/// overlay pass shows the open ones, and the node keeps them for its next run.
#[derive(Debug, Default)]
pub(crate) struct PopupStates {
    entries: SmallVec<[PopupEntry; 1]>,
    /// What each popup's body keeps in memory, by popup id: it lasts while the component shows, so a popup closed and
    /// opened again finds what its body kept.
    bodies: BodyMemories,
    /// A `Trigger::Context` opener ran in this run: the node's access node offers the context menu.
    context_menu: bool,
    /// The frame time of the run.
    now: f64,
}

impl PopupStates {
    pub(crate) fn new() -> PopupStates {
        PopupStates::default()
    }

    /// A run of the node `cx` builds begins: every popup's state is its starting state, no close is reported yet,
    /// and a popup a hover opener runs reads whether the pointer is over its layer now.
    pub(crate) fn begin_run(&mut self, cx: &BuildCx<'_>) {
        for entry in &mut self.entries {
            entry.began = entry.open;
            entry.closed = None;
            entry.hovered = entry.watch_hover && entry.open && entry.layer.is_some_and(|layer| layer_hovered(cx, layer, entry.id.value()));
        }
        self.context_menu = false;
        self.now = cx.now();
    }

    /// Whether any popup is open: the component's `OPEN` state.
    pub(crate) fn any_open(&self) -> bool {
        self.entries.iter().any(|e| e.open)
    }

    pub(crate) fn is_open(&self, popup: Id) -> bool {
        self.entries.iter().any(|e| e.id == popup && e.open)
    }

    fn entry(&mut self, popup: Id) -> &mut PopupEntry {
        match self.entries.iter().position(|e| e.id == popup) {
            Some(at) => &mut self.entries[at],
            None => {
                self.entries.push(PopupEntry {
                    id: popup,
                    open: false,
                    began: false,
                    bound: None,
                    closed: None,
                    watch_hover: false,
                    hovered: false,
                    layer: None,
                    opened_at: None,
                });
                self.entries.last_mut().expect("just pushed")
            }
        }
    }

    /// Opens or closes `popup`; whether that changed it.
    pub(crate) fn set(&mut self, popup: Id, open: bool) -> bool {
        let now = self.now;
        let entry = self.entry(popup);
        let changed = entry.open != open;
        entry.open = open;
        if changed && open {
            entry.opened_at = Some(now);
        }
        changed
    }

    /// Whether `popup` is open and opened in this frame: what its body resets on opening resets once, in every build
    /// round of that frame.
    pub(crate) fn opening(&self, popup: Id) -> bool {
        self.entries.iter().any(|e| e.id == popup && e.open && e.opened_at == Some(self.now))
    }

    /// Closes an open `popup` for `reason`, reported by `closing` for the rest of the run.
    pub(crate) fn close(&mut self, popup: Id, reason: CloseReason) -> bool {
        let entry = self.entry(popup);
        if !entry.open {
            return false;
        }
        entry.open = false;
        entry.closed.get_or_insert(reason);
        true
    }

    /// Why `popup` closed in this run.
    pub(crate) fn closing(&self, popup: Id) -> Option<CloseReason> {
        self.entries.iter().find(|e| e.id == popup).and_then(|e| e.closed)
    }

    /// Whether the run left `popup` in a different state from the one it began with.
    pub(crate) fn changed(&self, popup: Id) -> bool {
        self.entries.iter().find(|e| e.id == popup).is_some_and(|e| e.open != e.began)
    }

    /// The open state `popup` began the run with.
    pub(crate) fn began(&self, popup: Id) -> bool {
        self.entries.iter().find(|e| e.id == popup).is_some_and(|e| e.began)
    }

    pub(crate) fn bound(&self, popup: Id) -> Option<bool> {
        self.entries.iter().find(|e| e.id == popup).and_then(|e| e.bound)
    }

    pub(crate) fn set_bound(&mut self, popup: Id, bound: Option<bool>) {
        self.entry(popup).bound = bound;
    }

    /// A `Trigger::Hover` opener runs `popup`: its layer reports hover, which keeps it open while the pointer is on it.
    pub(crate) fn watch_hover(&mut self, popup: Id) {
        self.entry(popup).watch_hover = true;
    }

    pub(crate) fn watches_hover(&self, popup: Id) -> bool {
        self.entries.iter().any(|e| e.id == popup && e.watch_hover)
    }

    /// Whether the pointer is over `popup`'s layer.
    pub(crate) fn hovered(&self, popup: Id) -> bool {
        self.entries.iter().any(|e| e.id == popup && e.hovered)
    }

    /// `popup` showed on `layer` in this run.
    pub(crate) fn shown_on(&mut self, popup: Id, layer: Layer) {
        self.entry(popup).layer = Some(layer);
    }

    /// A `Trigger::Context` opener ran: the access node offers `ShowContextMenu`.
    pub(crate) fn offer_context_menu(&mut self) {
        self.context_menu = true;
    }

    /// Whether the access node offers `ShowContextMenu` (step 16 reads it).
    pub(crate) fn offers_context_menu(&self) -> bool {
        self.context_menu
    }

    /// The memory of `popup`'s body, moved out for a run of the body; [`put_body`](Self::put_body) gives it back.
    pub(crate) fn take_body(&mut self, popup: Id) -> Memory {
        match self.bodies.0.iter().position(|(id, _)| *id == popup) {
            Some(at) => self.bodies.0.swap_remove(at).1,
            None => Memory::default(),
        }
    }

    pub(crate) fn put_body(&mut self, popup: Id, memory: Memory) {
        self.bodies.0.push((popup, memory));
    }
}

/// The body memories of a component's popups.
#[derive(Default)]
struct BodyMemories(SmallVec<[(Id, Memory); 1]>);

impl std::fmt::Debug for BodyMemories {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.0.iter().map(|(id, _)| id)).finish()
    }
}

/// A close a layer's body decided (a container's `request_close_as`, a popup body's close), kept
/// with the layer child until its owner's overlay pass reads it.
pub(crate) type BodyClose = Rc<Cell<Option<CloseReason>>>;

/// Provided to a container shown as a layer's content (`OverlayUi::show_container`): the
/// container `id` is the body of that layer already, so it fills the layer's region and makes no
/// layer of its own, and it reports its close through `closed`.
#[derive(Clone)]
pub(crate) struct LayerHost {
    pub(crate) id: Id,
    pub(crate) closed: BodyClose,
}

impl LayerHost {
    /// The container closed for `why`: its popup closes in its owner's overlay pass.
    pub(crate) fn close(&self, why: CloseReason) {
        if self.closed.get().is_none() {
            self.closed.set(Some(why));
        }
    }
}

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
    /// The layer's key and surface, as its element gave them (the harness lists layers by them).
    pub(crate) key: u64,
    pub(crate) surface: Surface,
    /// ui-rt asked it to go; its owner's next overlay pass closes it.
    pub(crate) dismissed: Option<DismissReason>,
    /// The pointer is over it (a toast stack holds its timers while it is).
    pub(crate) hovered: bool,
    /// The frame under the content, in the node's coordinates, recorded in the build.
    pub(crate) frame: DisplayList,
    /// The size and tokens `frame` was recorded for.
    frame_for: Option<(Vec2, usize, bool)>,
    /// The env cell of the layer frame the body's surfaces meet (`LayerFrame`): the popup's own
    /// rounded frame in window coordinates, written before the body runs.
    frame_cell: Option<State<Option<LayerFrame>>>,
    /// The Enter and Escape its scope took for its body, and whether its body has content to clear.
    pub(crate) scope: super::intents::ScopeIntents,
    /// A close its body decided, read by its owner's overlay pass.
    pub(crate) closed: BodyClose,
    /// The frame time it opened at, for `Dismiss::timeout`.
    pub(crate) since: Option<f64>,
    /// What its modal block lets through, as its body's last run registered it (`runtime::sampler`).
    pub(crate) reach: Option<ModalReach>,
    /// Its entry in its window's layer stack, for the layer queries.
    pub(crate) entry: Option<OpenEntry>,
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
        LayerKept {
            kept: Kept::new(),
            key: self.key,
            surface: self.surface,
            dismissed: None,
            hovered: false,
            frame: DisplayList::default(),
            frame_for: None,
            frame_cell: None,
            scope: Default::default(),
            closed: BodyClose::default(),
            since: None,
            reach: None,
            entry: None,
        }
    }

    fn layout(&self, lk: &mut LayerKept, cx: &mut LayoutCx<'_>, _: Constraints) -> Vec2 {
        lk.kept.place_children(cx, Vec2::ZERO);
        lk.kept.size
    }

    fn relayout_boundary(&self) -> bool {
        true
    }

    fn paint(&self, lk: &LayerKept, cx: &mut PaintCx<'_>) {
        // A popup fades in as it opens: paint only, no build per frame.
        super::motion::paint_level(&lk.kept, cx);
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
    /// Its entry in the window's layer stack (`runtime::overlay::OpenEntry`); `None` for a layer
    /// the layer queries do not list (a tooltip, the toast stack).
    pub(crate) entry: Option<OpenEntry>,
    /// A container shown as its content takes it as its own layer (`LayerHost`): the id it shows under.
    pub(crate) host: Option<Id>,
    /// An activation inside closes it (`Dismiss::on_activate`).
    pub(crate) on_activate: bool,
    /// An Enter its body leaves untaken closes it (`Dismiss::enter`).
    pub(crate) enter: bool,
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
        // A popup or menu goes when the user turns to another window or app, as a press outside it would.
        if overlay.rank == Rank::Popup && overlay.dismiss.outside != Outside::Keep {
            spec = spec.dismiss_on_away();
        }
        // A command run from outside closes a popup as a menu pick does; a flyout its host opens and closes stays.
        if overlay.rank == Rank::Popup && overlay.dismiss.commands {
            spec = spec.dismiss_on_command();
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
        let mut entry = OpenEntry::new(Id::NULL, overlay.rank, overlay.keys, Rect::NOTHING, layer);
        entry.role = overlay.role;
        entry.anchor = anchor;
        let (on_activate, enter) = (overlay.dismiss.on_activate, overlay.dismiss.enter && overlay.focus != FocusPolicy::NONE);
        LayerShow {
            layer,
            spec,
            surface,
            pad,
            share,
            anchor,
            role,
            hover: false,
            sweep: None,
            focus: overlay.focus,
            entry: Some(entry),
            host: None,
            on_activate,
            enter,
        }
    }
}

/// The close a ui-rt dismissal means. A press in another window or the app losing focus is a
/// press outside; a command run from outside closes as a pick inside would.
pub(crate) fn close_reason(reason: DismissReason) -> CloseReason {
    match reason {
        DismissReason::Escape => CloseReason::Escape,
        DismissReason::FocusLeft => CloseReason::FocusLeft(FocusMove::Other),
        DismissReason::Command => CloseReason::Activated,
        _ => CloseReason::Outside,
    }
}

/// The dismissal the layer child `key` on `layer` of the node being built got since its owner
/// last built.
pub(crate) fn dismissed(cx: &BuildCx<'_>, layer: Layer, key: u64) -> Option<DismissReason> {
    cx.peek_layer::<LayerScope>(layer, key).and_then(|p| p.local.dismissed)
}

/// Why the layer child `key` on `layer` of the node being built closes: its dismissal from ui-rt,
/// else a close its body decided. Taken: the next read answers `None`.
pub(crate) fn layer_closed(cx: &BuildCx<'_>, layer: Layer, key: u64) -> Option<CloseReason> {
    let peek = cx.peek_layer::<LayerScope>(layer, key)?;
    peek.local.dismissed.map(close_reason).or_else(|| peek.local.closed.take())
}

/// Whether the pointer is over the layer child `key` on `layer` of the node being built (one that
/// reports hover).
pub(crate) fn layer_hovered(cx: &BuildCx<'_>, layer: Layer, key: u64) -> bool {
    cx.peek_layer::<LayerScope>(layer, key).is_some_and(|p| p.local.hovered)
}

/// The frame time the layer child `key` on `layer` of the node being built opened at.
pub(crate) fn opened_at(cx: &BuildCx<'_>, layer: Layer, key: u64) -> Option<f64> {
    cx.peek_layer::<LayerScope>(layer, key).and_then(|p| p.local.since)
}

/// The content of a layer child: measured before a new layer is placed, then shown at its place.
/// What it keeps lives with its owner (a popup's body memory on the owner's node, a container's
/// own node), not with the layer child, so it outlasts a close.
pub(crate) trait LayerBody<R> {
    /// The size the content takes on `ui`, a measuring cursor at the body's region.
    fn measure(&mut self, ui: &mut Ui<'_, '_>) -> Vec2;
    /// Shows the content on `ui` at the body's region; a close it decides goes in `closed`.
    fn show(&mut self, ui: &mut Ui<'_, '_>, closed: &BodyClose) -> R;
}

impl<R, F: FnMut(&mut Ui<'_, '_>) -> R> LayerBody<R> for F {
    fn measure(&mut self, ui: &mut Ui<'_, '_>) -> Vec2 {
        self(ui);
        ui.flow.content_size()
    }

    fn show(&mut self, ui: &mut Ui<'_, '_>, _: &BodyClose) -> R {
        self(ui)
    }
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
    mut body: &mut dyn FnMut(&mut Ui<'_, '_>) -> R,
) -> R {
    show_layer_with(cx, id, key, show, rerun, &mut body)
}

/// [`show_layer`] with any [`LayerBody`]: a container shown as a layer's content measures and
/// shows through its own entry points.
pub(crate) fn show_layer_with<R: Clone + 'static>(
    cx: &mut BuildCx<'_>,
    id: Id,
    key: u64,
    show: LayerShow,
    rerun: bool,
    body: &mut dyn LayerBody<R>,
) -> R {
    let viewport = cx.env().viewport.get(cx);
    let rtl = is_rtl(cx);
    let LayerShow { layer, mut spec, surface, pad, share, anchor, role, hover, sweep, focus, entry, host, on_activate, enter } = show;
    // A layer child the layer queries list moves the window's open-layer cell when it goes.
    if entry.is_some() {
        spec = spec.listed();
    }
    // The modal reach its body registered on its last run.
    if layer == Layer::Modal {
        spec = spec.reach(cx.peek_layer::<LayerScope>(layer, key).and_then(|p| p.local.reach));
    }
    let element = LayerScope { key, surface, hover };
    cx.inline_layer(layer, key, spec, element, |lk, state, cx| {
        lk.kept.focus.scope = focus;
        let kept_value = lk.kept.value.as_ref().and_then(|v| v.downcast_ref::<R>()).cloned();
        if let Some(value) = kept_value.filter(|_| !rerun && !state.must_run() && !state.path) {
            cx.keep_children();
            return value;
        }
        // The text fields in its body follow its Escape rules: whether an Escape closes it, keeps their typing, or is
        // its Cancel (a modal).
        if focus != FocusPolicy::NONE || spec.escape {
            let closes = spec.escape || matches!(focus.cancel, craft_behavior::CancelRule::Close);
            cx.provide(ScopeEscape { closes, keeps_typing: focus.escape_keeps_typing, modal: layer == Layer::Modal });
        }
        let run = LayerRun { id, layer, spec, surface, pad, share, anchor, viewport, role, hover, sweep, rtl, entry, host, on_activate, enter };
        let out = run_layer(lk, state, cx, run, body);
        if state.new {
            super::focus::enter_layer(&lk.kept, cx);
        }
        out
    })
}

struct LayerRun {
    id: Id,
    layer: Layer,
    spec: LayerSpec,
    surface: Surface,
    pad: f32,
    share: Vec2,
    anchor: Option<Rect>,
    viewport: Vec2,
    role: accesskit::Role,
    hover: bool,
    sweep: Option<State<Sweep>>,
    /// The owner's flow direction: the popup frame mirrors with it.
    rtl: bool,
    entry: Option<OpenEntry>,
    host: Option<Id>,
    on_activate: bool,
    enter: bool,
}

fn run_layer<R: Clone + 'static>(lk: &mut LayerKept, state: InlineState, cx: &mut BuildCx<'_>, run: LayerRun, body: &mut dyn LayerBody<R>) -> R {
    cx.count_scope();
    cx.hover_events(run.hover);
    lk.dismissed = None;
    if state.new {
        lk.since = Some(cx.now());
        lk.closed.set(None);
        if run.surface == Surface::Popup {
            let level = lk.kept.level.get_mut();
            Level::aim(level, 0.0, motion::OPEN_FADE);
            Level::aim(level, 1.0, motion::OPEN_FADE);
        }
    }
    // A layer shown inside a dialog host's `stack_place`, or opened from inside one, belongs to that host's dialog.
    let context = Context::of_build(cx);
    if let Some(dialog) = context.service::<StackPlace, _>(|place| place.0).or_else(|| running_dialog(cx)) {
        cx.provide(RunningDialog(dialog));
    }
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
        frame(body.measure(&mut measuring))
    } else {
        current
    };
    let at = run.spec.place(size, run.viewport, run.anchor, Vec2::ZERO).to_pos2();
    let region = Rect::from_min_size(at + Vec2::splat(run.pad), room);
    // The body's surfaces meet the popup's corners: its frame is known here, from the size it is placed by.
    let frame_cell = *lk.frame_cell.get_or_insert_with(|| cx.state(None));
    let layer_frame = (run.surface != Surface::Plain).then(|| LayerFrame { rect: Rect::from_min_size(at, size), radii: popup_radii(&tokens) });
    frame_cell.set_if_changed(cx, layer_frame);
    if let Some(mut cells) = EnvCells::of(cx) {
        cells.frame = frame_cell;
        cx.provide(cells);
    }
    if let Some(id) = run.host {
        cx.provide(LayerHost { id, closed: lk.closed.clone() });
    }
    if run.on_activate {
        let mut chain = cx.provided::<ActivationChain>().unwrap_or_default();
        chain.0.push(lk.closed.clone());
        cx.provide(chain);
    }
    // A modal's body says what of the canvas below it stays usable while it runs.
    let modal = run.layer == Layer::Modal;
    if modal {
        crate::runtime::sampler::begin_modal(&context);
    }
    let mut ui = Ui::new(cx, Flow::new(run.id, at, region, gap, tokens.clone()).with_sweep(run.sweep));
    ui.flow.scope = lk.scope.take_for_body();
    let out = body.show(&mut ui, &lk.closed);
    let Ui { cx, flow, .. } = ui;
    if run.enter && flow.scope.submit && lk.closed.get().is_none() {
        lk.closed.set(Some(CloseReason::Enter));
    }
    if modal {
        let reach = crate::runtime::sampler::end_modal(&context, cx);
        if lk.reach != reach {
            lk.reach = reach;
            cx.set_layer_reach(reach);
        }
    }
    lk.scope.clearable = flow.scope.clearable;
    let real = frame(flow.content_size());
    if real != size && !state.new {
        cx.rebuild();
    }
    let frame_for = Some((real, Arc::as_ptr(&tokens) as usize, run.rtl));
    if lk.frame_for != frame_for {
        lk.frame = record_frame(cx, run.surface, real, &tokens, run.rtl);
        lk.frame_for = frame_for;
        cx.mark_changed();
    }
    if lk.kept.placed != flow.placed || lk.kept.size != real {
        lk.kept.placed = flow.placed;
        lk.kept.size = real;
        cx.request_layout();
    }
    lk.kept.id = run.id;
    lk.kept.note_id(cx, run.id);
    let entry = run.entry.map(|entry| OpenEntry { id: run.id, rect: Rect::from_min_size(at, real), ..entry });
    if lk.entry != entry {
        super::layers::note_changed(cx);
    }
    lk.entry = entry;
    if lk.kept.access.is_none() {
        lk.kept.access = Some(accesskit::Node::new(run.role));
    }
    lk.kept.value = Some(Box::new(out.clone()) as Box<dyn Any>);
    out
}

/// The frame of a layer child of `size` with `surface`, through the content-paint API. A tooltip
/// takes the popup look, as the contract's tooltip does.
fn record_frame(cx: &mut BuildCx<'_>, surface: Surface, size: Vec2, tokens: &Arc<Tokens>, rtl: bool) -> DisplayList {
    if surface == Surface::Plain {
        return DisplayList::default();
    }
    let scale = cx.scale();
    let fonts = crate::fonts::stack_facts(cx);
    let (text, images) = cx.paint_services();
    let services = FrameServices::new(text, images, scale, fonts);
    let ui_tokens = UiTokens::new(tokens.clone(), craft_theme::Contrast::Standard, craft_theme::Size::Compact);
    let canvas = Canvas::new(&services, Pos2::ZERO, Rect::EVERYTHING);
    let record = RefCell::new(PaintRecord::default());
    {
        let env = DrawEnv { tokens: &ui_tokens, direction: if rtl { Direction::Rtl } else { Direction::Ltr }, ink: Ink::OnPanel };
        Draw::new(&canvas, &record, env).surface(Rect::from_min_size(Pos2::ZERO, size), SurfaceKind::Popup, Corners::All);
    }
    canvas.finish().0
}

/// The corner radii of a popup surface under `tokens`, as the frame the body's surfaces meet.
fn popup_radii(tokens: &Arc<Tokens>) -> CornerRadii {
    let ui_tokens = UiTokens::new(tokens.clone(), craft_theme::Contrast::Standard, craft_theme::Size::Compact);
    let r = crate::style::surface_style(&ui_tokens, SurfaceKind::Popup).radius;
    CornerRadii { top_left: f32::from(r.nw), top_right: f32::from(r.ne), bottom_right: f32::from(r.se), bottom_left: f32::from(r.sw) }
}

/// Whether the flow direction of the node `cx` builds is right to left. A tracked read: the node
/// builds again when the direction changes.
pub(crate) fn is_rtl(cx: &mut BuildCx<'_>) -> bool {
    EnvCells::of(cx).is_some_and(|cells| cells.direction.get(cx) == Direction::Rtl)
}

/// The dialog of a dialog host whose [`Ui::stack_place`] runs, while it runs: a window service.
#[derive(Default)]
struct StackPlace(Option<Id>);

/// The host's dialog a layer child shows for, provided on the layer child to everything it owns, so a layer opened
/// from inside it belongs to the same dialog in every later build.
#[derive(Clone, Copy)]
struct RunningDialog(Id);

/// The dialog of a dialog host the node `cx` builds shows in ([`Ui::stack_place`]).
pub(crate) fn running_dialog(cx: &BuildCx<'_>) -> Option<Id> {
    cx.provided::<RunningDialog>().map(|dialog| dialog.0)
}

/// The layer child of the layer `id` names: the layer's own id, or the id of a container that is its layer's content.
fn layer_child(cx: &BuildCx<'_>, id: Id) -> Option<NodeId> {
    let node = super::ids::node_of(cx, id)?;
    match cx.runtime().local::<LayerScope>(node) {
        Some(_) => Some(node),
        None => super::layers::layer_child_of(cx.runtime(), node),
    }
}

/// Layer commands (docs/contract-rt.md 15.9).
impl Ui<'_, '_> {
    /// Brings the open layer `id`, with the layers opened from inside it, in front of the other layers of its kind, as
    /// a press on it does.
    pub fn raise_layer(&mut self, id: Id) {
        if self.is_measuring() {
            return;
        }
        if let Some(node) = layer_child(self.cx, id) {
            self.cx.raise_layer_child(node);
        }
    }

    /// Closes the layer `id` (a popup, a dialog, a container shown as a layer) for `why`: its owner builds and reads
    /// the close as one its body asked for, and reports it as the layer's close reason.
    pub fn close_layer(&mut self, id: Id, why: CloseReason) {
        if self.is_measuring() {
            return;
        }
        let Some(node) = layer_child(self.cx, id) else { return };
        let rt = self.cx.runtime();
        let Some(closed) = rt.local::<LayerScope>(node).map(|lk| lk.closed.clone()) else { return };
        let owner = rt.owner(node);
        if closed.get().is_none() {
            closed.set(Some(why));
        }
        if let Some(owner) = owner {
            self.cx.request_build_of(owner);
        }
    }

    /// Runs `show` for the dialog host's `dialog`: the layers it shows, and the layers opened from inside them, belong
    /// to `dialog` ([`running_dialog`](Self::running_dialog)), so a component inside tells one dialog from the next
    /// one the host shows with the same ids. The host shows its dialogs bottom first, and the layers stack in that
    /// order, which is the stack's order (`place`).
    pub fn stack_place<R>(&mut self, _place: usize, dialog: Id, show: impl FnOnce(&mut Ui<'_, '_>) -> R) -> R {
        let context = self.ctx().clone();
        let outer = context.service::<StackPlace, _>(|now| now.0.replace(dialog));
        let out = show(self);
        context.service::<StackPlace, _>(|now| now.0 = outer);
        out
    }
}
