//! Running the label step: for a container's row or a leaf child, with its label scope.

use super::leaf::{LabelLeaf, colon};
use super::place::{Placed, decide, hidden_control, label_gap};
use super::props::{FieldLayout, LabelKind, SideLabel};
use crate::contract::Props;
use crate::contract::Show;
use crate::contract::Ui;
use crate::contract::cx::Under;
use crate::primitives::Id;
use crate::runtime::access::scope::{FieldLabel, ResetFact, ScopeFacts, scope_facts};
use crate::runtime::env::{Env, Label};
use crate::runtime::focus::FocusTarget;
use crate::runtime::memory::sub;
use craft_behavior::Flow;
use craft_ui_rt::{Rect, State, Vec2};
use std::sync::Arc;

/// What the step needs of the row it runs for.
pub(crate) struct LabelIn<'a> {
    /// The scope the controls register with; the container's own id, or a derived id for `labelled`.
    pub scope: Id,
    /// The component whose eviction takes the label scope's facts.
    pub owner: Id,
    /// The node the label's node sits under in AccessKit: the composite's own id, when it has a node.
    pub parent: Option<Id>,
    pub row: Rect,
    /// The width the mount offers the row. A content-sized row is as wide as its content took, so the choice between
    /// beside and above reads the room, not the row.
    pub room: f32,
    /// The row sits in a column that takes its width from its content: a beside label never stacks for want of room,
    /// since the column is as wide as the label, the gap and the control need.
    pub content_row: bool,
    /// The row is a cell of an `egui::Grid` (`LeafLabel::egui_cell`): the row is as wide as its label and control need,
    /// so the label neither wraps nor stacks for want of room, and the row is that wide from its left edge in any
    /// direction.
    pub egui_cell: bool,
    pub text: &'a Label,
    pub side: &'a SideLabel,
    /// The help line the container's footer shows; it joins the control's description with the label's detail.
    pub help: Option<&'a Label>,
    pub enabled: bool,
    pub size: craft_theme::ControlSize,
    /// The control the label names: a click on the label moves focus to it.
    pub control: Id,
    /// The narrowest the control can be; a row too narrow for the label column, the gap and this stacks the label above.
    /// In an egui grid's cell (`egui_cell`), the width the control takes there: a leaf's ideal width.
    pub control_min: f32,
    /// The control is a leaf child shown in the flow with its label after it, not a field's body.
    pub leaf: bool,
    /// The row is a row of a vertical flow, not a member of a row's line (`in_column`): under a form's label column, a
    /// control with its label after it, a leaf's or a field's, starts in the control column.
    pub in_column: bool,
    /// The leaf child draws only its mark (`Component::LABELLED_AFTER`): the label is part of its hit target.
    pub claims: bool,
    /// The container shows its control's validity message as its status line (`FooterRegion::status`).
    pub status: bool,
    /// The size of the content shown after the label's text, inside the label; zero without one.
    pub trailing: Vec2,
}

/// The label fact the scope records: the name's text, and the detail and the help line (resolved) as the control's
/// description.
fn field_label(env: &Env, input: &LabelIn<'_>, id: Id, rect: Rect) -> FieldLabel {
    let mut label = FieldLabel::new(id, rect, input.text.as_str()).role(input.side.role).status(input.status);
    label.control = input.control;
    let described: Vec<String> =
        [input.side.detail.as_ref(), input.help].into_iter().flatten().map(|text| env.view().resolve(text).as_str().to_owned()).collect();
    if described.is_empty() { label } else { label.detail(described.join(". ")) }
}

/// Draws trailing content in the rect the label step gives it.
pub(crate) type ShowTrailing<'a> = Box<dyn FnOnce(&mut Ui, Rect) + 'a>;

/// Content the leaf label step shows after the label's text, inside the label (`BodyCx::labelled_with`): its size
/// and how to show it in the rect the step gives it.
pub(crate) struct Trailing<'a> {
    pub size: Vec2,
    pub show: ShowTrailing<'a>,
}

