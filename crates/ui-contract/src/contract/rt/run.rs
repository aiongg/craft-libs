//! A component's run inside its scope's build, and the keep decision before it (docs/contract-rt.md
//! sections 3.3, 4, 7, 8 and 15).
//!
//! The real `contract::Component` hooks run here in the pipeline's order: 1 env and 2 style
//! ([`begin`], also for a kept node, whose settled `Cx` reads them), 3 id (the reconcile,
//! `Flow::child`), 4 value, 5 measure (or the node's size cache), 6 place, 7 parts, 8 sense, 9
//! update, 10 overlay, 11 apply, 12 state and 13 paint (once per recorded pointer look), 15 settle,
//! 16 report (access once per run, never per look), 17 finish with the bound command's fire. A node
//! whose type, component key and room are unchanged and that carries no mark runs none of them:
//! `finish` gets a settled `Cx` built from what the node kept.

use std::cell::{RefCell, RefMut};
use std::sync::Arc;

use craft_behavior::{Flow as Geometry, Intent, PartRef, PressPhase, PressState, Proposal};
use craft_theme::Tokens;
use craft_ui_rt::{BuildCx, Gesture, HitShape, InlineState, InteractFact, NodeId, PointerButton, Pos2, Rect, Sense, Vec2, Writes as _};
use smallvec::SmallVec;

use super::access::Access;
use super::focus::RingShape;
use super::key::{component_key, fold, type_key};
use super::node::{ComponentNode, Kept, KeptPart, NodeInput, Place, Pointer, Variant, part_id};
use super::overlay::PopupStates;
use super::response::{self, InputFacts};
use super::sweep;
use super::tip;
use super::ui::{Flow, Ui};
use crate::contract::Context;
use crate::contract::Response;
use crate::contract::cx::RunIo;
use crate::contract::overlay::{drive_toggles, note_activation, run_overlay};
use crate::contract::paint_ui::UiHost;
use crate::contract::run::{Measuring, NoShape, ShapeHooks, ShowAt, TypeFacts, core_for, paint_ui};
use crate::contract::steps::{self, MeasureIn, ValueStep};
use crate::contract::{Activation, Common, Component, Cx, CxCore, Id, Internal, Meta, Resolved, TipFn, Validity, ValueOf};
use crate::primitives::{Canvas, FrameServices, PartResult, Services};
use crate::runtime::access::{Role, fold};
use crate::runtime::commands::{CommandBoard, FireSource, InboxCells, InboxIn, register_inboxes};
use crate::runtime::env::{AppValues, Env};
use crate::runtime::memory::{MemoryStage, sub};

/// What steps 1 and 2 resolve before the keep decision: `Common` moved out of the component, the env
/// the node shows in, the bound command and the style from the window's cache.
pub(crate) struct Begun<C: Component> {
    pub(crate) env: Env,
    pub(crate) common: Common,
    pub(crate) step1: steps::EnvStep,
    pub(crate) style: Arc<C::Style>,
    pub(crate) facts: TypeFacts,
}

impl<C: Component> Begun<C> {
    /// `Common` after step 1, for the node `id`.
    pub(crate) fn resolved(&self, id: Id) -> Resolved<'_> {
        steps::resolved(id, &self.step1, &self.common)
    }
}

/// Steps 1 and 2 under the env the node's parent places it in. `Env::of` registers its reads for the
/// scope that shows the node, so an env write reruns that scope (contract-rt 15.13 item 6).
pub(crate) fn begin<C: Component>(component: &mut C, ui: &mut Ui<'_, '_>) -> Begun<C> {
    let env = Env::of(ui);
    begin_in(component, ui, env)
}

/// Steps 1 and 2 under `env`: the env the parent hands the node (a composite's child inherits its
/// enabled state and size through it).
pub(crate) fn begin_in<C: Component>(component: &mut C, ui: &mut Ui<'_, '_>, env: Env) -> Begun<C> {
    let common = std::mem::take(component.common_mut(Internal::new()));
    let step1 = steps::env(&env, &common, steps::variants::<C>(common.variant, &env), &mut |command| ui.resolve_command(command.clone()));
    let ctx = ui.ctx.clone();
    let style = steps::style::<C>(ui.build_cx(), &ctx, &steps::style_cx(&env, &step1), component.style_key());
    Begun { env, common, step1, style, facts: TypeFacts::of::<C>() }
}

/// Whether a node keeps its outputs: it ran before with this component key and carries no mark.
/// The bridge's own key decides (ui-rt's `key_moved` is not consulted), so a run that writes its
/// binding keeps the key taken after the write. Its room may have moved: what it shows depends on
/// its size, which the size cache or `measure` gives for the new room.
pub(crate) fn holds(kept: &Kept, key: Option<u128>, state: InlineState) -> bool {
    !state.new && !state.marked && key.is_some() && kept.key == key && !kept.variants.is_empty()
}

/// The sweep group of a sweeping toggle: the id of the layout it is shown in.
pub(crate) fn sweep_tag<C: Component>(component: &C, flow: &Flow) -> u64 {
    if component.sweep_value().is_some() { flow.id.value() | 1 } else { 0 }
}

/// The build of one run, lent per call (INT-1b decision 2): the canvas borrows the paint services
/// per draw, and `PaintUi::child`, `MeasureCx::child` and a collection's content build a `Ui` over
/// it per call. The children shown through it are placed in `flow`, the node's own flow.
pub(crate) struct RunHost<'b, 'c> {
    cx: RefCell<&'b mut BuildCx<'c>>,
    flow: RefCell<Option<Flow>>,
    ctx: Context,
    env: Env,
    scale: f32,
}

impl<'b, 'c> RunHost<'b, 'c> {
    pub(crate) fn new(cx: &'b mut BuildCx<'c>, flow: Flow, ctx: Context, env: Env) -> RunHost<'b, 'c> {
        let scale = cx.scale();
        RunHost { cx: RefCell::new(cx), flow: RefCell::new(Some(flow)), ctx, env, scale }
    }

    /// The run's build, for a step that needs it between draws.
    pub(crate) fn cx(&self) -> RefMut<'_, &'b mut BuildCx<'c>> {
        self.cx.borrow_mut()
    }

    /// The flow the children were placed in.
    pub(crate) fn into_flow(self) -> Flow {
        self.flow.into_inner().expect("no `Ui` over the run outlives it")
    }

    fn services<R>(&self, f: impl FnOnce(&FrameServices<'_>) -> R) -> R {
        let mut cx = self.cx.borrow_mut();
        let fonts = crate::fonts::stack_facts(&mut cx);
        let (text, images) = cx.paint_services();
        f(&FrameServices::new(text, images, self.scale, fonts))
    }
}

