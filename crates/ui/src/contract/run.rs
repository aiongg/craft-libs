//! The generic shell of the pipeline (1.6): one `run` calls the 17 steps for every shape. What differs by shape
//! (the body step, the item step, the size of a body or collection) goes through `ShapeHooks`.

use super::cx::PartsCx;
use super::steps::{self, MeasureIn, PartsStep, ShapeKind, Step, StopsIn};
use super::{Activation, BodyCx, Collection, Common, Component, Container, Cx, CxCore, Internal, ItemCx, Meta, OverlayUi, PaintUi, Props, Resolved, Show, TipFn};
use super::{Inner, ItemFn};
use crate::primitives::sensing::{self, MemberSpec, PartDecl, RootSpec};
use crate::primitives::{FaceRecord, PaintRecord, PartResult, face_id};
use crate::runtime::env::{AppValues, ArenaId, Env, EnvView};
use crate::runtime::memory::{MemoryStage, sub};
use crate::runtime::egui::{alloc, child};
use crate::runtime::{focus, inspect, keys, layout_memory, memory};
use crate::style::{ComponentStyle as _, Ink, StateSet, StyleCx};
use craft_behavior::{
    Flow, Input, ItemLayout, ItemSource, KeyRole, PartRef, PressState, Proposal, ScrollPos, Sense,
};
use craft_theme::ControlSize;
use egui::{Context, Id, Rect, Response, Ui, UiBuilder, Vec2};
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// Where a component is placed.
#[derive(Clone, Copy)]
pub(crate) enum ShowAt {
    Flow,
    Rect(Rect),
}

/// What differs by shape. Extension points: F5 replaces `BodyHooks::content` with the full body step (header,
/// footer, scrolling, move and resize parts, mounts) and F6 `ItemsHooks::content` with the full item step.
pub(crate) trait ShapeHooks<C: Component> {
    fn shape(&self) -> ShapeKind;

    /// Whether the root is an arena root: its sense covers its members.
    fn arena(&self, _comp: &C) -> bool {
        false
    }

    /// The component's size: the measured content with what the shape adds.
    fn sized(&mut self, _comp: &C, _ui: &Ui, _id: Id, measured: Vec2, _scx: &StyleCx<'_>) -> Vec2 {
        measured
    }

    /// Step 14: the body or the items.
    fn content(&mut self, comp: &mut C, ui: &mut Ui, env: &Env, cx: &mut Cx<'_, C>, values: &AppValues);

    /// Step 17: what the shape records for next frame.
    fn record(&mut self, _ctx: &Context, _id: Id) {}
}

struct LeafHooks;

impl<C: Component> ShapeHooks<C> for LeafHooks {
    fn shape(&self) -> ShapeKind {
        ShapeKind::Leaf
    }

    fn content(&mut self, _: &mut C, _: &mut Ui, _: &Env, _: &mut Cx<'_, C>, _: &AppValues) {}
}

struct BodyHooks<'b> {
    body: &'b mut dyn FnMut(&mut Ui, &mut BodyCx<'_>),
    extent: Option<Vec2>,
}