/// What the label step needs of one leaf child it labels (`PaintUi::labelled`, `BodyCx::labelled`).
pub(crate) struct LeafLabel<'a> {
    /// The child's id; the label scope derives from it.
    pub id: Id,
    /// The component whose eviction takes the label scope's facts.
    pub owner: Id,
    /// The node the label's node sits under in AccessKit, when the caller has one.
    pub parent: Option<Id>,
    /// The row the label and the child share.
    pub row: Rect,
    /// The row is a cell of an `egui::Grid` (`Scoped::egui_cell`): egui places it from the cell's left edge in any
    /// direction, and gives the column the width it takes from the next frame. `row` then gives only the cell's left
    /// edge, top and height.
    pub egui_cell: bool,
    pub text: &'a Label,
    pub side: &'a SideLabel,
    /// What the child inherits (`leaf_under`).
    pub under: &'a Under,
}

/// What the label step hands its leaf child in every measure and run: the enabled state `under` gives with the
/// child's own (a disabled field's label does not scrub), the size, and the inside label of `side`.
pub(crate) fn leaf_under<C: Show>(env: &Env, side: &SideLabel, child: &C, (enabled, size): (bool, craft_theme::ControlSize)) -> Under {
    let enabled = enabled && child.common(crate::contract::Internal::new()).enabled.unwrap_or(true);
    Under { enabled, size, inside: super::props::inside(env.view(), side, enabled) }
}

/// A leaf child's label step that has run: the rect left for the child and the label scope it shows under.
pub(crate) struct LeafPlaced {
    pub control: Rect,
    /// The label's rect; `Rect::NOTHING` for a hidden label.
    pub label: Rect,
    /// The side label the child shows under (the label scope).
    pub scope: FieldLabel,
}

impl LeafPlaced {
    /// `env` with the label scope given to the control: only the control sees it, as with the inside label.
    pub(crate) fn env_for(&self, env: &Env) -> Env {
        under_label(env, self.scope.clone())
    }
}

/// `env` with `label` as the label scope of the control shown under it.
pub(crate) fn under_label(env: &Env, label: FieldLabel) -> Env {
    env.derive_local(|scoped| scoped.field_label = Some(Arc::new(label)))
}

/// The label step for one leaf child, decided: measures the child's narrowest width and decides where the label, with
/// content of `trailing` size inside it, and the child sit in `at.row`. Nothing shows and no room is taken; `show_leaf`
/// then shows the label where the row's place in flow puts it, and the caller shows the child at the control's rect
/// under `LeafPlaced::env_for`.
pub(crate) fn decide_leaf<C: Show>(ui: &mut Ui, env: &Env, at: &LeafLabel<'_>, child: &mut C, trailing: Vec2) -> Decided {
    // Read before the child measures: nothing it does there may count as a child placed in flow.
    let in_column = in_column(ui, env);
    // An egui grid's cell is as wide as the row takes, and the control shows at its ideal width there.
    let proposal = if at.egui_cell { craft_behavior::Proposal::Ideal } else { craft_behavior::Proposal::Min };
    let control_min = crate::contract::run::measure_under(child, ui, Some(at.id), proposal, Some(at.under)).x;
    let facts = LeafFacts { control_min, in_column, claims: C::LABELLED_AFTER };
    decide_step(ui, env, &leaf_input(env, at, at.row, facts, trailing))
}

/// Shows the label of a leaf child's step `decided` for `at`, moved down by `down`, with the `trailing` content inside
/// it, and returns the rect left for the child.
pub(crate) fn show_leaf(ui: &mut Ui, env: &Env, at: &LeafLabel<'_>, decided: Decided, down: f32, trailing: Option<Trailing<'_>>) -> LeafPlaced {
    let size = trailing.as_ref().map_or(Vec2::ZERO, |trailing| trailing.size);
    let input = leaf_input(env, at, at.row.translate(Vec2::new(0.0, down)), decided.facts, size);
    let (placed, scope) = show(ui, env, &input, decided.moved(down), trailing);
    LeafPlaced { control: placed.control, label: placed.label, scope }
}