impl UiHost for &RunHost<'_, '_> {
    fn with_ui(&mut self, f: &mut dyn FnMut(&mut Ui<'_, '_>)) {
        let mut cx = self.cx.borrow_mut();
        let flow = self.flow.borrow_mut().take().expect("one `Ui` over a run at a time");
        let mut ui = Ui { cx: &mut **cx, flow, ctx: self.ctx.clone(), env: self.env.clone() };
        f(&mut ui);
        *self.flow.borrow_mut() = Some(ui.flow);
    }
}

impl Services for RunHost<'_, '_> {
    fn scale(&self) -> f32 {
        self.scale
    }

    fn fonts(&self) -> crate::fonts::StackFacts {
        self.services(|s| s.fonts())
    }

    fn layout(&self, text: &craft_ui_rt::RichText) -> craft_ui_rt::TextLayout {
        self.services(|s| s.layout(text))
    }

    fn metrics(&self, layout: &craft_ui_rt::TextLayout) -> craft_ui_rt::TextMetrics {
        self.services(|s| s.metrics(layout))
    }

    fn image(&self, key: craft_ui_rt::ImageKey, options: craft_ui_rt::ImageOptions, make: &mut dyn FnMut() -> Arc<craft_ui_rt::RgbaImage>) {
        self.services(|s| s.image(key, options, make));
    }

    fn image_if(
        &self,
        key: craft_ui_rt::ImageKey,
        options: craft_ui_rt::ImageOptions,
        make: &mut dyn FnMut() -> Option<Arc<craft_ui_rt::RgbaImage>>,
    ) -> bool {
        self.services(|s| s.image_if(key, options, make))
    }

    fn face_over_interface(&self, key: &str, load: &mut dyn FnMut() -> Option<crate::fonts::FaceBytes>) -> crate::fonts::FaceState {
        self.services(|s| s.face_over_interface(key, load))
    }

    fn face(&self, key: &str, job: crate::primitives::FaceJob<'_>) -> crate::fonts::FaceState {
        self.services(|s| s.face(key, job))
    }

    fn face_bytes(&self, key: &str) -> Option<crate::fonts::FaceBytes> {
        self.services(|s| s.face_bytes(key))
    }

    fn max_image_side(&self) -> u32 {
        self.services(|s| s.max_image_side())
    }
}

/// Shows a leaf under `place`, the env its parent hands it: keeps it, or runs its hooks. In a
/// measuring run it only reports its size: a kept node's from its size cache, otherwise `measure`'s.
pub(crate) fn show_leaf<C: Component>(slot: &mut Option<C>, ui: &mut Ui<'_, '_>, (at, place): (ShowAt, Env), tip: Option<TipFn<'_>>) -> C::Output {
    let component = slot.take().expect("a component is shown once");
    show_shaped(component, ui, (at, place), tip, &mut NoShape)
}

/// Shows a component with the shape `hooks` add to a leaf's run (a collection, 15.5).
pub(crate) fn show_shaped<C: Component>(
    mut component: C,
    ui: &mut Ui<'_, '_>,
    (at, place): (ShowAt, Env),
    tip: Option<TipFn<'_>>,
    hooks: &mut dyn ShapeHooks<C>,
) -> C::Output {
    let begun = begin_in(&mut component, ui, place);
    let (id, child) = ui.flow.child(type_key::<C>(), begun.common.id.clone());
    let key = component_key(&component);
    let room = ui.flow.room();
    let tag = sweep_tag(&component, &ui.flow);
    let command = begun.step1.command.clone();
    let shaped = hooks.shaped();
    if let Some(under) = ui.flow.measuring {
        let peeked = under.and_then(|u| ui.cx.peek::<ComponentNode>(u, child));
        let node = peeked.as_ref().map(|p| p.node);
        // A kept node answers from its size cache and hands the open column groups its captured reports.
        let cached = peeked.filter(|p| !shaped && holds(p.local, key, p.state)).and_then(|p| {
            let size = p.local.cached(room)?;
            crate::layout::replay(&ui.ctx, &p.local.column_reports);
            Some(size)
        });
        let mut stage = MemoryStage::open_peek(&ui.ctx, id);
        let value = steps::value(&mut component, &mut stage, id, command.as_deref());
        hooks.read(&mut component, &mut stage, id, &begun.env);
        let ctx = ui.ctx.clone();
        let size = cached.unwrap_or_else(|| measure(&mut component, &begun, id, room, &value, ui.cx, node, &ctx, (&stage, hooks)));
        let rect = place_in(&mut ui.flow, None, at, size);
        return finish_measured(component, id, rect, &begun, value, ctx);
    }
    let context = ui.ctx.clone();
    let Ui { cx, flow, .. } = ui;
    let node =
        ComponentNode { key: key.map_or(0, fold), tag, stop: C::STOPS && steps::takes_focus(&begun.resolved(id)), text: component.text_input() };
    cx.keyed(child, |cx| {
        cx.inline(node, |kept, state, cx| {
            let kept_outputs = holds(kept, key, state);
            // The kept stage runs in this window for this node, and answers `appeared` for the run that created the
            // node; its session and saved slots go back to the runtime's store when the run ends.
            kept.memory = std::mem::take(&mut kept.memory).within(&context, id).appearing(state.new);
            let out = (|| {
                let value = steps::value(&mut component, &mut kept.memory, id, command.as_deref());
                hooks.read(&mut component, &mut kept.memory, id, &begun.env);
                // A shape's step 5 runs whenever the build reaches it: its content (a collection's rows)
                // lays out from what step 5 found (15.4).
                let cached = kept.cached(room).filter(|_| kept_outputs && !shaped);
                let size = match cached {
                    Some(size) => size,
                    None => {
                        let node = Some(cx.node());
                        measure(&mut component, &begun, id, room, &value, cx, node, &context, (&kept.memory, hooks))
                    }
                };
                let rect = place_in(flow, Some(cx.node()), at, size);
                if kept_outputs && size == kept.size {
                    kept.cache(room, size);
                    // Kept, it reports nothing: the column reports its last run captured join the open groups.
                    crate::layout::replay(&context, &kept.column_reports);
                    if shaped {
                        // The shape's content reconciles its children itself; its kept hooks stay.
                        let tokens = flow.tokens.clone();
                        return settle_shaped(component, kept, cx, rect, &begun, value, context, &tokens, hooks);
                    }
                    if state.path {
                        // A node in one of its layer children is marked: reach them.
                        reach_layers(&mut component, kept, cx, rect, &begun, &flow.tokens, context.clone());
                    } else {
                        cx.keep_children();
                    }
                    return settle(component, kept, cx, rect, &begun, value, context);
                }
                let tokens = super::env::tokens(cx);
                let capture = context.clone();
                crate::layout::begin_capture(&capture);
                let out = run_shaped(
                    component,
                    kept,
                    cx,
                    Ran {
                        id,
                        key,
                        place: Place { room },
                        size,
                        rect,
                        tokens: &tokens,
                        sweep: flow.sweep,
                        begun: &begun,
                        value,
                        tip,
                        context,
                        flow_reveal: &mut flow.reveal,
                    },
                    hooks,
                );
                kept.column_reports = crate::layout::end_capture(&capture);
                out
            })();
            kept.memory.flush();
            out
        })
    })
}

