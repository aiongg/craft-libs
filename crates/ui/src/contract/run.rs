//! The generic shell of the pipeline (1.6): one `run` calls the 17 steps for every shape. What differs by shape
//! (the body step, the item step, the size of a body or collection) goes through `ShapeHooks`.

use super::cx::PartsCx;
use super::steps::{self, MeasureIn, PartsStep, ShapeKind, Step, StopsIn, Trace};
use super::{
    Activation, BodyCx, Collection, Common, Component, Container, Cx, CxCore, Internal, Meta, OverlayUi, PaintUi, Resolved, Show, TipFn, Validity,
};
use super::{Inner, ItemFn};
use super::{overlay, tooltip};
use crate::primitives::sensing::{self, MemberSpec, PartDecl, RootSpec};
use crate::primitives::{FaceRecord, PaintRecord, PartResult, face_id};
use crate::runtime::access::{self, Politeness, Role, scope};
use crate::runtime::env::{AppValues, Env, EnvView};
use crate::runtime::memory::{MemoryStage, sub};
use crate::runtime::overlay::CloseReason;
use crate::runtime::{commands, focus, inspect, keys, memory};
use crate::style::{ComponentStyle as _, Ink, StateSet, StyleCx};
use craft_behavior::{Flow, Input, ItemSource, KeyRole, PartRef, PressState, Proposal, Roving, Sense};
use craft_theme::ControlSize;
use egui::{Context, Id, Rect, Response, Ui, Vec2};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

/// Where a component is placed.
#[derive(Clone, Copy)]
pub(crate) enum ShowAt {
    Flow,
    Rect(Rect),
}

/// The pass a host runs: the `Ui` it runs in, the placement that `Ui` imposes and the env it runs under.
pub(crate) type HostPass<'a> = dyn FnMut(&mut Ui, Option<ShowAt>, Option<&Env>) + 'a;

/// Where a component's steps 6 to 16 run: calls `pass` with the `Ui` they run in and the placement that `Ui`
/// imposes (`None` keeps the caller's), and the env it runs under when that is not the caller's (a viewport's own
/// window facts). A host that opens a new egui layer or viewport installs `env` there. F5 adds the layer and
/// viewport hosts, which call `pass` once per child pass.
pub(crate) trait Host {
    fn run(&mut self, id: Id, parent: &mut Ui, env: &Env, pass: &mut HostPass<'_>);

    /// Why the host asks the component to close, decided after its passes ran.
    fn dismissed(&self) -> Option<CloseReason> {
        None
    }
}

/// What a shape's `host` hook reads to choose a host.
pub(crate) struct HostCx<'a> {
    /// The parent `Ui`.
    pub ui: &'a Ui,
    pub env: &'a Env,
    pub id: Id,
    /// The component's label after step 1: a window's title.
    pub label: &'a crate::runtime::env::Label,
}

/// The parent `Ui` is the host `Ui`.
pub(super) struct Inline;

impl Host for Inline {
    fn run(&mut self, _: Id, parent: &mut Ui, _: &Env, pass: &mut HostPass<'_>) {
        pass(parent, None, None);
    }
}

/// What differs by shape. The item step grows through the defaulted hooks at steps 4, 7, 8, 9, 11 and 17 (F6), and
/// the body step through `content` and `host` (F5).
pub(crate) trait ShapeHooks<C: Component> {
    fn shape(&self) -> ShapeKind;

