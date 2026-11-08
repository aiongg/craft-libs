//! The body step (16.3): the regions of a container (header, body with its scroll region, footer), open and
//! gate state, scroll offset, move and resize, and the zoomed canvas body.

use super::cx::{BodyCx, BodyOwner};
use super::label::{self, LabelIn};
use super::mount::{PaneGrip, pane_extent_id, pane_grip_id};
use super::overlay::{OverlayFit, Place, PlacementChange, PlacementKind};
use super::run::{ShapeHooks, ShowAt};
use super::{Container, ContainerProps, Cx, DragBand, FooterPlace, Gap, Internal, Mount, Notch, Pad, PaneEdge, PartsCx, Scroll, SetPosition, Sizing};
use crate::layout::{self, ColumnGroup};
use crate::primitives::{Rule, ScrollBar, Surface};
use crate::runtime::access::scope;
use crate::runtime::access::{self, Politeness, Role};
use crate::runtime::commands::{self, CommandSet as _, WindowCommand};
use crate::runtime::egui::pass;
use crate::runtime::egui::{alloc, areas, child, input, response, style, viewports};
use crate::runtime::env::{AppValues, ArenaId, Env};
use crate::runtime::focus::FocusPolicy;
use crate::runtime::memory::{MemoryStage, SaveKey, sub};
use crate::runtime::motion::{Driver, Presence, spec};
use crate::runtime::overlay::CloseReason;
use crate::runtime::windows::{self, ResizeEdge, WindowAction, WindowHit};
use crate::runtime::{keys, layout_memory, pointer, wheel};
use crate::style::ComponentStyle as _;
use crate::style::{Corners, RuleKind, StyleCx, UiTokens, rule_look, scroll_bar_look, surface_style};
use craft_behavior::layout_memory::measure_tolerance;
use craft_behavior::{
    Axis, CancelRule, CursorIcon, EditEvent, GestureId, GesturePhase, HitLevel, HitPriority, Intent, KeyRole, Mixed, PartKey, PressPhase, Roving,
    ScrollPos, ScrollSpan, WheelNotch, WheelTaker,
};
use craft_host::target::{Caps, WindowChrome, WindowState};
use craft_keymap::{Chord, Key, Mods};
use egui::{Align, Context, Id, Layout, Pos2, Rect, Shape, Ui, UiBuilder, Vec2, ViewportId};
use serde::{Deserialize, Serialize};
use std::any::Any;

const SCROLL_Y: PartKey = PartKey::new("body_scroll_y");
const SCROLL_X: PartKey = PartKey::new("body_scroll_x");
pub(crate) const MOVE_BAND: PartKey = PartKey::new("move_band");
pub(crate) const REORDER_BAND: PartKey = PartKey::new("reorder_band");
const GRIP: PartKey = PartKey::new("resize_grip");
/// The grip on an edge pane's inner edge.
const PANE_GRIP: PartKey = PartKey::new("pane_grip");

/// The open state a container with no `Container::open` binding keeps: for the session, or saved under `save`.
#[derive(Clone, Default, Serialize, Deserialize)]
struct OpenState(Option<bool>);

fn open_slot<'s>(stage: &'s mut MemoryStage, id: Id, save: Option<&SaveKey>) -> &'s mut OpenState {
    match save {
        Some(key) => stage.saved::<OpenState>(key),
        None => stage.session::<OpenState>(sub(id, "open")),
    }
}

/// The scroll offset a container with no `Component::scroll` binding keeps for the session.
#[derive(Clone, Default)]
struct ScrollState {
    offset: (f64, f64),
    /// A bottom-anchored region follows the end while nothing moved it away.
    follow_end: Option<bool>,
}

/// What the scroll region and the regions measured last pass.
#[derive(Clone, Default)]
struct BodyView {
    extent: (f32, f32),
    view: Option<[f32; 4]>,
}

/// Where a container on a layer sits and how large the user made it.
#[derive(Clone, Default, Serialize, Deserialize, PartialEq)]
pub(crate) struct Placement {
    pub pos: Option<[f32; 2]>,
    pub size: Option<[f32; 2]>,
}

impl Placement {
    /// The placement that puts the layer at `rect`.
    fn at(rect: Rect) -> Placement {
        Placement { pos: Some(rect.min.into()), size: Some(rect.size().into()) }
    }

    /// The placement that puts the layer's top left at `pos` and leaves its size to its content.
    fn at_pos(pos: Pos2) -> Placement {
        Placement { pos: Some(pos.into()), size: None }
    }

    /// The placement the caller of `overlay` gives, when it places the layer.
    fn given(overlay: &super::Overlay) -> Option<Placement> {
        overlay.placed.map(Placement::at).or_else(|| overlay.placed_at.map(Placement::at_pos))
    }

    /// The rect it names, taking the position and size it lacks from `rect`.
    fn rect_over(&self, rect: Rect) -> Rect {
        Rect::from_min_size(self.pos.map_or(rect.min, Pos2::from), self.size.map_or(rect.size(), Vec2::from))
    }
}

pub(crate) fn placement_id(id: Id) -> Id {
    sub(id, "placement")
}

/// The one store of a layer's placement: the saved tier when `save` is set, else shown memory. A layer whose overlay
/// is `placed` has the caller's rect instead: the slot reads it and writes nothing, since the caller owns the
/// placement and learns of the user's changes from `PlacementChange` reports.
struct PlacementSlot<'s> {
    stage: &'s mut MemoryStage,
    id: Id,
    save: Option<&'s SaveKey>,
    controlled: Option<Placement>,
}

impl<'s> PlacementSlot<'s> {
    fn new(stage: &'s mut MemoryStage, id: Id, save: Option<&'s SaveKey>, overlay: &super::Overlay) -> Self {
        PlacementSlot { stage, id, save, controlled: Placement::given(overlay) }
    }

    fn stored(&mut self) -> &mut Placement {
        match self.save {
            Some(key) => self.stage.saved::<Placement>(key),
            None => self.stage.shown::<Placement>(placement_id(self.id)),
        }
    }

    fn read(&mut self) -> Placement {
        match &self.controlled {
            Some(placement) => placement.clone(),
            None => self.stored().clone(),
        }
    }

    fn write(&mut self, placement: Placement) {
        if self.controlled.is_none() {
            *self.stored() = placement;
        }
    }
}

/// The placement of `id` as the last pass left it, read without opening a stage: the layer host places by it before
/// the container's own pass runs.
fn placement_of(ctx: &Context, id: Id, save: Option<&SaveKey>) -> Placement {
    match save {
        Some(key) => MemoryStage::peek_saved(ctx, key),
        None => MemoryStage::peek(ctx, placement_id(id)),
    }
}

/// The placement a layer is shown at: the caller's `placed` rect or `placed_at` position, else the saved or session placement, else for a
/// floating resizable layer with a default size (`Overlay::size`) that size, within the screen less `dialog_pad` on
/// each side, centred or at the overlay's float position. A saved size and position win over the default.
pub(crate) fn shown_placement(ctx: &Context, id: Id, save: Option<&SaveKey>, overlay: &super::Overlay, dialog_pad: f32) -> Placement {
    if let Some(given) = Placement::given(overlay) {
        return given;
    }
    let mut placement = placement_of(ctx, id, save);
    let (Some(default), Place::Float(float), true) = (overlay.size, &overlay.place, overlay.fit == OverlayFit::Resizable) else {
        return placement;
    };
    if placement.size.is_some() {
        return placement;
    }
    let screen = areas::safe_rect(ctx);
    let room = (screen.size() - Vec2::splat(2.0 * dialog_pad)).max(Vec2::ZERO);
    let least = overlay.min_size.unwrap_or(Vec2::ZERO).min(room);
    let size = default.max(least).min(overlay.max_size.unwrap_or(Vec2::INFINITY)).min(room);
    let pos = placement.pos.map(Pos2::from).or(float.pos).unwrap_or_else(|| screen.center() - size / 2.0);
    let pos = Pos2::new(pos.x.min(screen.right() - size.x).max(screen.left()), pos.y.min(screen.bottom() - size.y).max(screen.top()));
    placement.pos = Some(pos.into());
    placement.size = Some(size.into());
    placement
}

/// The overlay a layer-like mount shows its container in.
fn overlay_of(props: &ContainerProps) -> Option<&super::Overlay> {
    match &props.mount {
        Mount::Layer(overlay) | Mount::Viewport { fallback: overlay, .. } => Some(overlay),
        _ => None,
    }
}

/// The opener of a layer-like mount's overlay: a container with no `open` binding takes its open state from it.
fn opener_of(props: &ContainerProps) -> Option<&super::overlay::Opener> {
    overlay_of(props)?.opener.as_ref()
}

/// Which move and resize parts of the body step stand for the OS window of `viewport` (16.3, 21.2): the one rule
/// every move band and grip of an OS window reads. Both need a frame the app draws (the chrome is not `Native`). The
/// band stands on a maximized window too, so its double click restores the window and a drag is the platform's (it
/// restores and moves); the grips stand only on a window that is neither maximized nor full screen. The parts stand
/// whether or not a hit-test sink is installed: the sink reads them as the window's exclusive parts.
#[derive(Clone, Copy)]
struct WindowParts {
    band: bool,
    grips: bool,
}

impl WindowParts {
    /// The parts of a layer or a full-window mount, which move the container and never the OS window.
    const LAYER: WindowParts = WindowParts { band: true, grips: true };

    fn of(ctx: &Context, viewport: ViewportId) -> WindowParts {
        let window = windows::shell_of(ctx, viewport).window;
        let framed_by_app = window.chrome != WindowChrome::Native;
        let sized_by_os = matches!(window.state, WindowState::Maximized | WindowState::Fullscreen);
        WindowParts { band: framed_by_app && window.state != WindowState::Fullscreen, grips: framed_by_app && !sized_by_os }
    }
}

/// Whether the platform answers the hit test of `viewport`'s undecorated window itself, as it does for a window whose
/// hit-test sink is installed (`WindowHost::install_hit_sink`): it takes the move, edge and caption hits from the
/// window's exclusive parts, so a press on the move band or a grip never reaches the app as a move.
fn answered_by_platform(ctx: &Context, viewport: ViewportId) -> bool {
    let shell = windows::shell_of(ctx, viewport);
    shell.caps.contains(Caps::OS_RESIZES_UNDECORATED) && shell.window.chrome == WindowChrome::Integrated && windows::hit_sink_installed(ctx, viewport)
}

/// The viewport of the OS window a container's move and resize parts act on: a viewport mount's own window, the
/// window a `Window`, `Pane` or `Inline` container is shown in. A layer (also a viewport mount's fallback layer)
/// and a full-window mount are not an OS window of their own.
fn os_window(props: &ContainerProps, ctx: &Context, id: Id) -> Option<ViewportId> {
    match &props.mount {
        Mount::Viewport { .. } if viewports::available(ctx) => Some(viewports::viewport_id(id)),
        Mount::Window { .. } | Mount::Pane(_) | Mount::Inline => Some(pass::current_viewport(ctx)),
        Mount::Viewport { .. } | Mount::Layer(_) | Mount::FullWindow => None,
    }
}

/// The floor of a resizable layer's size: the larger of the style minimum and the overlay's `min_size`.
fn window_min(props: &ContainerProps, style_min: Vec2) -> Vec2 {
    overlay_of(props).and_then(|overlay| overlay.min_size).map_or(style_min, |min| style_min.max(min))
}

/// The instance id of the `window` command scope a movable or resizable layer container holds (16.3), whatever
/// scope its own commands use: the container's id when it publishes the window scope itself, else a sub-id.
fn window_instance(props: &ContainerProps, id: Id) -> Option<Id> {
    let overlay = overlay_of(props)?;
    (overlay.movable || overlay.resize.grips).then(|| if props.command_scope == Some(WindowCommand::SCOPE) { id } else { sub(id, "window") })
}

/// The scroll region of a container: its own, or for a layer the vertical one that takes the overflow of any fit
/// (none when the body is one collection, which owns the scroll, or the overlay turns its body scroll off).
fn scroll_of(props: &ContainerProps) -> Option<Scroll> {
    props.scroll.or_else(|| overlay_of(props).filter(|overlay| overlay.body_scroll && !props.collection_body).map(|_| Scroll::vertical()))
}