impl<C: Container> ShapeHooks<C> for BodyHooks<'_> {
    fn shape(&self) -> ShapeKind {
        ShapeKind::Body
    }

    fn arena(&self, comp: &C) -> bool {
        comp.props(Internal::new()).arena
    }

    fn sized(&mut self, _: &C, ui: &Ui, id: Id, measured: Vec2, _: &StyleCx<'_>) -> Vec2 {
        // The body's size is last frame's, from layout memory; the first frame runs it in no room and step 17
        // records the real size, which asks for the one discard pass.
        measured + layout_memory::read(ui.ctx(), id).unwrap_or(Vec2::ZERO)
    }

    fn content(&mut self, comp: &mut C, ui: &mut Ui, env: &Env, cx: &mut Cx<'_, C>, values: &AppValues) {
        let (id, rect) = (cx.core.id, cx.core.content_rect);
        let body_id = sub(id, "body");
        let ink = cx.style.surface().and_then(|look| look.kind).map_or(env.ink, |kind| kind.ink());
        let arena = comp.props(Internal::new()).arena;
        let inner = env.derive(|scoped| {
            scoped.values.extend(values);
            scoped.ink = ink;
            if arena {
                scoped.arena = Some(ArenaId::new(id));
            }
        });
        let rtl = env.direction.is_rtl();
        let mut body_ui = alloc::child(ui, UiBuilder::new().max_rect(rect).layout(child::column_layout(rtl)).scope_id(body_id));
        let installed = Env::install_for(&body_ui, inner);
        {
            let mut body_cx = BodyCx::new(body_id, cx.core.env, cx.core.resolved, rect, &mut *cx.core.memory);
            (self.body)(&mut body_ui, &mut body_cx);
        }
        Env::restore(&cx.core.ctx, installed);
        self.extent = Some(body_ui.min_rect().size());
    }

    fn record(&mut self, ctx: &Context, id: Id) {
        if let Some(extent) = self.extent {
            steps::record_size(ctx, id, extent);
        }
    }
}

struct ItemsHooks<'s, 'r, C: Collection> {
    source: &'s dyn ItemSource<C::Key>,
    item: &'s mut ItemFn<'r, C>,
    layout: Option<C::ItemLayout>,
}

impl<C: Collection> ShapeHooks<C> for ItemsHooks<'_, '_, C> {
    fn shape(&self) -> ShapeKind {
        ShapeKind::Item
    }

    fn arena(&self, _: &C) -> bool {
        true
    }

    fn sized(&mut self, comp: &C, ui: &Ui, _: Id, measured: Vec2, scx: &StyleCx<'_>) -> Vec2 {
        let width = Some(ui.available_width()).filter(|width| width.is_finite()).unwrap_or(0.0);
        let layout = comp.item_layout(self.source, width, scx);
        let content = layout.content_size();
        self.layout = Some(layout);
        measured + Vec2::new(content.x as f32, content.y as f32)
    }

    fn content(&mut self, comp: &mut C, ui: &mut Ui, env: &Env, cx: &mut Cx<'_, C>, values: &AppValues) {
        let Some(layout) = self.layout.as_ref() else { return };
        let (id, rect) = (cx.core.id, cx.core.content_rect);
        let rtl = env.direction.is_rtl();
        let inner = env.derive(|scoped| {
            scoped.values.extend(values);
            scoped.arena = Some(ArenaId::new(id));
        });
        let mut items_ui = alloc::child(ui, UiBuilder::new().max_rect(rect).scope_id(sub(id, "items")));
        items_ui.set_clip_rect(rect.intersect(ui.clip_rect()));
        let installed = Env::install_for(&items_ui, inner);
        let (offset, flow) = (ScrollPos::default(), Flow::new(rect, rtl));
        let mut visible = Vec::new();
        layout.visible(offset, rect.size(), &mut visible);
        for at in visible {
            let key = self.source.key(at);
            let item_id = sub(id, ("item", key_hash(&key)));
            let local = layout.rect(at, offset).translate(rect.min.to_vec2());
            let item_rect = if layout.mirrors() { flow.to_physical(local) } else { local };
            let mut item_cx = ItemCx { key: &key, at, rect: item_rect, env: cx.core.env, id: sub(item_id, "item"), memory: &mut *cx.core.memory, tip: None };
            item_cx.memory.claim(item_cx.id);
            let item = (self.item)(&key, &mut item_cx);
            drop(item_cx);
            Show::show_at(Props::id(item, item_id), &mut items_ui, item_rect);
        }
        Env::restore(&cx.core.ctx, installed);
        let _ = comp;
    }
}

fn key_hash<K: Hash>(key: &K) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}

