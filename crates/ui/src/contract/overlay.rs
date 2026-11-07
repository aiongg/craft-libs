//! Chapter 15: the `Overlay` data, its presets, popup handles, `OverlayUi` and the overlay host.
//!
//! One host (`host`) creates the egui layer through `runtime::egui::areas` and applies rank, backdrop, fit, scroll,
//! dismissal and the focus scope. `OverlayUi::show` and the tooltip and toast hosts run it.

use super::{BodyCx, Component, Cx, Resolved};
use crate::runtime::egui::areas::{self, LayerSpec};
use crate::runtime::egui::memory::{self as store, Service};
use crate::runtime::env::Env;
use crate::runtime::focus::{self, FocusPolicy, Initial};
use crate::runtime::memory::{MemoryStage, sub};
use crate::runtime::motion::{self, MotionSpec, spec};
use crate::runtime::overlay::{self, Backdrop, CloseReason, OpenEntry, Rank};
use crate::runtime::{frame, keys, pointer};
use craft_behavior::overlay::{Align, MinWidth, PlaceRequest, Side};
use craft_behavior::{CancelRule, Intent, OwnerClass};
use egui::{Align2, Context, Id, Pos2, Rect, Response, Ui};
use std::collections::HashMap;

pub use crate::runtime::overlay::{OpenFilter, any_open, family, is_open, request_close, top};

/// Where an overlay hangs from.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum Anchor {
    /// A rect in the opener's layer space.
    Rect(Rect),
    /// The opener's own rect.
    Response,
    /// A point in the opener's layer space.
    Point(Pos2),
    /// Where the pointer was when the overlay opened.
    Pointer,
    /// A corner or edge of the viewport's safe rect, inset by the panel padding.
    Viewport(Align2),
}

/// Where a floating layer sits.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct FloatPlace {
    /// A screen position for the layer's left-top; `None` centres it in the viewport.
    pub pos: Option<Pos2>,
}

impl FloatPlace {
    /// Centred in the viewport.
    pub const fn centred() -> Self {
        FloatPlace { pos: None }
    }

    /// At `pos`.
    pub const fn at(pos: Pos2) -> Self {
        FloatPlace { pos: Some(pos) }
    }
}

/// Where an egui layer sits.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum Place {
    /// Hung from an anchor.
    #[non_exhaustive]
    Anchored {
        /// What it hangs from.
        anchor: Anchor,
        /// The preferred side.
        side: Side,
        /// The alignment along the anchor's edge.
        align: Align,
    },
    /// Free-standing.
    Float(FloatPlace),
}

impl Place {
    /// An anchored place.
    pub const fn anchored(anchor: Anchor, side: Side, align: Align) -> Self {
        Place::Anchored { anchor, side, align }
    }
}

/// A fraction of the viewport per axis.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ViewportShare {
    /// The share of the width.
    pub x: f32,
    /// The share of the height.
    pub y: f32,
}

impl ViewportShare {
    /// A share of `x` of the width and `y` of the height.
    pub const fn new(x: f32, y: f32) -> Self {
        ViewportShare { x, y }
    }
}

/// How an egui layer sizes.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum OverlayFit {
    /// From its content.
    Content,
    /// From its content at first; the user's size is saved.
    Resizable,
    /// From its content up to a share of the viewport; it scrolls beyond.
    ContentCapped(ViewportShare),
}

/// How the user may resize a layer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Resize {}

/// What a press outside the layer and its anchor does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Outside {
    /// Closes the layer; the press reaches the widget below.
    #[default]
    Close,
    /// Nothing.
    Keep,
    /// Closes the layer and the press reaches nothing below.
    Absorb,
}

/// When a layer closes by itself. Escape goes through the focus policy's `CancelRule`.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Dismiss {
    /// A press outside.
    pub outside: Outside,
    /// Enter pressed inside and left untaken.
    pub enter: bool,
    /// Focus moved out of the layer's scope.
    pub focus_left: bool,
    /// A component inside was activated.
    pub on_activate: bool,
    /// The anchor is no longer shown.
    pub anchor_gone: bool,
    /// Seconds after opening.
    pub timeout: Option<f32>,
}

impl Dismiss {
    /// An outside press closes; nothing else does.
    pub const fn new() -> Self {
        Dismiss { outside: Outside::Close, enter: false, focus_left: false, on_activate: false, anchor_gone: true, timeout: None }
    }

