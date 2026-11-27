//! The pipeline's shared pieces: where a component is placed, what the pipeline reads of a type's
//! constants, the run's `CxCore` and `PaintUi`, and the face run. The driver that runs the steps in
//! order lives in `contract/rt/run.rs` (docs/contract-rt.md section 15).

use super::cx::RunIo;
use super::steps;
use super::{Component, CxCore, Internal, PaintUi, Resolved, Show, Validity};
use crate::contract::Id;
use crate::contract::rt::Pointer;
use crate::primitives::{Canvas, FaceRecord, PaintRecord, PartResult};
use crate::runtime::access::{Access, Role};
use crate::runtime::commands::ResolvedCommand;
use crate::runtime::env::Env;
use crate::runtime::memory::MemoryStage;
use crate::style::{Ink, StateSet, StyleView};
use craft_behavior::{Flow, Input, PartRef, PressState, Proposal};
use craft_ui_rt::{Rect, Sense, Vec2};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

pub(crate) use crate::contract::rt::container::{measure_container, show_container, show_container_taking};
pub(crate) use crate::contract::rt::show_leaf;

/// What a shape adds to a leaf's run at the driver's steps (contract-rt 15.4, 15.5): a collection's
/// rows. Each hook defaults to the leaf's behaviour; `content` runs whenever the build reaches the
/// node (its own hooks kept or not), as a container's body does.
pub(crate) trait ShapeHooks<C: Component> {
    /// The shape adds steps: its step 5 and `content` run whenever the build reaches it. Only the
    /// leaf's `NoShape` says no.
    fn shaped(&self) -> bool {
        true
    }

    /// Step 8: the sense of the node's own shape, from the component's.
    fn sense(&self, _comp: &C, own: Sense) -> Sense {
        own
    }

    /// The key role the focused node's key stage reads, where the component's `KeyUse` does not
    /// say it all.
    fn key_role(&self, _comp: &C) -> Option<craft_behavior::KeyRole> {
        None
    }

    /// Step 4: the shape's part of the value step (selection snapshot, scroll offset).
    fn read(&mut self, _comp: &mut C, _stage: &mut MemoryStage, _id: Id, _env: &Env) {}

    /// Step 5: what the shape's content adds to the component's measured content.
    fn pass_extent(&mut self, _comp: &mut C, _at: &Measuring<'_, '_>, _style: &C::Style, _shown: Option<Arc<crate::contract::ValueOf<C>>>) -> Vec2 {
        Vec2::ZERO
    }

    /// Step 5's size after the style's padding and floors: a collection that fills takes the room
    /// its parent gives instead of its rows' width.
    fn sized(&self, _comp: &C, size: Vec2) -> Vec2 {
        size
    }

    /// The node's first line from its top, for row alignment, at its measured height: a collection
    /// whose rows run along X is one line. `None` defers to the component.
    fn first_line(&self, _comp: &C, _height: f32) -> Option<f32> {
        None
    }

    /// What the shape lays out before `parts`, from the rect and the content rect: a collection's rows
    /// for `PartsCx::rect_of`.
    fn geometry(
        &mut self,
        _comp: &C,
        _cx: &crate::contract::MeasureCx<'_, C>,
        _rects: (Rect, Rect),
        _rtl: bool,
    ) -> Option<std::rc::Rc<dyn std::any::Any>> {
        None
    }

