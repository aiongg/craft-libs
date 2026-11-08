//! Chapter 15: the `Overlay` data, its presets, popup handles, `OverlayUi` and the overlay host.
//!
//! One host (`host`) creates the egui layer through `runtime::egui::areas` and applies rank, backdrop, fit, scroll,
//! dismissal and the focus scope. `OverlayUi::show` and the tooltip and toast hosts run it.

use super::{BodyCx, Component, Cx, PartBuilder, Resolved};
use crate::runtime::access::{self, PopupKind, PopupShown};
use crate::runtime::commands::FireSource;
use crate::runtime::egui::areas::{self, LayerSpec};
use crate::runtime::egui::memory::Service;
use crate::runtime::env::Env;
use crate::runtime::focus::{self, FocusPolicy, Initial};
use crate::runtime::memory::{MemoryStage, sub};
use crate::runtime::motion::{self, MotionSpec, spec};
use crate::runtime::overlay::{self, Backdrop, CloseReason, OpenEntry, Rank};
use crate::runtime::windows::ResizeEdge;
use crate::runtime::{frame, keys, layout_memory, pointer};
use craft_behavior::PartRef;
use craft_behavior::overlay::{Align, MinWidth, PlaceRequest, Side};
use craft_behavior::{CancelRule, GesturePhase, Intent, OwnerClass, PointerButton};
use craft_keymap::Mods;
use egui::{Align2, Context, Id, Pos2, Rect, Response, Ui, Vec2};
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
pub struct Resize {
    /// Edge and corner grips resize the layer.
    pub grips: bool,
}

impl Resize {
    /// The layer keeps its size.
    pub const NONE: Resize = Resize { grips: false };
    /// Grips on every edge and corner.
    pub const GRIPS: Resize = Resize { grips: true };
}

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

/// A component that opens a free-standing popup, and how. A layer with several openers keeps the first as its
/// `Overlay.opener`, which keys the layer's open state, and the others in `others`.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Opener {
    /// The opener's id.
    pub id: Id,
    /// The opener's rect in its layer space.
    pub rect: Rect,
    /// The opener's egui layer: its rect maps to the screen through this layer's transform.
    pub layer: areas::Layer,
    /// The trigger.
    pub trigger: Trigger,
    /// Whether the opener was enabled in the pass that recorded it; a disabled opener closes the layer.
    pub enabled: bool,
    /// Where the layer hangs when this opener opened it. `None` hangs it at the opener's rect, or, for a
    /// `Trigger::Context` opener a pointer release opened, at the pointer.
    pub anchor: Option<Anchor>,
    /// The other openers of the same layer.
    pub others: Vec<Opener>,
    /// Whether the opener's response fired `trigger` in the pass that recorded it. A container with no `open`
    /// binding applies it to the layer's open slot.
    pub(crate) fired: bool,
    /// The fired context request was a secondary-button release, not a key.
    pub(crate) by_pointer: bool,
    /// The opener is pressed with the primary button, for `Trigger::Hold`.
    pub(crate) pressed: bool,
}

impl Opener {
    fn new(anchor: &Response, trigger: Trigger) -> Opener {
        let (fired, by_pointer, pressed) = pointer::with_pass_pointer(&anchor.ctx, |pass| {
            let Some(frame) = pass.frame.as_ref() else { return (false, false, false) };
            let on_opener = pass.press_targets.click == Some(anchor.id);
            let inside = frame
                .pos
                .is_some_and(|pos| anchor.rect.contains(areas::layer_map(&anchor.ctx, anchor.layer_id).map_or(pos, |map| map.to_layer(pos))));
            let by_pointer = inside && frame.release == Some(PointerButton::Secondary);
            let fired = match trigger {
                Trigger::Click => on_opener && inside && frame.release == Some(PointerButton::Primary),
                Trigger::Press => on_opener && frame.press.is_some_and(|press| press.button == PointerButton::Primary),
                Trigger::Context => by_pointer || keys::context_request(&anchor.ctx, anchor.id).is_some(),
                Trigger::Hover | Trigger::Hold | Trigger::Manual => false,
            };
            (fired, fired && by_pointer, on_opener && pass.primary_down)
        });
        Opener {
            id: anchor.id,
            rect: anchor.rect,
            layer: anchor.layer_id,
            trigger,
            enabled: anchor.enabled(),
            anchor: None,
            others: Vec::new(),
            fired,
            by_pointer,
            pressed,
        }
    }

    /// This opener and the others, first first.
    fn all(&self) -> impl Iterator<Item = &Opener> {
        std::iter::once(self).chain(&self.others)
    }

    /// The opener the layer's open slot says opened it, else the first.
    fn active(&self, ctx: &Context) -> (&Opener, bool) {
        let by = with_popups(ctx, |popups| popups.opened_by.get(&self.id).copied());
        let found = by.and_then(|(id, trigger, pointer)| {
            self.all().find(|opener| opener.id == id && opener.trigger == trigger).map(|opener| (opener, pointer))
        });
        match found {
            Some(active) => active,
            None => (self, false),
        }
    }
}

/// The anchor a layer opened by `opener` hangs from: the opener's own, else the layer's `Pointer` or `Viewport`
/// place, else the pointer for a context request a secondary release made, else the opener's rect.
fn opener_anchor(opener: &Opener, by_pointer: bool, place: &Place) -> Anchor {
    if let Some(anchor) = &opener.anchor {
        return anchor.clone();
    }
    match place {
        Place::Anchored { anchor: anchor @ (Anchor::Pointer | Anchor::Viewport(_)), .. } => anchor.clone(),
        _ if opener.trigger == Trigger::Context && by_pointer => Anchor::Pointer,
        _ => Anchor::Response,
    }
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
    /// What the anchor tells assistive technology it opens.
    pub popup: PopupKind,
    /// The size an `OverlayFit::Resizable` layer has before the user resizes it; `None` sizes it from its content.
    pub size: Option<Vec2>,
    /// The least an `OverlayFit::Resizable` layer may be resized to.
    pub min_size: Option<Vec2>,
    /// The most a movable or resizable layer may be resized to.
    pub max_size: Option<Vec2>,
    /// The rect the caller places the layer at, in the layer's parent space. The container reports the user's moves
    /// and resizes as `PlacementChange`s and keeps no placement of its own.
    pub placed: Option<Rect>,
    /// The position the caller places the layer's top left at, in the layer's parent space; its size comes from its
    /// content and size limits. Reported and owned like `placed`.
    pub placed_at: Option<Pos2>,
    /// The rect a move or resize keeps the layer inside.
    pub bounds: Option<Rect>,
    /// The layer's own opacity, below the open and close motion's.
    pub opacity: f32,
}