    /// Sets what an outside press does.
    pub const fn outside(mut self, outside: Outside) -> Self {
        self.outside = outside;
        self
    }

    /// Closes on an untaken Enter.
    pub const fn enter(mut self, on: bool) -> Self {
        self.enter = on;
        self
    }

    /// Closes when focus leaves the scope.
    pub const fn focus_left(mut self, on: bool) -> Self {
        self.focus_left = on;
        self
    }

    /// Closes when a component inside is activated.
    pub const fn on_activate(mut self, on: bool) -> Self {
        self.on_activate = on;
        self
    }

    /// Closes after `seconds`.
    pub const fn timeout(mut self, seconds: Option<f32>) -> Self {
        self.timeout = seconds;
        self
    }
}

impl Default for Dismiss {
    fn default() -> Self {
        Dismiss::new()
    }
}

/// How a free-standing popup's trigger fires.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Trigger {
    /// A click on the opener toggles.
    Click,
    /// A press on the opener toggles.
    Press,
    /// A context request (right click, the Menu key, AccessKit `ShowContextMenu`) opens.
    Context,
    /// The pointer resting on the opener opens, after the tooltip delay.
    Hover,
    /// A press held for `HOLD` seconds opens.
    Hold,
    /// Only `PopupHandle::open` and the binding open it.
    Manual,
}

/// The component that opens a free-standing popup, and how.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct Opener {
    /// The opener's id.
    pub id: Id,
    /// The opener's rect in its layer space.
    pub rect: Rect,
    /// The trigger.
    pub trigger: Trigger,
}

/// Data describing an egui layer. Presets fill focus and dismissal; builders change single fields.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Overlay {
    /// Where it sits.
    pub place: Place,
    /// How it sizes.
    pub fit: OverlayFit,
    /// The narrowest it may be.
    pub min_width: MinWidth,
    /// Whether the user may move it.
    pub movable: bool,
    /// How the user may resize it.
    pub resize: Resize,
    /// Its rank.
    pub rank: Rank,
    /// The layer's key class, for both popup forms.
    pub keys: OwnerClass,
    /// Its focus scope.
    pub focus: FocusPolicy,
    /// Its dismissal rules.
    pub dismiss: Dismiss,
    /// Open and close motion.
    pub motion: MotionSpec,
    /// The component that opens it when it is free-standing.
    pub opener: Option<Opener>,
}

/// The share of the viewport a dropdown list may take.
const DROPDOWN_SHARE: ViewportShare = ViewportShare::new(1.0, 0.4);
/// The share of the viewport a dialog or modal may take.
const DIALOG_SHARE: ViewportShare = ViewportShare::new(0.8, 0.9);

impl Overlay {
    fn base(rank: Rank, place: Place, fit: OverlayFit, keys: OwnerClass, focus: FocusPolicy) -> Overlay {
        Overlay {
            place,
            fit,
            min_width: MinWidth::None,
            movable: false,
            resize: Resize {},
            rank,
            keys,
            focus,
            dismiss: Dismiss::new(),
            motion: spec::OPEN_FADE,
            opener: None,
        }
    }

    fn below() -> Place {
        Place::anchored(Anchor::Response, Side::Below, Align::Start)
    }

    /// A popover: a popup anchored below, taking focus in.
    pub fn popover() -> Overlay {
        Overlay::base(Rank::Popup, Overlay::below(), OverlayFit::Content, OwnerClass::Popup, FocusPolicy::POPUP)
    }

    /// A dropdown list: as wide as its anchor, capped, focus stays on the anchor.
    pub fn dropdown() -> Overlay {
        let focus = FocusPolicy::POPUP.trap(false).restore(false).initial(Initial::KeepAnchor);
        let mut overlay = Overlay::base(Rank::Popup, Overlay::below(), OverlayFit::ContentCapped(DROPDOWN_SHARE), OwnerClass::PassivePopup, focus);
        overlay.min_width = MinWidth::Anchor;
        overlay
    }

    /// A menu: a popup the arrow keys move through. A press outside closes it and goes no further.
    pub fn menu() -> Overlay {
        let mut overlay = Overlay::base(Rank::Popup, Overlay::below(), OverlayFit::Content, OwnerClass::Menu, FocusPolicy::MENU);
        overlay.dismiss = Dismiss::new().outside(Outside::Absorb);
        overlay
    }