/// What a label step reads of its control before it decides.
#[derive(Clone, Copy)]
struct LeafFacts {
    control_min: f32,
    in_column: bool,
    claims: bool,
}

/// The step's input for the leaf child of `at` in `row`, with content of `trailing` size inside the label.
fn leaf_input<'a>(env: &Env, at: &LeafLabel<'a>, row: Rect, facts: LeafFacts, trailing: Vec2) -> LabelIn<'a> {
    LabelIn {
        scope: sub(at.id, "field"),
        owner: at.owner,
        parent: at.parent,
        row,
        room: row.width(),
        content_row: env.content.0,
        egui_cell: at.egui_cell,
        text: at.text,
        side: at.side,
        help: None,
        enabled: at.under.enabled,
        size: at.under.size,
        control: at.id,
        control_min: facts.control_min,
        leaf: true,
        in_column: facts.in_column,
        claims: facts.claims,
        status: false,
        trailing,
    }
}

/// The label step for one leaf child in `at.row`, decided and shown where it was decided: returns the rect left for
/// the child. The caller shows the child there, under `LeafPlaced::env_for`.
pub(crate) fn leaf<C: Show>(ui: &mut Ui, env: &Env, at: &LeafLabel<'_>, child: &mut C, trailing: Option<Trailing<'_>>) -> LeafPlaced {
    let decided = decide_leaf(ui, env, at, child, trailing.as_ref().map_or(Vec2::ZERO, |trailing| trailing.size));
    show_leaf(ui, env, at, decided, 0.0, trailing)
}

/// Runs the label step in `ui`: shows the label and the `trailing` content inside it (`input.trailing` is its size),
/// and returns where the label and the control sit and the label scope the control shows under (`under_label`).
pub(crate) fn run(ui: &mut Ui, env: &Env, input: &LabelIn<'_>, trailing: Option<Trailing<'_>>) -> (Placed, FieldLabel) {
    let decided = decide_step(ui, env, input);
    show(ui, env, input, decided, trailing)
}

/// The label step decided for a row, before anything shows: where the label and the control sit, and the label leaf
/// measured under the env it shows in.
pub(crate) struct Decided {
    placed: Placed,
    /// `None` for a label that shows no leaf.
    leaf: Option<DecidedLeaf>,
    /// What the step read of its control.
    facts: LeafFacts,
    /// The label scope's facts cell, which the control writes.
    cell: State<ScopeFacts>,
    /// The child of the control a click on the label focuses, when the control named one.
    target: Option<Id>,
}

struct DecidedLeaf {
    leaf: LabelLeaf,
    /// The env the label leaf measures and shows under.
    env: Env,
    reset: Option<ResetFact>,
}

impl Decided {
    /// The rect left for the control.
    pub(crate) fn control(&self) -> Rect {
        self.placed.control
    }

    /// The label's rect; `Rect::NOTHING` for a hidden label.
    pub(crate) fn label(&self) -> Rect {
        self.placed.label
    }

    /// The step moved down by `down`, as the row's place in flow moves it.
    fn moved(mut self, down: f32) -> Self {
        let by = Vec2::new(0.0, down);
        self.placed.label = self.placed.label.translate(by);
        self.placed.control = self.placed.control.translate(by);
        self.placed.trailing = self.placed.trailing.translate(by);
        self
    }
}