/// Step 6: the rect the node takes: the next place in the flow, or the rect it is shown at.
fn place_in(flow: &mut Flow, node: Option<NodeId>, at: ShowAt, size: Vec2) -> Rect {
    match at {
        ShowAt::Flow => flow.allocate(node, size, false),
        ShowAt::Rect(rect) => flow.place_at(node, rect),
    }
}

/// The flow a run measures and paints its children in: the node's room from `origin`, so a child
/// shown from `paint` has the room it was measured in and answers from its size cache (owner rules:
/// a body reuses its last measurement while its inputs are unchanged). `under` is the node whose
/// kept children answer `MeasureCx::child`.
fn run_flow(id: Id, origin: Pos2, room: Vec2, tokens: Arc<Tokens>, under: Option<Option<NodeId>>) -> Flow {
    let flow = Flow::new(id, origin, Rect::from_min_size(origin, room), 0.0, tokens);
    match under {
        Some(node) => flow.measuring(node),
        None => flow,
    }
}

/// Step 5: `measure` over the room's content width, plus padding, the shape's extent and the style's
/// floors. `node` is the node whose kept children answer `MeasureCx::child` (none when nothing is
/// kept).
#[allow(clippy::too_many_arguments)]
fn measure<C: Component>(
    component: &mut C,
    begun: &Begun<C>,
    id: Id,
    room: Vec2,
    value: &ValueStep<ValueOf<C>>,
    cx: &mut BuildCx<'_>,
    node: Option<NodeId>,
    ctx: &Context,
    (memory, hooks): (&MemoryStage, &mut dyn ShapeHooks<C>),
) -> Vec2 {
    cx.count_measure();
    let resolved = begun.resolved(id);
    let tokens = super::env::tokens(cx);
    let host = RunHost::new(cx, run_flow(id, Pos2::ZERO, room, tokens, Some(node)), ctx.clone(), begun.env.clone());
    let mut lent: &RunHost<'_, '_> = &host;
    let cell: RefCell<&mut dyn UiHost> = RefCell::new(&mut lent);
    let input = MeasureIn { env: &begun.env, proposal: Proposal::Ideal, room: Some(room.x), host: &cell };
    let content = component.measure(&steps::measure_cx::<C>(&input, &begun.style, &resolved, value.value.as_deref()));
    let extent = match hooks.shaped() {
        true => {
            let scx = steps::style_cx(&begun.env, &begun.step1);
            let at = Measuring { ui: &cell, at: ShowAt::Flow, room, place: &begun.env, env: &begun.env, scx: &scx, resolved: &resolved, memory };
            hooks.pass_extent(component, &at, &begun.style, value.value.clone())
        }
        false => Vec2::ZERO,
    };
    hooks.sized(component, steps::measure(&*begun.style, &input, content, extent, &resolved))
}

/// The facts of a run decided before its hooks.
pub(crate) struct Ran<'a, C: Component> {
    pub(crate) id: Id,
    pub(crate) key: Option<u128>,
    pub(crate) place: Place,
    pub(crate) size: Vec2,
    pub(crate) rect: Rect,
    pub(crate) tokens: &'a Arc<Tokens>,
    pub(crate) sweep: Option<craft_ui_rt::State<sweep::Sweep>>,
    pub(crate) begun: &'a Begun<C>,
    pub(crate) value: ValueStep<ValueOf<C>>,
    pub(crate) tip: Option<TipFn<'a>>,
    pub(crate) context: Context,
    /// The reveal the enclosing flow hands its scroll body (`Ui::scroll_to_rect`).
    pub(crate) flow_reveal: &'a mut Option<Pos2>,
}

/// The `Cx` a kept node's `finish` gets: no input, the kept facts, `is_settled` true.
fn settled_core<'a>(
    begun_env: &'a Env,
    resolved: &'a Resolved<'a>,
    memory: &'a mut MemoryStage,
    popups: &'a mut PopupStates,
    response: Response,
    begun: &steps::EnvStep,
    run: RunIo<'a>,
) -> CxCore<'a> {
    core_for(begun_env, resolved, memory, popups, response, begun.command.clone(), run)
}

/// The output of a kept node: `finish` on a settled `Cx` with the kept facts and no input.
pub(crate) fn settle<C: Component>(
    component: C,
    kept: &mut Kept,
    cx: &mut BuildCx<'_>,
    rect: Rect,
    begun: &Begun<C>,
    value: ValueStep<ValueOf<C>>,
    context: Context,
) -> C::Output {
    let node = cx.node();
    let response = Response::new(kept.id, rect, Some(node), cx.interaction(node), kept.dragging, InputFacts::default());
    let resolved = begun.resolved(kept.id);
    let mut core = settled_core(&begun.env, &resolved, &mut kept.memory, &mut kept.popups, response, &begun.step1, RunIo::settled(context));
    place_core(&mut core, &component, &begun.style, rect, &resolved, begun.env.direction.is_rtl());
    finish_with(component, &mut core, &begun.style, value)
}

/// The output of a component a measuring run reached: `finish` on a settled `Cx` with no node
/// behind it, so its response has no interaction state and logs no reads.
pub(crate) fn finish_measured<C: Component>(
    component: C,
    id: Id,
    rect: Rect,
    begun: &Begun<C>,
    value: ValueStep<ValueOf<C>>,
    context: Context,
) -> C::Output {
    let response = Response::new(id, rect, None, Default::default(), false, InputFacts::default());
    let resolved = begun.resolved(id);
    let mut stage = MemoryStage::open_peek(&context, id);
    let mut popups = PopupStates::default();
    let mut core = settled_core(&begun.env, &resolved, &mut stage, &mut popups, response, &begun.step1, RunIo::measuring(context));
    core.rect = rect;
    core.content_rect = rect;
    core.visible = rect;
    finish_with(component, &mut core, &begun.style, value)
}

fn finish_with<'a, C: Component>(component: C, core: &'a mut CxCore<'a>, style: &'a C::Style, value: ValueStep<ValueOf<C>>) -> C::Output {
    component.finish(Cx { core, style, shown: value.value, mixed: value.mixed, events: Vec::new(), applied: 0, validity: Validity::Ok })
}

/// The geometry fields of a core: the rect, the content rect inside the padding (mirrored for right
/// to left) and the visible rect.
fn place_core<C: Component>(core: &mut CxCore<'_>, component: &C, style: &C::Style, rect: Rect, resolved: &Resolved<'_>, rtl: bool) {
    let flow = Geometry::new(rect, rtl && !resolved.ltr_axis);
    core.rect = rect;
    core.content_rect = flow.inset(steps::padding(component.padding(rect, style), resolved.pad));
    core.flow = flow;
    core.visible = rect;
}