#[allow(clippy::too_many_arguments, reason = "one argument per field of the context the pipeline builds once per show")]
fn blank_core<'a>(
    env: EnvView<'a>,
    resolved: &'a Resolved<'a>,
    memory: &'a mut MemoryStage,
    ctx: Context,
    ink: Ink,
    response: Response,
    command: Option<Arc<crate::runtime::commands::ResolvedCommand>>,
) -> CxCore<'a> {
    let rect = Rect::NOTHING;
    CxCore {
        env,
        resolved,
        id: resolved.id,
        rect,
        content_rect: rect,
        visible: rect,
        flow: Flow::new(rect, false),
        response,
        command,
        state: StateSet::default(),
        stated: false,
        ink,
        intents: keys::Intents::default(),
        memory,
        ctx,
        paint: RefCell::new(PaintRecord::default()),
        results: HashMap::new(),
        face_mode: false,
        active: None,
        member_of: None,
        activation: None,
        activated_member: None,
        press: PressState::IDLE,
        cursor: None,
        part_cursors: Vec::new(),
        claims: Vec::new(),
        reveal: None,
        press_cancel: None,
    }
}

fn paint_ui<'a, C: Component>(ui: &'a mut Ui, env: &'a Env, cx: &'a Cx<'_, C>) -> PaintUi<'a> {
    let focus = cx.core.active.map(|part| (part, cx.core.state.flags.contains(crate::style::Flags::FOCUS_VISIBLE)));
    PaintUi {
        ui,
        env,
        id: cx.core.id,
        rect: cx.core.rect,
        record: &cx.core.paint,
        results: &cx.core.results,
        response: &cx.core.response,
        active: focus.filter(|_| cx.core.state.flags.contains(crate::style::Flags::FOCUSED)),
        enabled: cx.core.resolved.enabled,
        size: cx.core.resolved.size,
        kids: None,
    }
}

/// A rate limit for the log of a rejected size: one line per id per second.
#[derive(Clone, Default)]
struct SizeLog(f64);