/// Decides the label step for `input`: measures the label under the env it shows in and places it and the control in
/// `input.row`. Nothing shows. What the control last told the scope (its reset, a scrub by the label) is a tracked read
/// of the scope's facts cell, so a change shows the label again.
fn decide_step(ui: &mut Ui, env: &Env, input: &LabelIn<'_>) -> Decided {
    let ctx = ui.ctx().clone();
    let rtl = env.direction.is_rtl();
    let (gap, slot_side) = (label_gap(env), env.tokens.size.handle.sm);
    let (cell, facts_read) = scope_facts(ui.build_cx(), input.scope);
    let reset = facts_read.reset.clone();
    let flow = Flow::new(input.row, rtl);
    let content = input.trailing.x > 0.0;
    let facts = LeafFacts { control_min: input.control_min, in_column: input.in_column, claims: input.claims };
    if input.side.shows_no_leaf(content) {
        let control = hidden_control(&ctx, input, flow, gap);
        return Decided {
            placed: Placed { label: Rect::NOTHING, control, trailing: Rect::NOTHING, caption: 0.0 },
            leaf: None,
            facts,
            cell,
            target: facts_read.target,
        };
    }
    let slot = if reset.is_some() && input.side.reset_dot { slot_side } else { 0.0 };
    let mut leaf = LabelLeaf::new(input.side.kind.shown(input.text).clone(), slot);
    leaf.scrub = input.enabled && (input.side.scrub || facts_read.scrub);
    leaf.resettable = reset.is_some();
    leaf.differs = reset.as_ref().is_some_and(|fact| fact.differs);
    leaf.default = reset.as_ref().and_then(|fact| fact.default.clone());
    leaf.named = !input.side.content_only(content);
    if matches!(input.side.kind, LabelKind::Short(_)) {
        leaf.words = Some(input.text.as_str().to_owned());
    }
    leaf.colon = colon(env, input).filter(|_| leaf.named);
    leaf.after = input.side.layout == FieldLayout::After;
    let linked = input.side.press == super::props::LabelPress::Link;
    leaf.link = linked;
    leaf.claimed = input.claims;
    leaf.trailing = input.trailing;
    if let Some(tooltip) = input.side.tooltip.clone() {
        leaf = leaf.tooltip(tooltip);
    } else if !leaf.named {
        leaf = leaf.tooltip(input.text.clone());
    }
    match &input.side.kind {
        LabelKind::Glyph(glyph) => leaf.glyph = Some(glyph.resolved(env.view())),
        _ => {
            leaf.icon = input.side.icon.clone();
            leaf.detail = input.side.detail.as_ref().map(|detail| env.view().resolve(detail));
        }
    }
    // The label's node is a child of the node that shows it, so its access node sits under that node's.
    let kid_env = crate::contract::cx::child_env(env, &Under::new(input.enabled, input.size));
    let glyph = leaf.glyph.is_some();
    let mut placed = Env::run_in(ui, kid_env.clone(), |kids| decide(&ctx, env, input, kids, &mut leaf, glyph, flow, facts_read.line));
    if input.trailing.x > 0.0 {
        // The label without its reset dot, which keeps its own press; the content sits at its end, on the first line.
        let (_, unslotted) = Flow::new(placed.label, rtl).take_end(slot);
        let cell = unslotted.take_end(input.trailing.x).0;
        let middle = cell.top() + leaf.first_line.get() / 2.0;
        placed.trailing = Rect::from_min_size(craft_ui_rt::pos2(cell.min.x, middle - input.trailing.y / 2.0), input.trailing);
    }
    Decided { placed, leaf: Some(DecidedLeaf { leaf, env: kid_env, reset }), facts, cell, target: facts_read.target }
}