    /// A submenu: beside its parent entry.
    pub fn submenu() -> Overlay {
        Overlay::menu().place(Place::anchored(Anchor::Response, Side::End, Align::Start))
    }

    /// A tooltip: below the control, no focus, never takes input.
    pub fn tooltip() -> Overlay {
        let mut overlay = Overlay::base(Rank::Tooltip, Overlay::below(), OverlayFit::Content, OwnerClass::PassivePopup, FocusPolicy::NONE);
        overlay.dismiss = Dismiss::new().outside(Outside::Keep);
        overlay
    }

    /// A dialog: modal, floating, movable and resizable. `.modeless()` gives rank `Window`.
    pub fn dialog() -> Overlay {
        let mut overlay = Overlay::base(
            Rank::modal(Backdrop::Scrim),
            Place::Float(FloatPlace::centred()),
            OverlayFit::ContentCapped(DIALOG_SHARE),
            OwnerClass::Dialog,
            FocusPolicy::DIALOG,
        );
        overlay.movable = true;
        overlay.dismiss = Dismiss::new().outside(Outside::Keep);
        overlay
    }

    /// A modal: centred, content capped.
    pub fn modal() -> Overlay {
        let mut overlay = Overlay::dialog();
        overlay.movable = false;
        overlay
    }

    /// A toast stack: the viewport's bottom end corner.
    pub fn toast() -> Overlay {
        let place = Place::anchored(Anchor::Viewport(Align2::RIGHT_BOTTOM), Side::Above, Align::End);
        let mut overlay = Overlay::base(Rank::Toast, place, OverlayFit::Content, OwnerClass::Widget, FocusPolicy::NONE);
        overlay.dismiss = Dismiss::new().outside(Outside::Keep);
        overlay
    }

    /// A floating panel: a window-rank layer with no scope.
    pub fn floating() -> Overlay {
        let mut overlay = Overlay::base(Rank::Window, Place::Float(FloatPlace::centred()), OverlayFit::Content, OwnerClass::Widget, FocusPolicy::NONE);
        overlay.dismiss = Dismiss::new().outside(Outside::Keep);
        overlay
    }

    /// Drag feedback at the pointer.
    pub fn ghost() -> Overlay {
        let mut overlay = Overlay::base(
            Rank::Tooltip,
            Place::anchored(Anchor::Pointer, Side::End, Align::Start),
            OverlayFit::Content,
            OwnerClass::PassivePopup,
            FocusPolicy::NONE,
        );
        overlay.dismiss = Dismiss::new().outside(Outside::Keep);
        overlay
    }

    /// Rank `Window` and no backdrop.
    pub fn modeless(mut self) -> Overlay {
        self.rank = Rank::Window;
        self
    }

    /// Sets where it sits.
    pub fn place(mut self, place: Place) -> Overlay {
        self.place = place;
        self
    }

    /// Sets how it sizes.
    pub fn fit(mut self, fit: OverlayFit) -> Overlay {
        self.fit = fit;
        self
    }

    /// Sets the narrowest width.
    pub fn min_width(mut self, min_width: MinWidth) -> Overlay {
        self.min_width = min_width;
        self
    }

    /// Sets whether the user may move it.
    pub fn movable(mut self, on: bool) -> Overlay {
        self.movable = on;
        self
    }

    /// Sets the focus scope.
    pub fn focus(mut self, focus: FocusPolicy) -> Overlay {
        self.focus = focus;
        self
    }

    /// Sets the dismissal rules.
    pub fn dismiss(mut self, dismiss: Dismiss) -> Overlay {
        self.dismiss = dismiss;
        self
    }

    /// Sets the open and close motion.
    pub fn motion(mut self, motion: MotionSpec) -> Overlay {
        self.motion = motion;
        self
    }

    /// Records the opener and anchors the layer at its rect.
    pub fn opened_by(mut self, anchor: &Response, trigger: Trigger) -> Overlay {
        self.opener = Some(Opener { id: anchor.id, rect: anchor.rect, trigger });
        self.place = Place::anchored(Anchor::Rect(anchor.rect), Side::Below, Align::Start);
        self
    }
}

