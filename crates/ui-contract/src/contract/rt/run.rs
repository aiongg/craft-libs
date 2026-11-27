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

use std::sync::Arc;

use craft_behavior::{Flow as Geometry, Intent, PartRef, PressPhase, PressState, Proposal};
use craft_theme::Tokens;
use craft_ui_rt::{BuildCx, Gesture, HitShape, InlineState, InteractFact, NodeId, PointerButton, Pos2, Rect, Sense, Vec2};
use smallvec::SmallVec;

use super::access::Access;
use super::key::{component_key, fold, type_key};
use super::node::{ComponentNode, Kept, KeptPart, NodeInput, Place, Pointer, Variant, part_id};
use super::overlay::OverlayUi;
use super::response::{self, InputFacts, Response};
use super::sweep;
use super::tip;
use super::ui::{Flow, Ui};
use crate::contract::run::{ShowAt, TypeFacts, core_for, paint_ui};
use crate::contract::steps::{self, MeasureIn, ValueStep};
use crate::contract::{Activation, Common, Component, Cx, CxCore, Id, Internal, Meta, Resolved, TipFn, Validity, ValueOf};
use crate::primitives::{Canvas, FrameServices, PartResult};
use crate::runtime::access::Role;
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

/// Steps 1 and 2. `Env::of` registers its reads for the scope that shows the node, so an env write
/// reruns that scope (contract-rt 15.13 item 6).
pub(crate) fn begin<C: Component>(component: &mut C, ui: &mut Ui<'_, '_>) -> Begun<C> {
    let env = Env::of(ui);
    let common = std::mem::take(component.common_mut(Internal::new()));
    let step1 = steps::env(&env, &common, steps::variants::<C>(common.variant, &env), &mut |command| Some(ui.resolve_command(command.clone())));
    let style = steps::style::<C>(ui.build_cx(), &steps::style_cx(&env, &step1), component.style_key());
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

/// Shows a leaf: keeps it, or runs its hooks. In a measuring run it only reports its size: a kept
/// node's from its size cache, otherwise `measure`'s.
pub(crate) fn show_leaf<C: Component>(mut component: C, ui: &mut Ui<'_, '_>, at: ShowAt, tip: Option<TipFn<'_>>) -> C::Output {
    let begun = begin(&mut component, ui);
    let (id, child) = ui.flow.child(type_key::<C>(), begun.common.id.clone());
    let key = component_key(&component);
    let room = ui.flow.room();
    let tag = sweep_tag(&component, &ui.flow);
    let command = begun.step1.command.clone();
    if let Some(under) = ui.flow.measuring {
        let cached =
            under.and_then(|u| ui.cx.peek::<ComponentNode>(u, child)).filter(|p| holds(p.local, key, p.state)).and_then(|p| p.local.cached(room));
        let mut stage = MemoryStage::default();
        let value = steps::value(&mut component, &mut stage, id, command.as_deref());
        let size = cached.unwrap_or_else(|| measure(&mut component, &begun, id, room, &value, ui.cx));
        let rect = place(&mut ui.flow, None, at, size);
        return finish_measured(component, id, rect, &begun, value);
    }
    let Ui { cx, flow, .. } = ui;
    let node =
        ComponentNode { key: key.map_or(0, fold), tag, stop: C::STOPS && steps::takes_focus(&begun.resolved(id)), text: component.text_input() };
    cx.keyed(child, |cx| {
        cx.inline(node, |kept, state, cx| {
            let kept_outputs = holds(kept, key, state);
            let value = steps::value(&mut component, &mut kept.memory, id, command.as_deref());
            let size = match kept.cached(room).filter(|_| kept_outputs) {
                Some(size) => size,
                None => measure(&mut component, &begun, id, room, &value, cx),
            };
            let rect = place(flow, Some(cx.node()), at, size);
            if kept_outputs && size == kept.size {
                kept.cache(room, size);
                if state.path {
                    // A node in one of its layer children is marked: reach them.
                    reach_layers(&mut component, kept, cx, rect, &begun, &flow.tokens, flow.sweep);
                } else {
                    cx.keep_children();
                }
                return settle(component, kept, cx, rect, &begun, value);
            }
            let tokens = super::env::tokens(cx);
            run(
                component,
                kept,
                cx,
                Ran { id, key, place: Place { room }, size, rect, tokens: &tokens, sweep: flow.sweep, begun: &begun, value, tip },
            )
        })
    })
}

/// Step 6: the rect the node takes: the next place in the flow, or the rect it is shown at.
fn place(flow: &mut Flow, node: Option<NodeId>, at: ShowAt, size: Vec2) -> Rect {
    match at {
        ShowAt::Flow => flow.allocate(node, size, false),
        ShowAt::Rect(rect) => flow.place_at(node, rect),
    }
}

/// Step 5: `measure` over the room's content width, plus padding and the style's floors.
fn measure<C: Component>(component: &mut C, begun: &Begun<C>, id: Id, room: Vec2, value: &ValueStep<ValueOf<C>>, cx: &mut BuildCx<'_>) -> Vec2 {
    let resolved = begun.resolved(id);
    let scale = cx.scale();
    let (text, images) = cx.paint_services();
    let services = FrameServices::new(text, images, scale);
    let input = MeasureIn { env: &begun.env, proposal: Proposal::Ideal, room: Some(room.x), services: Some(&services) };
    let content = component.measure(&steps::measure_cx::<C>(&input, &begun.style, &resolved, value.value.as_deref()));
    steps::measure(&*begun.style, &input, content, Vec2::ZERO, &resolved)
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
}

/// The `Cx` a kept node's `finish` gets: no input, the kept facts, `is_settled` true.
fn settled_core<'a>(
    begun_env: &'a Env,
    resolved: &'a Resolved<'a>,
    memory: &'a mut MemoryStage,
    response: Response,
    begun: &steps::EnvStep,
) -> CxCore<'a> {
    let mut core = core_for(begun_env, resolved, memory, response, begun.command.clone());
    core.settled = true;
    core
}