    /// Step 7, after the component's own `parts`.
    fn parts(&mut self, _comp: &mut C, _rect: Rect, _pcx: &mut crate::contract::PartsCx<'_, C>) {}

    /// Step 8, after the sense step.
    fn sensed(&mut self, _comp: &mut C, _cx: &mut super::Cx<'_, C>) {}

    /// Step 9, after the component's `update`.
    fn updated(&mut self, _comp: &mut C, _cx: &mut super::Cx<'_, C>) {}

    /// Step 11, after the value step's apply.
    fn apply(&mut self, _comp: &mut C, _cx: &mut super::Cx<'_, C>) {}

    /// Step 13, into the node's paint before the component's own paint: what lies under the shape's
    /// content (a collection's surface).
    fn paint(&self, _comp: &C, _ui: &mut PaintUi<'_>, _rect: Rect) {}

    /// After `content`: the member rows' looks this run recorded, for the node's `Kept.members`.
    fn take_member_looks(&mut self) -> Vec<crate::contract::rt::items::MemberLooks> {
        Vec::new()
    }

    /// Step 13, after the paint: the shape's children, on a `Ui` over the node's build. On a kept
    /// node it gets the settled `Cx`.
    fn content(&mut self, _comp: &mut C, _ui: &mut super::Ui, _env: &Env, _cx: &mut super::Cx<'_, C>, _values: &crate::runtime::env::AppValues) {}

    /// Step 15, after the settle step.
    fn settle(&mut self, _comp: &mut C, _cx: &mut super::Cx<'_, C>) {}
}

/// A leaf adds nothing to its run.
pub(crate) struct NoShape;

impl<C: Component> ShapeHooks<C> for NoShape {
    fn shaped(&self) -> bool {
        false
    }
}

/// What a shape's step 5 reads: the run's build (children measure through it), the room, the env
/// the parent places the node in and its own, its style context and resolved props.
pub(crate) struct Measuring<'a, 's> {
    /// The run's `Ui`, lent per call.
    pub ui: &'a RefCell<&'s mut (dyn super::paint_ui::UiHost + 's)>,
    /// Where the node is placed.
    pub at: ShowAt,
    /// The node's room.
    pub room: Vec2,
    /// The env the parent installed, under which the node's children are placed.
    pub place: &'a Env,
    pub env: &'a Env,
    pub scx: &'a crate::style::StyleCx<'s>,
    pub resolved: &'a Resolved<'s>,
    /// The node's memory stage (the kept node's in a measuring run, else a fresh one): motion reads
    /// such as `StyleCx::exiting` go through it (`StyleCx::owned_by`).
    pub memory: &'a MemoryStage,
}

impl Measuring<'_, '_> {
    /// Runs `f` on the run's `Ui`.
    pub(crate) fn with_ui<R>(&self, f: impl FnOnce(&mut super::Ui<'_, '_>) -> R) -> R {
        super::paint_ui::host_ui(&mut **self.ui.borrow_mut(), f)
    }
}

/// Shows a collection (15.5): its node runs the leaf steps with the item step's hooks, and its rows
/// reconcile in `content` whenever the build reaches it. The item step reads the collection's view of
/// `source` (`Collection::filtered`) itself, so the view keeps the row whose rename runs.
pub(crate) fn show_items<'r, C: crate::contract::Collection>(
    slot: &mut Option<C>,
    ui: &mut super::Ui,
    source: &dyn craft_behavior::ItemSource<C::Key>,
    item: &mut super::ItemFn<'r, C>,
) -> C::Output {
    let component = slot.take().expect("a collection is shown once");
    let mut hooks = crate::contract::items::ItemsHooks::<C>::new(source, item);
    let place = Env::of(ui);
    crate::contract::rt::run::show_shaped(component, ui, (ShowAt::Flow, place), None, &mut hooks)
}

/// Where a component is placed.
#[derive(Clone, Copy)]
pub(crate) enum ShowAt {
    Flow,
    Rect(Rect),
}

/// The flags a host owns for its faces.
pub(crate) const HOST_FLAGS: crate::style::Flags = crate::style::Flags::INVALID.union(crate::style::Flags::WARN);

/// A run's `CxCore` before step 7: the resolved props, the node's memory, the response and the bound
/// command; geometry, input and focus are filled by the driver.
pub(crate) fn core_for<'a>(
    env: &'a Env,
    resolved: &'a Resolved<'a>,
    memory: &'a mut MemoryStage,
    popups: &'a mut crate::contract::rt::overlay::PopupStates,
    response: crate::contract::Response,
    command: Option<Arc<ResolvedCommand>>,
    run: RunIo<'a>,
) -> CxCore<'a> {
    let rect = Rect::NOTHING;
    CxCore {
        env: env.view(),
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
        parted: false,
        ink: env.ink,
        intents: Default::default(),
        memory,
        paint: RefCell::new(PaintRecord::default()),
        results: HashMap::new(),
        face_mode: false,
        active: None,
        focus: Default::default(),
        lent: Default::default(),
        activation: None,
        injected: None,
        keep_open: false,
        part_clicked: false,
        collection_selection: None,
        activated_member: None,
        press: PressState::IDLE,
        cursor: None,
        part_cursors: Vec::new(),
        claims: Vec::new(),
        reveal: None,
        member_tip: None,
        header_stops: Vec::new(),
        roving_member: false,
        press_cancel: None,
        scroll_moves: Vec::new(),
        value_parts: Default::default(),
        own_reset_parts: Default::default(),
        state_layer_parts: Default::default(),
        toggles: Vec::new(),
        reset_part: None,
        roving_arrival: None,
        roving_lead: None,
        collection_events: None,
        visible_rows: 0..0,
        header: Default::default(),
        column_requests: Vec::new(),
        gestures: Default::default(),
        texts: None,
        selections: Vec::new(),
        faded: false,
        node: false,
        fade: None,
        focus_shapes: Vec::new(),
        focus_shape: None,
        editor: None,
        editor_inputs: Vec::new(),
        text_invalid: false,
        text_unparsed: None,
        text_takes_pointer: false,
        grid: crate::runtime::access::GridPlace::default(),
        escape_to_list: false,
        text_access: RefCell::new(None),
        value_text: RefCell::new(None),
        submitted: false,
        escaped: false,
        row: None,
        popups,
        run,
    }
}

/// The `PaintUi` of one recorded look: content paint into `canvas` (in window coordinates, recorded
/// in the node's), the state step's ink and flags, the parts' results and the look's pointer state.
pub(crate) fn paint_ui<'a>(
    host: &'a mut (dyn crate::contract::paint_ui::UiHost + 'a),
    canvas: &'a Canvas<'a>,
    env: &'a Env,
    core: &'a CxCore<'_>,
    style: &'a dyn StyleView,
    pointer: Pointer,
) -> PaintUi<'a> {
    let focus = core.active.map(|part| (part, core.state.flags.contains(crate::style::Flags::FOCUS_VISIBLE)));
    PaintUi {
        env,
        ink: core.ink,
        data: crate::primitives::DataMode::of(&core.state),
        id: core.id,
        rect: core.rect,
        record: &core.paint,
        results: &core.results,
        active: focus.filter(|_| core.state.flags.contains(crate::style::Flags::FOCUSED)),
        // Only a composite with parts paints faces, so the others skip the look list.
        text: if core.results.is_empty() { None } else { style.texts().first().map(|text| &text.color) },
        muted: if core.results.is_empty() { None } else { style.texts().get(1).map(|text| &text.color) },
        flags: core.state.flags & HOST_FLAGS,
        enabled: core.resolved.enabled,
        size: core.resolved.size,
        selections: &core.selections,
        fade: core.fade,
        node: core.node,
        canvas,
        host_canvas: None,
        pointer,
        showed_child: std::cell::Cell::new(false),
        host,
    }
}

/// What the pipeline reads of a component type's constants.
#[derive(Clone, Copy)]
pub(crate) struct TypeFacts {
    pub name: &'static str,
    pub role: Role,
    pub alt_roles: &'static [Role],
    pub input: Input,
    pub overlays: bool,
    pub parts: bool,
    pub stops: bool,
    pub labelled_after: bool,
    pub binds_view: bool,
    /// `DISABLED` is `Faded`.
    pub faded: bool,
    /// `RULE`.
    pub rule: bool,
}

impl TypeFacts {
    pub(crate) fn of<C: Component>() -> Self {
        TypeFacts {
            name: std::any::type_name::<C>(),
            role: C::ROLE,
            alt_roles: C::ALT_ROLES,
            input: C::INPUT,
            overlays: C::OVERLAYS,
            parts: C::PARTS,
            stops: C::STOPS,
            labelled_after: C::LABELLED_AFTER,
            binds_view: C::BINDS_VIEW,
            faded: C::DISABLED == super::DisabledLook::Faded,
            rule: C::RULE,
        }
    }

    /// Whether the component's input is a text part or an editor.
    pub(crate) fn text(&self) -> bool {
        matches!(self.input, Input::Text | Input::Editor)
    }
}

/// Measures `comp` as `show` would in `ui`'s room, without a node: steps 1, 2, 4 (read only) and 5
/// (`Show::measure_in`, a composite's `MeasureCx::child` and `face`). `id` is the id the show would
/// give it (`None`: one under `ui`'s scope); `under` is the composite's resolved `enabled`, `size`
/// and inside label a child or face of it shows under. A child a measuring flow's node keeps with the
/// same component key answers from its size cache for the room, so a composite measured again with
/// unchanged inputs runs none of its children's `measure` (owner rules: a body reuses its last
/// measurement while its inputs are unchanged).
pub(crate) fn measure_under<C: Component>(
    comp: &mut C,
    ui: &mut crate::contract::Ui<'_, '_>,
    id: Option<Id>,
    proposal: Proposal,
    under: Option<&crate::contract::cx::Under>,
) -> Vec2 {
    let env = Env::of(ui);
    let env = match under {
        Some(under) => crate::contract::cx::child_env(&env, under),
        None => env,
    };
    let common = std::mem::take(comp.common_mut(Internal::new()));
    let room = ui.flow.room();
    if proposal == Proposal::Ideal
        && let (Some(id), Some(Some(node))) = (id, ui.flow.measuring)
        && let Some(kept) = ui.cx.peek::<crate::contract::rt::node::ComponentNode>(node, id.value())
        && crate::contract::rt::run::holds(kept.local, crate::contract::rt::key::component_key(comp), kept.state)
        && let Some(size) = kept.local.cached(room)
    {
        *comp.common_mut(Internal::new()) = common;
        return size;
    }
    let step1 = steps::env(&env, &common, steps::variants::<C>(common.variant, &env), &mut |command| ui.resolve_command(command.clone()));
    let ctx = ui.ctx.clone();
    let style = steps::style::<C>(ui.build_cx(), &ctx, &steps::style_cx(&env, &step1), comp.style_key());
    let id = id.unwrap_or_else(|| ui.id().with("measure"));
    let resolved = steps::resolved(id, &step1, &common);
    let mut stage = MemoryStage::open_peek(ui.ctx(), id);
    let value = steps::value(comp, &mut stage, id, step1.command.as_deref());
    ui.build_cx().count_measure();
    let size = {
        let host: RefCell<&mut dyn super::paint_ui::UiHost> = RefCell::new(&mut *ui);
        let input = steps::MeasureIn { env: &env, proposal, room: Some(room.x), host: &host };
        let content = comp.measure(&steps::measure_cx::<C>(&input, &style, &resolved, value.value.as_deref()));
        steps::measure(&*style, &input, content, Vec2::ZERO, &resolved)
    };
    *comp.common_mut(Internal::new()) = common;
    size
}

/// What a face reads of the composite that paints it.
pub(crate) struct FaceSource<'a> {
    pub id: Id,
    pub result: &'a PartResult,
    /// `Some(keyboard visible)` when the part is the active part of the focused composite.
    pub focus: Option<bool>,
    /// The text colour of the host part, which a face reads through `StyleCx::host_ink`.
    pub ink: Option<&'a crate::style::Stateful<crate::style::StyleColor>>,
    /// The muted text colour of the host part, which a face reads through `StyleCx::host_muted_ink`.
    pub muted: Option<&'a crate::style::Stateful<crate::style::StyleColor>>,
    /// The flags the host owns for its faces: whether its value is invalid or warns.
    pub flags: crate::style::Flags,
    /// Whether the host is enabled: a face of a disabled host paints with the Disabled state.
    pub enabled: bool,
    /// The host is faded (`DisabledLook::Faded`): its faces paint their enabled look inside its fade.
    pub faded: bool,
    /// The host's step 16 fills the part's sub-node from the face's record.
    pub reported: bool,
}

/// The craft `Id` of the face of `part` in composite `owner`.
fn face_id(owner: Id, part: PartRef) -> Id {
    owner.with(("face", part.key, part.index))
}

/// A face run: steps 1, 2, 4 (read only), 12 and 13 of the face component, inside the composite's
/// paint, recording into the host's canvas at the call position. It has no node, senses nothing,
/// keeps no memory and fires nothing; the pointer facts it reads are the host part's, through the
/// part's result, so they count as the host paint's reads. Returns the face's first text baseline
/// and, for a reported face, the record step 16 fills the part's sub-node from.
pub(crate) fn paint_face<C: Show>(
    ui: &mut (dyn super::paint_ui::UiHost + '_),
    env: &Env,
    host: &Canvas<'_>,
    source: &FaceSource<'_>,
    part: PartRef,
    mut face: C,
) -> (Option<f32>, Option<FaceRecord>) {
    let scale = host.pixels_per_point();
    let context = super::paint_ui::host_ui(ui, |ui| ui.ctx().clone());
    let common = std::mem::take(face.common_mut(Internal::new()));
    debug_assert!(common.tooltip.is_none(), "a face instance carries no tooltip; use `PartBuilder::tooltip`");
    let step1 = steps::env(env, &common, steps::variants::<C>(common.variant, env), &mut |_| None);
    let scx = steps::style_cx(env, &step1);
    let scx = source.ink.map_or(scx, |ink| scx.hosted(ink, source.muted));
    let style = steps::style_cached::<C>(&scx, face.style_key());
    let id = face_id(source.id, part);
    let resolved = steps::resolved(id, &step1, &common);
    let mut stage = MemoryStage::open(&context, id);
    stage.face = true;
    let value = steps::value(&mut face, &mut stage, id, step1.command.as_deref());
    let rect = source.result.rect;
    let part_state = steps::part_state(source.result, source.faded || (step1.enabled && source.enabled), source.focus);
    let response = crate::contract::Response::new(id, source.result.rect, None, Default::default(), false, Default::default());
    let mut popups = Default::default();
    let mut core = core_for(env, &resolved, &mut stage, &mut popups, response, step1.command.clone(), RunIo::settled(context));
    core.face_mode = true;
    core.faded = source.faded;
    let flow = Flow::new(rect, env.direction.is_rtl());
    core.rect = rect;
    core.content_rect = flow.inset(face.padding(rect, &style));
    core.flow = flow;
    core.visible = rect;
    let cx = super::Cx::<C> {
        core: &mut core,
        style: &style,
        shown: value.value,
        mixed: value.mixed,
        events: Vec::new(),
        applied: 0,
        validity: Validity::Ok,
    };
    // A face shows its own inputs as the state step reads them for a component.
    let flags = part_state.flags | source.flags | steps::value_flags(&face, &cx);
    let on = f32::from(u8::from(flags.intersects(crate::style::Flags::ON)));
    let state = StateSet::new(part_state.state, flags, crate::style::StateMix::new(part_state.t.hover, part_state.t.press, on));
    cx.core.state = state;
    cx.core.stated = true;
    cx.core.ink = steps::surface_ink(StyleView::surface(&*style), &state, env.ink);
    if let Some(items) = steps::surface_items(env, cx.core, &*style, scale) {
        host.extend(items);
    }
    let no_parts = HashMap::new();
    {
        let mut pui = paint_ui(ui, host, env, cx.core, &*style, Pointer::Rest);
        // A face of an inactive part paints its data as `draw_part` does for the host.
        pui.data = if source.result.inactive { crate::primitives::DataMode::Inactive } else { crate::primitives::DataMode::of(&state) };
        pui.results = &no_parts;
        pui.active = None;
        pui.text = None;
        pui.muted = None;
        pui.flags = crate::style::Flags::empty();
        pui.enabled = step1.enabled && source.enabled;
        pui.selections = &[];
        pui.fade = None;
        pui.node = false;
        face.paint(&mut pui, &cx);
    }
    let baseline = cx.core.paint.borrow().baseline;
    if !source.reported {
        return (baseline, None);
    }
    let name = cx.core.paint.borrow().elided.clone().unwrap_or_else(|| step1.label.as_str().to_owned());
    let mut access = Access::default();
    face.access(&mut access, &cx);
    let record = FaceRecord {
        part,
        name,
        flags,
        role: C::ROLE,
        numeric: access.numeric,
        expanded: access.expanded,
        enabled: step1.enabled,
        baseline,
        value: access.value,
        color: access.color,
    };
    (baseline, Some(record))
}