/// Whether the footer sits in the end region: it asks to, and the container has one.
fn footer_in_end(props: &ContainerProps) -> bool {
    props.end.is_some() && props.footer.is_some_and(|footer| footer.place.in_end())
}

/// Whether the end region takes the full height under the header: it asks to, or the footer sits in it.
fn end_full_height(props: &ContainerProps) -> bool {
    props.end.is_some_and(|end| end.full_height) || footer_in_end(props)
}

/// The keyboard mode of a floating container (16.3): arrows move or resize it until Enter or Escape.
#[derive(Clone, Copy, PartialEq)]
enum WindowModeKind {
    Move,
    Size,
}

/// A container's window gesture, the keyboard mode or a pointer drag, with the placement Escape restores and the
/// rect the layer had when the gesture began. Kept in session memory.
#[derive(Clone, Default, PartialEq)]
struct WindowMode {
    kind: Option<WindowModeKind>,
    restore: Placement,
    began: Option<Rect>,
}

/// Whether the second press of a double click went down on the container's move band in this pass.
#[derive(Clone, Default)]
struct BandDouble(bool);

impl BodyCx<'_> {
    /// Whether the second press of a double click went down on the container's move band in this pass. The body step
    /// only reports it; the container decides what it means (a title bar maximizes the window, a floating frame
    /// minimizes it). A band double click that the container never reads does nothing.
    pub fn band_double_pressed(&self) -> bool {
        self.memory.recall::<BandDouble>(sub(self.id, "band_double")).0
    }
}

/// What every change of a layer's rect keeps to: the least and most size, and the rect it stays inside.
struct Limits {
    min: Vec2,
    max: Vec2,
    bounds: Option<Rect>,
}

impl Limits {
    fn new(overlay: &super::Overlay, min: Vec2) -> Limits {
        Limits { min, max: overlay.max_size.unwrap_or(Vec2::INFINITY).max(min), bounds: overlay.bounds }
    }

    /// `rect` moved by `delta`, kept inside the bounds.
    fn moved(&self, rect: Rect, delta: Vec2) -> Rect {
        let rect = rect.translate(delta);
        let Some(bounds) = self.bounds else { return rect };
        let keep = |low: f32, high: f32, bounds_low: f32, bounds_high: f32| low.min(bounds_high - (high - low)).max(bounds_low) - low;
        rect.translate(Vec2::new(keep(rect.min.x, rect.max.x, bounds.min.x, bounds.max.x), keep(rect.min.y, rect.max.y, bounds.min.y, bounds.max.y)))
    }

    /// `rect` with the edges `dx` and `dy` (-1: low edge, 1: high edge, 0: none) moved by `delta`, within the size
    /// limits and the bounds. The opposite edge stays.
    fn resized(&self, rect: Rect, dx: i8, dy: i8, delta: Vec2) -> Rect {
        let axis = |low: f32, high: f32, edge: i8, delta: f32, min: f32, max: f32, bounds: Option<(f32, f32)>| {
            let (bounds_low, bounds_high) = bounds.unwrap_or((f32::NEG_INFINITY, f32::INFINITY));
            match edge {
                -1 => ((low + delta).max((high - max).max(bounds_low)).min(high - min), high),
                1 => (low, (high + delta).min((low + max).min(bounds_high)).max(low + min)),
                _ => (low, high),
            }
        };
        let (left, right) = axis(rect.min.x, rect.max.x, dx, delta.x, self.min.x, self.max.x, self.bounds.map(|bounds| (bounds.min.x, bounds.max.x)));
        let (top, bottom) = axis(rect.min.y, rect.max.y, dy, delta.y, self.min.y, self.max.y, self.bounds.map(|bounds| (bounds.min.y, bounds.max.y)));
        Rect::from_min_max(Pos2::new(left, top), Pos2::new(right, bottom))
    }
}

/// The edge or corner the grip with direction (`dx`, `dy`) of `GRIPS` sits on.
fn edge_of(dx: i8, dy: i8) -> ResizeEdge {
    match (dx, dy) {
        (-1, 0) => ResizeEdge::Left,
        (1, 0) => ResizeEdge::Right,
        (0, -1) => ResizeEdge::Top,
        (-1, -1) => ResizeEdge::TopLeft,
        (1, -1) => ResizeEdge::TopRight,
        (-1, 1) => ResizeEdge::BottomLeft,
        (1, 1) => ResizeEdge::BottomRight,
        _ => ResizeEdge::Bottom,
    }
}

/// The arrow chords, with and without Shift, a window mode holds, with the direction each moves.
const MODE_ARROWS: [(Key, [i8; 2]); 4] = [(Key::Left, [-1, 0]), (Key::Right, [1, 0]), (Key::Up, [0, -1]), (Key::Down, [0, 1])];

/// The eight edge and corner grips of a resizable layer, in the order of `GRIPS`: which edges each moves.
const GRIPS: [(i8, i8, CursorIcon); 8] = [
    (-1, 0, CursorIcon::ResizeHorizontal),
    (1, 0, CursorIcon::ResizeHorizontal),
    (0, -1, CursorIcon::ResizeVertical),
    (0, 1, CursorIcon::ResizeVertical),
    (-1, -1, CursorIcon::ResizeNwSe),
    (1, 1, CursorIcon::ResizeNwSe),
    (1, -1, CursorIcon::ResizeNeSw),
    (-1, 1, CursorIcon::ResizeNeSw),
];

fn pad_of(tokens: &UiTokens, pad: Pad) -> f32 {
    let pads = &tokens.tokens().space.pad;
    match pad {
        Pad::Control => pads.control,
        Pad::Regular => pads.panel,
        Pad::Roomy => pads.dialog,
        _ => 0.0,
    }
}

fn gap_of(tokens: &UiTokens, gap: Gap) -> f32 {
    let gaps = &tokens.tokens().space.gap;
    match gap {
        Gap::Tight => gaps.tight,
        Gap::Section => gaps.section,
        _ => gaps.row,
    }
}

fn rect_of(parts: [f32; 4]) -> Rect {
    Rect::from_min_max(Pos2::new(parts[0], parts[1]), Pos2::new(parts[2], parts[3]))
}

/// The regions of a container inside its rect.
struct Regions {
    header: Rect,
    body: Rect,
    /// The end region: the line's end edge, as high as the line or, full height, from under the header to the
    /// container's bottom, less a footer placed in it.
    end: Rect,
    footer: Rect,
}

/// The scope a container with a reorder band joins in its regions' command chain: no command belongs to it, so the
/// chain tells `commands::focus_within` that focus is inside the container.
const REORDER_SCOPE: commands::CommandScope = commands::CommandScope::new("reorder");

/// Registers the container as an instance of its command scope and of the window scope of its window.
fn register_instances(ctx: &Context, props: &ContainerProps, id: Id, window: Option<Id>) {
    if let Some(scope) = props.command_scope {
        commands::register_instance(ctx, scope, id);
    }
    if let Some(window) = window {
        commands::register_instance(ctx, WindowCommand::SCOPE, window);
    }
}

/// What every region of a container shares: its id, direction, whether it is an AccessKit node and its env.
struct RegionFrame<'e> {
    id: Id,
    rtl: bool,
    node: bool,
    env: &'e Env,
}

impl RegionFrame<'_> {
    /// The ui builder of the region `name` in `rect`.
    fn region(&self, name: &'static str, rect: Rect) -> UiBuilder {
        let builder = UiBuilder::new().max_rect(rect).layout(child::column_layout(self.rtl)).scope_id(sub(self.id, name));
        if self.node { access::with_parent(builder, self.id) } else { builder }
    }
}

/// One axis of a scroll region: its track and its span.
struct Bar {
    key: PartKey,
    axis: Axis,
    track: Rect,
}

pub(super) struct BodyHooks<'b> {
    pub(super) body: &'b mut dyn FnMut(&mut Ui, &mut BodyCx<'_>),
    /// The body's size from last frame's layout memory; `None` before its first run.
    remembered: Option<Vec2>,
    /// The header's size from last pass; `None` without a header or before its first measure.
    header_last: Option<Vec2>,
    footer_last: Vec2,
    /// The end region's size from last pass; `None` without one or before its first measure.
    end_last: Option<Vec2>,
    /// The gap between an inline header and the body, and between a header and its trailing region.
    region_gap: f32,
    /// How far the widest region wants to exceed the width the room allows, from this pass's measure.
    overflow: f32,
    /// The room the mount offers, in which a content-sized body runs.
    room: Vec2,
    open: bool,
    open_at_read: bool,
    open_bound: bool,
    gate: Option<bool>,
    gate_at_read: Option<bool>,
    close: bool,
    /// Why the container closes this pass, the first reason given.
    reason: Option<CloseReason>,
    /// The container is closed and its layer is fading out: the body shows as it was and nothing is written.
    exiting: bool,
    offset: ScrollPos,
    offset_at_read: ScrollPos,
    scroll_bound: bool,
    moved: bool,
    last: BodyView,
    /// This pass's sizes, recorded at step 17.
    body_size: Option<Vec2>,
    header_size: Option<Vec2>,
    footer_size: Option<Vec2>,
    end_size: Option<Vec2>,
    extent: Option<Vec2>,
    view: Option<Rect>,
    save: Option<SaveKey>,
    layer: bool,
    /// How open the body is: 0 to 1, animating when the container's `Collapse` does.
    presence: Presence,
    /// The distance one arrow press moves or resizes a window in its keyboard mode.
    nudge: f32,
    /// The window mode the container is in, and what Escape restores.
    mode: WindowMode,
    /// The edge an edge pane's grip sits on this pass, with the least extent it gives; none for other mounts.
    pane_grip: Option<(ResizeEdge, f32)>,
    /// The placement changes of this pass, delivered to `Container::placement_changed` at the end of the update.
    reports: Vec<PlacementChange>,
    /// The host gives the container a rect of its own size: a full window, an OS viewport or a layer the user sized.
    fixed: bool,
    /// A bottom-anchored region is following the end of its content.
    follows_end: bool,
}

