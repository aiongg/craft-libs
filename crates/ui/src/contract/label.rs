//! The label step (B2, 16.3): a side label's placement, label scope, scrub, double-click reset and reset dot. A
//! container with `ContainerProps.label` runs it in its body step and `PaintUi::labelled` runs it for one leaf
//! child, so the two line up and behave alike.

use super::{Component, Props};
use super::{Cx, MeasureCx, PaintUi, PartsCx, Show};
use crate::primitives::{IconSpec, TextRun};
use crate::runtime::access::{
    self,
    scope::{self, FieldLabel},
};
use crate::runtime::egui::{alloc, child};
use crate::runtime::env::{Env, Label};
use crate::runtime::keys;
use crate::runtime::memory::sub;
use crate::style::{ComponentStyle, ResetMarkLook, StyleColor, StyleCx};
use craft_behavior::{ColumnWidth, Flow, IconId, LabelColumn, Metrics, PartKey, PressPhase, PressState, Sense, ValueDrag};
use egui::{Id, Rect, Response, Ui, UiBuilder, Vec2};
use std::cell::Cell;

pub use crate::runtime::access::scope::FieldLabelRole;

/// How a side label is shown.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum LabelKind {
    /// The label's text.
    #[default]
    Text,
    /// No visible label: the text still names the control.
    Hidden,
}

/// Where a side label sits relative to its control.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub enum FieldLayout {
    /// In a column before the control, above it when the row is too narrow.
    #[default]
    Beside,
    /// Above the control.
    Above,
    /// Above the control, at most this wide.
    Caption(CellWidth),
}

/// The width of a caption's cell.
pub type CellWidth = ColumnWidth;

/// What `ContainerProps.label` holds: how the label is shown and what it does. The text is `Common.label`.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct SideLabel {
    /// How the label is shown.
    pub kind: LabelKind,
    /// Where it sits.
    pub layout: FieldLayout,
    /// Whether it names or describes its control.
    pub role: FieldLabelRole,
    /// Dragging the label scrubs the control's value.
    pub scrub: bool,
    /// An icon before the text, as large as the control's icons.
    pub icon: Option<IconId>,
    /// A line after the text in the muted text colour. It joins the control's description, not its name.
    pub detail: Option<Label>,
}

impl SideLabel {
    /// A text label beside its control that names it.
    pub const fn new() -> Self {
        SideLabel { kind: LabelKind::Text, layout: FieldLayout::Beside, role: FieldLabelRole::Names, scrub: false, icon: None, detail: None }
    }

    /// How the label is shown.
    #[must_use]
    pub const fn kind(mut self, kind: LabelKind) -> Self {
        self.kind = kind;
        self
    }

    /// Where the label sits.
    #[must_use]
    pub const fn layout(mut self, layout: FieldLayout) -> Self {
        self.layout = layout;
        self
    }

    /// Whether the label names or describes its control.
    #[must_use]
    pub const fn role(mut self, role: FieldLabelRole) -> Self {
        self.role = role;
        self
    }

    /// Lets a drag on the label scrub the control's value.
    #[must_use]
    pub const fn scrub(mut self, on: bool) -> Self {
        self.scrub = on;
        self
    }

