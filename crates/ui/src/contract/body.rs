//! The body step (16.3): the regions of a container (header, body with its scroll region, footer), open and
//! gate state, scroll offset, move and resize, and the zoomed canvas body.

use super::cx::BodyCx;
use super::label::{self, LabelIn};
use super::run::{ShapeHooks, ShowAt};
use super::{Container, ContainerProps, Cx, Gap, Internal, Mount, Notch, Pad, PartsCx, Scroll, Sizing};
use crate::layout;
use crate::primitives::{Rule, ScrollBar, Surface};
use crate::runtime::access::scope;
use crate::runtime::access::{self, Politeness, Role};
use crate::runtime::commands::{self, CommandSet as _, WindowCommand};
use crate::runtime::egui::{alloc, areas, child, response, style, viewports};
use crate::runtime::env::{AppValues, ArenaId, Env};
use crate::runtime::focus::FocusPolicy;
use crate::runtime::memory::{MemoryStage, SaveKey, sub};
use crate::runtime::motion::{Driver, Presence, spec};
use crate::runtime::overlay::CloseReason;
use crate::runtime::{keys, layout_memory, pointer, wheel};
use crate::style::ComponentStyle as _;
use crate::style::{Corners, RuleKind, StyleCx, UiTokens, rule_look, scroll_bar_look, surface_style};
use craft_behavior::{
    Axis, CancelRule, CursorIcon, EditEvent, GestureId, HitLevel, HitPriority, Mixed, PartKey, PressPhase, Roving, ScrollPos, ScrollSpan, WheelNotch,
    WheelTaker,
};
use craft_keymap::{Chord, Key, Mods};
use egui::{Context, Id, Pos2, Rect, Shape, Ui, UiBuilder, Vec2};
use serde::{Deserialize, Serialize};
use std::any::Any;

const SCROLL_Y: PartKey = PartKey::new("body_scroll_y");
const SCROLL_X: PartKey = PartKey::new("body_scroll_x");
pub(crate) const MOVE_BAND: PartKey = PartKey::new("move_band");
const GRIP: PartKey = PartKey::new("resize_grip");

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

pub(crate) fn placement_id(id: Id) -> Id {
    sub(id, "placement")
}

/// The one store of a layer's placement: the saved tier when `save` is set, else shown memory.
fn placement_slot<'s>(stage: &'s mut MemoryStage, id: Id, save: Option<&SaveKey>) -> &'s mut Placement {
    match save {
        Some(key) => stage.saved::<Placement>(key),
        None => stage.shown::<Placement>(placement_id(id)),
    }
}

/// The placement of `id` as the last pass left it, read without opening a stage: the layer host places by it before
/// the container's own pass runs.
pub(crate) fn placement_of(ctx: &Context, id: Id, save: Option<&SaveKey>) -> Placement {
    match save {
        Some(key) => MemoryStage::peek_saved(ctx, key),
        None => MemoryStage::peek(ctx, placement_id(id)),
    }
}

/// The overlay a layer-like mount shows its container in.
fn overlay_of(props: &ContainerProps) -> Option<&super::Overlay> {
    match &props.mount {
        Mount::Layer(overlay) | Mount::Viewport { fallback: overlay } => Some(overlay),
        _ => None,
    }
}

/// The instance id of the `window` command scope a movable or resizable layer container holds (16.3), whatever
/// scope its own commands use: the container's id when it publishes the window scope itself, else a sub-id.
fn window_instance(props: &ContainerProps, id: Id) -> Option<Id> {
    let overlay = overlay_of(props)?;
    (overlay.movable || overlay.resize.grips).then(|| if props.command_scope == Some(WindowCommand::SCOPE) { id } else { sub(id, "window") })
}

/// The scroll region of a container: its own, or for a layer the vertical one that takes the overflow of any fit.
fn scroll_of(props: &ContainerProps) -> Option<Scroll> {
    props.scroll.or_else(|| overlay_of(props).map(|_| Scroll::vertical()))
}