impl BodyHooks<'_> {
    /// AccessKit scroll actions aimed at the container's node: a page along an axis, or an offset.
    fn access_scroll<C: Container>(&mut self, props: &ContainerProps, cx: &mut Cx<'_, C>) {
        use craft_behavior::{AccessAction as Action, AccessData as ActionData};
        let Some(scroll) = scroll_of(props) else { return };
        let rtl = cx.core.env.direction().is_rtl();
        let view = self.last.view.map_or(Vec2::ZERO, |view| rect_of(view).size());
        while let Some(request) = cx.core.intents.take_scroll_access(false) {
            let (vertical, toward_end) = match request.action {
                Action::ScrollUp => (true, false),
                Action::ScrollDown => (true, true),
                Action::ScrollLeft => (false, rtl),
                Action::ScrollRight => (false, !rtl),
                _ => {
                    if let Some(ActionData::SetScrollOffset(point)) = request.data {
                        self.offset = ScrollPos { x: point.x, y: point.y };
                        self.moved = true;
                    }
                    continue;
                }
            };
            let (visible, extent, offset) =
                if vertical { (view.y, self.last.extent.1, self.offset.y) } else { (view.x, self.last.extent.0, self.offset.x) };
            let span = ScrollSpan::new(0.0..f64::from(extent).max(f64::from(visible)), offset..offset + f64::from(visible));
            let start = craft_behavior::scroll::page(&span, toward_end).start;
            if vertical {
                self.offset.y = start;
            } else {
                self.offset.x = start;
            }
            self.moved = true;
        }
        self.clamp_offset(scroll, view);
    }

    /// Alt+Up and Alt+Down while focus is anywhere in a section whose header is a reorder band. A section has no stop of
    /// its own, so the keys would reach the stops in its header: the container holds the chords for itself and turns
    /// them into `Intent::MoveItem` for its own update.
    fn reorder_keys<C: Container>(&mut self, props: &ContainerProps, cx: &mut Cx<'_, C>) {
        if !props.header.is_some_and(|header| header.band == DragBand::Reorder) {
            return;
        }
        let (id, ctx) = (cx.core.id, cx.core.ctx.clone());
        let within = commands::focus_within(&ctx, id);
        for (key, toward_end) in [(Key::Up, false), (Key::Down, true)] {
            let chord = Chord::new(Mods::ALT, key);
            if within {
                keys::capture(&ctx, id, chord);
            }
            while keys::take_claimed(&ctx, id, chord) {
                cx.core.intents.push(Intent::move_item(toward_end));
            }
        }
    }

    /// The `window.Move` and `window.Size` commands and the keys of the mode they start (16.3): arrows move or
    /// resize by a step (Shift: four steps), Enter, the command again or a primary press keeps the placement, Escape
    /// restores it, and focus leaving the container ends the mode. The mode holds its chords through the key stage, so no
    /// component under focus sees them. Each step is reported as a `PlacementKind::Keyboard` change.
    fn window_keys<C: Container>(&mut self, props: &ContainerProps, cx: &mut Cx<'_, C>) {
        let (id, ctx) = (cx.core.id, cx.core.ctx.clone());
        let (Some(window), Some(overlay)) = (window_instance(props, id), overlay_of(props)) else { return };
        let limits = Limits::new(overlay, window_min(props, cx.style.metrics().min_size));
        let rect = cx.core.rect;
        let mut slot = PlacementSlot::new(&mut *cx.core.memory, id, self.save.as_ref(), overlay);
        let current = slot.read();
        let step = |rect: Rect, phase: GesturePhase| PlacementChange { rect, phase, pointer: None, kind: PlacementKind::Keyboard };
        let started = commands::take_for_instance(&ctx, Some(window), WindowCommand::from_ref).map(|(command, _)| command);
        let mut mode = self.mode.clone();
        if let Some(command) = started {
            let kind = match command {
                WindowCommand::Move => WindowModeKind::Move,
                _ => WindowModeKind::Size,
            };
            if mode.kind.is_some() {
                self.reports.push(step(current.rect_over(rect), GesturePhase::Commit));
            }
            if mode.kind == Some(kind) {
                mode = WindowMode::default();
            } else {
                mode = WindowMode { kind: Some(kind), restore: current.clone(), began: Some(current.rect_over(rect)) };
                self.reports.push(step(current.rect_over(rect), GesturePhase::Begin));
            }
        }
        // A primary press ends the mode and keeps the placement reached, like Enter.
        let pressed = || pointer::with_frame(&ctx, |frame, _| frame.is_some_and(|frame| frame.press.is_some()));
        if mode.kind.is_some() && (!commands::focus_within(&ctx, id) || pressed()) {
            self.reports.push(step(current.rect_over(rect), GesturePhase::Commit));
            mode = WindowMode::default();
        }
        if let Some(kind) = mode.kind {
            let unit = self.nudge;
            let mut placement = current;
            for (key, direction) in MODE_ARROWS {
                for (mods, scale) in [(Mods::NONE, 1.0), (Mods::SHIFT, 4.0)] {
                    let chord = Chord::new(mods, key);
                    keys::capture(&ctx, id, chord);
                    while keys::take_claimed(&ctx, id, chord) {
                        let delta = Vec2::new(f32::from(direction[0]), f32::from(direction[1])) * unit * scale;
                        let before = placement.rect_over(rect);
                        let next = match kind {
                            WindowModeKind::Move => limits.moved(before, delta),
                            WindowModeKind::Size => limits.resized(before, 1, 1, delta),
                        };
                        placement = Placement::at(next);
                        self.reports.push(step(next, GesturePhase::Update));
                    }
                }
            }
            for key in [Key::Enter, Key::Escape] {
                let chord = Chord::new(Mods::NONE, key);
                keys::capture(&ctx, id, chord);
                if keys::take_claimed(&ctx, id, chord) {
                    if key == Key::Escape {
                        placement = mode.restore.clone();
                        self.reports.push(step(mode.began.unwrap_or(rect), GesturePhase::Cancel));
                    } else {
                        self.reports.push(step(placement.rect_over(rect), GesturePhase::Commit));
                    }
                    mode = WindowMode::default();
                }
            }
            slot.write(placement);
        }
        self.mode = mode;
    }

    /// A drag of the move band or a grip of a layer (not an OS window): the layer follows the pointer within the
    /// limits, and each step is reported. Escape, which cancels the press, restores the placement from before.
    fn window_drag<C: Container>(&mut self, props: &ContainerProps, cx: &mut Cx<'_, C>) {
        let Some(overlay) = overlay_of(props) else { return };
        // A band that moves the window below starts the OS move of the window the layer is shown in.
        if overlay.moves_window {
            let viewport = pass::current_viewport(&cx.core.ctx);
            self.window_os(viewport, cx);
            return;
        }
        let id = cx.core.id;
        let limits = Limits::new(overlay, window_min(props, cx.style.metrics().min_size));
        let rect = cx.core.rect;
        // A part the container did not declare this pass (no band, no grips) has no result.
        let phase = |key: (PartKey, Option<u64>)| cx.core.results.get(&key).map(|result| result.press.phase);
        let mut pressed = vec![(None, phase((MOVE_BAND, None)))];
        pressed.extend((0..GRIPS.len()).map(|index| (Some(index), phase((GRIP, Some(index as u64))))));
        let Some((grip, phase)) = pressed.into_iter().find_map(|(grip, phase)| match phase? {
            phase @ (PressPhase::DragStart { .. } | PressPhase::Drag { .. } | PressPhase::Release { .. } | PressPhase::Cancel { .. }) => {
                Some((grip, phase))
            }
            _ => None,
        }) else {
            return;
        };
        let mut slot = PlacementSlot::new(&mut *cx.core.memory, id, self.save.as_ref(), overlay);
        let current = slot.read();
        let (origin, pos, gesture) = match phase {
            PressPhase::DragStart { start, pos, .. } => {
                self.mode.began = Some(current.rect_over(rect));
                self.mode.restore = current.clone();
                (start.origin, Some(pos), GesturePhase::Begin)
            }
            PressPhase::Drag { start, pos, .. } => (start.origin, Some(pos), GesturePhase::Update),
            PressPhase::Release { start, pos, .. } => (start.origin, Some(pos), GesturePhase::Commit),
            PressPhase::Cancel { start, .. } => (start.origin, None, GesturePhase::Cancel),
            _ => return,
        };
        let Some(began) = self.mode.began else { return };
        let (dx, dy, _) = grip.map_or((0, 0, CursorIcon::Grab), |index| GRIPS[index]);
        let kind = if grip.is_some() { PlacementKind::Size(edge_of(dx, dy)) } else { PlacementKind::Move };
        let next = match pos {
            Some(pos) if grip.is_some() => limits.resized(began, dx, dy, pos - origin),
            Some(pos) => limits.moved(began, pos - origin),
            None => began,
        };
        let change = |rect: Rect, phase: GesturePhase| PlacementChange { rect, phase, pointer: pos, kind };
        if gesture == GesturePhase::Begin {
            self.reports.push(change(began, GesturePhase::Begin));
        }
        self.reports.push(change(next, if gesture == GesturePhase::Begin { GesturePhase::Update } else { gesture }));
        // A move leaves a content-sized layer its size; a resize fixes both.
        slot.write(match (gesture, grip) {
            (GesturePhase::Cancel, _) => self.mode.restore.clone(),
            (_, Some(_)) => Placement::at(next),
            (_, None) => Placement { pos: Some(next.min.into()), size: current.size },
        });
        if matches!(gesture, GesturePhase::Commit | GesturePhase::Cancel) {
            self.mode.began = None;
        }
    }

    /// A press on the move band or a grip of a container in an OS window starts the OS move or resize. Where the
    /// platform answers the hit test (`answered_by_platform`), the parts stand for its sink only.
    fn window_os<C: Container>(&self, viewport: ViewportId, cx: &Cx<'_, C>) {
        if answered_by_platform(&cx.core.ctx, viewport) {
            return;
        }
        // The parts stand only by `WindowParts`, and the results of a part not registered this pass are gone.
        let started =
            |key: (PartKey, Option<u64>)| cx.core.results.get(&key).is_some_and(|result| matches!(result.press.phase, PressPhase::DragStart { .. }));
        if started((MOVE_BAND, None)) {
            windows::act(&cx.core.ctx, viewport, WindowAction::StartMove);
        }
        for (index, (dx, dy, _)) in GRIPS.into_iter().enumerate() {
            if started((GRIP, Some(index as u64))) {
                windows::act(&cx.core.ctx, viewport, WindowAction::StartResize(edge_of(dx, dy)));
            }
        }
    }

    /// Records for the container's header hooks whether the second press of a double click went down on the move
    /// band in this pass (`BodyCx::band_double_pressed`).
    fn band_double<C: Container>(&self, cx: &mut Cx<'_, C>) {
        let Some(band) = cx.core.results.get(&(MOVE_BAND, None)) else { return };
        let double = band.press.double_pressed();
        cx.core.memory.shown::<BandDouble>(sub(sub(cx.core.id, "body"), "band_double")).0 = double;
    }

    /// The move band and the grips (16.3). A layer's move and resize the layer; those of an OS window (`os_window`)
    /// stand by `WindowParts` and join the window's exclusive list, which the hit-test sink reads. A layer's parts never
    /// join it, or the sink would move the OS window from a dialog inside it.
    fn move_parts<C: Container>(props: &ContainerProps, regions: &Regions, rect: Rect, pcx: &mut PartsCx<'_, C>) {
        let ctx = pcx.ui.ctx().clone();
        let handle = pcx.tokens.tokens().size.handle.sm;
        let reason = "the Move and Size commands of the window command set do the same";
        let os = os_window(props, &ctx, pcx.id);
        let stands = os.map_or(WindowParts::LAYER, |viewport| WindowParts::of(&ctx, viewport));
        let exclusive = |hit: WindowHit| os.is_some().then_some(hit);
        // A layer moves when its overlay says so; other mounts follow the header's band.
        let movable = match &props.mount {
            // A band that moves the window below follows the rule of the window the layer is shown in.
            Mount::Layer(overlay) | Mount::Viewport { fallback: overlay, .. } if overlay.moves_window => {
                let viewport = pass::current_viewport(&ctx);
                WindowParts::of(&ctx, viewport).grips && !answered_by_platform(&ctx, viewport)
            }
            Mount::Layer(overlay) | Mount::Viewport { fallback: overlay, .. } => overlay.movable && stands.band,
            _ => props.header.is_some_and(|header| header.band == super::DragBand::Move) && stands.band,
        };
        let band = move_band(props, regions, rect);
        if props.header.is_some() && movable && band.is_positive() {
            // The band lies under the line's own parts and children, like the reorder band, so they keep their presses.
            let part = pcx
                .part(MOVE_BAND, band)
                .sense(craft_behavior::Sense::DRAG)
                .cursor(CursorIcon::Grab)
                .drag_cursor(CursorIcon::Grabbing)
                .pointer_only(reason);
            if let Some(hit) = exclusive(WindowHit::Move) {
                part.exclusive(hit);
            }
        }
        // The grips of a layer straddle its edge; those of a window lie inside it, where the window has pixels.
        let (grips, inset) = match &props.mount {
            Mount::Layer(overlay) | Mount::Viewport { fallback: overlay, .. } => (overlay.resize.grips, 0.0),
            Mount::Window { resize } => (resize.grips, handle / 2.0),
            _ => (false, 0.0),
        };
        if !grips || !stands.grips {
            return;
        }
        for (index, (dx, dy, cursor)) in GRIPS.into_iter().enumerate() {
            let along = |edge: i8, low: f32, high: f32| match edge {
                -1 => (low - handle / 2.0 + inset, low + handle / 2.0 + inset),
                1 => (high - handle / 2.0 - inset, high + handle / 2.0 - inset),
                _ => (low + handle / 2.0 + inset, high - handle / 2.0 - inset),
            };
            let (x0, x1) = along(dx, rect.min.x, rect.max.x);
            let (y0, y1) = along(dy, rect.min.y, rect.max.y);
            let level = if dx != 0 && dy != 0 { HitLevel::Handle } else { HitLevel::Edge };
            let part = pcx
                .part(GRIP.at(index as u64), Rect::from_min_max(Pos2::new(x0, y0), Pos2::new(x1, y1)))
                .sense(craft_behavior::Sense::DRAG)
                .priority(HitPriority::new(level, 0))
                .cursor(cursor)
                .pointer_only(reason);
            if let Some(hit) = exclusive(WindowHit::Resize(edge_of(dx, dy))) {
                part.exclusive(hit);
            }
        }
    }

    /// Tells the platform whether the OS may move the container's OS window from where the pointer is (21.2
    /// `WindowDrag`; macOS `NSWindow.movable`): only where a press would go to the move band under the press-target
    /// rule (8.3), so a press on a control over the band never drags the window. Bands of one window combine: any band
    /// that would take the press allows the move. A pass with no band writes nothing; `AppShell` writes `false` for a
    /// pass with no title bar.
    fn window_movable<C: Container>(&self, cx: &Cx<'_, C>) {
        let Some(band) = cx.core.results.get(&(MOVE_BAND, None)) else { return };
        let ctx = &cx.core.ctx;
        let takes_press = band.hovered
            && pointer::pass_pointer(ctx).hover.is_some_and(|at| {
                let (click, drag) = input::hit_targets(ctx);
                pointer::press_winner(ctx, at, pointer::PressTargets { click, drag })
                    .is_some_and(|(winner, root)| root == cx.core.id && winner.part.is_some_and(|part| part.key == MOVE_BAND))
            });
        let this_pass = viewports::pass_number(ctx);
        let allowed = viewports::movable_record(ctx).is_some_and(|(pass, movable)| pass == this_pass && movable);
        viewports::write_movable(ctx, allowed || takes_press);
    }

    /// The grip of an edge pane (`Mount::Pane`) on its inner edge, the edge toward the centre, inside the pane: dragging
    /// it gives the pane a new extent along its edge (`pane_drag`). A pane its caller lays out (`ShellPane::at`) and the
    /// central pane have none. Returns the edge the grip sits on and the least extent it gives.
    fn pane_grip<C: Container>(props: &ContainerProps, rect: Rect, pcx: &mut PartsCx<'_, C>) -> Option<(ResizeEdge, f32)> {
        let Mount::Pane(pane) = &props.mount else { return None };
        let rtl = pcx.flow.is_rtl();
        let edge = match pane.edge {
            _ if pane.at.is_some() => return None,
            PaneEdge::Start if rtl => ResizeEdge::Left,
            PaneEdge::Start => ResizeEdge::Right,
            PaneEdge::End if rtl => ResizeEdge::Right,
            PaneEdge::End => ResizeEdge::Left,
            PaneEdge::Top => ResizeEdge::Bottom,
            PaneEdge::Bottom => ResizeEdge::Top,
            _ => return None,
        };
        let tokens = pcx.tokens.tokens();
        let (handle, least) = (tokens.size.handle.sm, tokens.size.pane.least);
        let (grip, cursor) = match edge {
            ResizeEdge::Left => (Rect::from_min_max(rect.min, Pos2::new(rect.min.x + handle, rect.max.y)), CursorIcon::ResizeHorizontal),
            ResizeEdge::Right => (Rect::from_min_max(Pos2::new(rect.max.x - handle, rect.min.y), rect.max), CursorIcon::ResizeHorizontal),
            ResizeEdge::Top => (Rect::from_min_max(rect.min, Pos2::new(rect.max.x, rect.min.y + handle)), CursorIcon::ResizeVertical),
            _ => (Rect::from_min_max(Pos2::new(rect.min.x, rect.max.y - handle), rect.max), CursorIcon::ResizeVertical),
        };
        pcx.part(PANE_GRIP, grip)
            .sense(craft_behavior::Sense::DRAG)
            .priority(HitPriority::new(HitLevel::Edge, 0))
            .cursor(cursor)
            .pointer_only("no command resizes a docked pane; its extent is the app's layout");
        Some((edge, least))
    }

    /// A drag of an edge pane's grip: the pane's extent follows the pointer, at least the least extent, and each step
    /// is reported as a `PlacementKind::Size` change of the pane's rect. While the gesture runs the pane host shows its
    /// extent (`PaneGrip`); at the commit a pane with no fixed extent keeps it in pane memory, and one whose caller fixes
    /// the extent shows the caller's from the next pass. Escape, which cancels the press, restores the extent.
    fn pane_drag<C: Container>(&mut self, props: &ContainerProps, cx: &mut Cx<'_, C>) {
        let (Mount::Pane(pane), Some((edge, least))) = (&props.mount, self.pane_grip) else { return };
        let Some(phase) = cx.core.results.get(&(PANE_GRIP, None)).map(|result| result.press.phase) else { return };
        let (origin, pos, gesture) = match phase {
            PressPhase::DragStart { start, pos, .. } => (start.origin, Some(pos), GesturePhase::Begin),
            PressPhase::Drag { start, pos, .. } => (start.origin, Some(pos), GesturePhase::Update),
            PressPhase::Release { start, pos, .. } => (start.origin, Some(pos), GesturePhase::Commit),
            PressPhase::Cancel { start, .. } => (start.origin, None, GesturePhase::Cancel),
            _ => return,
        };
        let (ctx, id, rect) = (cx.core.ctx.clone(), cx.core.id, cx.core.rect);
        let horizontal = matches!(edge, ResizeEdge::Left | ResizeEdge::Right);
        let along = |size: Vec2| if horizontal { size.x } else { size.y };
        let slot = cx.core.memory.shown::<PaneGrip>(pane_grip_id(id));
        let shown = slot.extent;
        let began = match slot.began {
            Some(began) if gesture != GesturePhase::Begin => began,
            _ => along(rect.size()),
        };
        let moved = pos.map_or(0.0, |pos| match edge {
            ResizeEdge::Right => pos.x - origin.x,
            ResizeEdge::Left => origin.x - pos.x,
            ResizeEdge::Bottom => pos.y - origin.y,
            _ => origin.y - pos.y,
        });
        let extent = (began + moved).max(least);
        let resized = |extent: f32| match edge {
            ResizeEdge::Right => Rect::from_min_size(rect.min, Vec2::new(extent, rect.height())),
            ResizeEdge::Left => Rect::from_min_max(Pos2::new(rect.max.x - extent, rect.min.y), rect.max),
            ResizeEdge::Bottom => Rect::from_min_size(rect.min, Vec2::new(rect.width(), extent)),
            _ => Rect::from_min_max(Pos2::new(rect.min.x, rect.max.y - extent), rect.max),
        };
        let change = |rect: Rect, phase: GesturePhase| PlacementChange { rect, phase, pointer: pos, kind: PlacementKind::Size(edge) };
        if gesture == GesturePhase::Begin {
            self.reports.push(change(resized(began), GesturePhase::Begin));
        }
        let (phase, extent) = match gesture {
            GesturePhase::Begin => (GesturePhase::Update, extent),
            GesturePhase::Cancel => (GesturePhase::Cancel, began),
            phase => (phase, extent),
        };
        self.reports.push(change(resized(extent), phase));
        *slot = if matches!(phase, GesturePhase::Commit | GesturePhase::Cancel) {
            PaneGrip::default()
        } else {
            PaneGrip { began: Some(began), extent: Some(extent) }
        };
        if phase == GesturePhase::Commit && pane.extent.is_none() {
            let size = rect.size();
            layout_memory::record(&ctx, pane_extent_id(id), if horizontal { Vec2::new(extent, size.y) } else { Vec2::new(size.x, extent) });
        }
        // The pane showed this pass at the extent of the last step: a commit or cancel that lands elsewhere shows in
        // the frame's discard pass.
        if matches!(phase, GesturePhase::Commit | GesturePhase::Cancel) && shown.is_some_and(|shown| (shown - extent).abs() > 0.5) {
            layout_memory::ask_discard(&ctx);
        }
    }
}