fn run<C: Component>(mut comp: C, ui: &mut Ui, at: ShowAt, tip: Option<TipFn<'_>>, hooks: &mut dyn ShapeHooks<C>) -> C::Output {
    let ctx = ui.ctx().clone();
    // Slots the last end hook evicted drop here, outside every lock and hook.
    memory::dispose_evicted(&ctx);
    let shape = hooks.shape();
    // A sizing pass runs steps 1 to 6 and allocates the size: no id registration, sensing or memory writes.
    let sizing = ui.is_sizing_pass();
    // 1 env: `Common` moves out of the component; `Resolved` borrows these locals.
    let env = Env::of(ui);
    let common: Common = std::mem::take(comp.common_mut(Internal::new()));
    let step1 = steps::env::<C>(&env, &common);
    // 2 style
    let scx = steps::style_cx(&ctx, &env, &step1);
    let style = steps::style::<C>(&ctx, &scx, comp.style_key());
    // 3 id, and the memory stage for it
    let id = steps::id::<C>(ui, &common.id, !sizing, true);
    if !sizing {
        inspect::begin(&ctx, id);
        for step in [Step::Env, Step::Style, Step::Id] {
            steps::mark(&ctx, id, shape, step);
        }
    }
    let mut stage = if sizing { MemoryStage::open_peek(&ctx, id) } else { MemoryStage::open(&ctx, id) };
    let resolved = steps::resolved(id, &step1, &common);
    // 4 value
    let shown = steps::value(&mut comp, &mut stage, id);
    if !sizing {
        steps::mark(&ctx, id, shape, Step::Value);
    }
    // 5 measure
    let room = match at {
        ShowAt::Rect(rect) => Some(rect.width()),
        ShowAt::Flow => Some(ui.available_width()).filter(|width| width.is_finite()),
    };
    let measured = {
        let input = MeasureIn { ui: &*ui, env: &env, proposal: Proposal::Ideal, room };
        steps::measure(&mut comp, &input, &style, &resolved, shown.as_deref())
    };
    if !sizing {
        steps::mark(&ctx, id, shape, Step::Measure);
    }
    let size = hooks.sized(&comp, ui, id, measured, &scx);
    let inert = !steps::guard(size, shape);
    if inert && size != Vec2::ZERO {
        let now = crate::runtime::frame::clock(&ctx);
        let log = stage.shown::<SizeLog>(sub(id, "size_log"));
        if now - log.0 >= 1.0 || log.0 > now {
            log.0 = now;
            log::debug!("{} measured the size {size:?} and shows nothing", std::any::type_name::<C>());
        }
    }
    let rtl = env.direction.is_rtl();
    let metrics = style.metrics();
    let mut cx = Cx::<C> {
        core: blank_core(env.view(), &resolved, &mut stage, ctx.clone(), env.ink, steps::inert_response(ui, id), step1.command.clone()),
        style: &style,
        shown,
    };
    let mut tip = tip;
    let mut real_pass = false;
    let mut activated = false;
    // 6 place
    let rect = steps::place(ui, at, if inert { Vec2::ZERO } else { size });
    if !sizing {
        steps::mark(&ctx, id, shape, Step::Place);
    }
    'pass: {
        // A sizing pass of the host `Ui` allocates the size and stops: no parts, no body, and the stage opened
        // at step 3 is written back unchanged.
        if sizing || ui.is_sizing_pass() {
            break 'pass;
        }
        real_pass = true;
        // 7 parts
        steps::mark(&ctx, id, shape, Step::Parts);
        let flow = Flow::new(rect, rtl);
        let mut parts = PartsStep { decls: Vec::new(), rovings: Vec::new() };
        if !inert {
            let last = *cx.core.memory.shown::<steps::LastState>(sub(id, "last_state"));
            let input = MeasureIn { ui: &*ui, env: &env, proposal: Proposal::Ideal, room: None };
            let measure = steps::measure_cx::<C>(&input, &style, &resolved, cx.shown.as_deref());
            let mut pcx = PartsCx {
                content_rect: flow.inset(metrics.padding),
                flow,
                decls: &mut parts.decls,
                rovings: &mut parts.rovings,
                last_state: (last.0, last.1),
                measure,
            };
            comp.parts(rect, &mut pcx);
        }
        if C::CANVAS_CONTROL {
            inspect::canvas_control(&ctx, std::any::type_name::<C>());
        }
        steps::check_parts(&ctx, std::any::type_name::<C>(), &parts);
        let own = if inert || !resolved.enabled { Sense::HOVER } else { comp.sense() };
        let arena = hooks.arena(&comp);
        let member = if arena || matches!(C::INPUT, Input::Text | Input::Editor) { None } else { env.arena };
        steps::register_stops(&StopsIn {
            ctx: &ctx,
            id,
            holder: member.map_or(id, |arena| arena.0),
            focusable: !inert && resolved.enabled && own.contains(Sense::FOCUS),
            keys: C::KEYS,
            role: KeyRole::new(),
            rtl,
            parts: &parts,
        });
        if !inert {
            steps::register_wheel(&ctx, id, rect, own, comp.bind().is_some());
        }
        // 8 sense
        steps::mark(&ctx, id, shape, Step::Sense);
        let decls: Vec<PartDecl> = parts.decls.iter().map(|(decl, _)| decl.clone()).collect();
        let enabled = resolved.enabled && !inert;
        let (response, results, press, activation, activated_member) = match member {
            Some(arena) => {
                let sensed = sensing::sense_member(ui, arena, MemberSpec { id, rect, sense: own, parts: &decls, enabled });
                (sensed.response, sensed.parts, sensed.press, sensed.activated, None)
            }
            None => {
                let sensed = sensing::sense_root(ui, RootSpec { id, rect, own_sense: own, own: &decls, enabled });
                (sensed.response, sensed.parts, sensed.press, sensed.activation, sensed.activated_member)
            }
        };
        cx.core.part_cursors = sensing::hovered_cursor(&decls, &results).into_iter().collect();
        cx.core.rect = rect;
        cx.core.content_rect = flow.inset(metrics.padding);
        cx.core.flow = flow;
        cx.core.visible = rect.intersect(ui.clip_rect());
        cx.core.response = response;
        cx.core.results = results;
        cx.core.press = press;
        cx.core.member_of = member.map(|arena| arena.0);
        cx.core.active = focus::active_part(&ctx, id);
        cx.core.activation = activation.then_some(Activation { by_key: false });
        cx.core.activated_member = activated_member.or_else(|| arena.then(|| keys::activated_member(&ctx, id)).flatten());
        cx.core.intents = keys::intents_of(&ctx, Some(id), rtl);
        if inert {
            break 'pass;
        }
        // 9 update
        steps::mark(&ctx, id, shape, Step::Update);
        steps::update_inputs(&mut cx.core);
        comp.update(&mut cx);
        steps::after_update(&mut cx.core);
        // 10 overlay
        steps::mark(&ctx, id, shape, Step::Overlay);
        comp.overlay(&mut OverlayUi::new(), &mut cx);
        // 11 apply
        steps::mark(&ctx, id, shape, Step::Apply);
        // 12 state
        steps::mark(&ctx, id, shape, Step::State);
        steps::state(&comp, &mut cx, false);
        // 13 paint
        steps::mark(&ctx, id, shape, Step::Paint);
        let mut values = AppValues::default();
        comp.content_env(&cx, &mut values);
        steps::paint_surface(ui, &env, &cx);
        comp.paint(&mut paint_ui(&mut *ui, &env, &cx), &cx);
        steps::paint_focus(ui, &cx);
        // 14 content, then `paint_over`
        if steps::mark(&ctx, id, shape, Step::Content) {
            hooks.content(&mut comp, ui, &env, &mut cx, &values);
        }
        comp.paint_over(&mut paint_ui(&mut *ui, &env, &cx), &cx);
        // 15 settle
        steps::mark(&ctx, id, shape, Step::Settle);
        steps::settle(&mut cx);
        // 16 report. F5-LATER: the tooltip runs from `tip` here.
        steps::mark(&ctx, id, shape, Step::Report);
        steps::report(&cx);
        let _ = tip.take();
        activated = comp.activated(&cx) && resolved.enabled;
    }
    // 17 finish, once: the stage is written back after `finish` returns, so writes made in `finish` are kept.
    if real_pass {
        steps::mark(&ctx, id, shape, Step::Finish);
        steps::before_finish(&mut cx, activated);
    }
    let output = comp.finish(cx);
    if real_pass {
        hooks.record(&ctx, id);
    }
    debug_assert!(!real_pass || crate::contract::Outcome::response(&output).id == id, "an output's Response is the component's own");
    if !sizing {
        stage.write_back();
    }
    output
}