/// Steps 7 to 17 of a leaf (no shape hooks).
pub(crate) fn run<C: Component>(component: C, kept: &mut Kept, cx: &mut BuildCx<'_>, ran: Ran<'_, C>) -> C::Output {
    run_shaped(component, kept, cx, ran, &mut NoShape)
}

/// Steps 7 to 17 after `measure` and the place, with the shape's hooks at steps 7, 8, 9, 11, 13 and
/// 15. The node's layout, paint and access run in this frame.
pub(crate) fn run_shaped<C: Component>(
    mut component: C,
    kept: &mut Kept,
    cx: &mut BuildCx<'_>,
    ran: Ran<'_, C>,
    hooks: &mut dyn ShapeHooks<C>,
) -> C::Output {
    cx.count_run();
    cx.mark_changed();
    let node = cx.node();
    let Ran { id, key, place, size, rect, tokens, sweep, begun, value, tip, context, flow_reveal } = ran;
    // Before the core borrows the node's memory and popups for the run.
    kept.note_id(cx, id);
    let resolved = begun.resolved(id);
    let facts = begun.facts;
    let style = &*begun.style;
    let rtl = begun.env.direction.is_rtl() && !resolved.ltr_axis;
    let board = cx.provided::<CommandBoard>();
    // Interaction reads of the hooks below (a `Response` read in `update`, `parts` or `access`) are
    // build reads of this node (contract-rt 15.6 rule 2). `finish`'s reads belong to the scope that
    // shows the node, which reads its output.
    let reads = response::begin_reads(false);

    // 8 sense, from the queue the node's retained half filled since its last run, and for a collection the
    // presses the nodes under it took.
    let mut input = std::mem::take(&mut kept.queue);
    input.extend(super::node::MenuBar::take(cx));
    if hooks.watches_presses() {
        input.extend(super::node::PressWatch::take(kept));
        super::node::PressWatch::provide(kept, cx);
    }
    let (mut in_facts, dragging) = input_facts(&input, rect.min, kept.dragging);
    in_facts.disabled = !resolved.enabled;
    kept.dragging = dragging;
    let interact = cx.interaction(node);
    let response = Response::new(id, rect, Some(node), interact, dragging, in_facts);
    let cells = kept.memory.recall::<InboxCells>(sub(id, "inbox"));
    let inbox = InboxIn::read(&cells, cx);
    // A row's facts from its collection, read so that a change runs the row (15.5).
    let row = super::items::row_facts(cx, id);
    let stop = C::STOPS && steps::takes_focus(&resolved);
    // Only a stop can hold focus, so only a stop reads it: a focus move reruns the stops it touches.
    let focused = stop && cx.read_interaction(node, InteractFact::Focused);
    let swept = match (component.sweep_value(), sweep) {
        (Some(_), Some(cell)) => sweep::take(cx, cell, node),
        _ => None,
    };
    let mut core = core_for(
        &begun.env,
        &resolved,
        &mut kept.memory,
        &mut kept.popups,
        response,
        begun.step1.command.clone(),
        RunIo {
            ime_history: kept.ime.history(),
            ime_body: kept.ime.body.clone().into(),
            hover_point: cx.hover_point(node),
            ..RunIo::new(&input, context.clone())
        },
    );
    place_core(&mut core, &component, style, rect, &resolved, begun.env.direction.is_rtl());
    // The popups live on the node; `PopupHandle` reads and changes them through the run's core.
    core.popups.begin_run(cx);
    core.run.inbox = inbox;
    core.run.swept = swept;
    core.row = row.clone();
    core.parted = true;
    core.node = true;
    core.faded = facts.faded && !resolved.enabled;
    core.focus = super::focus::facts(&kept.focus, focused, interact.focus_visible);
    core.intents = super::intents::of_queue(core.run.input, |sub| kept.parts.iter().find(|p| p.id == sub).map(|p| p.part), || rtl);
    let mut cxx = Cx {
        core: &mut core,
        style,
        shown: value.value,
        mixed: value.mixed,
        events: value.cancelled.into_iter().collect(),
        applied: 0,
        validity: Validity::Ok,
    };

    // 7 parts: declared in node coordinates and kept as hit shapes until the next run.
    let own = hooks.sense(&component, own_sense(&component, &resolved));
    // A band row's own shape senses hover only: its presses reach the collection's press step.
    let sense = if row.is_some() { own.intersection(Sense::HOVER) } else { own };
    let mut parts = steps::PartsStep::default();
    let shaped = hooks.shaped();
    if facts.parts || shaped {
        let host = RunHost::new(cx, run_flow(id, Pos2::ZERO, place.room, tokens.clone(), Some(Some(node))), context.clone(), begun.env.clone());
        let mut lent: &RunHost<'_, '_> = &host;
        let cell: RefCell<&mut dyn UiHost> = RefCell::new(&mut lent);
        let input = MeasureIn { env: &begun.env, proposal: Proposal::Ideal, room: Some(place.room.x), host: &cell };
        let items =
            hooks.geometry(&component, &steps::measure_cx::<C>(&input, style, &resolved, cxx.shown.as_deref()), (rect, cxx.core.content_rect), rtl);
        let mut extra = |c: &mut C, r: Rect, pcx: &mut crate::contract::PartsCx<'_, C>| hooks.parts(c, r, pcx);
        steps::declare_parts(&mut component, &mut cxx, rect, &mut parts, &input, Some((node, interact)), items, &mut extra);
    }
    kept.parts = steps::kept_parts(&parts, rect.min);
    kept.sense = sense;
    kept.sweep = sweep;
    kept.toggle = component.sweep_value();
    kept.cursor = component.cursor().as_ref().map(super::cursor::platform);
    let results = part_results(&kept.parts, &parts, own, cxx.core.run.input, rect, interact.hovered);
    cxx.core.results = results.1;
    cxx.core.press = results.0.press;
    cxx.core.toggles = steps::toggles(&parts);
    cxx.core.value_parts = steps::declared_with(&parts, |extras| extras.value);
    cxx.core.state_layer_parts = steps::declared_with(&parts, |extras| extras.state_layer);
    cxx.core.own_reset_parts = steps::declared_with(&parts, |extras| extras.own_reset);
    cxx.core.focus_shapes = steps::focus_shapes(&parts);
    cxx.core.active = cxx.core.focus.part.filter(|part| cxx.core.results.contains_key(&(part.key, part.index)));
    cxx.core.activation = activation_of(&resolved, cxx.core.run.input, own);
    hooks.sensed(&mut component, &mut cxx);
    // 8b the editor part takes the run's input in queue order.
    crate::contract::editor::run(&mut cxx, &mut parts.text, component.keys());
    // 8b the text part runs the editing engine over the same queue; selection parts take their edit commands.
    let scope = cx.provided::<crate::primitives::text_adapter::ScopeEscape>().unwrap_or_default();
    let access = cx.access_enabled();
    let text_keys = {
        let host = RunHost::new(cx, run_flow(id, Pos2::ZERO, place.room, tokens.clone(), Some(Some(node))), context.clone(), begun.env.clone());
        let text_in = crate::contract::text::TextIn {
            services: &host,
            keys: component.keys(),
            rtl,
            scrubs: own.contains(Sense::DRAG),
            scope,
            access,
            bar_returns_here: bar_returns_here(cx),
        };
        crate::contract::text::run(&mut cxx, &mut parts.text, text_in)
    };
    crate::contract::text::run_selections(cxx.core, &parts.text.selections);
    kept.ime.edit = cxx.core.edit_paint.take();

    // 9 update, then the stops it decides (a member that joins its body's roving group).
    steps::gesture_gate(&component, &mut cxx);
    steps::update_inputs(cxx.core);
    drive_toggles(cxx.core, facts.overlays, std::any::type_name::<C>());
    steps::edit_intents(&mut component, &mut cxx);
    steps::scrub_by_label(&component, &mut cxx);
    component.update(&mut cxx);
    hooks.updated(&mut component, &mut cxx);
    steps::record_context(cxx.core);
    if let Some(cursor) = crate::contract::cx::run_cursor(cxx.core) {
        kept.cursor = Some(super::cursor::platform(&cursor));
    }
    kept.focus.ring = stop.then(|| craft_ui_rt::Stroke::new(tokens.stroke.focus, tokens.color.line.focus));
    kept.focus.stop = stop.then(|| text_keys.or_else(|| cxx.core.editor.as_ref().map(|editor| editor.keys)).unwrap_or_else(|| component.keys()));
    kept.focus.press_only = facts.text() && resolved.read_only;
    kept.focus.role = hooks.key_role(&component);
    kept.focus.scope = component.focus_scope();
    kept.focus.roving = component.roving();
    kept.focus.holds_escape = cxx.core.run.holds_escape.get();

    // 10 overlay: the popups show their content as layer children of this node.
    if facts.overlays {
        let toggling = toggle_rects(cxx.core);
        let host = RunHost::new(cx, run_flow(id, rect.min, place.room, tokens.clone(), None), context.clone(), begun.env.clone());
        let mut lent: &RunHost<'_, '_> = &host;
        run_overlay(&mut component, &mut cxx, &mut lent, &begun.env, toggling, true);
    }

    // 11 apply: the run's edits reach the binding, and the shown value follows.
    let wrote = steps::apply_values(&mut component, &mut cxx).wrote();
    hooks.apply(&mut component, &mut cxx);
    steps::register_reset(&mut component, &mut cxx);

    // 12 state and 13 paint, once per pointer look the paint read (section 8, 15.6); then the
    // shape's content (a collection's rows), whose children join the ones the paint showed.
    let open = facts.overlays && cxx.core.popups.any_open();
    let host = RunHost::new(cx, run_flow(id, rect.min, place.room, tokens.clone(), None), context.clone(), begun.env.clone());
    let (variants, values) = record_variants(&mut component, &mut cxx, &host, begun, own, &kept.parts, open, &*hooks);
    kept.variants = variants;
    if shaped {
        let mut lent: &RunHost<'_, '_> = &host;
        crate::contract::paint_ui::host_ui(&mut lent, |ui| hooks.content(&mut component, ui, &begun.env, &mut cxx, &values));
        kept.members = hooks.take_member_looks();
    }
    kept.placed = host.into_flow().placed;

    // 15 settle: edits a paint adopted reach the binding; a reveal goes to the scroll owner.
    let settled = steps::settle(&mut component, &mut cxx);
    hooks.settle(&mut component, &mut cxx);
    let wrote = wrote || settled.wrote();
    let reveal = cxx.core.reveal.take();

    // 16 report: the access node, once per run and only while the window's access tree is on, then
    // the tooltip and the kept retained facts.
    kept.close_guard = component.close_guard();
    kept.type_name = std::any::type_name::<C>();
    kept.command = begun.step1.command.as_ref().map(|command| craft_keymap::CommandRef::clone(&command.command));
    kept.drag = component.drag_source();
    kept.drop = component.drop_target();
    let mut built = cx.access_enabled().then(|| report::<C>(&component, &cxx, &resolved, own, &kept.parts, row.as_deref(), cx)).flatten();
    kept.ime.text_runs = match (built.as_mut(), cxx.core.text_access.borrow().as_ref()) {
        (Some(built), Some(facts)) => crate::runtime::access::text::write(built, facts, cx.node()),
        _ => Vec::new(),
    };
    let (access, subs) = built.map_or((None, Vec::new()), |b| (Some(b.node), b.subs));
    kept.access = access;
    kept.access_subs = subs;
    for (text, politeness) in std::mem::take(&mut cxx.core.run.announcements) {
        let politeness = match politeness {
            crate::runtime::access::Politeness::Polite => craft_ui_rt::AnnouncePoliteness::Polite,
            crate::runtime::access::Politeness::Assertive => craft_ui_rt::AnnouncePoliteness::Assertive,
        };
        cx.announce(text, politeness);
    }
    let tip_text = crate::contract::tooltip::text_of(cxx.core, component.shows_label(), <C as Meta>::CANVAS_CONTROL, None);
    tip::show(&mut kept.tip, tip_text, true, cx, id, rect, tokens);
    let _ = tip;
    if let Some((target, spec)) = cxx.core.run.level.take() {
        super::motion::Level::aim(kept.level.get_mut(), target, spec);
    }
    response::end_reads(reads, cx);

    // What the run asked of its node, applied after it: the menu-bar registration, a build wake (a
    // wheel ease paying out, a popup's dwell), the drag it started and the press it cancelled.
    kept.menu_bar = cxx.core.run.menu_bar.get();
    super::node::MenuBar::register(kept.menu_bar, cx);
    kept.notch = component.wheel_notch();
    super::ime::after_run(&mut kept.ime, &cxx.core.run, rect.min, cx);
    let wake = cxx.core.run.wake.get().then(|| cx.now());
    if let Some(at) = [wake, cxx.core.run.build_at.get()].into_iter().flatten().reduce(f64::min) {
        cx.request_build_at(Some(at));
    }
    if let Some(source) = cxx.core.run.drag_start.borrow_mut().take() {
        kept.drag_session = cx.start_drag(source.start(tokens));
    }
    if let Some(reason) = cxx.core.press_cancel.take().filter(|_| kept.press.is_some()) {
        cx.cancel_press(reason);
    }
    if cxx.core.run.surrender.get() {
        cx.surrender_focus();
    }

    // Focus: the ring's shape for the part that holds focus (or the node's own), then the moves the
    // run asked for, applied after the run.
    let focus_moves = std::mem::take(&mut *cxx.core.run.focus_requests.borrow_mut());
    for target in &focus_moves {
        if let crate::runtime::focus::FocusTarget::Part(target, part) = target
            && *target == id
        {
            kept.focus.part = Some(*part);
        } else if matches!(target, crate::runtime::focus::FocusTarget::Id(target) if *target == id) {
            kept.focus.part = None;
        }
    }
    kept.focus.ring_shape = ring_shape(&cxx, &kept.focus, rect);

    // 17 finish: the activation fires the bound command once, at event time on the board's handler.
    let activated = component.activated(&cxx) && resolved.enabled;
    let injected = cxx.core.injected.take().filter(|_| resolved.enabled);
    let fired = activated || injected.is_some();
    steps::before_finish(cxx.core, fired, injected, board.as_ref(), cx);
    if fired {
        // The layers the activation happened in close, unless it keeps them open.
        note_activation(cx, cxx.core.keep_open);
    }
    let wants = std::mem::take(&mut cxx.core.run.inbox.wants);
    let fires = std::mem::take(&mut cxx.core.run.fires);
    kept.id = id;
    kept.key = if wrote { component_key(&component) } else { key };
    kept.place = place;
    kept.size = size;
    // The size cache holds only this run's size.
    kept.sizes.clear();
    kept.sizes.push((place.room, size));
    let out = component.finish(cxx);
    for target in focus_moves {
        super::ids::request_focus(cx, target);
    }
    if let Some(rect) = reveal
        && flow_reveal.is_none()
    {
        // Item 6: the nearest scroll body around glides to it, as for `Ui::scroll_to_rect`.
        *flow_reveal = Some(rect.min);
    }
    if let Some(board) = board {
        // Commands the run fired (`Cx::fire`) run after it, in order, from this node.
        for (command, source, mods) in fires {
            if !board.enabled(&command) {
                continue;
            }
            let fired = crate::runtime::commands::Fired::new(command, source, crate::runtime::commands::FirePhase::Press, cx.now()).mods(mods);
            board.run_from(cx, &fired);
        }
        let node = cx.node();
        register_inboxes(wants, node, id, &mut kept.memory, &board, cx.store_mut());
    }
    out
}