    /// Where steps 6 to 16 run.
    fn host(&mut self, comp: &C, at: HostCx<'_>) -> Box<dyn Host>;

    /// The host asked the component to close after its passes (a dismissal rule of the layer fired).
    fn dismissed(&mut self, _comp: &mut C, _cx: &mut Cx<'_, C>, _why: CloseReason) {}

    /// The focus scope pushed at step 6 and popped after step 16.
    fn focus(&self, _comp: &C) -> Option<crate::runtime::focus::FocusPolicy> {
        None
    }

    /// The size step 6 allocates, from the measured `size`: a body that fills reads the available rect.
    fn sized(&self, _comp: &C, _ui: &Ui, size: Vec2) -> Vec2 {
        size
    }

    /// The root's sense, from the component's own `own`.
    fn sense(&self, _comp: &C, own: Sense) -> Sense {
        own
    }

    /// Whether the root is an arena root: its sense covers its members.
    fn arena(&self, _comp: &C) -> bool {
        false
    }

    /// What the focused stop is, as far as the key table's rows differ.
    fn key_role(&self, _comp: &C) -> KeyRole {
        KeyRole::new()
    }

    /// The roving rules of the body's members, when the shape's body forms one group.
    fn roving(&self, _comp: &C) -> Option<Roving> {
        None
    }

    /// Whether a collapsible container is open, reported as its node's expanded state; `None` for a shape that does
    /// not collapse.
    fn expanded(&self, _comp: &C) -> Option<bool> {
        None
    }

    /// The politeness of a live region the shape's body is.
    fn live(&self, _comp: &C) -> Option<Politeness> {
        None
    }

    /// The axes the shape's body scrolls on, vertical then horizontal.
    fn scrolls(&self, _comp: &C) -> (bool, bool) {
        (false, false)
    }

    /// What the shape adds to the content size at step 5, before padding and `min_size`.
    fn extent(&mut self, _comp: &C, _ui: &Ui, _at: ShowAt, _id: Id, _scx: &StyleCx<'_>) -> Vec2 {
        Vec2::ZERO
    }

    /// The width by which the shape's content wants to be wider than the size it measured to, because the room cut
    /// it: the parent `Ui` counts it in the size it took, so a parent that sizes from its content widens.
    fn overflow(&self) -> f32 {
        0.0
    }

    /// The rows this pass places, for `PartsCx::rect_of`; computed before the component's `parts` runs.
    fn geometry(&mut self, _comp: &C, _content: Rect, _rtl: bool) -> Option<std::rc::Rc<dyn std::any::Any>> {
        None
    }

    /// Step 4: the shape's part of the value step (selection snapshot, scroll offset).
    fn read(&mut self, _comp: &mut C, _stage: &mut MemoryStage, _id: Id, _env: &Env) {}

    /// The command scope the component is an instance of: its own `Cx` and the items below see it in the chain.
    fn instance_scope(&self, _comp: &C) -> Option<commands::CommandScope> {
        None
    }

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

    /// Step 15, before the value events are applied: open, gate and scroll changes go to their owners.
    fn settle(&mut self, _comp: &mut C, _cx: &mut Cx<'_, C>) {}

    /// Step 17: what the shape records for next frame.
    fn record(&mut self, _stage: &mut MemoryStage, _ctx: &Context, _id: Id) {}
}

struct LeafHooks;

impl<C: Component> ShapeHooks<C> for LeafHooks {
    fn shape(&self) -> ShapeKind {
        ShapeKind::Leaf
    }

    fn host(&mut self, _: &C, _: HostCx<'_>) -> Box<dyn Host> {
        Box::new(Inline)
    }

    fn content(&mut self, _: &mut C, _: &mut Ui, _: &Env, _: &mut Cx<'_, C>, _: &AppValues) {}
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
        focus: Default::default(),
        member_of: None,
        activation: None,
        activated_member: None,
        press: PressState::IDLE,
        cursor: None,
        part_cursors: Vec::new(),
        claims: Vec::new(),
        reveal: None,
        press_cancel: None,
        scroll_moves: Vec::new(),
        value_parts: Vec::new(),
        root_sense: Sense::empty(),
        roving_arrival: None,
        collection_events: None,
        visible_rows: 0..0,
        gestures: Default::default(),
        texts: None,
        surface_slot: None,
        editor: None,
        editor_inputs: Vec::new(),
        text_invalid: false,
        text_access: std::cell::RefCell::new(None),
        submitted: false,
        escaped: false,
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

/// Steps 6 to 16, in the `Ui` the host gives, inside the component's focus scope.
fn pass<C: Component>(p: &mut Pass<'_, '_, C>, ui: &mut Ui, host_at: Option<ShowAt>, host_env: Option<&Env>) {
    let scope = if p.sizing || ui.is_sizing_pass() { None } else { p.hooks.focus(p.comp) };
    let (ctx, id) = (p.cx.core.ctx.clone(), p.resolved.id);
    if let Some(policy) = scope {
        focus::push_scope(&ctx, id, policy, None);
    }
    steps_6_to_16(p, ui, host_at, host_env);
    if scope.is_some() {
        focus::pop_scope(&ctx);
    }
}

/// A component wider than its room widens the flow `Ui` it was placed in, toward the end of the line.
fn expand_for_overflow(ui: &mut Ui, at: ShowAt, rect: Rect, overflow: f32, rtl: bool) {
    if overflow > 0.0 && matches!(at, ShowAt::Flow) {
        ui.expand_to_include_x(if rtl { rect.min.x - overflow } else { rect.max.x + overflow });
    }
}

/// What the component senses itself: a disabled one that shows a reason on focus stays a Tab stop and senses nothing else.
fn own_sense<C: Component>(hooks: &dyn ShapeHooks<C>, comp: &C, resolved: &Resolved, inert: bool) -> Sense {
    match (inert, resolved.enabled) {
        (true, _) => Sense::HOVER,
        (false, true) => hooks.sense(comp, comp.sense()),
        (false, false) if steps::takes_focus(resolved) => Sense::HOVER | comp.sense().intersection(Sense::FOCUS),
        (false, false) => Sense::HOVER,
    }
}

fn steps_6_to_16<C: Component>(p: &mut Pass<'_, '_, C>, ui: &mut Ui, host_at: Option<ShowAt>, host_env: Option<&Env>) {
    let ctx = p.cx.core.ctx.clone();
    let (id, rtl) = (p.resolved.id, p.env.direction.is_rtl());
    let (env, resolved, inert) = (host_env.unwrap_or(p.env), p.resolved, p.inert);
    let metrics = p.style.metrics();
    // 6 place
    let rect = steps::place(ui, host_at.unwrap_or(p.at), if inert { Vec2::ZERO } else { p.size });
    p.trace.mark(Step::Place);
    let overflow = p.hooks.overflow();
    expand_for_overflow(ui, host_at.unwrap_or(p.at), rect, overflow, rtl);
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
    let mut parts = PartsStep { decls: Vec::new(), rovings: Vec::new(), scrolls: Vec::new(), text: super::text::TextStep::default() };
    if !inert && C::INPUT == Input::Text {
        parts.text.commit_now = focus::commit_now(&ctx) == Some(id);
    }
    if !inert {
        let last = *cx.core.memory.shown::<steps::LastState>(sub(id, "last_state"));
        let input = MeasureIn { ui: &*ui, env, proposal: Proposal::Ideal, room: p.room };
        let measure = steps::measure_cx::<C>(&input, p.style, resolved, cx.shown.as_deref());
        let mut pcx = PartsCx {
            content_rect: flow.inset(metrics.padding),
            flow,
            decls: &mut parts.decls,
            rovings: &mut parts.rovings,
            scrolls: &mut parts.scrolls,
            text: &mut parts.text,
            last_state: (last.0, last.1),
            measure,
            items: p.hooks.geometry(comp, flow.inset(metrics.padding), rtl),
        };
        comp.parts(rect, &mut pcx);
        p.hooks.parts(comp, rect, &mut pcx);
    }
    if C::CANVAS_CONTROL && p.trace.recording() {
        inspect::canvas_control(&ctx, std::any::type_name::<C>());
    }
    steps::check_parts(&ctx, std::any::type_name::<C>(), &parts);
    let text_keys = parts.text.decl.as_ref().map(|decl| crate::primitives::text_adapter::keys_of(C::KEYS, &decl.options));
    let text_widget = parts.text.decl.as_ref().map(|decl| crate::primitives::text_adapter::text_id(id, decl.key));
    let editor_keys = parts
        .text
        .editor
        .as_ref()
        .map(|decl| (crate::primitives::ImeSurface::of(id, decl.key).widget(), super::editor::keys_of(C::KEYS, decl.purpose)));
    let (holder_widget, text_keys) = match (text_widget, text_keys, editor_keys) {
        (Some(widget), keys, _) => (Some(widget), keys),
        (None, _, Some((widget, keys))) => (Some(widget), Some(keys)),
        (None, keys, None) => (None, keys),
    };
    let own = own_sense(&*p.hooks, comp, resolved, inert);
    // An item never senses the whole row: the collection's root takes the click and the focus (17.2).
    let own = if env.item == Some(id) { own.difference(Sense::CLICK | Sense::FOCUS) } else { own };
    let arena = p.hooks.arena(comp);
    let member = if arena || matches!(C::INPUT, Input::Text | Input::Editor) { None } else { env.arena };
    let resettable = !matches!(C::INPUT, Input::Text | Input::Editor) && comp.bind().is_some_and(|bind| bind.reset_value().is_some());
    steps::register_stops(&StopsIn {
        ctx: &ctx,
        id,
        holder: holder_widget.unwrap_or_else(|| member.map_or(id, |arena| arena.0)),
        focusable: !inert && steps::takes_focus(resolved) && own.contains(Sense::FOCUS) && env.item != Some(id),
        text: holder_widget.is_some(),
        keys: text_keys.unwrap_or(C::KEYS),
        role: p.hooks.key_role(comp).resettable(resettable),
        rtl,
        parts: &parts,
        group: steps::member_group(&ctx, env.roving, id),
        body_roving: p.hooks.roving(comp),
        row: env.row,
    });
    // A body's wheel share is its scroll taker (body step, step 7); the gate value does not take the wheel.
    if !inert && p.hooks.shape() != ShapeKind::Body {
        steps::register_wheel(&ctx, id, rect, own, comp.bind().is_some());
    }
    // 8 sense
    p.trace.mark(Step::Sense);
    let decls: Vec<PartDecl> = parts.decls.iter().map(|(decl, _)| decl.clone()).collect();
    let enabled = resolved.enabled && !inert;
    let (mut response, results, press, activation, activated_member, root_sense) = match member {
        Some(arena) => {
            let sensed = sensing::sense_member(ui, arena, MemberSpec { id, rect, sense: own, parts: &decls, enabled });
            (sensed.response, sensed.parts, sensed.press, sensed.activated, None, Sense::empty())
        }
        None => {
            let sensed = sensing::sense_root(ui, RootSpec { id, rect, own_sense: own, own: &decls, enabled });
            (sensed.response, sensed.parts, sensed.press, sensed.activation, sensed.activated_member, sensed.sense)
        }
    };
    // The node exists before any child `Ui` runs, so the children find their parent (12.2). A member has no egui
    // widget, so it first registers the `Ui` it sits in as its parent.
    let node = matches!(C::ROLE, Role::Node(_)) && {
        if member.is_some() {
            access::register_parent(&ctx, id, ui.unique_id());
        }
        let created = access::create_node(&ctx, id);
        if created {
            scope::enter(&ctx, id);
        }
        created
    };
    cx.core.part_cursors = sensing::hovered_cursor(&decls, &results).into_iter().collect();
    cx.core.rect = rect;
    cx.core.content_rect = flow.inset(metrics.padding);
    cx.core.flow = flow;
    cx.core.visible = rect.intersect(ui.clip_rect());
    // A read-only component takes no activation: the response reports no click either.
    if !steps::takes_activation(resolved) {
        response.flags.remove(egui::response::Flags::CLICKED | egui::response::Flags::FAKE_PRIMARY_CLICKED);
    }
    cx.core.response = response;
    // The focus service holds last pass's registration: a part that is not in this pass's results is not active.
    cx.core.focus = focus::facts(&ctx, id);
    cx.core.active = cx.core.focus.part.filter(|part| results.contains_key(&(part.key, part.index)));
    cx.core.scroll_moves = super::cx::scroll_moves(&parts.scrolls, &results);
    cx.core.value_parts = parts.decls.iter().filter(|(_, extras)| extras.value).map(|(decl, _)| (decl.key, decl.index)).collect();
    cx.core.results = results;
    cx.core.root_sense = root_sense;
    cx.core.press = press;
    cx.core.member_of = member.map(|arena| arena.0);
    // A key or AccessKit activation of a member reaches its arena root, which names the member.
    let keyed = member.is_some_and(|arena| keys::activated_member(&ctx, arena.0) == Some(id));
    cx.core.activation = if !steps::takes_activation(resolved) {
        None
    } else if keyed {
        Some(Activation { by_key: true })
    } else {
        activation.then_some(Activation { by_key: false })
    };
    cx.core.activated_member = activated_member.or_else(|| arena.then(|| keys::activated_member(&ctx, id)).flatten());
    cx.core.intents = keys::intents_of(&ctx, Some(id), rtl);
    p.hooks.sensed(comp, cx);
    if inert {
        if node {
            steps::report_inert(cx);
        }
        return;
    }
    // The popups the text part and `update` register start over for this pass.
    if C::OVERLAYS {
        overlay::begin_owner(&ctx, id);
    }
    // 8b text part
    if text_widget.is_some() {
        let ink = crate::style::ink_color(&env.tokens, env.ink);
        super::text::run(cx, ui, &mut parts.text, text_keys.unwrap_or(C::KEYS), rtl, ink, env.tokens.stroke.thin);
    } else if holder_widget.is_some() {
        super::editor::run(cx, ui, &mut parts.text, text_keys.unwrap_or(C::KEYS));
    }
    // 9 update
    p.trace.mark(Step::Update);
    steps::gesture_gate(cx);
    steps::update_inputs(&mut cx.core);
    steps::edit_intents(comp, cx);
    steps::scrub_by_label(comp, cx);
    comp.update(cx);
    p.hooks.updated(comp, cx);
    steps::record_context(&cx.core);
    if let (Some(roving), Some(arrival)) = (p.hooks.roving(comp), cx.core.roving_arrival) {
        focus::register_group(&ctx, focus::Group::new(steps::body_group(id), roving).rtl(rtl).selected(arrival));
    }
    // 10 overlay
    p.trace.mark(Step::Overlay);
    if C::OVERLAYS {
        let open = overlay::open_map(&mut *cx.core.memory, id);
        let mut overlays = OverlayUi::new(&ctx, id, rect, &*ui, resolved, env.clone(), open);
        comp.overlay(&mut overlays, cx);
        overlay::apply_closes(&ctx, overlays.closed);
    }
    // What `update` and `overlay` left untaken goes back to the key service.
    steps::after_update(&mut cx.core);
    // 11 apply
    p.trace.mark(Step::Apply);
    steps::apply_values(comp, cx);
    p.hooks.apply(comp, cx);
    steps::register_reset(comp, cx);
    // 12 state
    p.trace.mark(Step::State);
    steps::state(comp, cx, C::OVERLAYS && overlay::popup_shown(&ctx, id), env.ink);
    if cx.core.state.flags.contains(crate::style::Flags::FOCUSED) {
        commands::note_focus(&ctx, id, &env.commands);
    }
    // 13 paint
    p.trace.mark(Step::Paint);
    let mut values = AppValues::default();
    comp.content_env(cx, &mut values);
    steps::paint_surface(ui, env, cx);
    // Paint and content stay inside the rect (grown by the type's declared outset), however small the room is.
    let outer_clip = ui.clip_rect();
    let inside = cx.core.rect.expand(C::paint_outset(&env.tokens)).intersect(outer_clip);
    ui.set_clip_rect(inside);
    comp.paint(&mut paint_ui(&mut *ui, env, cx), cx);
    ui.set_clip_rect(outer_clip);
    steps::paint_focus(ui, cx);
    ui.set_clip_rect(inside);
    // 14 content, then `paint_over`
    if p.trace.mark(Step::Content) {
        if C::ROLE == Role::Merged {
            scope::begin(&ctx, id, id, scope::Kind::Merge);
        }
        p.hooks.content(comp, ui, env, cx, &values);
    }
    comp.paint_over(&mut paint_ui(&mut *ui, env, cx), cx);
    ui.set_clip_rect(outer_clip);
    // 15 settle
    p.trace.mark(Step::Settle);
    p.hooks.settle(comp, cx);
    steps::settle(comp, cx);
    // 16 report, then the tooltip.
    p.trace.mark(Step::Report);
    steps::report(
        comp,
        cx,
        &steps::ReportIn {
            node,
            decls: &parts.decls,
            sense: own,
            live: p.hooks.live(comp),
            scrolls: p.hooks.scrolls(comp),
            expanded: p.hooks.expanded(comp),
            group: steps::member_group(&ctx, env.roving, id),
        },
    );
    tooltip::run(cx, tooltip::TipIn { ui: &*ui, env, rich: p.tip.take(), parts: &parts.decls, shows_label: comp.shows_label() });
    // Activations accumulate across the passes of a viewport mount.
    let activated = comp.activated(cx) && resolved.enabled;
    if activated {
        crate::runtime::overlay::note_activated(&ctx, id);
    }
    p.activated |= activated;
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
    let step1 = steps::env::<C>(&ctx, &env, &common);
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
    let value = steps::value(&mut comp, &mut stage, id, step1.command.as_deref());
    let (shown, mixed) = (value.value, value.mixed);
    let cancelled: Vec<_> = value.cancelled.into_iter().collect();
    hooks.read(&mut comp, &mut stage, id, &env);
    let own_env = hooks.instance_scope(&comp).map(|scope| env.derive_local(|scoped| scoped.commands = scoped.commands.pushed(scope, id)));
    let env = own_env.as_ref().unwrap_or(&env);
    trace.mark(Step::Value);
    // 5 measure
    let room = match at {
        ShowAt::Rect(rect) => Some(rect.width()),
        ShowAt::Flow => Some(ui.available_width()).filter(|width| width.is_finite()),
    };
    let extent = hooks.extent(&comp, ui, at, id, &scx);
    let size = {
        let input = MeasureIn { ui: &*ui, env, proposal: Proposal::Ideal, room };
        steps::measure(&mut comp, &input, &style, &resolved, shown.as_deref(), extent)
    };
    trace.mark(Step::Measure);
    // A control in a label scope reports its narrowest width: the label step decides next frame whether the row fits
    // the label beside it.
    if shape == ShapeKind::Leaf && !sizing && scope::in_label(&ctx) {
        let input = MeasureIn { ui: &*ui, env, proposal: Proposal::Min, room: None };
        let min = steps::measure(&mut comp, &input, &style, &resolved, shown.as_deref(), Vec2::ZERO);
        scope::record_control_min(&ctx, min.x);
    }
    let size = hooks.sized(&comp, ui, size);
    // A component shown at a rect fills it, so the rect is as much its size as the measure is.
    let inert = !steps::guard(size, shape) || matches!(at, ShowAt::Rect(rect) if !steps::guard(rect.size(), shape));
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
        mixed,
        // A cancel step 4 raised over a document that moved is reported, not applied: the app's value stays. One that
        // carries the value the gesture began with restores it at step 11.
        applied: if cancelled.iter().any(|event| matches!(event, craft_behavior::EditEvent::Cancel { before: Some(_), .. })) {
            0
        } else {
            cancelled.len()
        },
        events: cancelled,
        validity: Validity::Ok,
    };
    // 6 to 16, in the host's `Ui`, once per call of the host.
    let mut host = hooks.host(&comp, HostCx { ui: &*ui, env, id, label: &step1.label });
    let mut run = Pass {
        comp: &mut comp,
        cx: &mut cx,
        hooks: &mut *hooks,
        env,
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
    host.run(id, ui, env, &mut |host_ui, host_at, host_env| pass(&mut run, host_ui, host_at, host_env));
    let (real_pass, activated) = (run.real, run.activated);
    drop(run);
    // A layer's dismissal is decided after its body so a field inside commits in the press pass; its open state is
    // written here, after the report (the one write point outside step 15, besides `request_close`).
    if real_pass && let Some(why) = host.dismissed() {
        hooks.dismissed(&mut comp, &mut cx, why);
    }
    // 17 finish, once: the stage is written back after `finish` returns, so writes made in `finish` are kept.
    if real_pass {
        trace.mark(Step::Finish);
        steps::before_finish(&mut cx, activated);
        if shape == ShapeKind::Leaf {
            let baseline = cx.core.paint.borrow().baseline;
            if let Some(baseline) = baseline {
                scope::record_baseline(&ctx, baseline - cx.core.rect.top());
            }
        }
    }
    let output = comp.finish(cx);
    if real_pass {
        hooks.record(&mut stage, &ctx, id);
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
    let output = run(comp, ui, at, tip, &mut super::body::BodyHooks::new(&mut call));
    Inner { output, inner }
}

pub(crate) fn show_items<'r, C: Collection>(comp: C, ui: &mut Ui, source: &dyn ItemSource<C::Key>, item: &mut ItemFn<'r, C>) -> C::Output {
    run(comp, ui, ShowAt::Flow, None, &mut super::items::ItemsHooks::<C>::new(source, item))
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
    let common = comp.common(Internal::new()).clone();
    let step1 = steps::env::<C>(ui.ctx(), &env, &common);
    let (shown, _) = steps::peek_value(comp, ui.ctx(), id, step1.command.as_deref());
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
    let step1 = steps::env::<C>(&ctx, &env, &common);
    let scx = steps::style_cx(&ctx, &env, &step1);
    let style = steps::style::<C>(&ctx, &scx, face.style_key());
    let id = face_id(source.id, part);
    let resolved = steps::resolved(id, &step1, &common);
    let (shown, mixed) = steps::peek_value(&mut face, &ctx, id, step1.command.as_deref());
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
    let cx = Cx::<C> { core, style: &style, shown, mixed, events: Vec::new(), applied: 0, validity: Validity::Ok };
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
    FaceRecord { part, name, flags, role: C::ROLE, numeric: access.numeric.take(), expanded: access.expanded }
}