/// The output of a kept node: `finish` on a settled `Cx` with the kept facts and no input.
pub(crate) fn settle<C: Component>(
    component: C,
    kept: &mut Kept,
    cx: &mut BuildCx<'_>,
    rect: Rect,
    begun: &Begun<C>,
    value: ValueStep<ValueOf<C>>,
) -> C::Output {
    let node = cx.node();
    let response = Response::new(kept.id, rect, Some(node), cx.interaction(node), kept.dragging, InputFacts::default());
    let resolved = begun.resolved(kept.id);
    let mut core = settled_core(&begun.env, &resolved, &mut kept.memory, response, &begun.step1);
    place_core(&mut core, &component, &begun.style, rect, &resolved, begun.env.direction.is_rtl());
    finish_with(component, &mut core, &begun.style, value)
}

/// The output of a component a measuring run reached: `finish` on a settled `Cx` with no node
/// behind it, so its response has no interaction state and logs no reads.
pub(crate) fn finish_measured<C: Component>(component: C, id: Id, rect: Rect, begun: &Begun<C>, value: ValueStep<ValueOf<C>>) -> C::Output {
    let response = Response::new(id, rect, None, Default::default(), false, InputFacts::default());
    let resolved = begun.resolved(id);
    let mut stage = MemoryStage::default();
    let mut core = settled_core(&begun.env, &resolved, &mut stage, response, &begun.step1);
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

/// Steps 7 to 17 after `measure` and the place. The node's layout, paint and access run in this
/// frame.
pub(crate) fn run<C: Component>(mut component: C, kept: &mut Kept, cx: &mut BuildCx<'_>, ran: Ran<'_, C>) -> C::Output {
    cx.count_run();
    cx.mark_changed();
    let node = cx.node();
    let Ran { id, key, place, size, rect, tokens, sweep, begun, value, tip } = ran;
    let resolved = begun.resolved(id);
    let facts = begun.facts;
    let style = &*begun.style;
    let rtl = begun.env.direction.is_rtl() && !resolved.ltr_axis;
    let board = cx.provided::<CommandBoard>().cloned();
    // Interaction reads of the hooks below (a `Response` read in `update`, `parts` or `access`) are
    // build reads of this node (contract-rt 15.6 rule 2). `finish`'s reads belong to the scope that
    // shows the node, which reads its output.
    let reads = response::begin_reads(false);

    // 8 sense, from the queue the node's retained half filled since its last run.
    let input = std::mem::take(&mut kept.queue);
    let (in_facts, dragging) = input_facts(&input, rect.min, kept.dragging);
    kept.dragging = dragging;
    let interact = cx.interaction(node);
    let response = Response::new(id, rect, Some(node), interact, dragging, in_facts);
    let cells = kept.memory.recall::<InboxCells>(sub(id, "inbox"));
    let inbox = InboxIn::read(&cells, cx);
    let stop = C::STOPS && steps::takes_focus(&resolved);
    // Only a stop can hold focus, so only a stop reads it: a focus move reruns the stops it touches.
    let focused = stop && cx.read_interaction(node, InteractFact::Focused);
    let swept = match (component.sweep_value(), sweep) {
        (Some(_), Some(cell)) => sweep::take(cx, cell, node),
        _ => None,
    };
    let mut core = core_for(&begun.env, &resolved, &mut kept.memory, response, begun.step1.command.clone());
    place_core(&mut core, &component, style, rect, &resolved, begun.env.direction.is_rtl());
    core.inbox = inbox;
    core.input = input;
    core.swept = swept;
    core.parted = true;
    core.faded = facts.faded && !resolved.enabled;
    core.focus = super::focus::facts(&kept.focus, focused, cx.interaction(node).focus_visible);
    core.intents = super::intents::of_queue(&core.input, || rtl);
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
    let sense = own_sense(&component, &resolved);
    let mut parts = steps::PartsStep::default();
    if facts.parts {
        let input = MeasureIn { env: &begun.env, proposal: Proposal::Ideal, room: Some(place.room.x), services: None };
        steps::declare_parts(&mut component, &mut cxx, rect, &mut parts, &input);
    }
    kept.parts = steps::kept_parts(&parts, rect.min);
    kept.sense = sense;
    kept.sweep = sweep;
    kept.toggle = component.sweep_value();
    kept.cursor = component.cursor().map(Into::into);
    let results = part_results(&kept.parts, &parts, sense, &cxx.core.input, rect, interact.hovered);
    cxx.core.results = results.1;
    cxx.core.press = results.0.press;
    cxx.core.toggles = steps::toggles(&parts);
    cxx.core.value_parts = steps::declared_with(&parts, |extras| extras.value);
    cxx.core.state_layer_parts = steps::declared_with(&parts, |extras| extras.state_layer);
    cxx.core.own_reset_parts = steps::declared_with(&parts, |extras| extras.own_reset);
    cxx.core.focus_shapes = steps::focus_shapes(&parts);
    cxx.core.active = cxx.core.focus.part.filter(|part| cxx.core.results.contains_key(&(part.key, part.index)));
    cxx.core.activation = activation_of(&resolved, &cxx.core.input, sense);

    // 9 update, then the stops it decides (a member that joins its body's roving group).
    steps::gesture_gate(&component, &mut cxx);
    steps::update_inputs(cxx.core);
    steps::edit_intents(&mut component, &mut cxx);
    steps::scrub_by_label(&component, &mut cxx);
    component.update(&mut cxx);
    steps::record_context(cxx.core);
    if let Some((cursor, _)) = cxx.core.cursor {
        kept.cursor = Some(cursor.into());
    }
    let ring = stop.then(|| craft_ui_rt::Stroke::new(tokens.stroke.focus, tokens.color.line.focus));
    kept.focus.ring = ring;
    kept.focus.stop = stop.then(|| component.keys());
    kept.focus.scope = component.focus_scope();
    kept.focus.roving = component.roving();
    kept.focus.holds_escape = cxx.core.holds_escape.get();
    kept.ime.caret = cxx.core.ime_caret.get().map(|caret| caret.translate(Pos2::ZERO - rect.min));

    // 10 overlay: the popups show their content as layer children of this node.
    if facts.overlays {
        let mut overlays = OverlayUi::new(cx, id, rect, &mut kept.popups, true, tokens.clone(), sweep);
        component.overlay(&mut overlays, &mut cxx);
    }

    // 11 apply: the run's edits reach the binding, and the shown value follows.
    let wrote = steps::apply_values(&mut component, &mut cxx).wrote();
    steps::register_reset(&mut component, &cxx);

    // 12 state and 13 paint, once per pointer look the paint read (section 8, 15.6).
    let open = facts.overlays && kept.popups.any_open();
    let (variants, values) = record_variants(&mut component, &mut cxx, cx, begun, sense, &kept.parts, open);
    kept.variants = variants;
    let _ = values;

    // 15 settle: edits a paint adopted reach the binding; a reveal goes to the scroll owner.
    let settled = steps::settle(&mut component, &mut cxx);
    let wrote = wrote || settled.wrote();
    let reveal = cxx.core.reveal.take();

    // 16 report: the access node, once per run and only while the window's access tree is on, then
    // the tooltip and the kept retained facts.
    kept.close_guard = component.close_guard();
    kept.drag = component.drag_source();
    kept.drop = component.drop_target();
    let built = cx.access_enabled().then(|| report::<C>(&component, &cxx, &resolved, sense)).flatten();
    let (access, subs) = built.map_or((None, Vec::new()), |b| (Some(b.node), b.subs));
    kept.access = access;
    kept.access_subs = subs;
    tip::show(&mut kept.tip, tip_text(&resolved), true, cx, id, rect, tokens);
    let _ = tip;
    if let Some((target, spec)) = cxx.core.level.take() {
        super::motion::Level::aim(kept.level.get_mut(), target, spec);
    }
    response::end_reads(reads, cx);

    // 17 finish: the activation fires the bound command once, at event time on the board's handler.
    let activated = component.activated(&cxx) && resolved.enabled;
    let injected = cxx.core.injected.take().filter(|_| resolved.enabled);
    let changed = steps::value_changed(&cxx.events);
    steps::before_finish(cxx.core, activated || injected.is_some(), injected, changed, board.as_ref(), cx);
    let wants = std::mem::take(&mut cxx.core.inbox.wants);
    kept.id = id;
    kept.key = if wrote { component_key(&component) } else { key };
    kept.place = place;
    kept.size = size;
    // The size cache holds only this run's size.
    kept.sizes.clear();
    kept.sizes.push((place.room, size));
    let out = component.finish(cxx);
    if let Some(rect) = reveal {
        // Item 6: the reveal bubbles to the nearest scroll owner, which glides to it.
        super::scroll::reveal(cx, rect);
    }
    if let Some(board) = board {
        register_inboxes(wants, id, &mut kept.memory, &board, cx.store_mut());
    }
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

/// The tooltip text of `Common::tooltip`: its disabled text while the component is disabled.
fn tip_text(resolved: &Resolved<'_>) -> Option<String> {
    let tip = resolved.tooltip?;
    let label = if resolved.enabled { &tip.text } else { tip.disabled_text.as_ref().unwrap_or(&tip.text) };
    Some(label.as_str().to_owned()).filter(|text| !text.is_empty())
}

/// Step 16's access node: the label, description, value words and enabled and selected facts from
/// the resolved props, then the component's own `access`. Built once per run.
fn report<C: Component>(component: &C, cx: &Cx<'_, C>, resolved: &Resolved<'_>, sense: Sense) -> Option<super::access::Built> {
    let Role::Node(role) = C::ROLE else { return None };
    #[cfg(feature = "testing")]
    crate::runtime::measure::count_access_build_rt();
    let mut access = Access::default();
    if component.shows_label() && !resolved.label.is_empty() {
        access.label(resolved.label);
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
    access.disabled(!resolved.enabled);
    if let Some(selected) = resolved.selected {
        access.selected(selected);
    }
    component.access(&mut access, cx);
    access.finish(role, sense)
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
    sweep: Option<craft_ui_rt::State<sweep::Sweep>>,
) {
    let id = kept.id;
    if C::OVERLAYS {
        let node = cx.node();
        let response = Response::new(id, rect, Some(node), cx.interaction(node), kept.dragging, InputFacts::default());
        let resolved = begun.resolved(id);
        let mut core = settled_core(&begun.env, &resolved, &mut kept.memory, response, &begun.step1);
        place_core(&mut core, component, &begun.style, rect, &resolved, begun.env.direction.is_rtl());
        let mut cxx = Cx { core: &mut core, style: &*begun.style, shown: None, mixed: false, events: Vec::new(), applied: 0, validity: Validity::Ok };
        let mut overlays = OverlayUi::new(cx, id, rect, &mut kept.popups, false, tokens.clone(), sweep);
        component.overlay(&mut overlays, &mut cxx);
        if overlays.changed {
            cx.rebuild();
        }
    }
    tip::show(&mut kept.tip, None, false, cx, id, rect, tokens);
}

/// Steps 12 and 13 per pointer look: `Rest` always; `Hover` when the paint read `Hovered`; `Press`
/// when it read `Pressed` and the node takes presses. During a look's recording the pointer facts
/// answer that look's state. A paint that showed children records one look, with the current
/// pointer state, and its pointer reads become build reads (15.6 rule 5).
fn record_variants<C: Component>(
    component: &mut C,
    cxx: &mut Cx<'_, C>,
    build: &mut BuildCx<'_>,
    begun: &Begun<C>,
    sense: Sense,
    parts: &[KeptPart],
    open: bool,
) -> (SmallVec<[Variant; 1]>, AppValues) {
    let node = build.node();
    let now = build.interaction(node);
    let takes_press = sense.takes_press() || parts.iter().any(|p| p.sense.takes_press());
    let scale = build.scale();
    let (text, images) = build.paint_services();
    let services = FrameServices::new(text, images, scale);
    let mut values = AppValues::default();
    let mut paint_as = |look: Pointer, cxx: &mut Cx<'_, C>, values: &mut AppValues| {
        let (hovered, pressed) = match look {
            Pointer::Rest => (false, false),
            Pointer::Hover => (true, false),
            Pointer::Press => (true, true),
        };
        let ((variant, children), read) = crate::style::pointer_reads(|| {
            steps::state(component, cxx, open, hovered, pressed, begun.env.ink);
            let canvas = Canvas::new(&services, cxx.core.rect.min, Rect::EVERYTHING);
            let fade = steps::fade(&canvas, &begun.env, cxx.core);
            if let Some(items) = steps::surface_items(&begun.env, cxx.core, &*begun.style, scale) {
                canvas.extend(items);
            }
            steps::paint_part_layers(&canvas, &begun.env, cxx.core);
            component.paint(&mut paint_ui(&canvas, &begun.env, cxx.core, &*begun.style, look), cxx);
            if look == Pointer::Rest {
                component.content_env(cxx, values);
            }
            component.paint_over(&mut paint_ui(&canvas, &begun.env, cxx.core, &*begun.style, look), cxx);
            steps::unfade(&canvas, cxx.core, fade);
            let children = cxx.core.paint.borrow().children_shown;
            let (content, overlay) = canvas.finish();
            (Variant { look, content, overlay }, children)
        });
        (variant, children, read)
    };
    let mut out = SmallVec::new();
    let (rest, children, read) = paint_as(Pointer::Rest, cxx, &mut values);
    if children {
        // Children are reconciled once, in the first recording; the node keeps one look, recorded in
        // the current pointer state, and reruns when a pointer fact its paint read flips.
        let look = if now.pressed && read.press {
            Pointer::Press
        } else if now.hovered && read.hover {
            Pointer::Hover
        } else {
            Pointer::Rest
        };
        let shown = if look == Pointer::Rest { rest } else { paint_as(look, cxx, &mut values).0 };
        out.push(Variant { look: Pointer::Rest, ..shown });
    } else {
        out.push(rest);
        if read.hover {
            out.push(paint_as(Pointer::Hover, cxx, &mut values).0);
        }
        if read.press && takes_press {
            out.push(paint_as(Pointer::Press, cxx, &mut values).0);
        }
    }
    drop(paint_as);
    drop(services);
    if children {
        if read.hover {
            build.read_interaction(node, InteractFact::Hovered);
        }
        if read.press {
            build.read_interaction(node, InteractFact::Pressed);
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
                    Gesture::Down { .. } => {}
                }
            }
            NodeInput::Access(a) => facts.clicked |= a.action == accesskit::Action::Click && a.sub.is_none(),
            NodeInput::Intent { intent: Intent::Activate, .. } => facts.clicked = true,
            NodeInput::Focus(gained) => {
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
) -> (PartResult, crate::contract::cx::PartResults) {
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
    board.run(fired, cx.store_mut())
}

/// The source a component's own activation fires its bound command with.
pub(crate) fn activation_source(activation: Option<Activation>) -> FireSource {
    match activation {
        Some(Activation { by_key: true, .. }) => FireSource::Key,
        _ => FireSource::Component,
    }
}
