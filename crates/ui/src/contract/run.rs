//! The generic shell of the pipeline (1.6): one `run` calls the 17 steps for every shape. What differs by shape
//! (the body step, the item step, the size of a body or collection) goes through `ShapeHooks`.

use super::cx::PartsCx;
use super::steps::{self, MeasureIn, PartsStep, ShapeKind, Step, StopsIn, Trace};
use super::{
    Activation, BodyCx, Collection, Common, Component, Container, Cx, CxCore, Internal, ItemCx, Meta, OverlayUi, PaintUi, Props, Resolved, Show,
    TipFn,
};
use super::{Inner, ItemFn};
use crate::primitives::sensing::{self, MemberSpec, PartDecl, RootSpec};
use crate::primitives::{FaceRecord, PaintRecord, PartResult, face_id};
use crate::runtime::access::{Role, scope};
use crate::runtime::egui::{access, alloc, child};
use crate::runtime::env::{AppValues, ArenaId, Env, EnvView};
use crate::runtime::memory::{MemoryStage, sub};
use crate::runtime::{focus, inspect, keys, layout_memory, memory};
use crate::style::{ComponentStyle as _, Ink, StateSet, StyleCx};
use craft_behavior::{Flow, Input, ItemLayout, ItemSource, KeyRole, PartRef, PressState, Proposal, ScrollPos, Sense};
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

/// Where a component's steps 6 to 16 run: calls `pass` with the `Ui` they run in and the placement that `Ui`
/// imposes (`None` keeps the caller's). A host that opens a new egui layer or viewport installs `env` there. F5
/// adds the layer and viewport hosts, which call `pass` once per child pass.
pub(crate) trait Host {
    fn run(&mut self, id: Id, parent: &mut Ui, env: &Env, pass: &mut dyn FnMut(&mut Ui, Option<ShowAt>));
}

/// The parent `Ui` is the host `Ui`.
struct Inline;

impl Host for Inline {
    fn run(&mut self, _: Id, parent: &mut Ui, _: &Env, pass: &mut dyn FnMut(&mut Ui, Option<ShowAt>)) {
        pass(parent, None);
    }
}

/// What differs by shape. The item step grows through the defaulted hooks at steps 4, 7, 8, 9, 11 and 17 (F6), and
/// the body step through `content` and `host` (F5).
pub(crate) trait ShapeHooks<C: Component> {
    fn shape(&self) -> ShapeKind;

    /// Where steps 6 to 16 run.
    fn host(&mut self) -> Box<dyn Host>;

    /// Whether the root is an arena root: its sense covers its members.
    fn arena(&self, _comp: &C) -> bool {
        false
    }

    /// What the shape adds to the content size at step 5, before padding and `min_size`.
    fn extent(&mut self, _comp: &C, _ui: &Ui, _at: ShowAt, _id: Id, _scx: &StyleCx<'_>) -> Vec2 {
        Vec2::ZERO
    }

    /// Step 4: the shape's part of the value step (selection snapshot, scroll offset).
    fn read(&mut self, _comp: &mut C, _stage: &mut MemoryStage, _id: Id) {}