/// The move band of a container whose header drags: an inline header's band spans the header's whole line (header,
/// body and end region), beneath the line's children, so every free point of the line drags; a header on a line of
/// its own is the band.
fn move_band(props: &ContainerProps, regions: &Regions, rect: Rect) -> Rect {
    if !props.header.is_some_and(|header| header.inline) {
        return regions.header;
    }
    let line = regions.header.union(regions.body);
    let line = if props.end.is_some() { line.union(regions.end) } else { line };
    line.intersect(rect)
}

impl<'b> BodyHooks<'b> {
    pub(super) fn new(body: &'b mut dyn FnMut(&mut Ui, &mut BodyCx<'_>)) -> Self {
        BodyHooks {
            body,
            remembered: None,
            header_last: None,
            footer_last: Vec2::ZERO,
            end_last: None,
            region_gap: 0.0,
            overflow: 0.0,
            room: Vec2::ZERO,
            open: true,
            open_at_read: true,
            open_bound: false,
            gate: None,
            gate_at_read: None,
            close: false,
            reason: None,
            exiting: false,
            offset: ScrollPos::default(),
            offset_at_read: ScrollPos::default(),
            scroll_bound: false,
            moved: false,
            last: BodyView::default(),
            body_size: None,
            header_size: None,
            footer_size: None,
            end_size: None,
            extent: None,
            view: None,
            save: None,
            layer: false,
            presence: Presence { t: 1.0, visible: true, exiting: false },
            nudge: 0.0,
            mode: WindowMode::default(),
            pane_grip: None,
            reports: Vec::new(),
            fixed: false,
            follows_end: false,
        }
    }

    /// How the container's size follows its content here: a pane or a rect the host gives is filled; a layer sizes
    /// from its content until the user sizes it; an inline container follows its props.
    fn sizing(&self, props: &ContainerProps) -> Sizing {
        match &props.mount {
            Mount::Pane(_) | Mount::Window { .. } => Sizing::Fill,
            Mount::Layer(_) | Mount::Viewport { .. } | Mount::FullWindow if self.fixed => Sizing::Fill,
            Mount::Layer(_) | Mount::Viewport { .. } => Sizing::Content,
            _ => props.sizing,
        }
    }

    /// Whether the container fills the width of a content-sized column it sits in: the column holds it at its measured
    /// width and shows it across the width the column settles on (step 6).
    fn held(&self, props: &ContainerProps, env: &Env) -> bool {
        env.content_column && matches!(props.mount, Mount::Inline) && self.sizing(props) != Sizing::Content
    }

    /// Whether a region took another size than the pass placed it by.
    fn regions_moved(&self, tolerance: f32) -> bool {
        let moved = |took: Option<Vec2>, placed: Option<Vec2>| {
            took.is_some_and(|took| placed.is_none_or(|placed| (took - placed).abs().max_elem() > tolerance))
        };
        moved(self.body_size, self.remembered)
            || moved(self.header_size, self.header_last)
            || moved(self.footer_size, Some(self.footer_last))
            || moved(self.end_size, self.end_last)
    }

    /// A block whose regions took more than the extent its parent's flow placed it at claims what it took: on its
    /// first appearance (it was placed without a size entry) and in any pass in which a region's size changed. The
    /// parent's flow moves past the size the block really took (a grid cell is widened to it, so the cells after it
    /// are placed by it), so whatever contains the block measures its content true in this pass and records it, at
    /// every depth, and the tree settles in the one discard pass the first record asks for. A block that keeps its
    /// sizes claims nothing. A block that shrank keeps the extent it was placed at until the next pass places it by
    /// its record. While the body opens or closes it shows a part of its height by design and claims nothing.
    fn claim_extent(&self, props: &ContainerProps, ui: &mut Ui, env: &Env, rect: Rect, pad: f32, rtl: bool) {
        let held = self.held(props, env);
        if !matches!(props.mount, Mount::Inline) || (self.sizing(props) == Sizing::Fill && !held) {
            return;
        }
        let tolerance = measure_tolerance(crate::runtime::egui::scale::pixels_per_point(ui.ctx()));
        let animating = self.presence.t > 0.0 && self.presence.t < 1.0;
        if self.remembered.is_some() && (animating || !self.regions_moved(tolerance)) {
            return;
        }
        let (body, header, footer) =
            (self.body_size.unwrap_or(Vec2::ZERO), self.header_size.unwrap_or(Vec2::ZERO), self.footer_size.unwrap_or(Vec2::ZERO));
        let took = self.regions_size(props, body, header, footer, self.end_size.unwrap_or(Vec2::ZERO)) + Vec2::splat(2.0 * pad);
        let real = if rtl { Rect::from_min_size(Pos2::new(rect.max.x - took.x, rect.min.y), took) } else { Rect::from_min_size(rect.min, took) };
        // A held block took no width in the flow: it shows across the column, and only its content counts.
        let placed = if !held {
            rect
        } else if rtl {
            Rect::from_min_max(Pos2::new(rect.max.x, rect.min.y), rect.max)
        } else {
            Rect::from_min_max(rect.min, Pos2::new(rect.min.x, rect.max.y))
        };
        if !placed.expand(tolerance).contains_rect(real) {
            layout::widen_cell(ui, took);
            alloc::advance_past(ui, real.union(placed));
        }
    }

    /// The header region (B1): the trailing controls take the end edge first (their first control sits at the edge)
    /// and the header fills what they leave. Records the header's size and returns the close it asked for.
    fn header_region<C: Container>(
        &mut self,
        comp: &mut C,
        ui: &mut Ui,
        bcx: &mut BodyCx<'_>,
        frame: &RegionFrame<'_>,
        header: Rect,
        trailing: bool,
    ) -> Option<CloseReason> {
        let ctx = ui.ctx().clone();
        let (id, rtl) = (frame.id, frame.rtl);
        let mut header_rect = header;
        let mut controls = Vec2::ZERO;
        if trailing {
            let layout = if rtl { Layout::left_to_right(Align::Center) } else { Layout::right_to_left(Align::Center) };
            let builder = UiBuilder::new().max_rect(header).layout(layout).scope_id(sub(id, "controls"));
            let builder = if frame.node { access::with_parent(builder, id) } else { builder };
            let mut controls_ui = alloc::child(ui, builder);
            let installed = Env::install_for(&controls_ui, frame.env.clone());
            comp.controls(&mut controls_ui, bcx);
            Env::restore(&ctx, installed);
            controls = controls_ui.min_rect().size();
            if controls.x > 0.0 {
                let cut = controls.x + self.region_gap;
                if rtl {
                    header_rect.min.x = (header_rect.min.x + cut).min(header_rect.max.x);
                } else {
                    header_rect.max.x = (header_rect.max.x - cut).max(header_rect.min.x);
                }
            }
        }
        let mut header_ui = alloc::child(ui, frame.region("header", header_rect));
        let installed = Env::install_for(&header_ui, frame.env.clone());
        comp.header(&mut header_ui, bcx);
        Env::restore(&ctx, installed);
        let mut took = self.region_size(&header_ui);
        // A header that took the whole room records no width (see `region_size`); the trailing region adds to a
        // width that is known.
        if took.x > 0.0 && controls.x > 0.0 {
            took.x += controls.x + self.region_gap;
        }
        took.y = took.y.max(controls.y);
        self.header_size = Some(took);
        bcx.close_requested()
    }

    /// What a header or footer took. Before the body's width is known a region runs in the whole room and a `Fill`
    /// child takes all of it, so a region that took the whole room records no width then; one that took less
    /// records what its content needs, so the next pass offers it that much and nothing in it measures squeezed.
    fn region_size(&self, ui: &Ui) -> Vec2 {
        let size = ui.min_rect().size();
        if self.remembered.is_some() || size.x + 0.5 < ui.max_rect().width() { size } else { Vec2::new(0.0, size.y) }
    }

    /// The size the regions take together, from what the body, header, footer and end region took (zero for a region
    /// the container lacks or that took the whole room). An inline header shares the line; a footer in the end region
    /// stacks with its content; a full-height end region stands beside the body and the footer.
    fn regions_size(&self, props: &ContainerProps, body: Vec2, header: Vec2, footer: Vec2, end: Vec2) -> Vec2 {
        let in_end = footer_in_end(props);
        let end = if in_end { Vec2::new(end.x.max(footer.x), end.y + footer.y) } else { end };
        let across = if in_end { Vec2::ZERO } else { footer };
        let gap = |width: f32| if width > 0.0 { width + self.region_gap } else { 0.0 };
        let inline = props.header.is_some_and(|header| header.inline);
        // The line: the body with an inline header before it.
        let line = if inline { Vec2::new(body.x + gap(header.x), body.y.max(header.y)) } else { body };
        let (width, height) = if end_full_height(props) {
            (line.x.max(across.x) + gap(end.x), (line.y + across.y).max(end.y))
        } else {
            ((line.x + gap(end.x)).max(across.x), line.y.max(end.y) + across.y)
        };
        if inline { Vec2::new(width, height) } else { Vec2::new(width.max(header.x), height + header.y) }
    }

    /// The width the regions ask for in a content-sized container: what they took together last pass.
    fn wanted_width(&self, props: &ContainerProps) -> Option<f32> {
        self.remembered.map(|body| {
            let (header, end) = (self.header_last.unwrap_or(Vec2::ZERO), self.end_last.unwrap_or(Vec2::ZERO));
            self.regions_size(props, body, header, self.footer_last, end).x
        })
    }

    /// The width the header, body and footer share in a content-sized container: the widest region's, at most the
    /// room; the whole room before any pass.
    fn shared_width(&self, room: f32, props: &ContainerProps) -> f32 {
        self.wanted_width(props).map_or(room.max(0.0), |wanted| wanted.min(room.max(0.0)))
    }

    /// The regions inside `inner` (the rect minus padding and the padding tier), from last pass's sizes.
    fn regions(&self, props: &ContainerProps, inner: Rect, room: Vec2, rtl: bool) -> Regions {
        let inline = props.header.is_some_and(|header| header.inline);
        let (in_end, full) = (footer_in_end(props), end_full_height(props));
        let header_last = self.header_last.unwrap_or(Vec2::ZERO);
        let header_h = if props.header.is_some() { header_last.y } else { 0.0 };
        let footer_h = if props.footer.is_some() { self.footer_last.y } else { 0.0 };
        // A footer in the end region takes no band across the container: it stacks with the end region's content.
        let across_h = if in_end { 0.0 } else { footer_h };
        let end_h = self.end_last.map_or(0.0, |end| end.y) + if in_end { footer_h } else { 0.0 };
        // An inline header takes no height of its own: the line is as high as the taller of header and body.
        let stacked_h = if inline { 0.0 } else { header_h };
        let top = inner.min.y + stacked_h;
        let content_sized = self.sizing(props) == Sizing::Content;
        let (bottom, footer_top) = if content_sized {
            let body_room = (room.y - stacked_h - across_h).max(0.0);
            // A body that fills records the band it was given, which may predate the footer's measure: the line is
            // never taller than the room less header and footer, so the footer stays inside the room.
            let body_h = (self.remembered.map_or(0.0, |size| size.y) * self.presence.t).min(body_room);
            let line_h = if inline { body_h.max(header_h) } else { body_h };
            // A full-height end region stands beside the footer too, so only the part of it under the line counts.
            let end_line = if full { (end_h - across_h).max(0.0) } else { end_h };
            (top + body_room, top + line_h.max(end_line).min(body_room))
        } else {
            // A fill-width body is offered the room the parent gives this pass, never the height its content took last
            // pass: a collection that scrolls takes its rows up to that room, and from zero it would stay at zero.
            let offered = (self.sizing(props) == Sizing::FillWidth && room.y.is_finite()).then(|| top + (room.y - stacked_h - across_h).max(0.0));
            (offered.unwrap_or(inner.max.y - across_h), inner.max.y - across_h)
        };
        let width = if content_sized { self.shared_width(room.x, props) } else { inner.width() };
        let (left, right) = if rtl { (inner.max.x - width, inner.max.x) } else { (inner.min.x, inner.min.x + width) };
        let band = |top: f32, bottom: f32| Rect::from_min_max(Pos2::new(left, top), Pos2::new(right, bottom.max(top)));
        let header = band(inner.min.y, if props.header.is_some() { inner.min.y + header_h.max(0.0) } else { inner.min.y });
        let mut body = band(top, bottom);
        let header = if inline {
            // The header keeps the width it measured, none when it measured nothing; before its first measure it
            // takes the whole line. The body takes the rest.
            let head_w = self.header_last.map_or(width, |last| last.x.min(width));
            let cut = if head_w > 0.0 { (head_w + self.region_gap).min(width) } else { 0.0 };
            if rtl {
                body.max.x = right - cut;
                Rect::from_min_max(Pos2::new(right - head_w, header.min.y), header.max)
            } else {
                body.min.x = left + cut;
                Rect::from_min_max(header.min, Pos2::new(left + head_w, header.max.y))
            }
        } else {
            header
        };
        // The end region keeps the width it measured (with a footer in it, the wider of the two) and takes the end
        // edge; the body takes the rest. Before its first measure it spans the line and the body keeps its width.
        let measured = props.end.and(self.end_last).map(|end| {
            let footer_w = if in_end { self.footer_last.x } else { 0.0 };
            end.x.max(footer_w).min(width)
        });
        let taken = measured.unwrap_or(0.0);
        let cut = if taken > 0.0 { taken + self.region_gap } else { 0.0 };
        let end_w = measured.unwrap_or(width);
        let end_bottom = if full { footer_top + across_h } else { footer_top };
        let mut end = if rtl {
            Rect::from_min_max(Pos2::new(left, top), Pos2::new(left + end_w, end_bottom.max(top)))
        } else {
            Rect::from_min_max(Pos2::new(right - end_w, top), Pos2::new(right, end_bottom.max(top)))
        };
        if rtl {
            body.min.x = (body.min.x + cut).min(body.max.x);
        } else {
            body.max.x = (body.max.x - cut).max(body.min.x);
        }
        let footer = match props.footer.map(|footer| footer.place) {
            Some(FooterPlace::EndTop) if in_end => {
                let footer = Rect::from_min_max(end.min, Pos2::new(end.max.x, (end.min.y + footer_h).min(end.max.y)));
                end.min.y = footer.max.y;
                footer
            }
            _ if in_end => {
                let footer = Rect::from_min_max(Pos2::new(end.min.x, (end.max.y - footer_h).max(end.min.y)), end.max);
                end.max.y = footer.min.y;
                footer
            }
            // Beside a full-height end region the footer lies under the body only.
            _ if full && props.end.is_some() => Rect::from_min_max(Pos2::new(body.min.x, footer_top), Pos2::new(body.max.x, footer_top + footer_h)),
            _ => band(footer_top, footer_top + footer_h),
        };
        Regions { header, body, end, footer }
    }

    /// The room a content-sized body runs in: what the mount offers, less the padding.
    fn room_size(&self, rect: Rect, inner: Rect) -> Vec2 {
        self.room - (inner.min - rect.min) * 2.0
    }

    /// The room a layer offers its content: the safe rect, or the share of it the overlay caps its content at.
    fn layer_room(ctx: &Context, overlay: &super::Overlay) -> Rect {
        let bounds = areas::safe_rect(ctx);
        match overlay.fit {
            super::overlay::OverlayFit::ContentCapped(share) => {
                Rect::from_min_size(bounds.min, Vec2::new(bounds.width() * share.x, bounds.height() * share.y))
            }
            _ => bounds,
        }
    }

    fn scroll_geometry(&self, props: &ContainerProps, view: Rect, rtl: bool, tokens: &UiTokens) -> Vec<(Bar, ScrollSpan)> {
        let Some(scroll) = scroll_of(props) else { return Vec::new() };
        let thickness = scroll_bar_look(tokens).thickness;
        let (v_on, h_on) = (scroll.vertical_on(), scroll.horizontal_on());
        let (ex, ey) = self.last.extent;
        let mut bars = Vec::new();
        if v_on {
            let track = if rtl {
                Rect::from_min_max(view.min, Pos2::new(view.min.x + thickness, view.max.y))
            } else {
                Rect::from_min_max(Pos2::new(view.max.x - thickness, view.min.y), view.max)
            };
            let content = f64::from(ey).max(f64::from(view.height()));
            let span = ScrollSpan::new(0.0..content, self.offset.y..self.offset.y + f64::from(view.height()));
            bars.push((Bar { key: SCROLL_Y, axis: Axis::Y, track }, span));
        }
        if h_on {
            let track = Rect::from_min_max(
                Pos2::new(view.min.x, view.max.y - thickness),
                Pos2::new(view.max.x - if v_on { thickness } else { 0.0 }, view.max.y),
            );
            let content = f64::from(ex).max(f64::from(view.width()));
            let span = ScrollSpan::new(0.0..content, self.offset.x..self.offset.x + f64::from(view.width()));
            bars.push((Bar { key: SCROLL_X, axis: Axis::X, track }, span));
        }
        bars
    }

    fn max_offset(&self, view: Vec2) -> ScrollPos {
        ScrollPos { x: f64::from((self.last.extent.0 - view.x).max(0.0)), y: f64::from((self.last.extent.1 - view.y).max(0.0)) }
    }

    fn clamp_offset(&mut self, scroll: Scroll, view: Vec2) {
        let max = self.max_offset(view);
        self.offset.x = if scroll.horizontal_on() { self.offset.x.clamp(0.0, max.x) } else { 0.0 };
        self.offset.y = if scroll.vertical_on() { self.offset.y.clamp(0.0, max.y) } else { 0.0 };
    }
}