/// An id and a name; `Copy`, never a snapshot of the open state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopupHandle {
    id: Id,
    name: &'static str,
}

impl PopupHandle {
    pub(crate) fn new(opener: Id, name: &'static str) -> Self {
        PopupHandle { id: sub(opener, ("popup", name)), name }
    }

    /// The popup's id: its egui layer id.
    pub fn id(self) -> Id {
        self.id
    }

    /// The name given to `cx.popup`.
    pub fn name(self) -> &'static str {
        self.name
    }
}

/// The open slot of a popup, in its opener's memory stage: the one owner of the popup's open state.
#[derive(Clone, Copy, Default)]
struct OpenSlot(bool);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OpenOp {
    Toggle,
    Open,
    Close,
}

/// Popups registered this pass, the open changes waiting for step 15 and the popups shown this pass.
#[derive(Default)]
struct Popups {
    registered: Vec<(Id, Id)>,
    pending: Vec<(Id, OpenOp)>,
    shown: Vec<(Id, Id)>,
}

fn with_popups<R>(ctx: &Context, f: impl FnOnce(&mut Popups) -> R) -> R {
    Service::<Popups>::of(ctx).with(f)
}

/// Step 9, before `update`: the popups `owner` registers are registered again.
pub(crate) fn begin_owner(ctx: &Context, owner: Id) {
    with_popups(ctx, |popups| {
        popups.registered.retain(|(registered, _)| *registered != owner);
        popups.shown.retain(|(shown, _)| *shown != owner);
    });
}

fn register(ctx: &Context, owner: Id, popup: Id) {
    with_popups(ctx, |popups| {
        if !popups.registered.contains(&(owner, popup)) {
            popups.registered.push((owner, popup));
        }
    });
}

fn queue(ctx: &Context, popup: Id, op: OpenOp) {
    with_popups(ctx, |popups| popups.pending.push((popup, op)));
}

fn folded(ctx: &Context, popup: Id, slot: bool) -> bool {
    with_popups(ctx, |popups| {
        popups.pending.iter().filter(|(id, _)| *id == popup).fold(slot, |open, (_, op)| match op {
            OpenOp::Toggle => !open,
            OpenOp::Open => true,
            OpenOp::Close => false,
        })
    })
}

/// Whether `popup` is open in this pass: its slot with the changes queued so far applied.
fn open_now(stage: &mut MemoryStage, popup: Id) -> bool {
    let slot = stage.shown::<OpenSlot>(popup).0;
    folded(&stage.ctx().clone(), popup, slot)
}

/// Whether a popup of `owner` was shown in this pass; step 12 reads it as the `OPEN` flag.
pub(crate) fn popup_shown(ctx: &Context, owner: Id) -> bool {
    with_popups(ctx, |popups| popups.shown.iter().any(|(shown, _)| *shown == owner))
}

/// Step 10: the open state of every popup `owner` registered, with the closes `request_close` queued applied.
pub(crate) fn open_map(stage: &mut MemoryStage, owner: Id) -> HashMap<Id, bool> {
    let ctx = stage.ctx().clone();
    let popups: Vec<Id> = with_popups(&ctx, |popups| popups.registered.iter().filter(|(registered, _)| *registered == owner).map(|(_, popup)| *popup).collect());
    popups
        .into_iter()
        .map(|popup| {
            if overlay::take_close(&ctx, popup).is_some() {
                queue(&ctx, popup, OpenOp::Close);
            }
            (popup, open_now(stage, popup))
        })
        .collect()
}

/// Step 10, after the hook: the closes `show` decided become queued changes that step 15 writes.
pub(crate) fn apply_closes(ctx: &Context, closed: Vec<(Id, CloseReason)>) {
    for (popup, _) in closed {
        queue(ctx, popup, OpenOp::Close);
    }
}

/// Step 15: writes the open slots of the popups `owner` registered, then forgets the queued changes.
pub(crate) fn write_open(stage: &mut MemoryStage, owner: Id) {
    let ctx = stage.ctx().clone();
    let popups: Vec<Id> = with_popups(&ctx, |popups| popups.registered.iter().filter(|(registered, _)| *registered == owner).map(|(_, popup)| *popup).collect());
    for popup in popups {
        let open = open_now(stage, popup);
        stage.shown::<OpenSlot>(popup).0 = open;
        with_popups(&ctx, |popups| popups.pending.retain(|(id, _)| *id != popup));
    }
}