/// Whether the focus is on the window's menu bar, or in what it opened, and F10 took it from this node: the node's
/// text part keeps its selection for the bar's menus. An untracked read: a focus move to the bar runs the node that lost
/// it.
fn bar_returns_here(cx: &BuildCx<'_>) -> bool {
    let Some(menu) = cx.provided::<super::node::MenuBar>() else { return false };
    let rt = cx.runtime();
    let focused = rt.focused(cx.window());
    let within = |bar: NodeId| {
        let mut at = focused;
        while let Some(node) = at {
            if node == bar {
                return true;
            }
            at = rt.parent(node);
        }
        false
    };
    menu.returns_to(within) == Some(cx.node())
}

/// The ring round the part that holds focus, with the shape its `focus_shape` declared, else round
/// the node: the shape `Cx::focus_shape` set, else its rect with its surface's corners.
fn ring_shape<C: Component>(cx: &Cx<'_, C>, focus: &super::focus::FocusKept, rect: Rect) -> RingShape {
    if focus.ring.is_none() {
        return RingShape::Bounds;
    }
    let core = &*cx.core;
    if let Some(part) = focus.part
        && let Some(result) = core.results.get(&(part.key, part.index))
    {
        let shape = core.focus_shapes.iter().find(|(key, _)| *key == (part.key, part.index)).map(|(_, shape)| shape);
        return RingShape::of(shape, result.rect, craft_ui_rt::CornerRadii::ZERO, rect.min);
    }
    let radii =
        crate::style::StyleView::surface(cx.style).map_or(craft_ui_rt::CornerRadii::ZERO, |look| crate::primitives::items::radii(look.radius));
    RingShape::of(core.focus_shape.as_ref(), rect, radii, rect.min)
}