impl BodyHooks<'_> {
    /// Writes the open state when it changed since it was read: to the binding, else to the session slot.
    fn write_open<C: Container>(&mut self, comp: &mut C, cx: &mut Cx<'_, C>) {
        if self.open == self.open_at_read {
            return;
        }
        let id = cx.core.id;
        let opener = opener_of(comp.props(Internal::new())).cloned();
        match comp.open() {
            Some(mut bind) => {
                let (label, doc_key) = (cx.core.label_arc(), cx.core.resolved.doc_key);
                bind.apply(&EditEvent::set(GestureId::default(), Mixed::Same(self.open_at_read), self.open, label, doc_key));
            }
            None => match &opener {
                Some(opener) => super::overlay::set_opener_open(&mut *cx.core.memory, opener, self.open),
                None => open_slot(&mut *cx.core.memory, id, self.save.as_ref()).0 = Some(self.open),
            },
        }
        self.open_at_read = self.open;
    }
}

impl<C: Container> ShapeHooks<C> for BodyHooks<'_> {
    fn shape(&self) -> super::steps::ShapeKind {
        super::steps::ShapeKind::Body
    }

    fn host(&mut self, comp: &C, at: super::run::HostCx<'_>) -> Box<dyn super::run::Host> {
        let mounted = super::mount::host(comp.props(Internal::new()), &at, self.open, at.label);
        if mounted.exiting {
            self.exiting = true;
            self.open = true;
        }
        mounted.host
    }

    fn dismissed(&mut self, comp: &mut C, cx: &mut Cx<'_, C>, why: CloseReason) {
        self.close = true;
        self.open = false;
        comp.closed(why);
        self.write_open(comp, cx);
    }

    /// The scope of a layer, full-window or viewport mount is the host's (one scope per layer, from `Overlay.focus`;
    /// a full-window mount's overlay carries `props.focus`); `Inline` and `Pane` containers push `props.focus`.
    fn focus(&self, comp: &C) -> Option<FocusPolicy> {
        let props = comp.props(Internal::new());
        if matches!(props.mount, Mount::Layer(_) | Mount::Viewport { .. }) {
            debug_assert!(
                props.focus == FocusPolicy::NONE,
                "a layer or viewport mount takes its focus scope from `Overlay.focus`, not `ContainerProps.focus`"
            );
            return None;
        }
        if matches!(props.mount, Mount::FullWindow) {
            return None;
        }
        (props.focus != FocusPolicy::NONE).then_some(props.focus)
    }

    fn arena(&self, comp: &C) -> bool {
        comp.props(Internal::new()).arena
    }

    fn expanded(&self, comp: &C) -> Option<bool> {
        comp.props(Internal::new()).collapse.map(|_| self.open)
    }

    fn roving(&self, comp: &C) -> Option<Roving> {
        comp.props(Internal::new()).roving
    }

    fn key_role(&self, comp: &C) -> KeyRole {
        KeyRole::new().reorderable(comp.props(Internal::new()).header.is_some_and(|header| header.band == DragBand::Reorder))
    }

    fn position(&self, comp: &C) -> Option<SetPosition> {
        comp.props(Internal::new()).position
    }

    fn live(&self, comp: &C) -> Option<Politeness> {
        comp.props(Internal::new()).live
    }

    fn scrolls(&self, comp: &C) -> (bool, bool) {
        scroll_of(comp.props(Internal::new())).map_or((false, false), |scroll| (scroll.vertical_on(), scroll.horizontal_on()))
    }

    fn read(&mut self, comp: &mut C, stage: &mut MemoryStage, id: Id, env: &Env) {
        let ctx = stage.ctx().clone();
        let props = comp.props(Internal::new()).clone();
        let props = &props;
        let (has_header, has_footer, scroll) = (props.header.is_some(), props.footer.is_some(), scroll_of(props));
        let collapse = props.collapse;
        let initially_open = collapse.is_none_or(|collapse| collapse.initially_open);
        let save = props.save.clone();
        self.save = save.clone();
        // The overlay host decides a layer's Escape; so does a full-window mount's.
        self.layer = matches!(props.mount, Mount::Layer(_) | Mount::Viewport { .. } | Mount::FullWindow);
        self.remembered = layout_memory::read(&ctx, id);
        self.header_last = if has_header { layout_memory::read(&ctx, sub(id, "header")) } else { None };
        self.footer_last = if has_footer { layout_memory::read(&ctx, sub(id, "footer")).unwrap_or(Vec2::ZERO) } else { Vec2::ZERO };
        self.end_last = if props.end.is_some() { layout_memory::read(&ctx, sub(id, "end")) } else { None };
        self.region_gap = gap_of(&env.tokens, Gap::Tight);
        if !matches!(props.mount, Mount::Inline | Mount::Pane(_))
            && let Some(why) = crate::runtime::overlay::take_close(&ctx, id)
        {
            self.close = true;
            self.reason = Some(why);
        }
        self.open_bound = comp.open().is_some();
        self.open = match comp.open() {
            Some(bind) => matches!(bind.get(), Some(Mixed::Same(true))),
            None => match opener_of(props) {
                Some(opener) => super::overlay::opener_open(stage, opener),
                None => *open_slot(stage, id, save.as_ref()).0.get_or_insert(initially_open),
            },
        };
        self.open_at_read = self.open;
        self.presence = match collapse {
            Some(collapse) if collapse.animated => {
                Driver { stage: &*stage, base: id, policy: env.motion }.presence("collapse", self.open, spec::OPEN_RISE)
            }
            _ => Presence { t: f32::from(u8::from(self.open)), visible: self.open, exiting: false },
        };
        self.mode = stage.session::<WindowMode>(sub(id, "window_mode")).clone();
        self.fixed = match &props.mount {
            Mount::FullWindow => true,
            Mount::Viewport { .. } if viewports::available(&ctx) => true,
            Mount::Layer(_) | Mount::Viewport { .. } => {
                let placement = overlay_of(props)
                    .map_or_else(Placement::default, |overlay| shown_placement(&ctx, id, save.as_ref(), overlay, env.tokens.space.pad.dialog));
                placement.pos.is_some() && placement.size.is_some()
            }
            _ => false,
        };
        self.last = stage.recall::<BodyView>(sub(id, "view"));
        let Some(scroll) = scroll else { return };
        self.scroll_bound = comp.scroll().is_some();
        let held = match comp.scroll() {
            Some(bind) => match bind.get() {
                Some(Mixed::Same(pos)) => pos,
                _ => ScrollPos::default(),
            },
            None => {
                let state = stage.session::<ScrollState>(sub(id, "scroll"));
                ScrollPos { x: state.offset.0, y: state.offset.1 }
            }
        };
        self.offset = held;
        self.offset_at_read = held;
        let rtl = env.direction.is_rtl();
        let view = self.last.view.map_or(Vec2::ZERO, |view| rect_of(view).size());
        // Step 4 applies, in order, this pass's wheel share and the reveal request queued last pass.
        if let Some(share) = wheel::take(&ctx, id) {
            let sign = if rtl { 1.0 } else { -1.0 };
            self.offset.x += f64::from(sign * share.scroll.x);
            self.offset.y -= f64::from(share.scroll.y);
            self.moved = true;
        }
        if let (Some(rect), Some(view_rect)) = (super::take_reveal(&ctx, id), self.last.view.map(rect_of)) {
            let into = |low: f32, high: f32, view_low: f32, view_high: f32| {
                if low < view_low {
                    low - view_low
                } else if high > view_high {
                    high - view_high
                } else {
                    0.0
                }
            };
            self.offset.y += f64::from(into(rect.min.y, rect.max.y, view_rect.min.y, view_rect.max.y));
            let dx = into(rect.min.x, rect.max.x, view_rect.min.x, view_rect.max.x);
            self.offset.x += f64::from(if rtl { -dx } else { dx });
            self.moved = true;
        }
        for mv in super::steps::take_scroll_moves(&ctx, id) {
            let (visible, extent, offset, vertical) = if scroll.vertical_on() {
                (view.y, self.last.extent.1, self.offset.y, true)
            } else {
                (view.x, self.last.extent.0, self.offset.x, false)
            };
            let span = ScrollSpan::new(0.0..f64::from(extent).max(f64::from(visible)), offset..offset + f64::from(visible));
            if let Some(range) = craft_behavior::scroll::jump(&span, mv) {
                if vertical {
                    self.offset.y = range.start;
                } else {
                    self.offset.x = range.start;
                }
                self.moved = true;
            }
        }
        let follows = stage.session::<ScrollState>(sub(id, "scroll")).follow_end.unwrap_or(true);
        self.follows_end = scroll.anchor == super::ScrollAnchor::End && follows;
        if self.follows_end && !self.moved {
            self.offset.y = f64::from((self.last.extent.1 - view.y).max(0.0));
        }
        self.clamp_offset(scroll, view);
    }

    fn overflow(&self) -> f32 {
        self.overflow
    }

    fn extent(&mut self, comp: &C, ui: &Ui, at: ShowAt, _id: Id, scx: &StyleCx<'_>) -> Vec2 {
        let props = comp.props(Internal::new());
        self.room = match (overlay_of(props), self.fixed) {
            (Some(overlay), false) => Self::layer_room(ui.ctx(), overlay).size(),
            _ => match at {
                ShowAt::Rect(rect) => rect.size(),
                ShowAt::Flow => crate::layout::flow_room(ui),
            },
        };
        let remembered = self.remembered.unwrap_or(Vec2::ZERO);
        let body = if self.presence.visible { Vec2::new(remembered.x, remembered.y * self.presence.t) } else { Vec2::ZERO };
        let room = self.room.x - 2.0 * pad_of(scx.tokens, props.pad);
        let width = if self.remembered.is_some() { self.shared_width(room, props) } else { 0.0 };
        self.overflow = self.wanted_width(props).map_or(0.0, |wanted| (wanted - room.max(0.0)).max(0.0));
        let (header, end) = (self.header_last.unwrap_or(Vec2::ZERO), self.end_last.unwrap_or(Vec2::ZERO));
        let height = self.regions_size(props, body, header, self.footer_last, end).y;
        Vec2::new(width, height) + Vec2::splat(2.0 * pad_of(scx.tokens, props.pad))
    }

    fn sized(&self, comp: &C, ui: &Ui, size: Vec2) -> Vec2 {
        match self.sizing(comp.props(Internal::new())) {
            Sizing::Fill => {
                let room = crate::layout::flow_room(ui);
                // The available rect, not at least it: a fill child that took the whole room in an earlier pass (a
                // content-sized layer offers its whole room before the footer is known) must not keep its remembered
                // size once the room is less.
                Vec2::new(if room.x.is_finite() { room.x } else { size.x }, if room.y.is_finite() { room.y } else { size.y })
            }
            Sizing::FillWidth => {
                let room = crate::layout::flow_room(ui).x;
                Vec2::new(if room.is_finite() { size.x.max(room) } else { size.x }, size.y)
            }
            Sizing::Content => size,
        }
    }

    fn fills_width(&self, comp: &C) -> bool {
        self.sizing(comp.props(Internal::new())) != Sizing::Content
    }

    fn parts(&mut self, comp: &mut C, rect: Rect, pcx: &mut PartsCx<'_, C>) {
        let props = comp.props(Internal::new()).clone();
        let ctx = pcx.ui.ctx().clone();
        let rtl = pcx.flow.is_rtl();
        let tokens = pcx.tokens;
        let inner = pcx.content_rect.shrink(pad_of(tokens, props.pad));
        let room = self.room_size(rect, inner);
        let regions = self.regions(&props, inner, room, rtl);
        let scroll = scroll_of(&props);
        if self.open && scroll.is_some() {
            let view = regions.body;
            self.view = Some(view);
            if let Some(scroll) = scroll {
                self.clamp_offset(scroll, view.size());
            }
            for (bar, span) in self.scroll_geometry(&props, view, rtl, tokens) {
                pcx.scroll(bar.key, bar.track, bar.axis, span);
            }
            if let Some(scroll) = scroll {
                let notch = match scroll.notch {
                    Notch::Lines(lines) => WheelNotch::Points(lines * tokens.tokens().typography.body.line_height),
                    Notch::ViewShare(share) => WheelNotch::ViewShare(share),
                };
                wheel::register(&ctx, WheelTaker::new(pcx.id, view).vertical(scroll.vertical_on()).horizontal(scroll.horizontal_on()).notch(notch));
            }
        }
        self.nudge = gap_of(tokens, Gap::Row);
        // The reorder band lies under the header's own parts and children: its default priority loses to theirs, so
        // their buttons keep their clicks.
        if props.header.is_some_and(|header| header.band == DragBand::Reorder) && regions.header.height() > 0.0 {
            pcx.part(REORDER_BAND, regions.header)
                .sense(craft_behavior::Sense::DRAG)
                .cursor(CursorIcon::Grab)
                .drag_cursor(CursorIcon::Grabbing)
                .pointer_only("Alt+Up and Alt+Down move the section");
        }
        Self::move_parts(&props, &regions, rect, pcx);
        self.pane_grip = Self::pane_grip(&props, rect, pcx);
    }

    fn sensed(&mut self, comp: &mut C, cx: &mut Cx<'_, C>) {
        self.reorder_keys(comp.props(Internal::new()), cx);
    }

    fn updated(&mut self, comp: &mut C, cx: &mut Cx<'_, C>) {
        use super::Pointer as _;
        let props = comp.props(Internal::new());
        let (scroll, layer) = (scroll_of(props), matches!(props.mount, Mount::Layer(_) | Mount::Viewport { .. }));
        // The OS asked a viewport mount's window to close: the container closes at step 15.
        if matches!(props.mount, Mount::Viewport { .. }) && crate::runtime::windows::take_close_request(cx.core.memory.ctx(), cx.core.id) {
            self.close = true;
            self.reason = self.reason.or(Some(CloseReason::Requested));
        }
        if scroll.is_some() {
            if let Some(range) = cx.scroll_moved(SCROLL_Y) {
                self.offset.y = range.start;
                self.moved = true;
            }
            if let Some(range) = cx.scroll_moved(SCROLL_X) {
                self.offset.x = range.start;
                self.moved = true;
            }
            self.access_scroll(props, cx);
        }
        // AccessKit Expand, Collapse and Click reach a collapsible container as an activation: it folds or unfolds.
        if props.collapse.is_some() && cx.activation().is_some_and(|activation| activation.by_key) {
            self.open = !self.open;
        }
        self.band_double(cx);
        let band = props.header.is_some_and(|header| header.band == DragBand::Move);
        if (band || matches!(props.mount, Mount::Viewport { .. } | Mount::Window { .. }))
            && let Some(viewport) = os_window(props, cx.core.memory.ctx(), cx.core.id)
        {
            self.window_os(viewport, cx);
            self.window_movable(cx);
        }
        self.pane_drag(props, cx);
        if layer && !(matches!(props.mount, Mount::Viewport { .. }) && viewports::available(cx.core.memory.ctx())) {
            self.window_keys(props, cx);
            self.window_drag(props, cx);
        }
        for change in std::mem::take(&mut self.reports) {
            comp.placement_changed(&change);
        }
    }

    fn content(&mut self, comp: &mut C, ui: &mut Ui, env: &Env, cx: &mut Cx<'_, C>, values: &AppValues) {
        let props = comp.props(Internal::new()).clone();
        let (id, ctx, rect) = (cx.core.id, cx.core.ctx.clone(), cx.core.rect);
        let rtl = env.direction.is_rtl();
        let node = matches!(C::ROLE, Role::Node(_));
        let scroll = scroll_of(&props);
        let pad = pad_of(&env.tokens, props.pad);
        let inner = cx.core.content_rect.shrink(pad);
        let room = self.room_size(rect, inner);
        let regions = self.regions(&props, inner, room, rtl);
        let surface_slot = ui.painter().add(Shape::Noop);
        self.gate = cx.shown.as_deref().and_then(|value| (value as &dyn Any).downcast_ref::<bool>()).copied();
        self.gate_at_read = self.gate;
        let window = window_instance(&props, id).filter(|window| *window != id);
        register_instances(&ctx, &props, id, window);
        let region_env = env.derive(|scoped| {
            // The container's own `enabled(false)` greys everything inside it, as a scope would.
            scoped.enabled &= cx.core.resolved.enabled;
            if let Some(window) = window {
                scoped.commands = scoped.commands.pushed(WindowCommand::SCOPE, window);
            }
            scoped.values.extend(values);
            scoped.ink = cx.core.ink;
            scoped.grid = None;
            scoped.content_column = false;
            if props.header.is_some_and(|header| header.band == DragBand::Reorder) {
                scoped.commands = scoped.commands.pushed(REORDER_SCOPE, id);
            }
            if let Some(scope) = props.command_scope {
                scoped.commands = scoped.commands.pushed(scope, id);
            }
            if let Some(theme) = &props.theme {
                scoped.tokens = theme.tokens().clone();
            }
        });
        let body_id = sub(id, "body");
        let owner = BodyOwner::new(cx.style, gap_of(&env.tokens, props.body_gap));
        let frame = RegionFrame { id, rtl, node, env: &region_env };
        let (resolved, env_view) = (cx.core.resolved, region_env.view());
        let mut open = self.open;
        let mut gate = self.gate;
        let mut close: Option<CloseReason> = None;
        let field_message = if props.label.is_some() { scope::last_message(&ctx, id) } else { None };
        // B1 header
        if props.header.is_some() {
            let mut bcx = BodyCx::new(body_id, env_view, resolved, regions.header, &mut *cx.core.memory, &mut open, gate.as_mut())
                .with_field_message(field_message.clone())
                .with_owner(owner);
            let trailing = props.header.is_some_and(|header| header.trailing);
            close = close.or(self.header_region(comp, ui, &mut bcx, &frame, regions.header, trailing));
        }
        crate::runtime::focus::begin_body(&ctx);
        // B2 label: the control gets the rect the label leaves; the label column adds to the container's extent.
        let mut body_rect = regions.body;
        if let Some(side) = props.label.as_ref() {
            let resolved = cx.core.resolved;
            let input = LabelIn {
                scope: id,
                owner: id,
                parent: node.then_some(id),
                row: regions.body,
                room: if self.sizing(&props) == Sizing::Content {
                    layout::ambient_room(&ctx).map_or(room.x, |offered| room.x.max(offered))
                } else {
                    regions.body.width()
                },
                text: resolved.label_value,
                side,
                help: props.help.as_ref(),
                enabled: resolved.enabled,
                size: resolved.size,
                control_min: scope::last_control_min(&ctx, id),
            };
            body_rect = label::run(ui, env, &input).control;
        }
        let label_extra = (regions.body.size() - body_rect.size()).max(Vec2::ZERO);
        // B3 body, if open, under the body env; the gate is read after the header changed it.
        let animated = props.collapse.is_some_and(|collapse| collapse.animated);
        let is_open = open || (animated && self.presence.visible);
        let enabled = gate.unwrap_or(true);
        if is_open {
            // What the body and the end region share: gate, size, roving group and commands. The body adds its arena,
            // transform and scroll owner; the end region lies outside them.
            let line_env = region_env.derive_local(|scoped| {
                scoped.enabled &= enabled;
                if let Some(size) = props.child_size {
                    scoped.size = size;
                }
                if props.roving.is_some() {
                    scoped.roving = Some(super::steps::body_group(id));
                }
            });
            // A held fill-width container is part of the content column it sits in: its own stack is one too.
            let content_column =
                (self.sizing(&props) == Sizing::Content || self.held(&props, env)) && matches!(props.layout, layout::BodyLayout::Stack);
            let body_env = line_env.derive_local(|scoped| {
                if props.arena {
                    scoped.arena = Some(ArenaId::new(id));
                }
                scoped.transform = props.transform;
                scoped.content_column = content_column;
                if scroll.is_some() {
                    scoped.scroll_owner = Some(id);
                }
            });
            let mut bcx = BodyCx::new(body_id, body_env.view(), resolved, body_rect, &mut *cx.core.memory, &mut open, gate.as_mut())
                .with_field_message(field_message.clone())
                .with_owner(owner);
            let gap = Vec2::splat(gap_of(&env.tokens, props.body_gap));
            let layout_id = sub(id, "layout");
            // While the body opens or closes, it shows only the part of its last height that the presence reaches.
            let reveal_clip = (animated && self.presence.t < 1.0).then(|| {
                Rect::from_min_max(
                    body_rect.min,
                    Pos2::new(body_rect.max.x, body_rect.min.y + self.remembered.map_or(0.0, |size| size.y) * self.presence.t),
                )
            });
            let (layout, body) = (&props.layout, &mut *self.body);
            let label_column = props.label_column;
            let mut run = |parent: &mut Ui, at: Rect, bcx: &mut BodyCx<'_>| -> Option<Vec2> {
                let mut body_ui = alloc::child(parent, {
                    let builder = UiBuilder::new().max_rect(at).layout(child::column_layout(rtl)).scope_id(body_id);
                    if node { access::with_parent(builder, id) } else { builder }
                });
                style::set_item_spacing(&mut body_ui, gap);
                if let Some(clip) = reveal_clip {
                    body_ui.set_clip_rect(clip.intersect(body_ui.clip_rect()));
                }
                let installed = Env::install_for(&body_ui, body_env.clone());
                // The size the body took: a scrolling body takes the visible part of its content, since its region
                // is a child that allocates nothing in `body_ui`.
                let took = match scroll {
                    Some(scroll) => {
                        let off = self.offset;
                        let big = super::MAX_EXTENT;
                        let bar = scroll_bar_look(&body_env.tokens).thickness;
                        let inset = Vec2::new(
                            if scroll.vertical_on() && f64::from(self.last.extent.1) > f64::from(at.height()) + 0.5 { bar } else { 0.0 },
                            if scroll.horizontal_on() && f64::from(self.last.extent.0) > f64::from(at.width()) + 0.5 { bar } else { 0.0 },
                        );
                        let seen = Rect::from_min_max(
                            Pos2::new(if rtl { at.min.x + inset.x } else { at.min.x }, at.min.y),
                            Pos2::new(if rtl { at.max.x } else { at.max.x - inset.x }, at.max.y - inset.y),
                        );
                        let (width, height) =
                            (if scroll.horizontal_on() { big } else { seen.width() }, if scroll.vertical_on() { big } else { seen.height() });
                        let x = if scroll.horizontal_on() { off.x as f32 } else { 0.0 };
                        let min_x = if rtl { seen.max.x - width + x } else { seen.min.x - x };
                        let max = Rect::from_min_size(Pos2::new(min_x, seen.min.y - off.y as f32), Vec2::new(width, height));
                        let layout_of = *body_ui.layout();
                        let mut region_ui =
                            alloc::child(&mut body_ui, UiBuilder::new().scope_id(sub(id, "scroll_region")).layout(layout_of).max_rect(max));
                        region_ui.set_clip_rect(at.intersect(body_ui.clip_rect()));
                        layout::run_body(&mut region_ui, layout_id, layout, gap, |ui| {
                            shared_labels(ui, id, label_column, room.x, |ui| body(ui, bcx))
                        });
                        let extent = region_ui.min_rect().size();
                        crate::runtime::egui::widgets::note_scroll_region(&ctx, region_ui.clip_rect(), region_ui.min_rect().intersect(max));
                        self.extent = Some(extent);
                        // Along an axis it scrolls the body took what the view shows; along the other it took its content.
                        let took = extent + inset;
                        Some(Vec2::new(
                            if scroll.horizontal_on() { took.x.min(at.width()) } else { took.x },
                            if scroll.vertical_on() { took.y.min(at.height()) } else { took.y },
                        ))
                    }
                    // A row that places its block itself asks for its children's width, not the room it spans.
                    None => {
                        let (_, natural) = layout::run_body_measured(&mut body_ui, layout_id, layout, gap, |ui| {
                            shared_labels(ui, id, label_column, room.x, |ui| body(ui, bcx))
                        });
                        natural.map(|width| Vec2::new(width, body_ui.min_rect().height()))
                    }
                };
                Env::restore(&ctx, installed);
                Some(took.unwrap_or_else(|| body_ui.min_rect().size()))
            };
            self.body_size = match props.transform {
                Some(transform) => {
                    let parent = ui.layer_id();
                    let layer = areas::body_layer(parent, id);
                    let tf = areas::body_transform(body_rect.min, transform.scale, transform.offset);
                    if props.arena {
                        crate::primitives::sensing::set_layer(&ctx, ArenaId::new(id), layer);
                    }
                    let forward = areas::Forward {
                        id: sub(id, "transform"),
                        sense: response::to_egui(cx.core.root_sense),
                        rect: rect.intersect(ui.clip_rect()),
                    };
                    let body_layer = areas::BodyLayer {
                        parent,
                        layer,
                        transform: tf,
                        at: body_rect.min,
                        clip: body_rect.intersect(ui.clip_rect()),
                        forward: Some(forward),
                    };
                    areas::show_body_layer(&ctx, body_layer, |area_ui| run(area_ui, body_rect, &mut bcx))
                }
                None => run(ui, body_rect, &mut bcx),
            };
            close = close.or(bcx.close_requested());
            if let Some(size) = &mut self.body_size {
                *size += label_extra;
            }
            self.view = Some(body_rect);
            if props.end.is_some() {
                let mut end_cx = BodyCx::new(body_id, line_env.view(), resolved, regions.end, &mut *cx.core.memory, &mut open, gate.as_mut())
                    .with_field_message(field_message.clone())
                    .with_owner(owner);
                // The content runs in reading order, so its roving order is its order on the line; the region is as
                // wide as it measured, so the last child sits at the end edge. On the line it is centred; a
                // full-height region starts at its top.
                let cross = if end_full_height(&props) { Align::Min } else { Align::Center };
                let layout = if rtl { Layout::right_to_left(cross) } else { Layout::left_to_right(cross) };
                let builder = UiBuilder::new().max_rect(regions.end).layout(layout).scope_id(sub(id, "end"));
                let builder = if node { access::with_parent(builder, id) } else { builder };
                let mut end_ui = alloc::child(ui, builder);
                let installed = Env::install_for(&end_ui, line_env.clone());
                comp.end(&mut end_ui, &mut end_cx);
                Env::restore(&ctx, installed);
                close = close.or(end_cx.close_requested());
                self.end_size = Some(self.region_size(&end_ui));
            }
        } else {
            // A collapsed body keeps its last full size in layout memory, so opening it again places by that size
            // and asks for no discard pass.
            self.body_size = if props.collapse.is_some() { None } else { Some(Vec2::ZERO) };
        }
        // The label scope covers the body only: footer controls are not the field's controls.
        if props.label.is_some() {
            scope::finish(&ctx, id, None);
        }
        // B4 footer: under the control column of a label-scoped container, which starts where the label step put
        // the control.
        if props.footer.is_some() {
            let mut footer_rect = regions.footer;
            if props.label.is_some() {
                if rtl {
                    footer_rect.max.x = body_rect.max.x.clamp(footer_rect.min.x, footer_rect.max.x);
                } else {
                    footer_rect.min.x = body_rect.min.x.clamp(footer_rect.min.x, footer_rect.max.x);
                }
            }
            let offset = if rtl { regions.footer.max.x - footer_rect.max.x } else { footer_rect.min.x - regions.footer.min.x };
            let mut bcx = BodyCx::new(body_id, env_view, resolved, footer_rect, &mut *cx.core.memory, &mut open, gate.as_mut())
                .with_field_message(field_message)
                .with_owner(owner);
            let mut footer_ui = alloc::child(ui, frame.region("footer", footer_rect));
            // A footer in the end region is part of a column sized from its content: a fill-width child takes its
            // measured width there and shows across the column, so it never sets the column's width.
            let footer_env = if footer_in_end(&props) { region_env.derive_local(|scoped| scoped.content_column = true) } else { region_env.clone() };
            let installed = Env::install_for(&footer_ui, footer_env);
            comp.footer(&mut footer_ui, &mut bcx);
            Env::restore(&ctx, installed);
            close = close.or(bcx.close_requested());
            let mut took = self.region_size(&footer_ui);
            // A footer that took the whole room records no width (see `region_size`); one that is known adds the
            // column it starts after.
            if took.x > 0.0 {
                took.x += offset;
            }
            self.footer_size = Some(took);
        }
        self.open = open;
        self.gate = gate;
        if close.is_some() {
            self.close = true;
            self.reason = self.reason.or(close);
        }
        // The scroll bars, over the body.
        if is_open && scroll.is_some() {
            let look = scroll_bar_look(&env.tokens);
            if let Some(view) = self.view {
                for (bar, span) in self.scroll_geometry(&props, view, rtl, &env.tokens) {
                    let overflow = span.content.end - span.content.start > span.visible.end - span.visible.start;
                    if !overflow {
                        continue;
                    }
                    let length = match bar.axis {
                        Axis::X => bar.track.width(),
                        _ => bar.track.height(),
                    };
                    let (at, len) = craft_behavior::scroll::thumb(&span, length, look.min_thumb);
                    let thumb = super::cx::thumb_rect(bar.track, bar.axis, rtl, at, len);
                    let enabled = cx.core.resolved.enabled;
                    let state = super::steps::part_state(
                        cx.core.results.get(&(bar.key, Some(super::cx::THUMB))).unwrap_or(&crate::primitives::PartResult::IDLE),
                        enabled,
                        None,
                    );
                    ui.painter().extend(ScrollBar::shapes(&look, bar.track, thumb, &state));
                }
            }
        }
        // The surface at the final rect, then the rules.
        if let Some(kind) = props.surface {
            let shapes = Surface::shapes(&surface_style(&env.tokens, kind), None, rect, Corners::All, env.direction, ctx.pixels_per_point());
            ui.painter().set(surface_slot, Shape::Vec(shapes));
        }
        let rule = rule_look(&env.tokens, RuleKind::Divider);
        let ppp = ctx.pixels_per_point();
        self.claim_extent(&props, ui, env, rect, pad, rtl);
        let draw_rule = |from: f32, to: f32, y: f32| ui.painter().extend(Rule::shapes(&rule, Pos2::new(from, y), Pos2::new(to, y), ppp));
        if props.header.is_some_and(|header| header.rule && !header.inline) {
            draw_rule(inner.min.x, inner.max.x, regions.header.max.y);
        }
        // A footer across the container rules the whole width; one beside or in the end region rules its own.
        match props.footer.filter(|footer| footer.rule) {
            Some(footer) if footer.place == FooterPlace::EndTop && footer_in_end(&props) => {
                draw_rule(regions.footer.min.x, regions.footer.max.x, regions.footer.max.y);
            }
            Some(_) if end_full_height(&props) => draw_rule(regions.footer.min.x, regions.footer.max.x, regions.footer.min.y),
            Some(_) => draw_rule(inner.min.x, inner.max.x, regions.footer.min.y),
            None => {}
        }
    }

    fn settle(&mut self, comp: &mut C, cx: &mut Cx<'_, C>) {
        use super::Edit as _;
        if self.gate != self.gate_at_read
            && let Some(gate) = self.gate
            && let Ok(value) = (Box::new(gate) as Box<dyn Any>).downcast::<super::ValueOf<C>>()
        {
            cx.set(*value);
            // A viewport mount settles once per child pass: the baseline moves so a change applies once.
            self.gate_at_read = self.gate;
        }
        let id = cx.core.id;
        let (collapsible, policy) = {
            let props = comp.props(Internal::new());
            (props.collapse.is_some(), props.focus)
        };
        // A collapse is the user's own toggle: Escape never folds an inline container, only closes one with an open state.
        let closable = !collapsible && comp.open().is_some();
        if !self.layer
            && closable
            && policy.cancel != CancelRule::Ignore
            && crate::runtime::focus::focus_in_scope(&cx.core.ctx, id) == Some(true)
            && let Some(craft_behavior::focus::EscapeStep::CloseScope) =
                keys::scope_escape(&cx.core.ctx, policy.cancel, pointer::press_captured(&cx.core.ctx), false)
        {
            self.close = true;
            self.reason = self.reason.or(Some(CloseReason::Escape));
        }
        if self.exiting {
            // The container closed already; the fade only keeps its body on screen.
            self.open = self.open_at_read;
            self.close = false;
        }
        if self.close {
            self.open = false;
            comp.closed(self.reason.take().unwrap_or(CloseReason::Requested));
        }
        self.write_open(comp, cx);
        if self.offset != self.offset_at_read {
            match comp.scroll() {
                Some(mut bind) => {
                    let (label, doc_key) = (cx.core.label_arc(), cx.core.resolved.doc_key);
                    bind.apply(&EditEvent::set(GestureId::default(), Mixed::Same(self.offset_at_read), self.offset, label, doc_key));
                }
                None => {
                    let state = cx.core.memory.session::<ScrollState>(sub(cx.core.id, "scroll"));
                    state.offset = (self.offset.x, self.offset.y);
                }
            }
            self.offset_at_read = self.offset;
        }
    }

    fn record(&mut self, stage: &mut MemoryStage, ctx: &Context, id: Id) {
        // A layer is shown as soon as it opens, so a region that took another size than the pass placed by asks for
        // the discard pass at once, even when its size changed in the last frame too.
        if self.layer {
            if self.regions_moved(measure_tolerance(crate::runtime::egui::scale::pixels_per_point(ctx))) {
                layout_memory::ask_discard(ctx);
            }
        }
        if let Some(size) = self.body_size {
            super::steps::record_size(ctx, id, size);
        }
        if let Some(size) = self.header_size {
            super::steps::record_size(ctx, sub(id, "header"), size);
        }
        if let Some(size) = self.footer_size {
            super::steps::record_size(ctx, sub(id, "footer"), size);
        }
        if let Some(size) = self.end_size {
            super::steps::record_size(ctx, sub(id, "end"), size);
        }
        if self.extent.is_some() || self.view.is_some() {
            let view = stage.shown::<BodyView>(sub(id, "view"));
            if let Some(extent) = self.extent {
                // A bottom-anchored region that follows the end shows the rows that arrived this pass in this
                // frame: the repeated pass starts from the new extent.
                if self.follows_end && !self.moved && (extent.y - view.extent.1).abs() > 0.5 {
                    layout_memory::ask_discard(ctx);
                }
                view.extent = (extent.x, extent.y);
            }
            view.view = self.view.map(|view| [view.min.x, view.min.y, view.max.x, view.max.y]);
        }
        if *stage.session::<WindowMode>(sub(id, "window_mode")) != self.mode {
            *stage.session::<WindowMode>(sub(id, "window_mode")) = self.mode.clone();
        }
        if self.moved {
            let at_end = self.last.view.map(rect_of).is_some_and(|view| self.offset.y >= f64::from(self.last.extent.1 - view.height()) - 0.5);
            stage.session::<ScrollState>(sub(id, "scroll")).follow_end = Some(at_end);
        }
    }
}

/// Runs `body` in a column group named `labels` under the container `id` when `shared` is set, so the labelled
/// rows directly inside share the widest label. Otherwise the body runs behind a barrier: the labelled rows of a
/// nested container read no group of the container around it.
fn shared_labels<R>(ui: &mut Ui, id: Id, shared: bool, room: f32, body: impl FnOnce(&mut Ui) -> R) -> R {
    if shared {
        ColumnGroup::scope(ui, sub(id, "labels"), |ui, group| {
            group.offer(room);
            body(ui)
        })
    } else {
        ColumnGroup::barrier(ui, body)
    }
}
