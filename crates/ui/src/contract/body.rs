//! The body step (16.3): the regions of a container (header, body with its scroll region, footer), open and
//! gate state, scroll offset, move and resize, and the zoomed canvas body.

use super::cx::BodyCx;
use crate::style::ComponentStyle as _;
use super::run::{ShapeHooks, ShowAt};
use super::{Container, ContainerProps, Cx, Gap, Internal, Mount, Pad, PartsCx, Scroll, Sizing};
use crate::layout;
use crate::primitives::{ScrollBar, Surface};
use crate::runtime::access::Role;
use crate::runtime::egui::{access, alloc, areas, child, response};
use crate::runtime::env::{AppValues, ArenaId, Env};
use crate::runtime::focus::FocusPolicy;
use crate::runtime::memory::{MemoryStage, sub};
use crate::runtime::{commands, layout_memory, wheel};
use crate::style::{Corners, RuleKind, StyleCx, UiTokens, rule_look, scroll_bar_look, surface_style};
use craft_behavior::{
    Axis, CursorIcon, EditEvent, GestureId, HitLevel, HitPriority, Mixed, PartKey, PressPhase, ScrollPos, ScrollSpan, WheelTaker,
};
use egui::emath::TSTransform;
use egui::{Context, Id, Pos2, Rect, Shape, Stroke, Ui, UiBuilder, Vec2};
use serde::{Deserialize, Serialize};
use std::any::Any;

const SCROLL_Y: PartKey = PartKey::new("body_scroll_y");
const SCROLL_X: PartKey = PartKey::new("body_scroll_x");
const MOVE_BAND: PartKey = PartKey::new("move_band");
const GRIP: PartKey = PartKey::new("resize_grip");

/// The open state a container with no `Container::open` binding keeps for the session.
#[derive(Clone, Default)]
struct OpenState(Option<bool>);

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
    save: Option<crate::runtime::memory::SaveKey>,
}