    /// Shows `icon` before the text.
    #[must_use]
    pub fn icon(mut self, icon: IconId) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Shows `detail` after the text, in the muted text colour, and adds it to the control's description.
    #[must_use]
    pub fn detail(mut self, detail: impl Into<Label>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

/// How a drag on the side label changes a control's value: the drag maths over the control's number.
pub struct Scrub<V> {
    pub(crate) drag: ValueDrag,
    pub(crate) read: fn(&V) -> f64,
    pub(crate) write: fn(f64) -> V,
}

impl<V> Scrub<V> {
    /// A scrub by `drag`, reading the number a value holds with `read` and making a value of a number with `write`.
    pub const fn new(drag: ValueDrag, read: fn(&V) -> f64, write: fn(f64) -> V) -> Self {
        Scrub { drag, read, write }
    }
}

/// The part key of the reset dot.
const DOT: PartKey = PartKey::new("reset");

/// The style of the label leaf: no padding, and the look of the reset dot.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
struct LabelStyle {
    #[style(metrics)]
    metrics: Metrics,
    reset: ResetMarkLook,
    /// The side of the icon before the text.
    icon: f32,
    /// The gap between the icon, the text and the detail.
    gap: f32,
    /// The colour of the detail.
    muted: StyleColor,
}

/// What the label leaf reports.
struct LabelOutput {
    response: Response,
    press: PressState,
    double_click: bool,
    dot_click: bool,
}

impl super::Outcome for LabelOutput {
    type Value = super::NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// The side label as a leaf: its own node, press state and reset dot.
#[derive(super::Component)]
#[component(sample = LabelLeaf::sample)]
struct LabelLeaf {
    #[common]
    common: super::Common,
    scrub: bool,
    /// The control behind the label has a reset value.
    resettable: bool,
    /// The dot shows: the value differs from the reset value.
    differs: bool,
    /// The width the dot's slot takes at the end of the label, 0 without one.
    slot: f32,
    icon: Option<IconId>,
    /// The detail, resolved in the scope's language.
    detail: Option<Label>,
    baseline: Cell<f32>,
    double_click: bool,
    dot_click: bool,
}

impl LabelLeaf {
    fn detail_run<'a>(&self, detail: &'a Label, style: &LabelStyle) -> TextRun<'a> {
        TextRun::new(detail.as_str()).color(style.muted)
    }

    #[cfg(feature = "testing")]
    fn sample() -> Self {
        LabelLeaf::new(Label::from("Opacity"), 0.0)
    }

    fn new(text: Label, slot: f32) -> Self {
        LabelLeaf {
            common: super::Common::new(),
            scrub: false,
            resettable: false,
            differs: false,
            slot,
            icon: None,
            detail: None,
            baseline: Cell::new(0.0),
            double_click: false,
            dot_click: false,
        }
        .label(text)
    }
}

impl Component for LabelLeaf {
    const ROLE: super::Role = super::Role::LABEL;
    const OVERLAYS: bool = false;
    type Style = LabelStyle;
    type Output = LabelOutput;

    fn style(cx: &StyleCx<'_>, _: LabelLeafStyleKey) -> LabelStyle {
        LabelStyle {
            metrics: Metrics::new(),
            reset: cx.reset_mark(),
            icon: cx.control().icon,
            gap: cx.tokens.space.gap.inline,
            muted: cx.color(cx.tokens.color.text.muted),
        }
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let name = cx.layout(&TextRun::new(cx.resolved.label)).size();
        let (mut width, mut height) = (name.x, name.y);
        if self.icon.is_some() {
            width += cx.style.icon + cx.style.gap;
            height = height.max(cx.style.icon);
        }
        if let Some(detail) = &self.detail {
            let detail = cx.layout(&self.detail_run(detail, cx.style)).size();
            width += cx.style.gap + detail.x;
            height = height.max(detail.y);
        }
        // The text sits at the row's vertical centre, so an icon taller than the text moves the baseline down.
        self.baseline.set(cx.layout(&TextRun::new(cx.resolved.label)).first_baseline + (height - name.y) / 2.0);
        Vec2::new(width + self.slot, height)
    }

    fn sense(&self) -> Sense {
        if self.scrub {
            Sense::CLICK | Sense::DRAG
        } else if self.resettable {
            Sense::CLICK
        } else {
            Sense::HOVER
        }
    }