/// Shows the label step `decided` for `input`: the label and the `trailing` content inside it. Returns where the label
/// and the control sit, and the label scope the control shows under: the label's press, hover and activation, the
/// rect it offers a control that claims it, and the facts cell the control writes.
fn show(ui: &mut Ui, env: &Env, input: &LabelIn<'_>, decided: Decided, trailing: Option<Trailing<'_>>) -> (Placed, FieldLabel) {
    let rtl = env.direction.is_rtl();
    let label_id = sub(input.scope, "label");
    let Decided { placed, leaf, cell, target, .. } = decided;
    let Some(DecidedLeaf { leaf, env: kid_env, reset }) = leaf else {
        let mut label = field_label(env, input, label_id, Rect::NOTHING).shown(false);
        label.facts = Some(cell);
        return (placed, label);
    };
    let slot = if reset.is_some() && input.side.reset_dot { env.tokens.size.handle.sm } else { 0.0 };
    // The label without its reset dot, which keeps its own press.
    let (_, unslotted) = Flow::new(placed.label, rtl).take_end(slot);
    // A label shows with the control it names: where a bar's view shows the two only in part, neither shows.
    let cut = cut_by_bar_view(env, placed.label.union(placed.control));
    let shown = Env::run_in(ui, kid_env, |kids| {
        let clip = kids.clip_rect();
        if cut {
            kids.set_clip_rect(Rect::NOTHING);
        }
        let shown = Props::id(leaf, label_id).show_at(kids, placed.label);
        kids.set_clip_rect(clip);
        shown
    });
    // The content belongs to the label, not to the controls it names, so it shows before the control: a control that
    // claims the label lies over it and takes its presses.
    if let Some(trailing) = trailing {
        (trailing.show)(ui, placed.trailing);
    }
    let linked = input.side.press == super::props::LabelPress::Link;
    let activated = shown.activated && !linked;
    // A click on the label moves focus to the control it names; a control that claims the label takes the press
    // itself.
    if activated && !input.claims {
        ui.request_focus(FocusTarget::Id(target.unwrap_or_else(|| control_of(input))));
    }
    let mut label = field_label(env, input, label_id, placed.label)
        .with_press(shown.press)
        .with_hover(shown.hovered)
        .activated(activated)
        .linked(shown.activated && linked);
    // A double click on the name or a click on the reset dot resets the control that registered the reset.
    label.reset = reset
        .as_ref()
        .filter(|_| shown.double_click || shown.dot_click)
        .map(|fact| crate::runtime::access::scope::ask_reset(ui.ctx(), input.scope, fact.control));
    label.claim = input.claims.then(|| unslotted.rect());
    label.facts = Some(cell);
    (placed, label)
}

/// The control a label step names.
fn control_of(input: &LabelIn<'_>) -> Id {
    input.control
}

/// Whether `rect`, in the scrolling body of a bar, lies partly outside the body's view along x: a label and the control
/// it names show whole or not at all there.
fn cut_by_bar_view(env: &Env, rect: Rect) -> bool {
    if !env.values.get::<crate::contract::InBar>().0 {
        return false;
    }
    let _ = rect;
    todo!("the bar body's view rect on rt (scroll view of a bar) and the bar line's member notes, INT-2c")
}

/// Whether a row shown in `flow` is a row of a vertical flow, not a member of a row's line. The first child of an
/// unwrapped row that is itself a row of a vertical flow (`Scoped::column_row`) starts where that flow's rows start,
/// so it counts as one of them: a check box that starts a row of a form lines up with the form's controls. `env` is the
/// env `flow`'s children are placed under.
pub(crate) fn in_column(flow: &Ui, env: &Env) -> bool {
    !is_row(flow) || env.column_row
}

/// The flow of `ui` places its children side by side: a `Row`'s line.
fn is_row(ui: &Ui) -> bool {
    matches!(ui.flow.dir_ref(), crate::contract::rt::ui::Dir::Row { .. })
}

/// Runs the label step for one leaf child in the flow of `ui`: `label` sits beside the child or above it, with the
/// icon and detail of `side`, and the child takes the width the label leaves. The row is as high as the placement
/// measured: the child (plus the label line when the label stacks), and the label when it is taller. `id` is the
/// child's id, `owner` the component whose eviction takes the label scope's facts, and `under` the enabled state and
/// size the child inherits. `trailing` is content shown inside the label, after its text.
pub(crate) fn flow_leaf<C: Show>(
    (ui, env): (&mut Ui, Env),
    (id, owner): (Id, Id),
    under: (bool, craft_theme::ControlSize),
    label: Label,
    side: SideLabel,
    child: C,
    trailing: Option<Trailing<'_>>,
) -> C::Output {
    // In a grid the row is the grid's next cell, read under its column's group, as `run` shows any other cell: the
    // room before the row is the whole grid's, and a row placed there would stand under the one before it.
    let ctx = ui.ctx().clone();
    match crate::layout::grid_cell(ui) {
        Some((grid, column)) => crate::layout::ColumnGroup::cell(&ctx, grid, column, || {
            flow_leaf_row((ui, env), (id, owner), under, label, side, child, (trailing, true))
        }),
        None => flow_leaf_row((ui, env), (id, owner), under, label, side, child, (trailing, false)),
    }
}