impl<'b> BodyHooks<'b> {
    pub(super) fn new(body: &'b mut dyn FnMut(&mut Ui, &mut BodyCx<'_>)) -> Self {
        BodyHooks {
            body,
            remembered: None,
            header_last: Vec2::ZERO,
            footer_last: Vec2::ZERO,
            room: Rect::NOTHING,
            open: true,
            open_at_read: true,
            open_bound: false,
            gate: None,
            gate_at_read: None,
            close: false,
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
        }
    }

    /// The regions inside `inner` (the rect minus padding and the padding tier), from last pass's sizes.
    fn regions(&self, props: &ContainerProps, inner: Rect, room: Vec2) -> Regions {
        let header_h = if props.header.is_some() { self.header_last.y } else { 0.0 };
        let footer_h = if props.footer.is_some() { self.footer_last.y } else { 0.0 };
        let top = inner.min.y + header_h;
        let content_sized = props.sizing == Sizing::Content;
        let (bottom, footer_top) = if content_sized {
            let body_h = if self.open { self.remembered.map_or(0.0, |size| size.y) } else { 0.0 };
            (top + (room.y - header_h - footer_h).max(0.0), top + body_h)
        } else {
            (inner.max.y - footer_h, inner.max.y - footer_h)
        };
        let width = if content_sized { room.x.max(0.0) } else { inner.width() };
        let band = |top: f32, bottom: f32| Rect::from_min_max(Pos2::new(inner.min.x, top), Pos2::new(inner.min.x + width, bottom.max(top)));
        Regions {
            header: band(inner.min.y, if props.header.is_some() { inner.min.y + header_h.max(0.0) } else { inner.min.y }),
            body: band(top, bottom),
            footer: band(footer_top, footer_top + footer_h),
        }
    }

    /// The room a content-sized body runs in: what the mount offers, less the padding.
    fn room_size(&self, props: &ContainerProps, ctx: &Context, rect: Rect, inner: Rect) -> Vec2 {
        let offered = match &props.mount {
            Mount::Inline | Mount::Pane(_) => self.room.size(),
            _ => ctx.content_rect().size(),
        };
        offered - (inner.min - rect.min) * 2.0
    }

    fn scroll_geometry(&self, props: &ContainerProps, view: Rect, rtl: bool, tokens: &UiTokens) -> Vec<(Bar, ScrollSpan)> {
        let Some(scroll) = props.scroll else { return Vec::new() };
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
            let track = Rect::from_min_max(Pos2::new(view.min.x, view.max.y - thickness), Pos2::new(view.max.x - if v_on { thickness } else { 0.0 }, view.max.y));
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

impl<C: Container> ShapeHooks<C> for BodyHooks<'_> {
    fn shape(&self) -> super::steps::ShapeKind {
        super::steps::ShapeKind::Body
    }

    fn host(&mut self, comp: &C) -> Box<dyn super::run::Host> {
        super::mount::host(comp.props(Internal::new()))
    }

    fn focus(&self, comp: &C) -> Option<FocusPolicy> {
        let props = comp.props(Internal::new());
        let policy = match &props.mount {
            Mount::Layer(overlay) | Mount::Viewport { fallback: overlay } => overlay.focus,
            _ => props.focus,
        };
        (policy != FocusPolicy::NONE).then_some(policy)
    }

    fn arena(&self, comp: &C) -> bool {
        comp.props(Internal::new()).arena
    }

    fn read(&mut self, comp: &mut C, stage: &mut MemoryStage, id: Id, env: &Env) {
        let ctx = stage.ctx().clone();
        let props = comp.props(Internal::new());
        let (has_header, has_footer, scroll) = (props.header.is_some(), props.footer.is_some(), props.scroll);
        let initially_open = props.collapse.is_none_or(|collapse| collapse.initially_open);
        let save = props.save.clone();
        self.save = save.clone();
        let layer = matches!(props.mount, Mount::Layer(_) | Mount::Viewport { .. });
        self.remembered = layout_memory::read(&ctx, id);
        self.header_last = if has_header { layout_memory::read(&ctx, sub(id, "header")).unwrap_or(Vec2::ZERO) } else { Vec2::ZERO };
        self.footer_last = if has_footer { layout_memory::read(&ctx, sub(id, "footer")).unwrap_or(Vec2::ZERO) } else { Vec2::ZERO };
        self.open_bound = comp.open().is_some();
        self.open = match comp.open() {
            Some(bind) => matches!(bind.get(), Some(Mixed::Same(true))),
            None => *stage.session::<OpenState>(sub(id, "open")).0.get_or_insert(initially_open),
        };
        self.open_at_read = self.open;
        if layer {
            let empty = *stage.shown::<Placement>(placement_id(id)) == Placement::default();
            if empty && let Some(key) = &save {
                let saved = stage.saved::<Placement>(key).clone();
                *stage.shown::<Placement>(placement_id(id)) = saved;
            }
        }
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
        let follows = stage.session::<ScrollState>(sub(id, "scroll")).follow_end.unwrap_or(true);
        if scroll.anchor == super::ScrollAnchor::End && follows && !self.moved {
            self.offset.y = f64::from((self.last.extent.1 - view.y).max(0.0));
        }
        self.clamp_offset(scroll, view);
    }

    fn extent(&mut self, comp: &C, ui: &Ui, at: ShowAt, _id: Id, scx: &StyleCx<'_>) -> Vec2 {
        self.room = match at {
            ShowAt::Rect(rect) => rect,
            ShowAt::Flow => ui.available_rect_before_wrap(),
        };
        let props = comp.props(Internal::new());
        let body = if self.open { self.remembered.unwrap_or(Vec2::ZERO) } else { Vec2::ZERO };
        let width = body.x.max(self.header_last.x).max(self.footer_last.x);
        let height = body.y + self.header_last.y + self.footer_last.y;
        Vec2::new(width, height) + Vec2::splat(2.0 * pad_of(scx.tokens, props.pad))
    }

    fn sized(&self, comp: &C, ui: &Ui, size: Vec2) -> Vec2 {
        match comp.props(Internal::new()).sizing {
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
        let room = self.room_size(&props, &ctx, rect, inner);
        let regions = self.regions(&props, inner, room);
        if self.open && props.scroll.is_some() {
            let view = regions.body;
            self.view = Some(view);
            if let Some(scroll) = props.scroll {
                self.clamp_offset(scroll, view.size());
            }
            for (bar, span) in self.scroll_geometry(&props, view, rtl, tokens) {
                pcx.scroll(bar.key, bar.track, bar.axis, span);
            }
            wheel::register(
                &ctx,
                WheelTaker::new(pcx.id, view).vertical(props.scroll.is_some_and(|scroll| scroll.vertical_on())).horizontal(props.scroll.is_some_and(|scroll| scroll.horizontal_on())),
            );
        }
        let handle = tokens.tokens().size.handle.sm;
        let reason = "the Move and Size commands of the window command set do the same";
        if props.header.is_some_and(|header| header.band == super::DragBand::Move) && regions.header.height() > 0.0 {
            pcx.part(MOVE_BAND, regions.header)
                .sense(craft_behavior::Sense::DRAG)
                .priority(HitPriority::new(HitLevel::Edge, 0))
                .exclusive()
                .cursor(CursorIcon::Grab)
                .drag_cursor(CursorIcon::Grabbing)
                .pointer_only(reason);
        }
        if let Mount::Layer(overlay) | Mount::Viewport { fallback: overlay } = &props.mount
            && overlay.resizable
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
        let (scroll, layer) = (props.scroll, matches!(props.mount, Mount::Layer(_) | Mount::Viewport { .. }));
        if scroll.is_some() {
            if let Some(range) = cx.scroll_moved(SCROLL_Y) {
                self.offset.y = range.start;
                self.moved = true;
            }
            if let Some(range) = cx.scroll_moved(SCROLL_X) {
                self.offset.x = range.start;
                self.moved = true;
            }
        }
        if !layer {
            return;
        }
        let id = cx.core.id;
        let min = cx.style.metrics().min_size;
        let rect = cx.core.rect;
        let band = cx.core.part(MOVE_BAND).press.phase;
        let grips: Vec<(usize, Vec2)> = (0..GRIPS.len())
            .filter_map(|index| match cx.core.part(GRIP.at(index as u64)).press.phase {
                PressPhase::Drag { delta, .. } => Some((index, delta)),
                _ => None,
            })
            .collect();
        let slot = cx.core.memory.shown::<Placement>(placement_id(id));
        if let PressPhase::Drag { delta, .. } = band {
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
        let pad = pad_of(&env.tokens, props.pad);
        let inner = cx.core.content_rect.shrink(pad);
        let room = self.room_size(&props, &ctx, rect, inner);
        let regions = self.regions(&props, inner, room);
        let surface_slot = ui.painter().add(Shape::Noop);
        self.gate = cx.shown.as_deref().and_then(|value| (value as &dyn Any).downcast_ref::<bool>()).copied();
        self.gate_at_read = self.gate;
        if let Some(scope) = props.command_scope {
            commands::register_instance(&ctx, scope, id);
        }
        let region_env = env.derive(|scoped| {
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
        let (resolved, env_view) = (cx.core.resolved, cx.core.env);
        let mut open = self.open;
        let mut gate = self.gate;
        let mut close = false;
        // B1 header
        if props.header.is_some() {
            let mut bcx = BodyCx::new(body_id, env_view, resolved, regions.header, &mut *cx.core.memory, &mut open, gate.as_mut());
            let mut header_ui = alloc::child(ui, region("header", regions.header));
            let installed = Env::install_for(&header_ui, region_env.clone());
            comp.header(&mut header_ui, &mut bcx);
            Env::restore(&ctx, installed);
            close |= bcx.close_requested();
            self.header_size = Some(header_ui.min_rect().size());
        }
        // B3 body, if open, under the body env; the gate is read after the header changed it.
        let is_open = open;
        let enabled = gate.unwrap_or(true);
        if is_open {
            let body_env = region_env.derive(|scoped| {
                if props.arena {
                    scoped.arena = Some(ArenaId::new(id));
                }
                scoped.transform = props.transform;
                scoped.enabled &= enabled;
                scoped.in_scroll |= props.scroll.is_some();
                if let Some(size) = props.child_size {
                    scoped.size = size;
                }
                if let Some(scope) = props.command_scope {
                    scoped.commands = scoped.commands.pushed(scope, id);
                }
            });
            let mut bcx = BodyCx::new(body_id, body_env.view(), resolved, regions.body, &mut *cx.core.memory, &mut open, gate.as_mut());
            let gap = Vec2::splat(gap_of(&env.tokens, props.body_gap));
            let layout_id = sub(id, "layout");
            let (layout, body) = (&props.layout, &mut *self.body);
            let mut run = |parent: &mut Ui, at: Rect, bcx: &mut BodyCx<'_>| -> Option<Vec2> {
                let mut body_ui = alloc::child(parent, {
                    let builder = UiBuilder::new().max_rect(at).layout(child::column_layout(rtl)).scope_id(body_id);
                    if node { access::with_parent(builder, id) } else { builder }
                });
                body_ui.spacing_mut().item_spacing = gap;
                let installed = Env::install_for(&body_ui, body_env.clone());
                match props.scroll {
                    Some(scroll) => {
                        let off = self.offset;
                        let big = super::MAX_EXTENT;
                        let (width, height) = (if scroll.horizontal_on() { big } else { at.width() }, if scroll.vertical_on() { big } else { at.height() });
                        let x = if scroll.horizontal_on() { off.x as f32 } else { 0.0 };
                        let min_x = if rtl { at.max.x - width + x } else { at.min.x - x };
                        let max = Rect::from_min_size(Pos2::new(min_x, at.min.y - off.y as f32), Vec2::new(width, height));
                        let layout_of = *body_ui.layout();
                        let mut region_ui = alloc::child(&mut body_ui, UiBuilder::new().id_salt("scroll_region").layout(layout_of).max_rect(max));
                        region_ui.set_clip_rect(at.intersect(body_ui.clip_rect()));
                        layout::run_body(&mut region_ui, layout_id, layout, gap, |ui| body(ui, bcx));
                        self.extent = Some(region_ui.min_rect().size());
                    }
                    None => layout::run_body(&mut body_ui, layout_id, layout, gap, |ui| body(ui, bcx)),
                }
                Env::restore(&ctx, installed);
                Some(body_ui.min_rect().size())
            };
            self.body_size = match props.transform {
                Some(transform) => {
                    let parent = ui.layer_id();
                    let layer = areas::body_layer(parent, id);
                    let scale = transform.scale;
                    let tf = TSTransform::new(regions.body.min.to_vec2() * (1.0 - scale) + transform.offset, scale);
                    if props.arena {
                        crate::primitives::sensing::set_layer(&ctx, ArenaId::new(id), layer);
                    }
                    let forward = areas::Forward { id: sub(id, "transform"), sense: response::to_egui(cx.core.root_sense), rect: rect.intersect(ui.clip_rect()) };
                    areas::show_body_layer(&ctx, parent, layer, tf, regions.body.min, Some(forward), |area_ui| run(area_ui, regions.body, &mut bcx))
                }
                None => run(ui, regions.body, &mut bcx),
            };
            close |= bcx.close_requested();
            self.view = Some(regions.body);
        } else {
            self.body_size = Some(Vec2::ZERO);
        }
        // B4 footer
        if props.footer.is_some() {
            let mut bcx = BodyCx::new(body_id, env_view, resolved, regions.footer, &mut *cx.core.memory, &mut open, gate.as_mut());
            let mut footer_ui = alloc::child(ui, region("footer", regions.footer));
            let installed = Env::install_for(&footer_ui, region_env.clone());
            comp.footer(&mut footer_ui, &mut bcx);
            Env::restore(&ctx, installed);
            close |= bcx.close_requested();
            self.footer_size = Some(footer_ui.min_rect().size());
        }
        self.open = open;
        self.gate = gate;
        self.close |= close;
        // The scroll bars, over the body.
        if is_open && props.scroll.is_some() {
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
                    let state = super::steps::part_state(cx.core.results.get(&(bar.key, Some(super::cx::THUMB))).unwrap_or(&crate::primitives::PartResult::IDLE), enabled, None);
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
        let stroke = Stroke::new(rule.width, rule.color);
        if props.header.is_some_and(|header| header.rule) {
            let y = regions.header.max.y;
            ui.painter().hline(inner.min.x..=inner.max.x, y, stroke);
        }
        if props.footer.is_some_and(|footer| footer.rule) {
            ui.painter().hline(inner.min.x..=inner.max.x, regions.footer.min.y, stroke);
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
        if self.close {
            self.open = false;
        }
        let id = cx.core.id;
        if self.open != self.open_at_read {
            match comp.open() {
                Some(mut bind) => {
                    let (label, doc_key) = (cx.core.label_arc(), cx.core.resolved.doc_key);
                    bind.apply(&EditEvent::set(GestureId::default(), Mixed::Same(self.open_at_read), self.open, label, doc_key));
                }
                None => cx.core.memory.session::<OpenState>(sub(id, "open")).0 = Some(self.open),
            }
            self.open_at_read = self.open;
        }
        if self.offset != self.offset_at_read {
            match comp.scroll() {
                Some(mut bind) => {
                    let (label, doc_key) = (cx.core.label_arc(), cx.core.resolved.doc_key);
                    bind.apply(&EditEvent::set(GestureId::default(), Mixed::Same(self.offset_at_read), self.offset, label, doc_key));
                }
                None => {
                    let state = cx.core.memory.session::<ScrollState>(sub(id, "scroll"));
                    state.offset = (self.offset.x, self.offset.y);
                }
            }
            self.offset_at_read = self.offset;
        }
    }

    fn record(&mut self, stage: &mut MemoryStage, ctx: &Context, id: Id) {
        if let Some(size) = self.body_size {
            super::steps::record_size(ctx, id, size);
        }
        if let Some(size) = self.header_size {
            super::steps::record_size(ctx, sub(id, "header"), size);
        }
        if let Some(size) = self.footer_size {
            super::steps::record_size(ctx, sub(id, "footer"), size);
        }
        if self.extent.is_some() || self.view.is_some() {
            let view = stage.shown::<BodyView>(sub(id, "view"));
            if let Some(extent) = self.extent {
                view.extent = (extent.x, extent.y);
            }
            view.view = self.view.map(|view| [view.min.x, view.min.y, view.max.x, view.max.y]);
        }
        if let Some(key) = &self.save {
            let placement = stage.shown::<Placement>(placement_id(id)).clone();
            *stage.saved::<Placement>(key) = placement;
        }
        if self.moved {
            let at_end = self.last.view.map(rect_of).is_some_and(|view| self.offset.y >= f64::from(self.last.extent.1 - view.height()) - 0.5);
            stage.session::<ScrollState>(sub(id, "scroll")).follow_end = Some(at_end);
        }
    }
}