/// The window rects of the parts that toggle a popup, which `OverlayUi::anchor_part` anchors to.
fn toggle_rects(core: &CxCore<'_>) -> Vec<(PartRef, Rect)> {
    core.toggles.iter().filter_map(|t| core.results.get(&(t.part.key, t.part.index)).map(|r| (t.part, r.rect))).collect()
}

/// The output of a kept shaped node (a collection): its content runs on the settled `Cx` (the
/// collection's rows reconcile, each keeping its node when its key and facts hold), and its layer
/// children are reached; its own hooks stay kept.
#[allow(clippy::too_many_arguments)]
fn settle_shaped<C: Component>(
    mut component: C,
    kept: &mut Kept,
    cx: &mut BuildCx<'_>,
    rect: Rect,
    begun: &Begun<C>,
    value: ValueStep<ValueOf<C>>,
    context: Context,
    tokens: &Arc<Tokens>,
    hooks: &mut dyn ShapeHooks<C>,
) -> C::Output {
    reach_layers(&mut component, kept, cx, rect, begun, tokens, context.clone());
    if hooks.watches_presses() {
        super::node::PressWatch::provide(kept, cx);
    }
    let node = cx.node();
    let response = Response::new(kept.id, rect, Some(node), cx.interaction(node), kept.dragging, InputFacts::default());
    let resolved = begun.resolved(kept.id);
    let room = kept.place.room;
    let mut core = settled_core(&begun.env, &resolved, &mut kept.memory, &mut kept.popups, response, &begun.step1, RunIo::settled(context.clone()));
    place_core(&mut core, &component, &begun.style, rect, &resolved, begun.env.direction.is_rtl());
    let mut cxx =
        Cx { core: &mut core, style: &*begun.style, shown: value.value, mixed: value.mixed, events: Vec::new(), applied: 0, validity: Validity::Ok };
    let mut values = AppValues::default();
    component.content_env(&cxx, &mut values);
    let host = RunHost::new(cx, run_flow(kept.id, rect.min, room, tokens.clone(), None), context, begun.env.clone());
    {
        let mut lent: &RunHost<'_, '_> = &host;
        crate::contract::paint_ui::host_ui(&mut lent, |ui| hooks.content(&mut component, ui, &begun.env, &mut cxx, &values));
    }
    let placed = host.into_flow().placed;
    let out = component.finish(cxx);
    kept.placed = placed;
    out
}

/// What the component senses itself: a disabled one that shows a reason on focus stays a Tab stop
/// and senses nothing else; one bound to an edit command keeps the focus where it is on a press.
fn own_sense<C: Component>(component: &C, resolved: &Resolved<'_>) -> Sense {
    let sense = component.sense();
    match resolved.enabled {
        true if resolved.command.is_some_and(|command| command.edit) => sense.difference(Sense::TAKE_FOCUS).union(Sense::KEEP_FOCUS),
        true => sense,
        false if steps::takes_focus(resolved) => Sense::HOVER | sense.intersection(Sense::FOCUS),
        false => Sense::HOVER,
    }
}