pub(crate) fn show_leaf<C: Component + Meta<Shape = super::shape::Leaf>>(comp: C, ui: &mut Ui, at: ShowAt, tip: Option<TipFn<'_>>) -> C::Output {
    run(comp, ui, at, tip, &mut LeafHooks)
}

pub(crate) fn show_container<C: Container, R>(
    comp: C,
    ui: &mut Ui,
    at: ShowAt,
    mut body: impl FnMut(&mut Ui, &mut BodyCx<'_>) -> R,
    tip: Option<TipFn<'_>>,
) -> Inner<C::Output, R> {
    let mut inner: Option<R> = None;
    let mut call = |ui: &mut Ui, cx: &mut BodyCx<'_>| inner = Some(body(ui, cx));
    let output = run(comp, ui, at, tip, &mut BodyHooks { body: &mut call, extent: None });
    Inner { output, inner }
}

pub(crate) fn show_items<'r, C: Collection>(
    comp: C,
    ui: &mut Ui,
    source: &dyn ItemSource<C::Key>,
    item: &mut ItemFn<'r, C>,
) -> C::Output {
    run(comp, ui, ShowAt::Flow, None, &mut ItemsHooks::<C> { source, item, layout: None })
}

/// Measures `comp` as `show` will show it, without allocating: steps 1 to 5 under the id `show` would use next. It
/// reads memory through a peek, registers no id and applies nothing. `under` is the composite's resolved
/// `(enabled, size)` when it measures a child it shows with `PaintUi::child`.
pub(crate) fn measure_under<C: Component>(
    comp: &mut C,
    ui: &Ui,
    id: Option<Id>,
    proposal: Proposal,
    under: Option<(bool, ControlSize)>,
) -> Vec2 {
    let env = match under {
        Some((enabled, size)) => super::cx::child_env(&Env::of(ui), enabled, size),
        None => Env::of(ui),
    };
    let id = id.unwrap_or_else(|| steps::id::<C>(ui, &comp.common(Internal::new()).id, false, false));
    let shown = steps::peek_value(comp, ui.ctx(), id);
    let common = comp.common(Internal::new()).clone();
    let step1 = steps::env::<C>(&env, &common);
    let scx = steps::style_cx(ui.ctx(), &env, &step1);
    let style = steps::style::<C>(ui.ctx(), &scx, comp.style_key());
    let resolved = steps::resolved(id, &step1, &common);
    let input = MeasureIn { ui, env: &env, proposal, room: None };
    steps::measure(comp, &input, &style, &resolved, shown.as_deref())
}