/// The share of the viewport a dropdown list may take. Half the width leaves room for the list to hang from the end
/// edge of its anchor in a right-to-left layout.
const DROPDOWN_SHARE: ViewportShare = ViewportShare::new(0.5, 0.4);
/// The share of the viewport a dialog or modal may take.
const DIALOG_SHARE: ViewportShare = ViewportShare::new(0.8, 0.9);
/// The share of the viewport a tooltip or a toast card may take before its text wraps.
const NOTE_SHARE: ViewportShare = ViewportShare::new(0.4, 1.0);

impl Overlay {
    fn base(rank: Rank, place: Place, fit: OverlayFit, keys: OwnerClass, focus: FocusPolicy) -> Overlay {
        Overlay {
            place,
            fit,
            min_width: MinWidth::None,
            movable: false,
            resize: Resize::NONE,
            rank,
            keys,
            focus,
            dismiss: Dismiss::new(),
            motion: spec::OPEN_FADE,
            opener: None,
            popup: PopupKind::Dialog,
            size: None,
            min_size: None,
            max_size: None,
            placed: None,
            placed_at: None,
            bounds: None,
            opacity: 1.0,
        }
    }

    fn below() -> Place {
        Place::anchored(Anchor::Response, Side::Below, Align::Start)
    }

    /// A popover: a popup anchored below, taking focus in. It closes when the focus moves out of it.
    pub fn popover() -> Overlay {
        let mut overlay = Overlay::base(Rank::Popup, Overlay::below(), OverlayFit::Content, OwnerClass::Popup, FocusPolicy::POPUP);
        overlay.dismiss = Dismiss::new().focus_left(true);
        overlay
    }

    /// A dropdown list: from its content, at least as wide as its anchor and capped, focus stays on the anchor. It
    /// closes when a row is activated.
    pub fn dropdown() -> Overlay {
        let focus = FocusPolicy::POPUP.trap(false).restore(false).initial(Initial::KeepAnchor);
        let mut overlay = Overlay::base(Rank::Popup, Overlay::below(), OverlayFit::ContentCapped(DROPDOWN_SHARE), OwnerClass::PassivePopup, focus);
        overlay.dismiss = Dismiss::new().on_activate(true);
        overlay.min_width = MinWidth::Anchor;
        overlay.popup = PopupKind::Listbox;
        overlay
    }

    /// A menu: a popup the arrow keys move through. A press outside closes it and goes no further.
    pub fn menu() -> Overlay {
        let mut overlay = Overlay::base(Rank::Popup, Overlay::below(), OverlayFit::Content, OwnerClass::Menu, FocusPolicy::MENU);
        overlay.dismiss = Dismiss::new().outside(Outside::Absorb);
        overlay.popup = PopupKind::Menu;
        overlay
    }

    /// A submenu: beside its parent entry.
    pub fn submenu() -> Overlay {
        Overlay::menu().place(Place::anchored(Anchor::Response, Side::End, Align::Start))
    }

    /// A tooltip: below the control, no focus, never takes input.
    pub fn tooltip() -> Overlay {
        let mut overlay =
            Overlay::base(Rank::Tooltip, Overlay::below(), OverlayFit::ContentCapped(NOTE_SHARE), OwnerClass::PassivePopup, FocusPolicy::NONE);
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
        overlay.resize = Resize::GRIPS;
        overlay.dismiss = Dismiss::new().outside(Outside::Keep);
        overlay
    }

    /// A modal: centred, content capped.
    pub fn modal() -> Overlay {
        let mut overlay = Overlay::dialog();
        overlay.movable = false;
        overlay.resize = Resize::NONE;
        overlay
    }

    /// A toast stack: the viewport's bottom end corner.
    pub fn toast() -> Overlay {
        let place = Place::anchored(Anchor::Viewport(Align2::RIGHT_BOTTOM), Side::Above, Align::End);
        let mut overlay = Overlay::base(Rank::Toast, place, OverlayFit::ContentCapped(NOTE_SHARE), OwnerClass::Widget, FocusPolicy::NONE);
        overlay.dismiss = Dismiss::new().outside(Outside::Keep);
        overlay
    }

    /// A floating panel or document window: a window-rank layer that is a pane of the app. F6 reaches it and Escape
    /// does not close it.
    pub fn floating() -> Overlay {
        let mut overlay =
            Overlay::base(Rank::Window, Place::Float(FloatPlace::centred()), OverlayFit::Content, OwnerClass::Widget, FocusPolicy::PANE);
        overlay.dismiss = Dismiss::new().outside(Outside::Keep);
        overlay
    }

    /// Whether edge grips resize the layer.
    pub fn resizable(mut self, on: bool) -> Overlay {
        self.resize = if on { Resize::GRIPS } else { Resize::NONE };
        self
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

    /// Hangs the layer from `anchor` on the side and alignment its preset gave. Without an opener, a point is in the
    /// parent's layer space; a floating layer keeps its place.
    pub fn anchor(mut self, anchor: Anchor) -> Overlay {
        if let Place::Anchored { anchor: hung, .. } = &mut self.place {
            *hung = anchor;
        }
        self
    }

    /// Sets the size a resizable layer has before the user resizes it.
    pub fn size(mut self, size: Vec2) -> Overlay {
        self.size = Some(size);
        self
    }

    /// Sets the least a resizable layer may be resized to.
    pub fn min_size(mut self, min_size: Vec2) -> Overlay {
        self.min_size = Some(min_size);
        self
    }

    /// Sets the least and the most a resizable layer may be resized to.
    pub fn size_limits(mut self, min: Vec2, max: Vec2) -> Overlay {
        self.min_size = Some(min);
        self.max_size = Some(max);
        self
    }

    /// Places the layer at `rect` in its parent's space. The container reports the moves and resizes the user makes
    /// as `PlacementChange`s to `Container::placement_changed`; the caller decides the next rect. A placed rect wins
    /// over a saved placement.
    pub fn placed(mut self, rect: Rect) -> Overlay {
        self.placed = Some(rect);
        self
    }

    /// Places the layer's top left at `pos` in its parent's space and sizes it from its content and size limits, so a
    /// layer whose content grows grows with it. Moves and resizes are reported and owned as for
    /// [`placed`](Self::placed); `placed` wins when both are set.
    pub fn placed_at(mut self, pos: Pos2) -> Overlay {
        self.placed_at = Some(pos);
        self
    }

    /// Sets the rect a move or resize keeps the layer inside.
    pub fn bounds(mut self, bounds: Rect) -> Overlay {
        self.bounds = Some(bounds);
        self
    }

    /// Sets the layer's opacity, from 0 (invisible) to 1.
    pub fn opacity(mut self, opacity: f32) -> Overlay {
        self.opacity = opacity;
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

    /// Records an opener. An anchored layer hangs from the rect of the opener that opened it, on the side and
    /// alignment its preset gave; a floating layer keeps its place, and the opener serves focus return and the
    /// trigger. Called again, it adds an opener: the layer opens from any of them, and the opener that opened it
    /// decides the anchor.
    pub fn opened_by(mut self, anchor: &Response, trigger: Trigger) -> Overlay {
        let opener = Opener::new(anchor, trigger);
        match &mut self.opener {
            Some(first) => first.others.push(opener),
            None => self.opener = Some(opener),
        }
        self.anchor(Anchor::Rect(anchor.rect))
    }

    /// Sets where the layer hangs when the opener named last opened it. A rect or point is in that opener's layer
    /// space.
    pub fn opener_anchor(mut self, anchor: Anchor) -> Overlay {
        debug_assert!(self.opener.is_some(), "`opener_anchor` follows `opened_by`");
        if let Some(first) = &mut self.opener {
            match first.others.last_mut() {
                Some(last) => last.anchor = Some(anchor),
                None => first.anchor = Some(anchor),
            }
        }
        self
    }
}

/// What kind of gesture a `PlacementChange` reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PlacementKind {
    /// The move band was dragged.
    Move,
    /// A grip was dragged: the edge or corner it sits on.
    Size(ResizeEdge),
    /// The keyboard Move or Size mode (`window.Move`, `window.Size`) changed the rect.
    Keyboard,
}

/// A move or resize of a layer the user made, reported to `Container::placement_changed`.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PlacementChange {
    /// The rect after the change, in the layer's parent space, kept inside the overlay's `bounds` and `size_limits`.
    /// On `Cancel` the rect from before the gesture.
    pub rect: Rect,
    /// Where the gesture stands.
    pub phase: GesturePhase,
    /// The pointer position of a pointer gesture.
    pub pointer: Option<Pos2>,
    /// What changed the rect.
    pub kind: PlacementKind,
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