/// The activation of a run: a key or AccessKit activation, or a primary click; none for a component
/// that takes none (read only, or no click sense).
fn activation_of(resolved: &Resolved<'_>, input: &[NodeInput], sense: Sense) -> Option<Activation> {
    if !steps::takes_activation(resolved) {
        return None;
    }
    input.iter().rev().find_map(|i| match i {
        NodeInput::Intent { intent: Intent::Activate, .. } => Some(Activation { by_key: true, mods: craft_keymap::Mods::NONE }),
        NodeInput::Access(a) if a.action == accesskit::Action::Click && a.sub.is_none() => {
            Some(Activation { by_key: true, mods: craft_keymap::Mods::NONE })
        }
        NodeInput::Pointer { gesture: Gesture::Up { click: true, .. }, button: PointerButton::Primary, part: 0, mods, .. } if sense.takes_press() => {
            Some(Activation { by_key: false, mods: *mods })
        }
        _ => None,
    })
}

/// Step 16's access node: the description, value words and access key from the resolved props,
/// then the component's own `access`, then the side label of its label scope; the label, the
/// state flags and the actions its sense and popups give. Built once per run.
fn report<C: Component>(
    component: &C,
    cx: &Cx<'_, C>,
    resolved: &Resolved<'_>,
    sense: Sense,
    parts: &[KeptPart],
    row: Option<&crate::runtime::env::ItemFacts>,
    build: &mut BuildCx<'_>,
) -> Option<super::access::Built> {
    let Role::Node(role) = C::ROLE else { return None };
    build.count_access_build();
    let mut access = Access::default();
    if let Some(row) = row {
        access.position(row.position);
    }
    if let Some(description) = resolved.description {
        access.description(description);
    }
    if let Some(words) = component.value_text(cx) {
        access.value(words);
    }
    if let Some(key) = resolved.access_key {
        access.access_key(key);
    }
    component.access(&mut access, cx);
    crate::contract::cx::apply_field_label(cx.core, &mut access);
    // A control shown with `PaintUi::inner` folds into its owner's node: it hands the owner its value
    // text and keeps no node of its own. An owner whose `access` set no value carries its control's.
    let ctx = cx.core.context();
    if let Some(owner) = fold::owner_of(ctx, cx.core.id) {
        fold::offer(ctx, owner, fold::Folded { inner: cx.core.id, value: access.value.take() });
        return None;
    }
    if access.value.is_none() {
        access.value = fold::offered(ctx, cx.core.id).and_then(|folded| folded.value);
    }
    let mut flags = cx.core.state.flags;
    flags.set(craft_theme::Flags::SELECTED, resolved.selected.unwrap_or(flags.contains(craft_theme::Flags::SELECTED)));
    let pressable: SmallVec<[PartRef; 4]> = parts.iter().filter(|p| p.sense.takes_press()).map(|p| p.part).collect();
    let facts = super::access::NodeFacts {
        label: if component.shows_label() { resolved.label } else { "" },
        flags,
        enabled: resolved.enabled,
        pressable: &pressable,
        clickable: sense.takes_press(),
        focusable: !sense.intersection(Sense::FOCUS).is_empty(),
        context_menu: cx.core.popups.offers_context_menu(),
    };
    super::access::finish(access, role, &facts)
}

/// Runs the overlay and tooltip passes of a node that keeps its own outputs while a build reaches
/// its layer children (a marked node under one of them, or a container whose body runs): the popups
/// keep their bodies unless marked, and a popup that opened or closed here makes the node run in
/// this frame.
pub(crate) fn reach_layers<C: Component>(
    component: &mut C,
    kept: &mut Kept,
    cx: &mut BuildCx<'_>,
    rect: Rect,
    begun: &Begun<C>,
    tokens: &Arc<Tokens>,
    context: Context,
) {
    let id = kept.id;
    if C::OVERLAYS {
        let node = cx.node();
        let response = Response::new(id, rect, Some(node), cx.interaction(node), kept.dragging, InputFacts::default());
        let resolved = begun.resolved(id);
        let mut core =
            settled_core(&begun.env, &resolved, &mut kept.memory, &mut kept.popups, response, &begun.step1, RunIo::settled(context.clone()));
        place_core(&mut core, component, &begun.style, rect, &resolved, begun.env.direction.is_rtl());
        core.popups.begin_run(cx);
        let mut cxx = Cx { core: &mut core, style: &*begun.style, shown: None, mixed: false, events: Vec::new(), applied: 0, validity: Validity::Ok };
        let host = RunHost::new(cx, run_flow(id, rect.min, kept.place.room, tokens.clone(), None), context, begun.env.clone());
        let changed = {
            let mut lent: &RunHost<'_, '_> = &host;
            run_overlay(component, &mut cxx, &mut lent, &begun.env, Vec::new(), false)
        };
        drop(host);
        drop(cxx);
        if changed {
            cx.rebuild();
        }
    }
    tip::show(&mut kept.tip, None, false, cx, id, rect, tokens);
}

/// Steps 12 and 13 per pointer look: `Rest` always; `Hover` when the paint read `Hovered`; `Press`
/// when it read `Pressed` and the node takes presses. During a look's recording the pointer facts
/// answer that look's state. The first recording shows the current pointer state: a paint that
/// showed children keeps that one recording (children reconcile once), and its pointer reads
/// become build reads (15.6 rule 5); otherwise the looks it read are recorded beside it.
#[allow(clippy::too_many_arguments)]
fn record_variants<C: Component>(
    component: &mut C,
    cxx: &mut Cx<'_, C>,
    host: &RunHost<'_, '_>,
    begun: &Begun<C>,
    sense: Sense,
    parts: &[KeptPart],
    open: bool,
    hooks: &dyn ShapeHooks<C>,
) -> (SmallVec<[Variant; 1]>, AppValues) {
    let (node, now) = {
        let cx = host.cx();
        let node = cx.node();
        (node, cx.interaction(node))
    };
    let takes_press = sense.takes_press() || parts.iter().any(|p| p.sense.takes_press());
    let scale = host.scale;
    let mut values = AppValues::default();
    let mut paint_as = |look: Pointer, cxx: &mut Cx<'_, C>, values: &mut AppValues| {
        let (hovered, pressed) = match look {
            Pointer::Rest => (false, false),
            Pointer::Hover => (true, false),
            Pointer::Press => (true, true),
        };
        let ((variant, children), read) = crate::style::pointer_reads(|| {
            steps::state(component, cxx, open, hovered, pressed, begun.env.ink);
            let canvas = Canvas::new(host, cxx.core.rect.min, Rect::EVERYTHING);
            let fade = steps::fade(&canvas, &begun.env, cxx.core);
            if let Some(items) = steps::surface_items(&begun.env, cxx.core, &*begun.style, scale) {
                canvas.extend(items);
            }
            steps::paint_part_layers(&canvas, &begun.env, cxx.core);
            let mut lent: &RunHost<'_, '_> = host;
            let mut pui = paint_ui(&mut lent, &canvas, &begun.env, cxx.core, &*begun.style, look);
            // What lies under the shape's content (a collection's surface), then the component's paint.
            hooks.paint(component, &mut pui, cxx.core.rect);
            component.paint(&mut pui, cxx);
            let mut children = pui.showed_child.get();
            drop(pui);
            if look == Pointer::Rest {
                component.content_env(cxx, values);
            }
            let mut pui = paint_ui(&mut lent, &canvas, &begun.env, cxx.core, &*begun.style, look);
            component.paint_over(&mut pui, cxx);
            children |= pui.showed_child.get();
            drop(pui);
            steps::unfade(&canvas, cxx.core, fade);
            let (content, overlay) = canvas.finish();
            (Variant { look, content, overlay }, children)
        });
        (variant, children, read)
    };
    let current = if now.pressed && takes_press {
        Pointer::Press
    } else if now.hovered {
        Pointer::Hover
    } else {
        Pointer::Rest
    };
    let mut out: SmallVec<[Variant; 1]> = SmallVec::new();
    let (first, children, read) = paint_as(current, cxx, &mut values);
    if children {
        // One look, recorded in the current pointer state; the node reruns when a pointer fact its
        // paint read flips.
        out.push(Variant { look: Pointer::Rest, ..first });
    } else {
        // The look the first recording shows: the facts its paint did not read answer as at rest.
        let shown = match current {
            Pointer::Press if read.press => Pointer::Press,
            Pointer::Press | Pointer::Hover if read.hover => Pointer::Hover,
            _ => Pointer::Rest,
        };
        let mut first = Some(first);
        let mut look_of = |look: Pointer, cxx: &mut Cx<'_, C>, values: &mut AppValues| match first.take_if(|_| look == shown) {
            Some(first) => Variant { look, ..first },
            None => paint_as(look, cxx, values).0,
        };
        out.push(look_of(Pointer::Rest, cxx, &mut values));
        if read.hover {
            out.push(look_of(Pointer::Hover, cxx, &mut values));
        }
        if read.press && takes_press {
            out.push(look_of(Pointer::Press, cxx, &mut values));
        }
    }
    drop(paint_as);
    if children {
        let mut cx = host.cx();
        if read.hover {
            cx.read_interaction(node, InteractFact::Hovered);
        }
        if read.press {
            cx.read_interaction(node, InteractFact::Pressed);
        }
    }
    (out, values)
}