/// The seconds a press must be held for `Trigger::Hold`.
pub const HOLD: f64 = 0.5;

/// The `Cx` trait that registers popups.
pub trait Overlays {
    /// Registers the popup `name` of this component for this pass and returns its handle. A component registers every
    /// popup its `overlay` hook shows in `update` or earlier; inside `overlay` the call only fetches the handle.
    fn popup(&mut self, name: &'static str) -> PopupHandle;
}

impl<C: Component> Overlays for Cx<'_, C> {
    fn popup(&mut self, name: &'static str) -> PopupHandle {
        let handle = PopupHandle::new(self.core.id, name);
        register(&self.core.ctx, self.core.id, handle.id);
        handle
    }
}

impl PopupHandle {
    /// Toggles or opens the popup from `trigger` at step 9, as the pass's pending open change.
    pub fn trigger<C: Component>(self, trigger: Trigger, cx: &mut Cx<'_, C>) {
        let core = &mut cx.core;
        let ctx = core.ctx.clone();
        let toggled = match trigger {
            Trigger::Click | Trigger::Press => core.activation.is_some(),
            Trigger::Context => core.intents.take(Intent::ContextMenu),
            Trigger::Hover => self.dwell(&ctx, core.memory, core.response.hovered(), super::tooltip::DELAY),
            Trigger::Hold => self.dwell(&ctx, core.memory, core.press.phase.is_press(), HOLD),
            Trigger::Manual => false,
        };
        if toggled {
            queue(&ctx, self.id, if matches!(trigger, Trigger::Click | Trigger::Press) { OpenOp::Toggle } else { OpenOp::Open });
        } else if trigger != Trigger::Manual && core.intents.take(Intent::Open) {
            queue(&ctx, self.id, OpenOp::Open);
        }
    }

    /// Whether `held` has lasted `delay` seconds, for a hold or hover trigger; asks for a pass at the deadline.
    fn dwell(self, ctx: &Context, stage: &mut MemoryStage, held: bool, delay: f64) -> bool {
        let now = frame::clock(ctx);
        let since = stage.shown::<Dwell>(sub(self.id, "dwell"));
        if !held {
            since.0 = None;
            return false;
        }
        let start = *since.0.get_or_insert(now);
        let due = start + delay;
        if now >= due {
            since.0 = None;
            return true;
        }
        crate::runtime::egui::repaint::request_after(ctx, crate::runtime::egui::pass::current_viewport(ctx), due - now);
        false
    }

    /// Queues an open that step 15 writes. For `Trigger::Manual` this and the binding are the only ways it opens.
    pub fn open<C: Component>(self, cx: &mut Cx<'_, C>) {
        queue(&cx.core.ctx, self.id, OpenOp::Open);
    }

    /// Queues a close; step 15 writes the slot, so this pass still shows the popup open.
    pub fn close<C: Component>(self, cx: &mut Cx<'_, C>) {
        queue(&cx.core.ctx, self.id, OpenOp::Close);
    }

    /// Whether the popup is open in this pass: valid from step 8, with the changes queued so far applied.
    pub fn is_open<C: Component>(self, cx: &mut Cx<'_, C>) -> bool {
        open_now(cx.core.memory, self.id)
    }
}

#[derive(Clone, Copy, Default)]
struct Dwell(Option<f64>);

/// What the `overlay` hook gets: one call per anchored popup of this component, built at step 10 with the open state
/// of every popup `cx.popup` registered this pass.
pub struct OverlayUi<'a> {
    pub(crate) ctx: &'a Context,
    pub(crate) owner: Id,
    /// The owner's rect in its own layer space.
    pub(crate) anchor: Rect,
    /// The owner's layer to the screen: anchors are layer-local, popups sit in screen space.
    pub(crate) to_screen: egui::emath::TSTransform,
    pub(crate) resolved: &'a Resolved<'a>,
    pub(crate) env: Env,
    pub(crate) open: HashMap<Id, bool>,
    /// Popups whose dismissal or body request was decided in this pass; step 15 writes their slots.
    pub(crate) closed: Vec<(Id, CloseReason)>,
}