    /// The popup of the repeat `index` of this name: a component with one popup per part or per entry gives each its
    /// own. Index 0 is the handle `cx.popup(name)` returns. `open`, `close`, `is_open` and `trigger` register the
    /// popup for the pass, so a handle derived in `update` needs no separate registration.
    #[must_use]
    pub fn at(self, index: usize) -> PopupHandle {
        match index {
            0 => self,
            _ => PopupHandle { id: sub(self.id, ("at", index)), name: self.name },
        }
    }

    fn registered<C: Component>(self, cx: &Cx<'_, C>) {
        register(&cx.core.ctx, cx.core.id, self.id);
    }

    /// The popup's id: its open state's and its entry in the layer stack.
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
    /// Every popup an owner showed or kept closed this pass, for the anchor's access node.
    anchors: Vec<(Id, PopupShown)>,
    /// The opener that opened each layer with openers, by the layer's first opener: its id and trigger (two
    /// openers on one response differ by trigger) and whether a pointer release opened it.
    opened_by: HashMap<Id, (Id, Trigger, bool)>,
}

fn with_popups<R>(ctx: &Context, f: impl FnOnce(&mut Popups) -> R) -> R {
    Service::<Popups>::run(ctx, f)
}

/// Step 8b, before the text part runs: the popups `owner` registers are registered again.
pub(crate) fn begin_owner(ctx: &Context, owner: Id) {
    with_popups(ctx, |popups| {
        popups.registered.retain(|(registered, _)| *registered != owner);
        popups.shown.retain(|(shown, _)| *shown != owner);
        popups.anchors.retain(|(anchor, _)| *anchor != owner);
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

/// The popups of `owner` in this pass, open or closed, for the anchor's access node.
pub(crate) fn popups_shown(ctx: &Context, owner: Id) -> Vec<PopupShown> {
    with_popups(ctx, |popups| popups.anchors.iter().filter(|(anchor, _)| *anchor == owner).map(|(_, popup)| *popup).collect())
}

/// Whether a popup of `owner` was shown in this pass; step 12 reads it as the `OPEN` flag.
pub(crate) fn popup_shown(ctx: &Context, owner: Id) -> bool {
    with_popups(ctx, |popups| popups.shown.iter().any(|(shown, _)| *shown == owner))
}

/// Step 10: the open state of every popup `owner` registered, with the closes `request_close` queued applied.
pub(crate) fn open_map(stage: &mut MemoryStage, owner: Id) -> HashMap<Id, bool> {
    let ctx = stage.ctx().clone();
    let popups: Vec<Id> =
        with_popups(&ctx, |popups| popups.registered.iter().filter(|(registered, _)| *registered == owner).map(|(_, popup)| *popup).collect());
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
    let popups: Vec<Id> =
        with_popups(&ctx, |popups| popups.registered.iter().filter(|(registered, _)| *registered == owner).map(|(_, popup)| *popup).collect());
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

/// What a part declared with `PartBuilder::toggles_at`: the popup it opens and whether it acts on press.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ToggleDecl {
    pub popup: &'static str,
    pub index: usize,
    /// The toggle acts on the pass the press began, and a release over the opened popup picks the row under it.
    pub on_press: bool,
}

impl ToggleDecl {
    /// A toggle of the popup `popup` repeat `index`, acting on release.
    pub(crate) const fn new(popup: &'static str, index: usize) -> Self {
        ToggleDecl { popup, index, on_press: false }
    }
}

impl super::cx::Toggle {
    fn handle(&self, owner: Id) -> PopupHandle {
        PopupHandle::new(owner, self.decl.popup).at(self.decl.index)
    }
}

impl PartBuilder<'_> {
    /// The part opens and closes the repeat `index` of the component's popup `popup`: `toggles` for a component with
    /// one popup per part. Open one popup at a time: opening the second closes the first.
    pub fn toggles_at(self, popup: &'static str, index: usize) -> Self {
        self.declared.toggles = Some(ToggleDecl::new(popup, index));
        self
    }

    /// The part's toggle acts on the pass the press began, not on release, as a menu bar's titles do: a release over
    /// a row of the popup it opened picks that row (press, drag, release), and a click on the part leaves the popup
    /// open. Follows `toggles` or `toggles_at`.
    pub fn on_press(self) -> Self {
        let toggle = self.declared.toggles.as_mut();
        debug_assert!(toggle.is_some(), "`on_press` follows `toggles` or `toggles_at`");
        if let Some(toggle) = toggle {
            toggle.on_press = true;
        }
        self
    }
}

/// Step 9, before `update`: what the component's parts declared with `PartBuilder::toggles`. A click on the part
/// or Enter or Space on it when focused toggles its popup (the pass its press began, for a part that declared `on_press`), AccessKit Expand and Collapse
/// on its sub-node open and close it, Alt+Down opens the popup of the focused toggling part (else the first one that
/// is closed), and Alt+Up closes the open ones. Opening one popup of the component closes the others. Each toggle
/// keeps the popup's open state for the part's `expanded`. A click a toggling part took is not left as the root's
/// activation.
pub(crate) fn drive_toggles<C: Component>(cx: &mut Cx<'_, C>) {
    if cx.core.toggles.is_empty() {
        return;
    }
    let mut toggles = std::mem::take(&mut cx.core.toggles);
    let owner = cx.core.id;
    let active = focus::active_part(&cx.core.ctx, owner);
    let mut handled = false;
    let mut opened = None;
    for (at, toggle) in toggles.iter().enumerate() {
        let popup = toggle.handle(owner);
        let part = toggle.part;
        // The root takes Expand and Collapse for a sole toggling part that has no node of its own.
        let sole = toggles.len() == 1;
        let named = |request: &craft_behavior::AccessRequest| {
            (request.target == Some(part) || (sole && request.target.is_none()))
                && matches!(request.action, craft_behavior::AccessAction::Expand | craft_behavior::AccessAction::Collapse)
        };
        while let Some(request) = cx.core.intents.take_access_where(named) {
            if request.action == craft_behavior::AccessAction::Expand {
                popup.open(cx);
                opened = Some(at);
            } else {
                popup.close(cx);
            }
            handled = true;
        }
        let phase = cx.core.results.get(&(part.key, part.index)).map(|result| result.press.phase);
        let on_press = toggle.decl.on_press;
        let by_pointer = match phase {
            Some(phase) if on_press => cx.core.memory.shown::<super::SeenPress>(sub(popup.id, "press")).began(&phase),
            Some(phase) => phase.is_click(),
            None => false,
        };
        let clicked = |request: &craft_behavior::AccessRequest| request.target == Some(part) && request.action == craft_behavior::AccessAction::Click;
        let by_access = cx.core.intents.take_access_where(clicked).is_some();
        let toggled = by_pointer || by_access || key_activated(cx, active, part);
        if toggled {
            popup.registered(cx);
            if !popup.is_open(cx) {
                opened = Some(at);
            }
            queue(&cx.core.ctx, popup.id, OpenOp::Toggle);
            handled = true;
        }
        // The toggling part took the click, or took its press: the root's pointer activation is not a second toggle.
        if (toggled || (on_press && phase.is_some_and(|phase| phase.is_click()))) && cx.core.activation.is_some_and(|activation| !activation.by_key) {
            cx.core.activation = None;
        }
    }
    let mut open = Vec::with_capacity(toggles.len());
    for toggle in &toggles {
        open.push(toggle.handle(owner).is_open(cx));
    }
    if let Some(keeper) = opened {
        // Opening one popup closes the others.
        for (at, toggle) in toggles.iter().enumerate() {
            if at != keeper && open[at] {
                toggle.handle(owner).close(cx);
                open[at] = false;
            }
        }
    }
    if open.contains(&true) {
        if cx.core.intents.take(Intent::Close) {
            for (toggle, _) in toggles.iter().zip(&open).filter(|(_, open)| **open) {
                toggle.handle(owner).close(cx);
            }
        }
        // An open popup keeps Alt+Down: nothing opens beside it.
        cx.core.intents.take(Intent::Open);
    } else if cx.core.intents.take(Intent::Open) && !handled {
        let target = active.and_then(|part| toggles.iter().find(|toggle| toggle.part == part)).or(toggles.first());
        if let Some(toggle) = target {
            toggle.handle(owner).open(cx);
        }
    }
    for toggle in &mut toggles {
        toggle.open = toggle.handle(owner).is_open(cx);
    }
    cx.core.toggles = toggles;
}

/// Whether a key (Enter or Space) activated `part`, the focused part of the component.
fn key_activated<C: Component>(cx: &Cx<'_, C>, active: Option<PartRef>, part: PartRef) -> bool {
    active == Some(part) && cx.core.activation.is_some_and(|activation| activation.by_key)
}

impl PopupHandle {
    /// Toggles or opens the popup from `trigger` at step 9, as the pass's pending open change.
    pub fn trigger<C: Component>(self, trigger: Trigger, cx: &mut Cx<'_, C>) {
        debug_assert!(C::OVERLAYS, "`{}` hosts a popup but sets `OVERLAYS = false`", std::any::type_name::<C>());
        self.registered(cx);
        let core = &mut cx.core;
        let ctx = core.ctx.clone();
        let resting = core.response.hovered() || core.focus.visible;
        let toggled = match trigger {
            Trigger::Click => core.activation.is_some(),
            Trigger::Press => core.memory.shown::<super::SeenPress>(sub(self.id, "press")).began(&core.press.phase),
            Trigger::Context => core.intents.take(Intent::ContextMenu),
            Trigger::Hover => self.dwell(&ctx, core.memory, "dwell", resting, super::tooltip::DELAY),
            Trigger::Hold => self.dwell(&ctx, core.memory, "dwell", core.press.phase.is_press(), HOLD),
            Trigger::Manual => false,
        };
        if toggled {
            queue(&ctx, self.id, if matches!(trigger, Trigger::Click | Trigger::Press) { OpenOp::Toggle } else { OpenOp::Open });
        } else if trigger != Trigger::Manual && core.intents.take(Intent::Open) {
            queue(&ctx, self.id, OpenOp::Open);
        } else if trigger == Trigger::Hover && open_now(core.memory, self.id) {
            // The pointer may cross the gap between the opener and the popup: the popup closes after a grace period
            // with the pointer on neither.
            let over = resting || pointer_in_popup(&ctx, self.id);
            if self.dwell(&ctx, core.memory, "leave", !over, super::tooltip::GRACE) {
                queue(&ctx, self.id, OpenOp::Close);
            }
        }
    }

    fn dwell(self, ctx: &Context, stage: &mut MemoryStage, name: &'static str, held: bool, delay: f64) -> bool {
        dwell(ctx, stage, self.id, name, held, delay)
    }

    /// Queues an open that step 15 writes. For `Trigger::Manual` this and the binding are the only ways it opens.
    pub fn open<C: Component>(self, cx: &mut Cx<'_, C>) {
        debug_assert!(C::OVERLAYS, "`{}` hosts a popup but sets `OVERLAYS = false`", std::any::type_name::<C>());
        self.registered(cx);
        queue(&cx.core.ctx, self.id, OpenOp::Open);
    }

    /// Queues a close; step 15 writes the slot, so this pass still shows the popup open.
    pub fn close<C: Component>(self, cx: &mut Cx<'_, C>) {
        debug_assert!(C::OVERLAYS, "`{}` hosts a popup but sets `OVERLAYS = false`", std::any::type_name::<C>());
        self.registered(cx);
        queue(&cx.core.ctx, self.id, OpenOp::Close);
    }

    /// Whether a toggle, `open` or `close` queued in this pass leaves the popup's open state different from the
    /// state the pass began with. Valid from step 9.
    pub fn changed<C: Component>(self, cx: &mut Cx<'_, C>) -> bool {
        self.registered(cx);
        let began = cx.core.memory.shown::<OpenSlot>(self.id).0;
        open_now(cx.core.memory, self.id) != began
    }

    /// Whether the popup is open in this pass: valid from step 8, with the changes queued so far applied.
    pub fn is_open<C: Component>(self, cx: &mut Cx<'_, C>) -> bool {
        debug_assert!(C::OVERLAYS, "`{}` hosts a popup but sets `OVERLAYS = false`", std::any::type_name::<C>());
        self.registered(cx);
        open_now(cx.core.memory, self.id)
    }
}

/// Whether the pointer is over the shown layer `popup`: a hover popup stays while the pointer is on it.
fn pointer_in_popup(ctx: &Context, popup: Id) -> bool {
    let shown = overlay::entry(ctx, popup);
    shown.zip(areas::pointer_pos(ctx)).is_some_and(|(entry, at)| entry.rect.contains(at))
}

#[derive(Clone, Copy, Default)]
struct Dwell(Option<f64>);

/// Whether `held` has lasted `delay` seconds, timed in the slot `name` of `id`; asks for a pass at the deadline.
fn dwell(ctx: &Context, stage: &mut MemoryStage, id: Id, name: &'static str, held: bool, delay: f64) -> bool {
    let now = frame::clock(ctx);
    let since = stage.shown::<Dwell>(sub(id, name));
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
    /// The rects of the owner's toggling parts, in the owner's layer space.
    pub(crate) parts: Vec<(PartRef, Rect)>,
}

/// The component whose overlays are shown: its id, its rect and the rects of its parts that toggle a popup.
pub(crate) struct OverlayOwner {
    pub(crate) id: Id,
    pub(crate) anchor: Rect,
    pub(crate) parts: Vec<(PartRef, Rect)>,
}

impl<'a> OverlayUi<'a> {
    pub(crate) fn new(ctx: &'a Context, owner: OverlayOwner, ui: &Ui, resolved: &'a Resolved<'a>, env: Env, open: HashMap<Id, bool>) -> Self {
        let to_screen = areas::layer_to_screen(ctx, ui.layer_id());
        let OverlayOwner { id, anchor, parts } = owner;
        OverlayUi { ctx, owner: id, anchor, to_screen, resolved, env, open, closed: Vec::new(), parts }
    }

    /// The anchor at the rect of the toggling part `part` of the owner. The layer's placement mirrors the side and
    /// alignment of an anchor in a right-to-left layout, so the same call serves both directions.
    pub fn anchor_part(&self, part: impl Into<PartRef>) -> Anchor {
        let part = part.into();
        let rect = self.parts.iter().find(|(declared, _)| *declared == part).map(|(_, rect)| *rect);
        debug_assert!(rect.is_some(), "`anchor_part` names a part that declared `toggles`");
        Anchor::Rect(rect.unwrap_or(self.anchor))
    }

    /// Shows the popup while its handle is open; the content is a body closure. `None` while closed.
    pub fn show<R>(&mut self, popup: PopupHandle, overlay: Overlay, mut body: impl FnMut(&mut Ui, &mut BodyCx<'_>) -> R) -> Option<R> {
        let Some(open) = self.open.get(&popup.id).copied() else {
            debug_assert!(false, "popup {:?} is shown but was not registered with `cx.popup` this pass", popup.name);
            return None;
        };
        let ctx = self.ctx.clone();
        with_popups(&ctx, |popups| popups.anchors.push((self.owner, PopupShown { id: popup.id, kind: overlay.popup, open })));
        let owner = overlay.opener.as_ref().map_or(self.owner, |opener| opener.id);
        let presence = motion::layer_presence(&ctx, popup.id, owner, self.env.motion, open, overlay.motion);
        if !presence.visible {
            forget_layer(&ctx, popup.id, owner);
            return None;
        }
        let anchor = resolve_anchor(&ctx, &overlay, self.anchor, self.to_screen);
        let layer = Layer { id: popup.id, owner, overlay: &overlay, anchor, env: &self.env, exiting: !open, fade: presence.t, size: None };
        let resolved = self.resolved;
        let mut result = None;
        let mut requested = false;
        let out = host(&ctx, &layer, &mut |ui| {
            let mut stage = if ui.is_sizing_pass() { MemoryStage::open_peek(&ctx, popup.id) } else { MemoryStage::open_owned(&ctx, popup.id, owner) };
            let rect = ui.available_rect_before_wrap();
            let mut shown_open = open;
            let env = Env::of(ui);
            let mut bcx = BodyCx::new(popup.id, env.view(), resolved, rect, &mut stage, &mut shown_open, None);
            result = Some(body(ui, &mut bcx));
            requested = bcx.close_requested().is_some();
            stage.write_back();
        });
        if overlay.opener.is_none() && !self.parts.is_empty() {
            overlay::note_peers(&ctx, popup.id, self.parts.iter().map(|(_, rect)| self.to_screen * *rect).collect());
        }
        if open {
            with_popups(&ctx, |popups| popups.shown.push((self.owner, popup.id)));
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
    /// The id of the layer's open state: the popup's or the container's. Its egui layer id derives from it.
    pub id: Id,
    /// The component whose memory stage holds the layer's bookkeeping: the opener or the container.
    pub owner: Id,
    pub overlay: &'a Overlay,
    /// The anchor in screen space.
    pub anchor: Rect,
    /// The opener's env; the layer's body starts from its capture (`Env::capture`).
    pub env: &'a Env,
    /// The layer is fading out.
    pub exiting: bool,
    /// The opacity the open and close motion gives.
    pub fade: f32,
    /// The size the layer's content takes in this pass, when its container measured it before the layer shows;
    /// a layer without one is placed by the size it took last pass.
    pub size: Option<egui::Vec2>,
}

/// What the host reports.
pub(crate) struct LayerOut {
    /// Why the layer asks to close, decided after its body ran.
    pub dismissed: Option<CloseReason>,
}

/// What a layer remembers while it is shown, in its owner's memory stage: gone when the layer hides.
#[derive(Clone, Copy, Default)]
struct LayerMemory {
    /// The pointer position when the layer first appeared, for `Anchor::Pointer`.
    pointer_at: Option<Pos2>,
    /// The stop that was focused when the layer first appeared.
    opener_stop: Option<Option<focus::StopKey>>,
    /// The pass time the layer first appeared, for `Dismiss.timeout`.
    since: Option<f64>,
    /// The focus was in the layer last time it was read: `Dismiss.focus_left` fires on a move from inside to outside.
    focus_inside: bool,
    /// The layers opened from inside this one have been asked to close.
    family_closed: bool,
    /// The least width the layer's body took in the last pass: a content-sized collection in the layer is at least
    /// this wide, so its rows fill the layer.
    min_width: f32,
}

/// The least width the body of the layer `id` takes, as of the last pass; 0 before the layer's first pass.
pub(crate) fn floor_width(ctx: &Context, id: Id) -> f32 {
    MemoryStage::peek::<LayerMemory>(ctx, memory_slot(id)).min_width
}

fn memory_slot(id: Id) -> Id {
    sub(id, "layer_memory")
}

fn recall(layer: &Layer<'_>, ctx: &Context) -> LayerMemory {
    MemoryStage::peek::<LayerMemory>(ctx, memory_slot(layer.id))
}

fn remember(layer: &Layer<'_>, ctx: &Context, memory: LayerMemory) {
    let mut stage = MemoryStage::open_owned(ctx, memory_slot(layer.id), layer.owner);
    *stage.shown::<LayerMemory>(memory_slot(layer.id)) = memory;
    stage.write_back();
}

/// The layer `id` is hidden: it forgets what it remembered, so its next appearance starts fresh. Every host caller
/// calls it when the layer's presence is hidden.
pub(crate) fn forget_layer(ctx: &Context, id: Id, owner: Id) {
    let mut stage = MemoryStage::open_owned(ctx, memory_slot(id), owner);
    let taken = stage.take::<LayerMemory>(memory_slot(id));
    stage.write_back();
    if taken.is_some_and(|memory| !memory.family_closed) {
        overlay::close_descendants(ctx, id);
    }
}

/// Asks the layers opened from inside `layer` to close once the layer closes or starts to fade out, in the pass that
/// decided it.
fn close_family(ctx: &Context, layer: &Layer<'_>, closing: bool, memory: &mut LayerMemory) {
    if closing && !memory.family_closed {
        memory.family_closed = true;
        overlay::close_descendants(ctx, layer.id);
    }
}

/// The egui layer id of the layer `id`.
fn area_id(id: Id) -> Id {
    sub(id, "area")
}

/// The anchor of `overlay` in screen space: the opener's rect through the opener's own layer, else `own` (the
/// owner's rect) or the given rect or point, in the space `own_to_screen` maps.
pub(crate) fn resolve_anchor(ctx: &Context, overlay: &Overlay, own: Rect, own_to_screen: egui::emath::TSTransform) -> Rect {
    if let Some(first) = &overlay.opener {
        let (opener, by_pointer) = first.active(ctx);
        let to_screen = areas::layer_to_screen(ctx, opener.layer);
        return match opener_anchor(opener, by_pointer, &overlay.place) {
            Anchor::Rect(rect) => to_screen * rect,
            Anchor::Point(at) => to_screen * Rect::from_min_max(at, at),
            _ => to_screen * opener.rect,
        };
    }
    match &overlay.place {
        Place::Anchored { anchor: Anchor::Rect(rect), .. } => own_to_screen * *rect,
        Place::Anchored { anchor: Anchor::Point(at), .. } => own_to_screen * Rect::from_min_max(*at, *at),
        _ => own_to_screen * own,
    }
}

/// The overlay with the place's anchor that of the opener that opened it, for the anchors the host reads from the
/// place (`Pointer`, `Viewport`).
fn opened_overlay(ctx: &Context, overlay: &Overlay) -> Option<Overlay> {
    let first = overlay.opener.as_ref()?;
    let (opener, by_pointer) = first.active(ctx);
    let mut resolved = overlay.clone();
    if let Place::Anchored { anchor, .. } = &mut resolved.place {
        *anchor = opener_anchor(opener, by_pointer, &overlay.place);
    }
    Some(resolved)
}

fn anchor_rect(ctx: &Context, layer: &Layer<'_>, bounds: Rect, memory: &mut LayerMemory) -> Rect {
    let point = |at: Pos2| Rect::from_min_max(at, at);
    match &layer.overlay.place {
        Place::Anchored { anchor: Anchor::Viewport(at), .. } => {
            let inset = layer.env.tokens.space.pad.panel;
            point(at.pos_in_rect(&bounds.shrink(inset)))
        }
        Place::Anchored { anchor: Anchor::Pointer, .. } => {
            point(*memory.pointer_at.get_or_insert_with(|| areas::pointer_pos(ctx).unwrap_or(bounds.center())))
        }
        _ => layer.anchor,
    }
}

/// Where the layer goes and the room its body has there.
struct Placed {
    at: Pos2,
    side: Side,
    /// The most the body may take from `at`: what the viewport leaves, within the fit's cap.
    room: egui::Vec2,
}

/// The pure placement of the anchored forms; the viewport centre or the given position for floats, kept inside the
/// safe rect.
fn placement(ctx: &Context, layer: &Layer<'_>, size: egui::Vec2, bounds: Rect, anchor: Rect, cap: egui::Vec2) -> Placed {
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
            // The room is what the side the layer took leaves for a body as large as the cap.
            let room = craft_behavior::overlay::place(&request.kept(Some(placed.side)).size(cap)).rect.size();
            Placed { at: placed.rect.min, side: placed.side, room }
        }
        Place::Float(float) => {
            let min = float.pos.unwrap_or_else(|| bounds.center() - size / 2.0);
            let max_min = Pos2::new((bounds.right() - size.x).max(bounds.left()), (bounds.bottom() - size.y).max(bounds.top()));
            let at = Pos2::new(min.x.clamp(bounds.left(), max_min.x), min.y.clamp(bounds.top(), max_min.y));
            Placed { at, side: Side::Below, room: (bounds.max - at).max(egui::Vec2::ZERO).min(cap) }
        }
    }
}

/// The overlay host: shows `body` on an egui layer for `layer` and decides its dismissal.
pub(crate) fn host(ctx: &Context, layer: &Layer<'_>, body: &mut dyn FnMut(&mut Ui)) -> LayerOut {
    let opened = opened_overlay(ctx, layer.overlay);
    let layer = &Layer { overlay: opened.as_ref().unwrap_or(layer.overlay), ..*layer };
    let overlay = layer.overlay;
    let bounds = areas::safe_rect(ctx);
    let mut memory = recall(layer, ctx);
    let anchor = anchor_rect(ctx, layer, bounds, &mut memory);
    let area = area_id(layer.id);
    // A container's layer is placed by the size its container measured for this pass, any other by the size it took last
    // pass. The layout memory entry under `size_id` marks that read, so a change of the size asks for
    // the discard pass that places the layer by its real size, under the rule every measured size follows: no ask when
    // the size also changed in the previous frame. A popup is placed by its anchor and opens from its opener's queued
    // request, which a discard pass would replay.
    let size_id = sub(layer.id, "layer_size");
    let sized = layer.owner == layer.id;
    if sized {
        let _ = layout_memory::read(ctx, size_id);
    }
    let size = layer.size.or_else(|| areas::layer_rect(ctx, area).map(|rect| rect.size())).unwrap_or(egui::Vec2::ZERO);
    let cap = match overlay.fit {
        OverlayFit::ContentCapped(share) => {
            egui::Vec2::new((bounds.width() * share.x).max(overlay.min_width.points(anchor.width())), bounds.height() * share.y)
        }
        _ => bounds.size(),
    };
    let placed = placement(ctx, layer, size, bounds, anchor, cap);
    let tier = overlay.rank.tier();
    if sized && areas::layer_rect(ctx, area).is_none() {
        areas::seed_area(ctx, areas::layer_of(tier, area), placed.at, placed.room);
    }
    // A modal's scrim; it takes the presses below the modal while the modal is open.
    let cover_id = sub(layer.id, "cover");
    let cover = match overlay.rank {
        Rank::Modal { backdrop } if backdrop != Backdrop::None => {
            let hole = match backdrop {
                Backdrop::ScrimWithHole(hole) => Some(hole),
                _ => None,
            };
            let scrim = crate::style::scrim_color(&layer.env.tokens).gamma_multiply(layer.fade);
            areas::show_cover_layer(ctx, cover_id, tier, bounds, scrim, hole, !layer.exiting);
            Some(areas::layer_of(tier, cover_id))
        }
        _ => None,
    };
    let parent = overlay::drawing_parent(ctx);
    let scope = (overlay.focus != FocusPolicy::NONE).then_some(overlay.focus);
    let min_width = overlay.min_width.points(anchor.width());
    let spec = LayerSpec { id: area, tier, at: placed.at, interactable: overlay.rank != Rank::Tooltip && !layer.exiting };
    // The opener of the layer's scope is the stop focused when the layer first appears: egui's modal gate takes focus
    // from the widgets below a modal layer by the first pass that shows the layer.
    let opener = *memory.opener_stop.get_or_insert_with(|| focus::focused_stop(ctx));
    overlay::begin_layer(ctx, layer.id, layer.exiting);
    let shown = areas::show_layer(ctx, &spec, |ui| {
        areas::fade(ui, layer.fade * overlay.opacity);
        // The layer is not inside its opener's arena, roving group, scroll or row: its content starts from the
        // opener's inherited env with the layer-local fields fresh.
        let installed = Env::install_for(ui, layer.env.capture().env());
        let real = !ui.is_sizing_pass();
        if real && (layer.exiting || matches!(overlay.rank, Rank::Popup | Rank::Modal { .. })) {
            access::report_layer(ctx, layer.id, ui.unique_id(), ui.max_rect(), overlay.rank.is_modal(), layer.exiting);
        }
        if let (Some(policy), true) = (scope, real) {
            focus::push_scope_from(ctx, layer.id, policy, None, opener, layer.exiting);
        }
        areas::bound(ui, placed.room, min_width.min(placed.room.x));
        if let (OverlayFit::Resizable, Some(least)) = (overlay.fit, overlay.min_size) {
            ui.set_min_size(least.min(placed.room));
        }
        body(ui);
        if scope.is_some() && real {
            focus::pop_scope(ctx);
        }
        Env::restore(ctx, installed);
    });
    overlay::end_layer(ctx);
    if sized {
        layout_memory::record(ctx, size_id, shown.rect.size());
    }
    let mut entry = OpenEntry::new(layer.id, overlay.rank, overlay.keys, shown.rect, shown.layer);
    entry.parent = parent;
    entry.anchor = matches!(overlay.place, Place::Anchored { .. }).then_some(anchor);
    entry.side = placed.side;
    entry.exiting = layer.exiting;
    entry.cover = cover;
    entry.scoped = scope.is_some();
    entry.absorbs = overlay.dismiss.outside == Outside::Absorb;
    overlay::record(ctx, entry);
    memory.min_width = min_width.min(placed.room.x);
    let dismissed = if layer.exiting { None } else { dismissal(ctx, layer, shown.rect, anchor, scope, &mut memory) };
    close_family(ctx, layer, layer.exiting || dismissed.is_some(), &mut memory);
    remember(layer, ctx, memory);
    LayerOut { dismissed }
}

/// The layer-stack side of an OS-window mount: records the window as an entry of its viewport's stack (rank, key
/// class, scope; no egui layer) and decides its dismissal after the body ran. `rect` is the window's inner rect.
pub(crate) fn host_window(ctx: &Context, layer: &Layer<'_>, rect: Rect) -> LayerOut {
    let overlay = layer.overlay;
    let mut memory = recall(layer, ctx);
    let scope = (overlay.focus != FocusPolicy::NONE).then_some(overlay.focus);
    let mut entry = OpenEntry::new(layer.id, overlay.rank, overlay.keys, rect, areas::layer_of(overlay.rank.tier(), area_id(layer.id)));
    entry.scoped = scope.is_some();
    entry.window = true;
    overlay::record(ctx, entry);
    let dismissed = dismissal(ctx, layer, rect, Rect::ZERO, scope, &mut memory);
    close_family(ctx, layer, dismissed.is_some(), &mut memory);
    remember(layer, ctx, memory);
    LayerOut { dismissed }
}

/// The dismissal rules of 15.4, decided after the body ran so a field inside still commits in the press pass.
fn dismissal(
    ctx: &Context,
    layer: &Layer<'_>,
    rect: Rect,
    anchor: Rect,
    scope: Option<FocusPolicy>,
    memory: &mut LayerMemory,
) -> Option<CloseReason> {
    let dismiss = layer.overlay.dismiss;
    // Both the Escape rule and the focus rule read the same fact: whether the focus is in the layer.
    let inside = focus::focus_in_layer(ctx, layer.id);
    let was_inside = memory.focus_inside;
    if let Some(now) = inside {
        memory.focus_inside = now;
    }
    if let Some(reason) = escape(ctx, layer, scope, inside) {
        return Some(reason);
    }
    if dismiss.anchor_gone && layer.overlay.opener.as_ref().is_some_and(|first| !first.active(ctx).0.enabled) {
        return Some(CloseReason::AnchorGone);
    }
    // The input hook removed the press and recorded it; presses inside the family and its anchors never get here.
    if dismiss.outside == Outside::Absorb {
        if overlay::absorbed_press(ctx).is_some() {
            return Some(CloseReason::Outside);
        }
    } else if press_outside(ctx, layer, rect, anchor) {
        if dismiss.outside == Outside::Close {
            return Some(CloseReason::Outside);
        }
        overlay::note_pressed_outside(ctx, layer.id);
    }
    if dismiss.on_activate && overlay::activated_inside(ctx, layer.id) {
        return Some(CloseReason::Activated);
    }
    if dismiss.enter && scope.is_some() && overlay::top(ctx) == Some(layer.id) && keys::take_submit(ctx) {
        return Some(CloseReason::Enter);
    }
    if dismiss.focus_left
        && let Some(policy) = scope
        && policy.kind != craft_behavior::ScopeKind::Group
        && was_inside
        && inside == Some(false)
        && overlay::is_open(ctx, layer.id)
    {
        return Some(CloseReason::FocusLeft);
    }
    if let Some(timeout) = dismiss.timeout {
        let now = frame::clock(ctx);
        let since = *memory.since.get_or_insert(now);
        let due = since + f64::from(timeout);
        if now >= due {
            return Some(CloseReason::Timeout);
        }
        crate::runtime::egui::repaint::request_after(ctx, crate::runtime::egui::pass::current_viewport(ctx), due - now);
    }
    None
}

/// Whether a primary press landed outside the layer, its anchor and the layers opened from inside it.
fn press_outside(ctx: &Context, layer: &Layer<'_>, rect: Rect, anchor: Rect) -> bool {
    let Some(press) = pointer::frame(ctx).press.filter(|press| press.button == craft_behavior::PointerButton::Primary).map(|press| press.origin)
    else {
        return false;
    };
    let inside_family = overlay::family(ctx, layer.id).iter().filter_map(|id| overlay::entry(ctx, *id)).any(|entry| entry.rect.contains(press));
    !rect.contains(press) && !anchor.contains(press) && !inside_family
}

/// Whether the focused stop sits in no scope that belongs to a shown layer.
fn focus_in_no_layer(ctx: &Context) -> bool {
    focus::focused_rec(ctx).is_none_or(|rec| focus::scope_chain(ctx, &rec).iter().all(|scope| overlay::entry(ctx, *scope).is_none()))
}

/// Escape reaches only the top layer, and only when focus is inside its scope (a modal also when it is in no layer):
/// a component inside that takes the Escape (a draft, a gesture) keeps it, and a captured press goes first.
fn escape(ctx: &Context, layer: &Layer<'_>, scope: Option<FocusPolicy>, inside: Option<bool>) -> Option<CloseReason> {
    let policy = scope?;
    if policy.cancel == CancelRule::Ignore || overlay::top(ctx) != Some(layer.id) {
        return None;
    }
    // A modal on top also owns Escape while the focus is in no layer (nothing focused, or a stop of the window
    // below): its card may have nothing to focus. So does a popup, whose body may have nothing to focus, and a
    // keep-anchor popup, which leaves the focus where it was.
    let owns_loose = layer.overlay.rank.is_modal() || layer.overlay.rank == Rank::Popup || policy.initial == Initial::KeepAnchor;
    if inside != Some(true) && !(owns_loose && focus_in_no_layer(ctx)) {
        return None;
    }
    let step = keys::scope_escape(ctx, policy.cancel, pointer::press_captured(ctx), false)?;
    matches!(step, craft_behavior::focus::EscapeStep::CloseScope).then_some(CloseReason::Escape)
}

/// Paints the ghost egui layer from runtime data: the drag preview and drop mark, and the painted cursor the cursor
/// service settled on. The end hook calls it once per pass, popup or no popup.
pub fn paint_frame_layers(ctx: &Context) {
    super::ghost::paint(ctx);
}

/// Open state a container with no `open` binding keeps in its opener's slot, and the readers of a popup's frame.
#[derive(Clone, Copy, Default)]
struct OpenerOpen {
    open: bool,
    /// The root frame an opener's trigger was last applied in: a frame's passes apply it once.
    applied: u64,
    /// The opener that opened the layer by id and trigger, and whether a pointer release did.
    by: Option<(Id, Trigger, bool)>,
}

/// The open state of a container with no binding: the slot of its first opener, with the triggers of every opener
/// applied once for the frame. A container with no opener keeps its own slot.
pub(crate) fn opener_open(stage: &mut MemoryStage, first: &Opener) -> bool {
    let ctx = stage.ctx().clone();
    let frame = crate::runtime::egui::pass::root_frame_nr(&ctx);
    let held = first.all().find(|opener| opener.trigger == Trigger::Hold).map(|opener| {
        let started = dwell(&ctx, stage, opener.id, "dwell", opener.pressed, HOLD);
        (opener, started)
    });
    let slot = stage.session::<OpenerOpen>(sub(first.id, "opens"));
    if let Some(opener) = first.all().find(|opener| opener.fired) {
        if slot.applied != frame {
            slot.open = !slot.open;
            slot.by = Some((opener.id, opener.trigger, opener.by_pointer));
        }
        slot.applied = frame;
    } else if let Some((opener, true)) = held
        && !slot.open
    {
        slot.open = true;
        slot.by = Some((opener.id, opener.trigger, false));
    }
    let (open, by) = (slot.open, slot.by);
    with_popups(&ctx, |popups| match by {
        Some(by) if open => popups.opened_by.insert(first.id, by),
        _ => popups.opened_by.remove(&first.id),
    });
    open
}

/// Writes the opener's slot: the container closed itself (Escape, a press outside, a row ran).
pub(crate) fn set_opener_open(stage: &mut MemoryStage, opener: &Opener, open: bool) {
    stage.session::<OpenerOpen>(sub(opener.id, "opens")).open = open;
}

/// Whether `part` is one of the `toggles` parts `toggles` lists with a sub-node in `nodes`: the sub-node reports the
/// popup state and serves Expand and Collapse, and the component's own node does not. A toggling part that is the
/// whole face of the component has no sub-node and leaves them to the root.
pub(crate) fn has_toggle_nodes(toggles: &[super::cx::Toggle], nodes: &[crate::runtime::access::SubNode]) -> bool {
    nodes.iter().any(|node| toggles.iter().any(|toggle| toggle.part == node.part))
}

/// The open state of a container's layer that has an opener and no binding, driven from the opener's side.
pub struct Popup;

impl Popup {
    /// Opens the container hung from `opener` (a [`PopupHandle`] or the id of the opener's response).
    pub fn open<C: Component>(opener: impl Into<Id>, cx: &mut Cx<'_, C>) {
        Self::write(opener.into(), cx, Some(true));
    }

    /// Closes the container hung from `opener`.
    pub fn close<C: Component>(opener: impl Into<Id>, cx: &mut Cx<'_, C>) {
        Self::write(opener.into(), cx, Some(false));
    }

    /// Opens the container hung from `opener` when it is closed, else closes it.
    pub fn toggle<C: Component>(opener: impl Into<Id>, cx: &mut Cx<'_, C>) {
        Self::write(opener.into(), cx, None);
    }

    fn write<C: Component>(opener: Id, cx: &mut Cx<'_, C>, to: Option<bool>) {
        let slot = cx.core.memory.session::<OpenerOpen>(sub(opener, "opens"));
        slot.open = to.unwrap_or(!slot.open);
        slot.by = None;
        frame::request_repaint(&cx.core.ctx);
    }

    /// Whether the layer `popup` shows first in this pass: true in the passes of the frame it first shows, false
    /// in the frames after, so what a body resets on opening resets once.
    pub fn opening<C: Component>(popup: PopupHandle, cx: &Cx<'_, C>) -> bool {
        overlay::opening(&cx.core.ctx, popup.id)
    }
}

impl From<PopupHandle> for Id {
    fn from(popup: PopupHandle) -> Id {
        popup.id
    }
}

impl<C: Component> Cx<'_, C> {
    /// This activation runs but does not close the layer chain it happened in (`Dismiss::on_activate`).
    pub fn keep_open(&mut self) {
        self.core.keep_open = true;
    }

    /// Injects an activation for this component, from `source`: the pipeline runs its bound command with that
    /// source, reports it to `Component::activated` readers and closes the layer chain as for a click.
    pub fn activate(&mut self, source: FireSource) {
        self.core.injected = Some(source);
    }

    /// The modifiers held this frame, Alt alone included.
    pub fn modifiers(&self) -> Mods {
        pointer::frame(&self.core.ctx).mods
    }
}