/// The keyboard mode of a floating container (16.3): arrows move or resize it until Enter or Escape.
#[derive(Clone, Copy, PartialEq)]
enum WindowModeKind {
    Move,
    Size,
}

/// A container's window mode with the placement Escape restores. Kept in session memory.
#[derive(Clone, Default, PartialEq)]
struct WindowMode {
    kind: Option<WindowModeKind>,
    restore: Placement,
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
    footer: Rect,
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
    /// How many blocks had been placed without a size entry once this container's own placement was counted.
    unknown_at_read: u64,
    header_last: Vec2,
    footer_last: Vec2,
    /// The room the mount offers, in which a content-sized body runs.
    room: Rect,
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

    /// The `window.Move` and `window.Size` commands and the keys of the mode they start (16.3): arrows move or
    /// resize by a step (Shift: four steps), Enter or the command again keeps the placement, Escape restores it,
    /// and focus leaving the container ends the mode. The mode holds its chords through the key stage, so no
    /// component under focus sees them.
    fn window_keys<C: Container>(&mut self, props: &ContainerProps, cx: &mut Cx<'_, C>) {
        let (id, ctx) = (cx.core.id, cx.core.ctx.clone());
        let Some(window) = window_instance(props, id) else { return };
        let started = commands::take_for_instance(&ctx, Some(window), WindowCommand::from_ref).map(|(command, _)| command);
        let slot = placement_slot(&mut *cx.core.memory, id, self.save.as_ref()).clone();
        let mut mode = self.mode.clone();
        if let Some(command) = started {
            let kind = match command {
                WindowCommand::Move => WindowModeKind::Move,
                _ => WindowModeKind::Size,
            };
            mode = if mode.kind == Some(kind) { WindowMode::default() } else { WindowMode { kind: Some(kind), restore: slot.clone() } };
        }
        if mode.kind.is_some() && !commands::focus_within(&ctx, id) {
            mode = WindowMode::default();
        }
        if let Some(kind) = mode.kind {
            let step = self.nudge;
            let min = cx.style.metrics().min_size;
            let rect = cx.core.rect;
            let mut placement = slot;
            for (key, direction) in MODE_ARROWS {
                for (mods, scale) in [(Mods::NONE, 1.0), (Mods::SHIFT, 4.0)] {
                    let chord = Chord::new(mods, key);
                    keys::capture(&ctx, id, chord);
                    while keys::take_claimed(&ctx, id, chord) {
                        let delta = Vec2::new(f32::from(direction[0]), f32::from(direction[1])) * step * scale;
                        let mut pos = placement.pos.map_or(rect.min, Pos2::from);
                        let mut size = placement.size.map_or(rect.size(), Vec2::from);
                        match kind {
                            WindowModeKind::Move => pos += delta,
                            WindowModeKind::Size => size = (size + delta).max(min),
                        }
                        placement = Placement { pos: Some(pos.into()), size: Some(size.into()) };
                    }
                }
            }
            for key in [Key::Enter, Key::Escape] {
                let chord = Chord::new(Mods::NONE, key);
                keys::capture(&ctx, id, chord);
                if keys::take_claimed(&ctx, id, chord) {
                    if key == Key::Escape {
                        placement = mode.restore.clone();
                    }
                    mode = WindowMode::default();
                }
            }
            *placement_slot(&mut *cx.core.memory, id, self.save.as_ref()) = placement;
        }
        self.mode = mode;
    }
}