impl<'a> OverlayUi<'a> {
    pub(crate) fn new(ctx: &'a Context, owner: Id, anchor: Rect, ui: &Ui, resolved: &'a Resolved<'a>, env: Env, open: HashMap<Id, bool>) -> Self {
        let to_screen = areas::layer_to_screen(ctx, ui.layer_id());
        OverlayUi { ctx, owner, anchor, to_screen, resolved, env, open, closed: Vec::new() }
    }

    /// Shows the popup while its handle is open; the content is a body closure. `None` while closed.
    pub fn show<R>(&mut self, popup: PopupHandle, overlay: Overlay, mut body: impl FnMut(&mut Ui, &mut BodyCx<'_>) -> R) -> Option<R> {
        let Some(open) = self.open.get(&popup.id).copied() else {
            debug_assert!(false, "popup {:?} is shown but was not registered with `cx.popup` this pass", popup.name);
            return None;
        };
        let ctx = self.ctx.clone();
        let owner = overlay.opener.map_or(self.owner, |opener| opener.id);
        let presence = motion::layer_presence(&ctx, popup.id, owner, self.env.motion, open, overlay.motion);
        if !presence.visible {
            forget_pointer(&ctx, popup.id);
            return None;
        }
        let anchor_local = match (&overlay.place, overlay.opener) {
            (_, Some(opener)) => opener.rect,
            (Place::Anchored { anchor: Anchor::Rect(rect), .. }, None) => *rect,
            (Place::Anchored { anchor: Anchor::Point(at), .. }, None) => Rect::from_min_max(*at, *at),
            _ => self.anchor,
        };
        let layer = Layer { id: popup.id, overlay: &overlay, anchor: self.to_screen * anchor_local, env: &self.env, exiting: !open, fade: presence.t };
        let resolved = self.resolved;
        let mut result = None;
        let mut requested = false;
        let env = self.env.clone();
        let out = host(&ctx, &layer, &mut |ui| {
            let mut stage = if ui.is_sizing_pass() { MemoryStage::open_peek(&ctx, popup.id) } else { MemoryStage::open_owned(&ctx, popup.id, owner) };
            let rect = ui.available_rect_before_wrap();
            let mut bcx = BodyCx::new(popup.id, env.view(), resolved, rect, &mut stage);
            result = Some(body(ui, &mut bcx));
            requested = bcx.close_requested;
            stage.write_back();
        });
        with_popups(&ctx, |popups| popups.shown.push((self.owner, popup.id)));
        if open {
            if requested {
                self.closed.push((popup.id, CloseReason::Requested));
            } else if let Some(reason) = out.dismissed {
                self.closed.push((popup.id, reason));
            }
        }
        result.filter(|_| open)
    }
}

/// What the host reads for one layer.
pub(crate) struct Layer<'a> {
    /// The layer's egui id.
    pub id: Id,
    pub overlay: &'a Overlay,
    /// The anchor in screen space.
    pub anchor: Rect,
    /// The env the layer's body starts with.
    pub env: &'a Env,
    /// The layer is fading out.
    pub exiting: bool,
    /// The opacity the open and close motion gives.
    pub fade: f32,
}

/// What the host reports.
pub(crate) struct LayerOut {
    /// Why the layer asks to close, decided after its body ran.
    pub dismissed: Option<CloseReason>,
}

#[derive(Clone, Copy, Default)]
struct Since(f64);

/// The stop that was focused when a layer first appeared.
#[derive(Clone, Copy, Default)]
struct NotedOpener(Option<focus::StopKey>);

fn pointer_slot(id: Id) -> Id {
    sub(id, "pointer_at")
}

fn forget_pointer(ctx: &Context, id: Id) {
    let _ = store::take::<Pos2>(ctx, pointer_slot(id));
    let _ = store::take::<Since>(ctx, sub(id, "since"));
    let _ = store::take::<NotedOpener>(ctx, sub(id, "opener_stop"));
}

fn anchor_rect(ctx: &Context, layer: &Layer<'_>, bounds: Rect) -> Rect {
    let point = |at: Pos2| Rect::from_min_max(at, at);
    match &layer.overlay.place {
        Place::Anchored { anchor: Anchor::Viewport(at), .. } => {
            let inset = layer.env.tokens.space.pad.panel;
            point(at.pos_in_rect(&bounds.shrink(inset)))
        }
        Place::Anchored { anchor: Anchor::Pointer, .. } => {
            let at = store::peek_clone::<Pos2>(ctx, pointer_slot(layer.id)).or_else(|| areas::pointer_pos(ctx)).unwrap_or(bounds.center());
            store::put(ctx, pointer_slot(layer.id), at);
            point(at)
        }
        _ => layer.anchor,
    }
}