/// `flow_leaf` for a row in the flow of `ui`, whose children are placed under `env`; `in_cell`: the row is the next
/// cell of the grid around it.
fn flow_leaf_row<C: Show>(
    (ui, env): (&mut Ui, Env),
    (id, owner): (Id, Id),
    under: (bool, craft_theme::ControlSize),
    label: Label,
    side: SideLabel,
    child: C,
    (trailing, in_cell): (Option<Trailing<'_>>, bool),
) -> C::Output {
    let mut child = child;
    let under = leaf_under(&env, &side, &child, under);
    let ideal = crate::contract::run::measure_under(&mut child, ui, Some(id), craft_behavior::Proposal::Ideal, Some(&under));
    if !in_cell {
        wrap_before(ui, &env, (&label, &side), ideal.x);
    }
    // A grid's cell; a probe offers it unbounded room, and the row then spans the grid's flow so it has an end edge.
    let flow = ui.available_rect_before_wrap();
    let avail = match crate::layout::flow_room(ui) {
        room if in_cell && room.x.is_finite() => Rect::from_min_size(flow.min, room),
        room if in_cell => Rect::from_min_size(flow.min, Vec2::new(flow.width(), room.y)),
        _ => flow,
    };
    // The label step decides where the label and the control sit at the flow's top, and the row then takes its place
    // in flow as any other child does: its control's line is the row's line, on a row's or bracket's line, centred in a
    // grid's cell or a horizontal layout. The row spans the room's own edges: a room that scrolls along x is unbounded
    // at its start edge, the left one under RTL.
    let row = Rect::from_min_max(avail.min, craft_ui_rt::pos2(avail.max.x, avail.min.y + ideal.y));
    // An `egui::Grid` offers a column with no width from the last frame its least cell width, where the label would
    // wrap and keep the column that narrow: in its cell the label is decided at its own width.
    let egui_cell = env.egui_cell;
    let at = LeafLabel { id, owner, parent: None, row, egui_cell, text: &label, side: &side, under: &under };
    let decided = decide_leaf(ui, &env, &at, &mut child, trailing.as_ref().map_or(Vec2::ZERO, |trailing| trailing.size));
    // In a column that sizes from its content the row is the label plus the control, never the room around them.
    // A row of a horizontal flow is as wide as its label and control: the room it has there is unbounded.
    // So is a grid's cell: the room before it is the whole grid's, and a row spanning it measures its column as wide
    // as the grid, so the grid could never set two such cells beside each other (two columns of check boxes). An egui
    // grid's cell is too, also inside a scope of its own there.
    let content = env.content.0 || is_row(ui) || in_cell || egui_cell;
    let control_width = if content { decided.control().width().min(ideal.x) } else { decided.control().width() };
    let rtl = env.direction.is_rtl();
    let span = |cell: Rect| Flow::new(Rect::from_min_size(cell.min, Vec2::new(cell.width(), ideal.y)), rtl).span(0.0, control_width);
    let (decided_control, decided_label) = (span(decided.control()), decided.label());
    // An egui grid's cell takes its extent from its left edge in any direction.
    let extent = if !content {
        avail.width()
    } else if rtl && !egui_cell {
        avail.max.x - decided_control.min.x.min(decided_label.min.x)
    } else {
        decided_control.max.x.max(decided_label.max.x) - avail.min.x
    };
    // The child's own height: a stacked label pushes the control down, and a label taller than the control reaches
    // below it.
    let height = (decided_control.bottom().max(decided_label.bottom()) - row.top()).max(ideal.y);
    let took = crate::layout::place_in_flow(ui, Vec2::new(extent, height));
    let placed = show_leaf(ui, &env, &at, decided, took.top() - row.top(), trailing);
    let control = span(placed.control);
    // The child runs under the inside label its measures saw and under its label scope.
    let place = placed.env_for(&env);
    let place = match under.inside.clone() {
        Some(inside) => place.derive_local(|scoped| scoped.inside_label = Some(inside as crate::runtime::env::Held)),
        None => place,
    };
    // The control and its label are one member of a bar's line: judged together, and hidden together, as the label
    // step hid the label.
    let shown_clip = ui.clip_rect();
    let cut = !ui.is_measuring() && cut_by_bar_view(&env, placed.label.union(placed.control));
    if cut {
        ui.set_clip_rect(Rect::NOTHING);
    }
    let output = crate::contract::run::show_leaf(
        &mut Some(crate::contract::Props::id(child, id)),
        ui,
        (crate::contract::run::ShowAt::Rect(control), place),
        None,
    );
    if cut {
        ui.set_clip_rect(shown_clip);
    }
    output
}

/// In a wrapping row (a `Row` with `wrap`), starts a new line before a labelled leaf that would not fit, at one line,
/// in what is left of the current one, as the row wraps any other child. The leaf is placed in the rest of the line
/// before its space is allocated, so egui's own wrap at the allocation comes too late: without this the row never
/// wraps a labelled radio or check box, and the last one's label wraps in the sliver left at the line's end (Page
/// Setup's "Text snaps to characters"). A leaf at the line's start stays, however wide. `control` is the control's
/// ideal width.
fn wrap_before(ui: &mut Ui, env: &Env, (label, side): (&Label, &SideLabel), control: f32) {
    if !matches!(ui.flow.dir_ref(), crate::contract::rt::ui::Dir::Row { wrap: true, .. }) {
        return;
    }
    let line = ui.max_rect();
    let rest = ui.available_rect_before_wrap();
    let used = if env.direction.is_rtl() { line.max.x - rest.max.x } else { rest.min.x - line.min.x };
    if used <= 0.5 || !rest.width().is_finite() {
        return;
    }
    let text = if side.shows_no_leaf(false) {
        0.0
    } else {
        LabelLeaf::new(side.kind.shown(label).clone(), 0.0).measure_in(ui, craft_behavior::Proposal::Ideal).x + label_gap(env)
    };
    if control + text > rest.width() {
        // The row's cursor may reach down without end (a room unbounded below), and `end_row` starts the next line
        // under it: the line is first set to the height its children took, so the next one starts under them, as
        // egui's own wrap places it.
        let top = ui.cursor().min.y;
        ui.set_row_height((ui.min_rect().bottom() - top).max(0.0));
        ui.end_row();
    }
}

/// Shows `child` in the flow of `ui`. A component whose `LABELLED_AFTER` is set and that carries a label shows it
/// through the label step, after the control, unless a label scope already names the control (a `Field`, `labelled`;
/// a member of a group, a radio, is named by the group and keeps its own text):
/// that scope's label is the visible text, so the control's own is not shown twice. Every other leaf shows as it is.
pub(crate) fn show_leaf_in_flow<C: Show>(mut child: C, ui: &mut Ui) -> C::Output {
    let env = Env::of(ui);
    let named_by_scope = env.arena.is_none() && env.view().field_label().is_some();
    let shows = C::LABELLED_AFTER && child.shows_label() && child.common(crate::contract::Internal::new()).label != Label::Empty && !named_by_scope;
    if !shows {
        return crate::contract::run::show_leaf(&mut Some(child), ui, (crate::contract::run::ShowAt::Flow, env), None);
    }
    let label = std::mem::take(&mut child.common_mut(crate::contract::Internal::new()).label);
    // The id the leaf would take shown on its own; the label step shows it under that id.
    let source = child.common(crate::contract::Internal::new()).id.clone();
    let id = ui.flow.child(crate::contract::rt::key::type_key::<C>(), source).0;
    let side = SideLabel::new().layout(FieldLayout::After);
    let side = match child.label_detail() {
        Some(detail) => side.detail(detail),
        None => side,
    };
    let under = (env.enabled, env.size);
    flow_leaf((ui, env), (id, id), under, label, side, child, None)
}