/// The response's input facts from a run's queue, and whether a drag of the node's press runs after
/// it (`dragging` before it).
fn input_facts(input: &[NodeInput], origin: Pos2, mut dragging: bool) -> (InputFacts, bool) {
    let mut facts = InputFacts::default();
    for i in input {
        match i {
            NodeInput::Pointer { gesture, pos, button, .. } => {
                facts.pointer = Some(*pos + origin.to_vec2());
                match *gesture {
                    Gesture::LongPress => facts.long_pressed = true,
                    Gesture::DragStart { .. } => {
                        facts.drag_started = true;
                        dragging = true;
                    }
                    Gesture::Drag { delta } => facts.drag_delta += delta,
                    Gesture::Up { click, clicks } => {
                        if click && *button == PointerButton::Primary {
                            facts.clicked = true;
                            facts.double_clicked |= clicks == 2;
                        }
                        facts.secondary_clicked |= click && *button == PointerButton::Secondary;
                        facts.drag_stopped |= dragging;
                        dragging = false;
                    }
                    Gesture::Cancel(_) => {
                        facts.drag_stopped |= dragging;
                        dragging = false;
                    }
                    Gesture::Down { .. } => facts.pressed |= *button == PointerButton::Primary,
                }
            }
            NodeInput::Access(a) => {
                let click = a.action == accesskit::Action::Click && a.sub.is_none();
                facts.clicked |= click;
                facts.activated |= click;
                facts.context |= a.action == accesskit::Action::ShowContextMenu && a.sub.is_none();
            }
            NodeInput::Intent { intent: Intent::Activate, .. } => {
                facts.clicked = true;
                facts.activated = true;
            }
            NodeInput::Intent { intent: Intent::ContextMenu, .. } => facts.context = true,
            NodeInput::Focus { gained, .. } => {
                facts.gained_focus |= *gained;
                facts.lost_focus |= !*gained;
            }
            _ => {}
        }
    }
    (facts, dragging)
}

/// The press of the node's own shape and of each declared part, folded from the run's queue: the
/// last phase on it, drag moves summed. Positions are window coordinates.
fn part_results(
    kept: &[KeptPart],
    parts: &steps::PartsStep,
    sense: Sense,
    input: &[NodeInput],
    rect: Rect,
    hovered: bool,
) -> (PartResult, crate::primitives::sensing::PartResults) {
    let press_of = |id: u64| {
        let mut state = PressState::IDLE;
        for i in input {
            let NodeInput::Pointer { gesture, pos, part, start, mods, .. } = i else { continue };
            if *part != id {
                continue;
            }
            let at = *pos + rect.min.to_vec2();
            let mut start = *start;
            start.origin += rect.min.to_vec2();
            let phase = match *gesture {
                Gesture::Down { .. } => PressPhase::down(start),
                Gesture::LongPress => PressPhase::held(start, at),
                Gesture::DragStart { .. } => PressPhase::drag_start(start, at),
                Gesture::Drag { delta } => PressPhase::drag(start, at, drag_delta(state.phase) + delta),
                Gesture::Up { click, .. } => PressPhase::release(start, at, click),
                Gesture::Cancel(reason) => PressPhase::cancel(start, reason),
            };
            state = PressState::new(phase).mods(*mods).captured(phase.is_drag());
        }
        state
    };
    let own = PartResult { rect, press: press_of(0), hovered, sense, ..PartResult::IDLE };
    let results = kept
        .iter()
        .map(|p| {
            let inactive = steps::inactive(parts, p.part);
            let result = PartResult {
                rect: bounds(p.shape).translate(rect.min.to_vec2()),
                press: press_of(p.id),
                sense: p.sense,
                inactive,
                ..PartResult::IDLE
            };
            ((p.part.key, p.part.index), result)
        })
        .collect();
    (own, results)
}

fn drag_delta(phase: PressPhase) -> Vec2 {
    match phase {
        PressPhase::Drag { delta, .. } => delta,
        _ => Vec2::ZERO,
    }
}

fn bounds(shape: HitShape) -> Rect {
    match shape {
        HitShape::Rect(r) | HitShape::RoundedRect(r, _) => r,
        HitShape::Circle(c, radius) => Rect::from_center_size(c, Vec2::splat(2.0 * radius)),
    }
}

/// The ui-rt part id of a declared part (re-exported for the parts step).
pub(crate) fn kept_part_id(part: PartRef) -> u64 {
    part_id(part)
}

/// Fires `command` from code (`Cx::fire`, `Ui::fire`): the board runs its handler at once.
pub(crate) fn fire(board: &CommandBoard, cx: &mut BuildCx<'_>, fired: &crate::runtime::commands::Fired) -> bool {
    board.run_from(cx, fired)
}

/// The source a component's own activation fires its bound command with.
pub(crate) fn activation_source(activation: Option<Activation>) -> FireSource {
    match activation {
        Some(Activation { by_key: true, .. }) => FireSource::Key,
        _ => FireSource::Pointer,
    }
}