/// Where the layer goes: the pure placement of the anchored forms, the viewport centre or the given position for floats.
fn placement(ctx: &Context, layer: &Layer<'_>, size: egui::Vec2, bounds: Rect, anchor: Rect) -> (Rect, Side) {
    match &layer.overlay.place {
        Place::Anchored { side, align, .. } => {
            let kept = overlay::entry(ctx, layer.id).map(|entry| entry.side);
            let request = PlaceRequest::new(anchor, size, bounds)
                .side(*side)
                .align(*align)
                .rtl(layer.env.direction.is_rtl())
                .min_width(layer.overlay.min_width)
                .kept(kept);
            let placed = craft_behavior::overlay::place(&request);
            (placed.rect, placed.side)
        }
        Place::Float(float) => {
            let min = float.pos.unwrap_or_else(|| bounds.center() - size / 2.0);
            (Rect::from_min_size(min, size), Side::Below)
        }
    }
}

/// The overlay host: shows `body` on an egui layer for `layer` and decides its dismissal.
pub(crate) fn host(ctx: &Context, layer: &Layer<'_>, body: &mut dyn FnMut(&mut Ui)) -> LayerOut {
    let overlay = layer.overlay;
    let bounds = areas::safe_rect(ctx);
    let anchor = anchor_rect(ctx, layer, bounds);
    let size = areas::layer_rect(ctx, layer.id).map_or(egui::Vec2::ZERO, |rect| rect.size());
    let (at, side) = placement(ctx, layer, size, bounds, anchor);
    let tier = overlay.rank.tier();
    // The cover under the layer: a modal's scrim, or a transparent one that swallows an outside press.
    let backdrop = match overlay.rank {
        Rank::Modal { backdrop } => Some(backdrop),
        _ if overlay.dismiss.outside == Outside::Absorb && !layer.exiting => Some(Backdrop::None),
        _ => None,
    };
    let cover_id = sub(layer.id, "cover");
    let cover = backdrop.filter(|backdrop| *backdrop != Backdrop::None || overlay.dismiss.outside == Outside::Absorb).map(|backdrop| {
        let scrim = match (backdrop, overlay.rank) {
            (Backdrop::None, _) => None,
            _ => Some(crate::style::scrim_color(&layer.env.tokens)),
        };
        let hole = match backdrop {
            Backdrop::ScrimWithHole(hole) => Some(hole),
            _ => None,
        };
        areas::show_cover_layer(ctx, cover_id, tier, bounds, scrim, hole);
        areas::layer_of(tier, cover_id)
    });
    let parent = overlay::drawing_parent(ctx);
    let scope = (overlay.focus != FocusPolicy::NONE).then_some(overlay.focus);
    let capped = match overlay.fit {
        OverlayFit::ContentCapped(share) => Some(egui::Vec2::new(bounds.width() * share.x, bounds.height() * share.y)),
        _ => None,
    };
    let spec = LayerSpec { id: layer.id, tier, at: at.min, interactable: overlay.rank != Rank::Tooltip && !layer.exiting };
    // The opener of the layer's scope is the stop focused when the layer first appears: egui's modal gate takes focus
    // from the widgets below a modal layer by the first pass that shows the layer.
    let opener_slot = sub(layer.id, "opener_stop");
    let noted = store::peek_clone::<NotedOpener>(ctx, opener_slot).or_else(|| {
        let noted = NotedOpener(focus::focused_stop(ctx));
        store::put(ctx, opener_slot, noted);
        Some(noted)
    });
    overlay::begin_layer(ctx, layer.id);
    let shown = areas::show_layer(ctx, &spec, |ui| {
        areas::fade(ui, layer.fade);
        let installed = Env::install_for(ui, layer.env.clone());
        let real = !ui.is_sizing_pass();
        if let (Some(policy), true) = (scope, real) {
            focus::push_scope_from(ctx, layer.id, policy, None, noted.and_then(|noted| noted.0));
        }
        match capped {
            Some(cap) => {
                areas::capped(ui, cap, |ui| body(ui));
            }
            None => body(ui),
        }
        if scope.is_some() && real {
            focus::pop_scope(ctx);
        }
        Env::restore(ctx, installed);
    });
    overlay::end_layer(ctx);
    let mut entry = OpenEntry::new(layer.id, overlay.rank, overlay.keys, shown.rect, shown.layer);
    entry.parent = parent;
    entry.anchor = matches!(overlay.place, Place::Anchored { .. }).then_some(anchor);
    entry.side = side;
    entry.exiting = layer.exiting;
    entry.cover = cover;
    overlay::record(ctx, entry);
    let dismissed = if layer.exiting { None } else { dismissal(ctx, layer, shown.rect, anchor, scope) };
    LayerOut { dismissed }
}

