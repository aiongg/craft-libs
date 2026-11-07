//! The label step (B2, 16.3): a side label's placement, label scope, scrub, double-click reset and reset dot. A
//! container with `ContainerProps.label` runs it in its body step and `PaintUi::labelled` runs it for one leaf
//! child, so the two line up and behave alike.

use super::{Component, Props};
use super::{Cx, MeasureCx, PaintUi, PartsCx, Show as _};
use crate::primitives::TextRun;
use crate::runtime::access::{
    self,
    scope::{self, FieldLabel},
};
use crate::runtime::egui::{alloc, child};
use crate::runtime::env::{Env, Label};
use crate::runtime::keys;
use crate::runtime::memory::sub;
use crate::style::{ComponentStyle, ResetMarkLook, StyleCx};
use craft_behavior::{ColumnWidth, Flow, LabelColumn, Metrics, PartKey, PressPhase, PressState, Sense, ValueDrag};
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
#[derive(Clone, Copy, Debug, Default, PartialEq)]
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
}

impl SideLabel {
    /// A text label beside its control that names it.
    pub const fn new() -> Self {
        SideLabel { kind: LabelKind::Text, layout: FieldLayout::Beside, role: FieldLabelRole::Names, scrub: false }
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
    baseline: Cell<f32>,
    double_click: bool,
    dot_click: bool,
}

impl LabelLeaf {
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
        LabelStyle { metrics: Metrics::new(), reset: cx.reset_mark() }
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let layout = cx.layout(&TextRun::new(cx.resolved.label));
        self.baseline.set(layout.first_baseline);
        let size = layout.galley.size();
        Vec2::new(size.x + self.slot, size.y)
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
        let (slot, rest) = flow.take_end(self.slot);
        let draw = ui.draw();
        draw.text(rest.rect(), &TextRun::new(cx.core.resolved.label));
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
        scope::begin(
            &ctx,
            input.scope,
            input.owner,
            scope::Kind::Label(FieldLabel::new(label_id, Rect::NOTHING, input.text.as_str()).role(input.side.role)),
        );
        return input.row;
    }
    let slot = if reset.is_some() { slot_side } else { 0.0 };
    let mut leaf = LabelLeaf::new(input.text.clone(), slot);
    leaf.scrub = input.side.scrub || scope::last_scrub(&ctx, input.scope);
    leaf.resettable = reset.is_some();
    leaf.differs = reset.is_some_and(|fact| fact.differs);
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
    let label = FieldLabel::new(label_id_for_scope, placed.label, input.text.as_str()).role(input.side.role).with_press(shown.press);
    scope::begin(&ctx, input.scope, input.owner, scope::Kind::Label(label));
    placed.control
}