/// What a face reads of the composite that paints it.
pub(crate) struct FaceSource<'a> {
    pub id: Id,
    pub result: &'a PartResult,
    /// `Some(keyboard visible)` when the part is the active part of the focused composite.
    pub focus: Option<bool>,
    pub response: &'a Response,
}

/// A face run is steps 1, 2, 4 (read only), 12 and 13 of the face component, inside the composite's step 13. It
/// registers no id, senses nothing, opens no memory stage and fires nothing.
pub(crate) fn paint_face<C: Show>(ui: &mut Ui, source: &FaceSource<'_>, part: PartRef, mut face: C) -> FaceRecord {
    let ctx = ui.ctx().clone();
    let env = Env::of(ui);
    let common = std::mem::take(face.common_mut(Internal::new()));
    debug_assert!(common.tooltip.is_none(), "a face instance carries no tooltip; use `PartBuilder::tooltip`");
    let step1 = steps::env::<C>(&env, &common);
    let scx = steps::style_cx(&ctx, &env, &step1);
    let style = steps::style::<C>(&ctx, &scx, face.style_key());
    let id = face_id(source.id, part);
    let resolved = steps::resolved(id, &step1, &common);
    let shown = steps::peek_value(&mut face, &ctx, id);
    // A peek that opens no lifetime stage and is dropped without a write-back; a write through it is a debug assertion.
    let mut stage = MemoryStage::open_peek(&ctx, id);
    stage.face = true;
    let rect = source.result.rect;
    let flow = Flow::new(rect, env.direction.is_rtl());
    let part_state = steps::part_state(source.result, step1.enabled, source.focus);
    let flags = part_state.flags | face.flags();
    let on = f32::from(u8::from(flags.intersects(crate::style::Flags::ON)));
    let state = StateSet::new(part_state.state, flags, crate::style::StateMix::new(part_state.t.hover, part_state.t.press, on));
    let mut core = blank_core(env.view(), &resolved, &mut stage, ctx.clone(), env.ink, source.response.clone(), None);
    core.face_mode = true;
    core.rect = rect;
    core.content_rect = flow.inset(style.metrics().padding);
    core.flow = flow;
    core.state = state;
    core.stated = true;
    core.visible = rect;
    let cx = Cx::<C> { core, style: &style, shown };
    steps::paint_surface(ui, &env, &cx);
    {
        let results = HashMap::new();
        let mut pui = PaintUi {
            ui: &mut *ui,
            env: &env,
            id,
            rect,
            record: &cx.core.paint,
            results: &results,
            response: source.response,
            active: None,
            enabled: step1.enabled,
            size: step1.size,
            kids: None,
        };
        face.paint(&mut pui, &cx);
    }
    let name = cx.core.paint.borrow().elided.clone().unwrap_or_else(|| step1.label.as_str().to_owned());
    // The face's own `access` adds its facts; the composite's step 16 fills the part's sub-node from the record.
    let mut access = super::Access::default();
    face.access(&mut access, &cx);
    FaceRecord { part, name, flags, value: None }
}