impl<'b> BodyHooks<'b> {
    pub(super) fn new(body: &'b mut dyn FnMut(&mut Ui, &mut BodyCx<'_>)) -> Self {
        BodyHooks {
            body,
            remembered: None,
            unknown_at_read: 0,
            header_last: Vec2::ZERO,
            footer_last: Vec2::ZERO,
            room: Rect::NOTHING,
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
            extent: None,
            view: None,
            save: None,
            layer: false,
            presence: Presence { t: 1.0, visible: true, exiting: false },
            nudge: 0.0,
            mode: WindowMode::default(),
            fixed: false,
            follows_end: false,
        }
    }

    /// How the container's size follows its content here: a pane or a rect the host gives is filled; a layer sizes
    /// from its content until the user sizes it; an inline container follows its props.
    fn sizing(&self, props: &ContainerProps) -> Sizing {
        match &props.mount {
            Mount::Pane(_) => Sizing::Fill,
            Mount::Layer(_) | Mount::Viewport { .. } | Mount::FullWindow if self.fixed => Sizing::Fill,
            Mount::Layer(_) | Mount::Viewport { .. } => Sizing::Content,
            _ => props.sizing,
        }
    }

    /// What a header or footer took. Before the body's width is known a region runs in the whole room and a `Fill`
    /// child takes all of it, so no width is recorded then.
    fn region_size(&self, ui: &Ui) -> Vec2 {
        let size = ui.min_rect().size();
        if self.remembered.is_some() { size } else { Vec2::new(0.0, size.y) }
    }

    /// The width the header, body and footer share in a content-sized container: what the regions took last pass,
    /// at most the room; the whole room before any pass.
    fn shared_width(&self, room: f32) -> f32 {
        match self.remembered {
            Some(body) => body.x.max(self.header_last.x).max(self.footer_last.x).min(room.max(0.0)),
            None => room.max(0.0),
        }
    }

    /// The regions inside `inner` (the rect minus padding and the padding tier), from last pass's sizes.
    fn regions(&self, props: &ContainerProps, inner: Rect, room: Vec2, rtl: bool) -> Regions {
        let header_h = if props.header.is_some() { self.header_last.y } else { 0.0 };
        let footer_h = if props.footer.is_some() { self.footer_last.y } else { 0.0 };
        let top = inner.min.y + header_h;
        let content_sized = self.sizing(props) == Sizing::Content;
        let (bottom, footer_top) = if content_sized {
            let body_h = self.remembered.map_or(0.0, |size| size.y) * self.presence.t;
            (top + (room.y - header_h - footer_h).max(0.0), top + body_h)
        } else {
            (inner.max.y - footer_h, inner.max.y - footer_h)
        };
        let width = if content_sized { self.shared_width(room.x) } else { inner.width() };
        let (left, right) = if rtl { (inner.max.x - width, inner.max.x) } else { (inner.min.x, inner.min.x + width) };
        let band = |top: f32, bottom: f32| Rect::from_min_max(Pos2::new(left, top), Pos2::new(right, bottom.max(top)));
        Regions {
            header: band(inner.min.y, if props.header.is_some() { inner.min.y + header_h.max(0.0) } else { inner.min.y }),
            body: band(top, bottom),
            footer: band(footer_top, footer_top + footer_h),
        }
    }

    /// The room a content-sized body runs in: what the mount offers, less the padding.
    fn room_size(&self, rect: Rect, inner: Rect) -> Vec2 {
        self.room.size() - (inner.min - rect.min) * 2.0
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
        match comp.open() {
            Some(mut bind) => {
                let (label, doc_key) = (cx.core.label_arc(), cx.core.resolved.doc_key);
                bind.apply(&EditEvent::set(GestureId::default(), Mixed::Same(self.open_at_read), self.open, label, doc_key));
            }
            None => open_slot(&mut *cx.core.memory, id, self.save.as_ref()).0 = Some(self.open),
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
        if self.remembered.is_none() && matches!(props.mount, Mount::Inline) && props.sizing == Sizing::Content {
            layout_memory::note_unknown(&ctx);
        }
        self.unknown_at_read = layout_memory::unknown_count(&ctx);
        self.header_last = if has_header { layout_memory::read(&ctx, sub(id, "header")).unwrap_or(Vec2::ZERO) } else { Vec2::ZERO };
        self.footer_last = if has_footer { layout_memory::read(&ctx, sub(id, "footer")).unwrap_or(Vec2::ZERO) } else { Vec2::ZERO };
        if !matches!(props.mount, Mount::Inline | Mount::Pane(_))
            && let Some(why) = crate::runtime::overlay::take_close(&ctx, id)
        {
            self.close = true;
            self.reason = Some(why);
        }
        self.open_bound = comp.open().is_some();
        self.open = match comp.open() {
            Some(bind) => matches!(bind.get(), Some(Mixed::Same(true))),
            None => *open_slot(stage, id, save.as_ref()).0.get_or_insert(initially_open),
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
                let placement = placement_of(&ctx, id, save.as_ref());
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

    fn extent(&mut self, comp: &C, ui: &Ui, at: ShowAt, _id: Id, scx: &StyleCx<'_>) -> Vec2 {
        let props = comp.props(Internal::new());
        self.room = match (overlay_of(props), self.fixed) {
            (Some(overlay), false) => Self::layer_room(ui.ctx(), overlay),
            _ => match at {
                ShowAt::Rect(rect) => rect,
                ShowAt::Flow => ui.available_rect_before_wrap(),
            },
        };
        let remembered = self.remembered.unwrap_or(Vec2::ZERO);
        let body = if self.presence.visible { Vec2::new(remembered.x, remembered.y * self.presence.t) } else { Vec2::ZERO };
        let width = if self.remembered.is_some() { self.shared_width(self.room.width() - 2.0 * pad_of(scx.tokens, props.pad)) } else { 0.0 };
        let height = body.y + self.header_last.y + self.footer_last.y;
        Vec2::new(width, height) + Vec2::splat(2.0 * pad_of(scx.tokens, props.pad))
    }

    fn sized(&self, comp: &C, ui: &Ui, size: Vec2) -> Vec2 {
        match self.sizing(comp.props(Internal::new())) {
            Sizing::Fill => {
                let room = ui.available_size_before_wrap();
                Vec2::new(if room.x.is_finite() { size.x.max(room.x) } else { size.x }, if room.y.is_finite() { size.y.max(room.y) } else { size.y })
            }
            _ => size,
        }
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
        let handle = tokens.tokens().size.handle.sm;
        let reason = "the Move and Size commands of the window command set do the same";
        // A layer moves when its overlay says so; other mounts follow the header's band.
        let movable = match &props.mount {
            Mount::Layer(overlay) | Mount::Viewport { fallback: overlay } => overlay.movable,
            _ => props.header.is_some_and(|header| header.band == super::DragBand::Move),
        };
        if props.header.is_some() && movable && regions.header.height() > 0.0 {
            pcx.part(MOVE_BAND, regions.header)
                .sense(craft_behavior::Sense::DRAG)
                .priority(HitPriority::new(HitLevel::Edge, 0))
                .exclusive()
                .cursor(CursorIcon::Grab)
                .drag_cursor(CursorIcon::Grabbing)
                .pointer_only(reason);
        }
        if let Mount::Layer(overlay) | Mount::Viewport { fallback: overlay } = &props.mount
            && overlay.resize.grips
        {
            for (index, (dx, dy, cursor)) in GRIPS.into_iter().enumerate() {
                let along = |edge: i8, low: f32, high: f32| match edge {
                    -1 => (low - handle / 2.0, low + handle / 2.0),
                    1 => (high - handle / 2.0, high + handle / 2.0),
                    _ => (low + handle / 2.0, high - handle / 2.0),
                };
                let (x0, x1) = along(dx, rect.min.x, rect.max.x);
                let (y0, y1) = along(dy, rect.min.y, rect.max.y);
                let level = if dx != 0 && dy != 0 { HitLevel::Handle } else { HitLevel::Edge };
                pcx.part(GRIP.at(index as u64), Rect::from_min_max(Pos2::new(x0, y0), Pos2::new(x1, y1)))
                    .sense(craft_behavior::Sense::DRAG)
                    .priority(HitPriority::new(level, 0))
                    .exclusive()
                    .cursor(cursor)
                    .pointer_only(reason);
            }
        }
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
        if !layer {
            return;
        }
        self.window_keys(props, cx);
        let id = cx.core.id;
        let min = cx.style.metrics().min_size;
        let rect = cx.core.rect;
        // A part the container did not declare this pass (no band, no grips) has no result.
        let phase = |key: (PartKey, Option<u64>)| cx.core.results.get(&key).map(|result| result.press.phase);
        let band = phase((MOVE_BAND, None));
        let grips: Vec<(usize, Vec2)> = (0..GRIPS.len())
            .filter_map(|index| match phase((GRIP, Some(index as u64))) {
                Some(PressPhase::DragStart { start, pos, .. }) => Some((index, pos - start.origin)),
                Some(PressPhase::Drag { delta, .. }) => Some((index, delta)),
                _ => None,
            })
            .collect();
        let slot = placement_slot(&mut *cx.core.memory, id, self.save.as_ref());
        let band_move = match band {
            Some(PressPhase::DragStart { start, pos, .. }) => Some(pos - start.origin),
            Some(PressPhase::Drag { delta, .. }) => Some(delta),
            _ => None,
        };
        if let Some(delta) = band_move {
            let pos = slot.pos.map_or(rect.min, Pos2::from);
            slot.pos = Some((pos + delta).into());
        }
        for (index, delta) in grips {
            let (dx, dy, _) = GRIPS[index];
            let mut pos = slot.pos.map_or(rect.min, Pos2::from);
            let mut size = slot.size.map_or(rect.size(), Vec2::from);
            if dx < 0 {
                let moved = delta.x.min(size.x - min.x);
                pos.x += moved;
                size.x -= moved;
            } else if dx > 0 {
                size.x = (size.x + delta.x).max(min.x);
            }
            if dy < 0 {
                let moved = delta.y.min(size.y - min.y);
                pos.y += moved;
                size.y -= moved;
            } else if dy > 0 {
                size.y = (size.y + delta.y).max(min.y);
            }
            slot.pos = Some(pos.into());
            slot.size = Some(size.into());
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
        if let Some(scope) = props.command_scope {
            commands::register_instance(&ctx, scope, id);
        }
        if let Some(window) = window {
            commands::register_instance(&ctx, WindowCommand::SCOPE, window);
        }
        let region_env = env.derive(|scoped| {
            if let Some(window) = window {
                scoped.commands = scoped.commands.pushed(WindowCommand::SCOPE, window);
            }
            scoped.values.extend(values);
            scoped.ink = cx.core.ink;
            scoped.grid = None;
            if let Some(theme) = &props.theme {
                scoped.tokens = theme.tokens().clone();
            }
        });
        let body_id = sub(id, "body");
        let region = |name: &'static str, rect: Rect| {
            let builder = UiBuilder::new().max_rect(rect).layout(child::column_layout(rtl)).scope_id(sub(id, name));
            if node { access::with_parent(builder, id) } else { builder }
        };
        let (resolved, env_view) = (cx.core.resolved, region_env.view());
        let mut open = self.open;
        let mut gate = self.gate;
        let mut close: Option<CloseReason> = None;
        // B1 header
        if props.header.is_some() {
            let mut bcx = BodyCx::new(body_id, env_view, resolved, regions.header, &mut *cx.core.memory, &mut open, gate.as_mut());
            let mut header_ui = alloc::child(ui, region("header", regions.header));
            let installed = Env::install_for(&header_ui, region_env.clone());
            comp.header(&mut header_ui, &mut bcx);
            Env::restore(&ctx, installed);
            close = close.or(bcx.close_requested());
            self.header_size = Some(self.region_size(&header_ui));
        }
        crate::runtime::focus::begin_body(&ctx);
        // B2 label: the control gets the rect the label leaves; the label column adds to the container's extent.
        let mut body_rect = regions.body;
        if let Some(side) = props.label {
            let resolved = cx.core.resolved;
            let input = LabelIn {
                scope: id,
                owner: id,
                parent: node.then_some(id),
                row: regions.body,
                room: if self.sizing(&props) == Sizing::Content { room.x } else { regions.body.width() },
                text: resolved.label_value,
                side: &side,
                enabled: resolved.enabled,
                size: resolved.size,
                control_min: scope::last_control_min(&ctx, id),
            };
            body_rect = label::run(ui, env, &input);
        }
        let label_extra = (regions.body.size() - body_rect.size()).max(Vec2::ZERO);
        // B3 body, if open, under the body env; the gate is read after the header changed it.
        let animated = props.collapse.is_some_and(|collapse| collapse.animated);
        let is_open = open || (animated && self.presence.visible);
        let enabled = gate.unwrap_or(true);
        if is_open {
            let body_env = region_env.derive(|scoped| {
                if props.arena {
                    scoped.arena = Some(ArenaId::new(id));
                }
                scoped.transform = props.transform;
                scoped.enabled &= enabled;
                if scroll.is_some() {
                    scoped.scroll_owner = Some(id);
                }
                if let Some(size) = props.child_size {
                    scoped.size = size;
                }
                if props.roving.is_some() {
                    scoped.roving = Some(super::steps::body_group(id));
                }
                if let Some(scope) = props.command_scope {
                    scoped.commands = scoped.commands.pushed(scope, id);
                }
            });
            let mut bcx = BodyCx::new(body_id, body_env.view(), resolved, body_rect, &mut *cx.core.memory, &mut open, gate.as_mut());
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
                        layout::run_body(&mut region_ui, layout_id, layout, gap, |ui| body(ui, bcx));
                        let extent = region_ui.min_rect().size();
                        self.extent = Some(extent);
                        Some((extent + inset).min(at.size()))
                    }
                    None => {
                        layout::run_body(&mut body_ui, layout_id, layout, gap, |ui| body(ui, bcx));
                        None
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
        } else {
            // A collapsed body keeps its last full size in layout memory, so opening it again places by that size
            // and asks for no discard pass.
            self.body_size = if props.collapse.is_some() { None } else { Some(Vec2::ZERO) };
        }
        // The label scope covers the body only: footer controls are not the field's controls.
        if props.label.is_some() {
            scope::finish(&ctx, id, None);
        }
        // B4 footer
        if props.footer.is_some() {
            let mut bcx = BodyCx::new(body_id, env_view, resolved, regions.footer, &mut *cx.core.memory, &mut open, gate.as_mut());
            let mut footer_ui = alloc::child(ui, region("footer", regions.footer));
            let installed = Env::install_for(&footer_ui, region_env.clone());
            comp.footer(&mut footer_ui, &mut bcx);
            Env::restore(&ctx, installed);
            close = close.or(bcx.close_requested());
            self.footer_size = Some(self.region_size(&footer_ui));
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
        let draw_rule = |y: f32| ui.painter().extend(Rule::shapes(&rule, Pos2::new(inner.min.x, y), Pos2::new(inner.max.x, y), ppp));
        if props.header.is_some_and(|header| header.rule) {
            draw_rule(regions.header.max.y);
        }
        if props.footer.is_some_and(|footer| footer.rule) {
            draw_rule(regions.footer.min.y);
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
        let closable = collapsible || comp.open().is_some();
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
        // A block inside took a placeholder place for want of its own entry: the sizes the regions took say nothing
        // yet, and recording them would pin a zero width. The block's own record asks for the pass that measures
        // this container again.
        if layout_memory::unknown_count(ctx) == self.unknown_at_read {
            if let Some(size) = self.body_size {
                super::steps::record_size(ctx, id, size);
            }
            if let Some(size) = self.header_size {
                super::steps::record_size(ctx, sub(id, "header"), size);
            }
            if let Some(size) = self.footer_size {
                super::steps::record_size(ctx, sub(id, "footer"), size);
            }
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