/// The dismissal rules of 15.4, decided after the body ran so a field inside still commits in the press pass.
fn dismissal(ctx: &Context, layer: &Layer<'_>, rect: Rect, anchor: Rect, scope: Option<FocusPolicy>) -> Option<CloseReason> {
    let dismiss = layer.overlay.dismiss;
    if let Some(reason) = escape(ctx, layer, scope) {
        return Some(reason);
    }
    if dismiss.outside != Outside::Keep
        && let Some(press) = areas::primary_press_at(ctx)
    {
        let inside_family = overlay::family(ctx, layer.id).iter().filter_map(|id| overlay::entry(ctx, *id)).any(|entry| entry.rect.contains(press));
        if !rect.contains(press) && !anchor.contains(press) && !inside_family {
            return Some(CloseReason::Outside);
        }
    }
    if dismiss.enter && scope.is_some() && overlay::top(ctx) == Some(layer.id) && keys::take_submit(ctx) {
        return Some(CloseReason::Enter);
    }
    if dismiss.focus_left
        && let Some(policy) = scope
        && policy.kind != craft_behavior::ScopeKind::Group
        && focus::focus_in_scope(ctx, layer.id) == Some(false)
        && overlay::is_open(ctx, layer.id)
    {
        return Some(CloseReason::FocusLeft);
    }
    if let Some(timeout) = dismiss.timeout {
        let now = frame::clock(ctx);
        let since = store::peek_clone::<Since>(ctx, sub(layer.id, "since")).unwrap_or_else(|| {
            store::put(ctx, sub(layer.id, "since"), Since(now));
            Since(now)
        });
        let due = since.0 + f64::from(timeout);
        if now >= due {
            return Some(CloseReason::Timeout);
        }
        crate::runtime::egui::repaint::request_after(ctx, crate::runtime::egui::pass::current_viewport(ctx), due - now);
    }
    None
}

/// Escape reaches only the top layer, and only when focus is inside its scope: a component inside that takes the
/// Escape (a draft, a gesture) keeps it, and a captured press goes first.
fn escape(ctx: &Context, layer: &Layer<'_>, scope: Option<FocusPolicy>) -> Option<CloseReason> {
    let policy = scope?;
    if policy.cancel == CancelRule::Ignore || overlay::top(ctx) != Some(layer.id) {
        return None;
    }
    if focus::focus_in_scope(ctx, layer.id) != Some(true) {
        return None;
    }
    let step = keys::scope_escape(ctx, policy.cancel, pointer::press_captured(ctx), false)?;
    matches!(step, craft_behavior::focus::EscapeStep::CloseScope).then_some(CloseReason::Escape)
}

/// Paints the ghost egui layer from runtime data: the drag preview and drop mark, and the painted cursor the cursor
/// service settled on. The end hook calls it once per pass, popup or no popup.
pub fn paint_frame_layers(ctx: &Context) {
    super::ghost::paint(ctx);
    paint_cursor(ctx);
}

fn paint_cursor(ctx: &Context) {
    let Some(at) = areas::pointer_pos(ctx) else { return };
    let Some(craft_behavior::Cursor::Painted(cursor)) = crate::runtime::cursor::current(ctx) else { return };
    let Some(env) = Env::root_of(ctx) else { return };
    let ink = areas::CursorInk {
        color: crate::style::ink_color(&env.tokens, crate::style::Ink::OnDark),
        width: crate::style::rule_look(&env.tokens, crate::style::RuleKind::Edge).width,
        arm: env.tokens.size.icon.sm,
    };
    areas::paint_cursor(ctx, at, &cursor, ink);
}