    fn parts(&mut self, _rect: Rect, cx: &mut PartsCx<'_, Self>) {
        if self.slot > 0.0 && self.differs {
            let (slot, _) = cx.flow.take_end(self.slot);
            cx.part(DOT, slot).sense(Sense::CLICK).pointer_only("the Reset command");
        }
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        use super::Pointer as _;
        self.double_click = self.resettable && matches!(cx.press().phase, PressPhase::Down { start, .. } if start.clicks >= 2);
        self.dot_click = self.slot > 0.0 && self.differs && cx.part(DOT).press.phase.is_click();
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let flow = Flow::new(cx.rect, cx.core.env.direction().is_rtl());
        let (slot, mut rest) = flow.take_end(self.slot);
        let draw = ui.draw();
        let middle = cx.rect.center().y;
        let centred = |rect: Rect, height: f32| Rect::from_min_size(egui::pos2(rect.min.x, middle - height / 2.0), Vec2::new(rect.width(), height));
        if let Some(icon) = &self.icon {
            let (cell, after) = rest.take_start(cx.style.icon);
            draw.icon(centred(cell, cx.style.icon), &IconSpec::svg(icon.clone()));
            rest = after.take_start(cx.style.gap).1;
        }
        let run = TextRun::new(cx.core.resolved.label);
        let name = draw.layout(&run, rest.rect().width()).size();
        let (name_rect, after) = rest.take_start(name.x);
        draw.text(centred(name_rect, name.y), &run);
        if let Some(detail) = &self.detail {
            let run = self.detail_run(detail, cx.style);
            let (_, detail_flow) = after.take_start(cx.style.gap);
            let size = draw.layout(&run, detail_flow.rect().width()).size();
            draw.text(centred(detail_flow.rect(), size.y), &run);
        }
        if self.differs && self.slot > 0.0 {
            draw.reset_mark(&cx.style.reset, slot.center());
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> LabelOutput {
        LabelOutput { response: cx.core.response.clone(), press: cx.core.press, double_click: self.double_click, dot_click: self.dot_click }
    }
}

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
    pub text: &'a Label,
    pub side: &'a SideLabel,
    pub enabled: bool,
    pub size: craft_theme::ControlSize,
    /// The narrowest the control can be; a row too narrow for the label column, the gap and this stacks the label above.
    pub control_min: f32,
}

/// The label fact the scope records: the name's text, and the detail (resolved) as the control's description.
fn field_label(env: &Env, input: &LabelIn<'_>, id: Id, rect: Rect) -> FieldLabel {
    let label = FieldLabel::new(id, rect, input.text.as_str()).role(input.side.role);
    match &input.side.detail {
        Some(detail) => label.detail(env.view().resolve(detail).as_str()),
        None => label,
    }
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
    pub text: &'a Label,
    pub side: &'a SideLabel,
    /// The enabled state and size the child inherits.
    pub enabled: bool,
    pub size: craft_theme::ControlSize,
}

/// A leaf child's label step that has run: the rect left for the child and the scope the caller closes.
pub(crate) struct LeafPlaced {
    pub control: Rect,
    scope: Id,
}

impl LeafPlaced {
    /// Closes the label scope after the child ran.
    pub(crate) fn finish(self, ctx: &egui::Context) {
        scope::finish(ctx, self.scope, None);
    }
}

/// The label step for one leaf child: measures the child's narrowest width, runs the step in `row` and returns the
/// rect left for the child. The caller shows the child there and then calls `LeafPlaced::finish`.
pub(crate) fn leaf<C: Show>(ui: &mut Ui, env: &Env, at: &LeafLabel<'_>, child: &mut C) -> LeafPlaced {
    let scope = sub(at.id, "field");
    let control_min = super::run::measure_under(child, &*ui, Some(at.id), craft_behavior::Proposal::Min, Some((at.enabled, at.size))).x;
    let input = LabelIn {
        scope,
        owner: at.owner,
        parent: at.parent,
        row: at.row,
        room: at.row.width(),
        text: at.text,
        side: at.side,
        enabled: at.enabled,
        size: at.size,
        control_min,
    };
    LeafPlaced { control: run(ui, env, &input), scope }
}

/// Runs the label step in `ui`: shows the label, opens the label scope and returns the rect left for the control.
/// The caller closes the scope with `scope::finish` after the control ran.
pub(crate) fn run(ui: &mut Ui, env: &Env, input: &LabelIn<'_>) -> Rect {
    let ctx = ui.ctx().clone();
    let rtl = env.direction.is_rtl();
    let tokens = &env.tokens;
    let (gap, slot_side) = (tokens.space.gap.inline, tokens.size.handle.sm);
    let reset = scope::last_reset(&ctx, input.scope);
    let flow = Flow::new(input.row, rtl);
    let label_id = sub(input.scope, "label");
    if input.side.kind == LabelKind::Hidden {
        scope::begin(&ctx, input.scope, input.owner, scope::Kind::Label(field_label(env, input, label_id, Rect::NOTHING)));
        return input.row;
    }
    let slot = if reset.is_some() { slot_side } else { 0.0 };
    let mut leaf = LabelLeaf::new(input.text.clone(), slot);
    leaf.scrub = input.side.scrub || scope::last_scrub(&ctx, input.scope);
    leaf.resettable = reset.is_some();
    leaf.differs = reset.is_some_and(|fact| fact.differs);
    leaf.icon = input.side.icon.clone();
    leaf.detail = input.side.detail.as_ref().map(|detail| env.view().resolve(detail));
    let kids_scope = sub(input.scope, "label_ui");
    let builder = UiBuilder::new().max_rect(input.row).layout(child::pinned_layout()).scope_id(kids_scope);
    let builder = match input.parent {
        Some(parent) => access::with_parent(builder, parent),
        None => builder,
    };
    let mut kids = alloc::child(ui, builder);
    let kid_env = super::cx::child_env(env, input.enabled, input.size);
    let installed = Env::install_for(&kids, kid_env);
    let natural = leaf.measure_in(&kids, craft_behavior::Proposal::Ideal);
    let baseline = leaf.baseline.get();
    let (column, stack_below) = match input.side.layout {
        FieldLayout::Beside => {
            let own = natural.x;
            let width = crate::layout::ambient_width(&ctx, &crate::layout::LABELS, own).filter(|width| *width > 0.0).unwrap_or(own);
            (width, width + gap + input.control_min)
        }
        FieldLayout::Above => (natural.x, f32::INFINITY),
        FieldLayout::Caption(cell) => {
            let width = match cell {
                ColumnWidth::Fixed(width) => width.min(natural.x),
                _ => natural.x,
            };
            (width, f32::INFINITY)
        }
    };
    // The room decides; a beside label gets the width it needs even where last frame's content was narrower.
    let stacks = input.room < stack_below;
    let flow = if stacks || input.row.width() >= stack_below {
        flow
    } else {
        let mut wide = input.row;
        if rtl {
            wide.min.x = wide.max.x - stack_below;
        } else {
            wide.max.x = wide.min.x + stack_below;
        }
        Flow::new(wide, rtl)
    };
    let placed = crate::layout::place_label(
        &kids,
        input.scope,
        &LabelColumn::new(column, if stacks { f32::INFINITY } else { 0.0 }, gap),
        flow,
        natural.min(Vec2::new(column, natural.y)),
        baseline,
    );
    let label_id_for_scope = sub(input.scope, "label");
    let shown = Props::id(leaf, label_id_for_scope).show_at(&mut kids, placed.label);
    Env::restore(&ctx, installed);
    if let Some(fact) = reset.filter(|_| shown.double_click || shown.dot_click) {
        keys::route_whole_reset(&ctx, fact.control);
    }
    let label = field_label(env, input, label_id_for_scope, placed.label).with_press(shown.press);
    scope::begin(&ctx, input.scope, input.owner, scope::Kind::Label(label));
    placed.control
}