    /// Step 7: the parts the shape declares itself (scroll parts, the item layout).
    fn parts(&mut self, _comp: &mut C, _rect: Rect, _pcx: &mut PartsCx<'_, C>) {}

    /// Step 8, after the pass fields of `Cx` are set: the item under a press.
    fn sensed(&mut self, _comp: &mut C, _cx: &mut Cx<'_, C>) {}

    /// Step 9, after `C::update`: navigation, selection, type-ahead and drops.
    fn updated(&mut self, _comp: &mut C, _cx: &mut Cx<'_, C>) {}

    /// Step 11: the selection change applied to its binding.
    fn apply(&mut self, _comp: &mut C, _cx: &mut Cx<'_, C>) {}

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

    fn host(&mut self) -> Box<dyn Host> {
        Box::new(Inline)
    }

    fn content(&mut self, _: &mut C, _: &mut Ui, _: &Env, _: &mut Cx<'_, C>, _: &AppValues) {}
}

struct BodyHooks<'b> {
    body: &'b mut dyn FnMut(&mut Ui, &mut BodyCx<'_>),
    /// The body's size from last frame's layout memory; `None` on the first frame.
    remembered: Option<Vec2>,
    /// The room the mount offers, in which a first-frame body runs.
    room: Rect,
    /// The size the body had in this pass, recorded at step 17.
    body_size: Option<Vec2>,
}

impl<C: Container> ShapeHooks<C> for BodyHooks<'_> {
    fn shape(&self) -> ShapeKind {
        ShapeKind::Body
    }

    fn host(&mut self) -> Box<dyn Host> {
        Box::new(Inline)
    }

    fn arena(&self, comp: &C) -> bool {
        comp.props(Internal::new()).arena
    }

    fn extent(&mut self, _: &C, ui: &Ui, at: ShowAt, id: Id, _: &StyleCx<'_>) -> Vec2 {
        self.remembered = layout_memory::read(ui.ctx(), id);
        self.room = match at {
            ShowAt::Rect(rect) => rect,
            ShowAt::Flow => ui.available_rect_before_wrap(),
        };
        self.remembered.unwrap_or(Vec2::ZERO)
    }

    fn content(&mut self, comp: &mut C, ui: &mut Ui, env: &Env, cx: &mut Cx<'_, C>, values: &AppValues) {
        let rtl = env.direction.is_rtl();
        let id = cx.core.id;
        // With no remembered size the body runs in the full room the mount offers; step 17 records its real size,
        // which asks for the one discard pass.
        let rect = match self.remembered {
            Some(_) => cx.core.content_rect,
            None => Flow::new(self.room, rtl).inset(cx.style.metrics().padding),
        };
        let body_id = sub(id, "body");
        let arena = comp.props(Internal::new()).arena;
        let inner = env.derive(|scoped| {
            scoped.values.extend(values);
            scoped.ink = cx.core.ink;
            if arena {
                scoped.arena = Some(ArenaId::new(id));
            }
        });
        let builder = UiBuilder::new().max_rect(rect).layout(child::column_layout(rtl)).scope_id(body_id);
        // The body's nodes are the container's children; a role with no node leaves them where egui puts them.
        let builder = if matches!(C::ROLE, Role::Node(_)) { access::with_parent(builder, id) } else { builder };
        let mut body_ui = alloc::child(ui, builder);
        let installed = Env::install_for(&body_ui, inner);
        {
            let mut body_cx = BodyCx::new(body_id, cx.core.env, cx.core.resolved, rect, &mut *cx.core.memory);
            (self.body)(&mut body_ui, &mut body_cx);
        }
        Env::restore(&cx.core.ctx, installed);
        self.body_size = Some(body_ui.min_rect().size());
    }

    fn record(&mut self, ctx: &Context, id: Id) {
        if let Some(size) = self.body_size {
            steps::record_size(ctx, id, size);
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

    fn host(&mut self) -> Box<dyn Host> {
        Box::new(Inline)
    }

    fn arena(&self, _: &C) -> bool {
        true
    }

    fn extent(&mut self, comp: &C, ui: &Ui, _: ShowAt, _: Id, scx: &StyleCx<'_>) -> Vec2 {
        let width = Some(ui.available_width()).filter(|width| width.is_finite()).unwrap_or(0.0);
        let layout = comp.item_layout(self.source, width, scx);
        let content = layout.content_size();
        self.layout = Some(layout);
        Vec2::new(content.x as f32, content.y as f32)
    }

    fn content(&mut self, comp: &mut C, ui: &mut Ui, env: &Env, cx: &mut Cx<'_, C>, values: &AppValues) {
        let Some(layout) = self.layout.as_ref() else { return };
        let (id, rect) = (cx.core.id, cx.core.content_rect);
        let rtl = env.direction.is_rtl();
        let inner = env.derive(|scoped| {
            scoped.values.extend(values);
            scoped.arena = Some(ArenaId::new(id));
        });
        let builder = UiBuilder::new().max_rect(rect).layout(child::column_layout(rtl)).scope_id(sub(id, "items"));
        let builder = if matches!(C::ROLE, Role::Node(_)) { access::with_parent(builder, id) } else { builder };
        let mut items_ui = alloc::child(ui, builder);
        items_ui.set_clip_rect(rect.intersect(ui.clip_rect()));
        let installed = Env::install_for(&items_ui, inner);
        let (offset, flow) = (ScrollPos::default(), Flow::new(rect, rtl));
        let mut visible = Vec::new();
        layout.visible(offset, rect.size(), &mut visible);
        // F3-LATER (F6): every item is a full `Show` with its own Tab stop; the item step makes it an arena member
        // with one roving stop for the collection.
        for at in visible {
            let key = self.source.key(at);
            let item_id = sub(id, ("item", key_hash(&key)));
            let local = layout.rect(at, offset).translate(rect.min.to_vec2());
            let item_rect = if layout.mirrors() { flow.to_physical(local) } else { local };
            let mut item_cx =
                ItemCx { key: &key, at, rect: item_rect, env: cx.core.env, id: sub(item_id, "item"), memory: &mut *cx.core.memory, tip: None };
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
        ink: cx.core.ink,
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

/// What steps 6 to 16 read and write besides the `Ui` the host gives. Built by steps 1 to 5 in the parent pass.
struct Pass<'r, 'a, C: Component> {
    comp: &'r mut C,
    cx: &'r mut Cx<'a, C>,
    hooks: &'r mut dyn ShapeHooks<C>,
    env: &'r Env,
    style: &'r C::Style,
    resolved: &'r Resolved<'r>,
    trace: Trace<'r>,
    tip: Option<TipFn<'r>>,
    /// The placement of the parent `Ui`, for a host that gives none.
    at: ShowAt,
    /// The width the parent offers, which step 5 and step 7's measure view both read.
    room: Option<f32>,
    size: Vec2,
    inert: bool,
    /// The parent `Ui` was in a sizing pass.
    sizing: bool,
    /// Set by a pass that ran past the sizing check.
    real: bool,
    activated: bool,
}

/// Steps 6 to 16, in the `Ui` the host gives.
fn pass<C: Component>(p: &mut Pass<'_, '_, C>, ui: &mut Ui, host_at: Option<ShowAt>) {
    let ctx = p.cx.core.ctx.clone();
    let (id, rtl) = (p.resolved.id, p.env.direction.is_rtl());
    let (env, resolved, inert) = (p.env, p.resolved, p.inert);
    let metrics = p.style.metrics();
    // 6 place
    let rect = steps::place(ui, host_at.unwrap_or(p.at), if inert { Vec2::ZERO } else { p.size });
    p.trace.mark(Step::Place);
    // A sizing pass of the parent or host `Ui` allocates the size and stops: no parts, no body, and the stage
    // opened at step 3 is written back unchanged.
    if p.sizing || ui.is_sizing_pass() {
        return;
    }
    p.real = true;
    let (comp, cx) = (&mut *p.comp, &mut *p.cx);
    // 7 parts
    p.trace.mark(Step::Parts);
    let flow = Flow::new(rect, rtl);
    let mut parts = PartsStep { decls: Vec::new(), rovings: Vec::new() };
    if !inert {
        let last = *cx.core.memory.shown::<steps::LastState>(sub(id, "last_state"));
        let input = MeasureIn { ui: &*ui, env, proposal: Proposal::Ideal, room: p.room };
        let measure = steps::measure_cx::<C>(&input, p.style, resolved, cx.shown.as_deref());
        let mut pcx = PartsCx {
            content_rect: flow.inset(metrics.padding),
            flow,
            decls: &mut parts.decls,
            rovings: &mut parts.rovings,
            last_state: (last.0, last.1),
            measure,
        };
        comp.parts(rect, &mut pcx);
        p.hooks.parts(comp, rect, &mut pcx);
    }
    if C::CANVAS_CONTROL && p.trace.recording() {
        inspect::canvas_control(&ctx, std::any::type_name::<C>());
    }
    steps::check_parts(&ctx, std::any::type_name::<C>(), &parts);
    let own = if inert || !resolved.enabled { Sense::HOVER } else { comp.sense() };
    let arena = p.hooks.arena(comp);
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
    p.trace.mark(Step::Sense);
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
    // The node exists before any child `Ui` runs, so the children find their parent (12.2). A member has no egui
    // widget, so it first registers the `Ui` it sits in as its parent.
    let node = matches!(C::ROLE, Role::Node(_)) && {
        if member.is_some() {
            access::register_parent(&ctx, id, ui.unique_id());
        }
        access::create_node(&ctx, id)
    };
    cx.core.part_cursors = sensing::hovered_cursor(&decls, &results).into_iter().collect();
    cx.core.rect = rect;
    cx.core.content_rect = flow.inset(metrics.padding);
    cx.core.flow = flow;
    cx.core.visible = rect.intersect(ui.clip_rect());
    cx.core.response = response;
    // The focus service holds last pass's registration: a part that is not in this pass's results is not active.
    cx.core.active = focus::active_part(&ctx, id).filter(|part| results.contains_key(&(part.key, part.index)));
    cx.core.results = results;
    cx.core.press = press;
    cx.core.member_of = member.map(|arena| arena.0);
    cx.core.activation = activation.then_some(Activation { by_key: false });
    cx.core.activated_member = activated_member.or_else(|| arena.then(|| keys::activated_member(&ctx, id)).flatten());
    cx.core.intents = keys::intents_of(&ctx, Some(id), rtl);
    p.hooks.sensed(comp, cx);
    if inert {
        if node {
            steps::report_inert(cx);
        }
        return;
    }
    // 9 update
    p.trace.mark(Step::Update);
    steps::update_inputs(&mut cx.core);
    comp.update(cx);
    p.hooks.updated(comp, cx);
    // 10 overlay
    p.trace.mark(Step::Overlay);
    comp.overlay(&mut OverlayUi::new(), cx);
    // What `update` and `overlay` left untaken goes back to the key service.
    steps::after_update(&mut cx.core);
    // 11 apply
    p.trace.mark(Step::Apply);
    p.hooks.apply(comp, cx);
    // 12 state
    p.trace.mark(Step::State);
    steps::state(comp, cx, false, env.ink);
    // 13 paint
    p.trace.mark(Step::Paint);
    let mut values = AppValues::default();
    comp.content_env(cx, &mut values);
    steps::paint_surface(ui, env, cx);
    comp.paint(&mut paint_ui(&mut *ui, env, cx), cx);
    steps::paint_focus(ui, cx);
    // 14 content, then `paint_over`
    if p.trace.mark(Step::Content) {
        if C::ROLE == Role::Merged {
            scope::begin(&ctx, id, scope::Kind::Merge);
        }
        p.hooks.content(comp, ui, env, cx, &values);
    }
    comp.paint_over(&mut paint_ui(&mut *ui, env, cx), cx);
    // 15 settle
    p.trace.mark(Step::Settle);
    steps::settle(cx);
    // 16 report. F5-LATER: the tooltip runs from `tip` here.
    p.trace.mark(Step::Report);
    steps::report(comp, cx, &steps::ReportIn { node, decls: &parts.decls, sense: own });
    let _ = p.tip.take();
    p.activated = comp.activated(cx) && resolved.enabled;
}

fn run<C: Component>(mut comp: C, ui: &mut Ui, at: ShowAt, tip: Option<TipFn<'_>>, hooks: &mut dyn ShapeHooks<C>) -> C::Output {
    let ctx = ui.ctx().clone();
    // Slots the last end hook evicted drop here, outside every lock and hook.
    memory::dispose_evicted(&ctx);
    let shape = hooks.shape();
    // Steps 1 to 5 read the sizing flag of the parent `Ui`; steps 6 to 16 read the host `Ui`'s. A sizing pass runs
    // steps 1 to 6 and allocates the size: no id registration, sensing or memory writes.
    let sizing = ui.is_sizing_pass();
    // The inspector flag is read once per run.
    let recording = !sizing && inspect::recording(&ctx);
    // 1 env: `Common` moves out of the component; `Resolved` borrows these locals.
    let env = Env::of(ui);
    let common: Common = std::mem::take(comp.common_mut(Internal::new()));
    let step1 = steps::env::<C>(&env, &common);
    // 2 style
    let scx = steps::style_cx(&ctx, &env, &step1);
    let style = steps::style::<C>(&ctx, &scx, comp.style_key());
    // 3 id, and the memory stage for it
    let id = steps::id::<C>(ui, &common.id, !sizing, true);
    let trace = Trace::new(&ctx, id, shape, recording);
    if recording {
        inspect::begin(&ctx, id);
        for step in [Step::Env, Step::Style, Step::Id] {
            trace.mark(step);
        }
    }
    let mut stage = if sizing { MemoryStage::open_peek(&ctx, id) } else { MemoryStage::open(&ctx, id) };
    let resolved = steps::resolved(id, &step1, &common);
    // 4 value
    let shown = steps::value(&mut comp, &mut stage, id);
    hooks.read(&mut comp, &mut stage, id);
    trace.mark(Step::Value);
    // 5 measure
    let room = match at {
        ShowAt::Rect(rect) => Some(rect.width()),
        ShowAt::Flow => Some(ui.available_width()).filter(|width| width.is_finite()),
    };
    let extent = hooks.extent(&comp, ui, at, id, &scx);
    let size = {
        let input = MeasureIn { ui: &*ui, env: &env, proposal: Proposal::Ideal, room };
        steps::measure(&mut comp, &input, &style, &resolved, shown.as_deref(), extent)
    };
    trace.mark(Step::Measure);
    let inert = !steps::guard(size, shape);
    if inert && size != Vec2::ZERO {
        let now = crate::runtime::frame::clock(&ctx);
        let log = stage.shown::<SizeLog>(sub(id, "size_log"));
        if now - log.0 >= 1.0 || log.0 > now {
            log.0 = now;
            log::debug!("{} measured the size {size:?} and shows nothing", std::any::type_name::<C>());
        }
    }
    let mut cx = Cx::<C> {
        core: blank_core(env.view(), &resolved, &mut stage, ctx.clone(), env.ink, steps::inert_response(ui, id), step1.command.clone()),
        style: &style,
        shown,
    };
    // 6 to 16, in the host's `Ui`, once per call of the host.
    let mut host = hooks.host();
    let mut run = Pass {
        comp: &mut comp,
        cx: &mut cx,
        hooks: &mut *hooks,
        env: &env,
        style: &*style,
        resolved: &resolved,
        trace,
        tip,
        at,
        room,
        size,
        inert,
        sizing,
        real: false,
        activated: false,
    };
    host.run(id, ui, &env, &mut |host_ui, host_at| pass(&mut run, host_ui, host_at));
    let (real_pass, activated) = (run.real, run.activated);
    drop(run);
    // 17 finish, once: the stage is written back after `finish` returns, so writes made in `finish` are kept.
    if real_pass {
        trace.mark(Step::Finish);
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
    let output = run(comp, ui, at, tip, &mut BodyHooks { body: &mut call, remembered: None, room: Rect::NOTHING, body_size: None });
    Inner { output, inner }
}

pub(crate) fn show_items<'r, C: Collection>(comp: C, ui: &mut Ui, source: &dyn ItemSource<C::Key>, item: &mut ItemFn<'r, C>) -> C::Output {
    run(comp, ui, ShowAt::Flow, None, &mut ItemsHooks::<C> { source, item, layout: None })
}

/// Measures `comp` as `show` will show it, without allocating: steps 1 to 5 under the id `show` would use next. It
/// reads memory through a peek, registers no id and applies nothing. `under` is the composite's resolved
/// `(enabled, size)` when it measures a child it shows with `PaintUi::child`.
pub(crate) fn measure_under<C: Component>(comp: &mut C, ui: &Ui, id: Option<Id>, proposal: Proposal, under: Option<(bool, ControlSize)>) -> Vec2 {
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
    steps::measure(comp, &input, &style, &resolved, shown.as_deref(), Vec2::ZERO)
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
    core.ink = steps::surface_ink(style.surface(), &state, env.ink);
    core.visible = rect;
    let cx = Cx::<C> { core, style: &style, shown };
    steps::paint_surface(ui, &env, &cx);
    {
        let results = HashMap::new();
        let mut pui = PaintUi {
            ui: &mut *ui,
            env: &env,
            ink: cx.core.ink,
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
    FaceRecord { part, name, flags, role: C::ROLE, numeric: access.numeric.take() }
}
