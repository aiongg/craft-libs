//! The label step (B2, 16.3): a side label's placement, label scope, scrub, double-click reset and reset dot. A
//! container with `ContainerProps.label` runs it in its body step and `PaintUi::labelled` runs it for one leaf
//! child, so the two line up and behave alike.

use super::{Component, Props};
use super::{Cx, MeasureCx, PaintUi, PartsCx, Show};
use crate::primitives::{IconSpec, TextRun, Wrap};
use crate::runtime::access::{
    self,
    scope::{self, FieldLabel},
};
use crate::runtime::egui::{alloc, child};
use crate::runtime::env::{Env, Label};
use crate::runtime::keys;
use crate::runtime::layout_memory;
use crate::runtime::memory::sub;
use crate::style::{ComponentStyle, ResetMarkLook, StyleColor, StyleCx};
use craft_behavior::{ColumnWidth, Flow, LabelColumn, Metrics, PartKey, PressState, Sense, ValueDrag};
use egui::{Align, Id, Rect, Response, Ui, UiBuilder, Vec2};
use std::cell::Cell;
use std::sync::Arc;

pub use crate::runtime::access::scope::FieldLabelRole;

/// How a side label is shown.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub enum LabelKind {
    /// The label's text.
    #[default]
    Text,
    /// No visible label: the text still names the control. In a body whose labelled rows share a label column, the
    /// row keeps the column, so its control starts in the control column.
    Hidden,
    /// An icon or a short text shown in place of the label; the text still names the control. Glyph labels share
    /// a column of their own, apart from the text labels'. The label's detail joins the description and is not shown.
    Glyph(GlyphLabel),
}

/// What a glyph label shows.
#[derive(Clone, Debug, PartialEq)]
enum Glyph {
    Icon(IconSpec<'static>),
    Text(Label),
}

/// The visible form of a glyph label: an icon, or a short text such as "W" or "H". The accessible name stays the
/// label's `Common.label`.
#[derive(Clone, Debug, PartialEq)]
pub struct GlyphLabel {
    shown: Glyph,
}

impl GlyphLabel {
    /// Shows `icon`, as large as the control's icons.
    pub fn icon(icon: impl Into<IconSpec<'static>>) -> Self {
        GlyphLabel { shown: Glyph::Icon(icon.into()) }
    }

    /// Shows `text`, resolved in the scope's language.
    pub fn text(text: impl Into<Label>) -> Self {
        GlyphLabel { shown: Glyph::Text(text.into()) }
    }

    /// What the label leaf draws: the icon, or the text resolved in `env`.
    fn resolved(&self, env: &Env) -> Glyph {
        match &self.shown {
            Glyph::Icon(icon) => Glyph::Icon(icon.clone()),
            Glyph::Text(text) => Glyph::Text(env.view().resolve(text)),
        }
    }
}

/// Where a beside label sits in its column.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum LabelAlign {
    /// Against the control: the text ends where the gap before the control begins, on the start side under RTL.
    #[default]
    End,
    /// At the row's start edge.
    Start,
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
    /// After the control, as check box and radio labels sit: the control takes its narrowest width and the label
    /// follows it, on the end side under RTL. The row never stacks.
    After,
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
    /// Where a beside label sits in its column.
    pub align: LabelAlign,
    /// Whether it names or describes its control.
    pub role: FieldLabelRole,
    /// Dragging the label scrubs the control's value.
    pub scrub: bool,
    /// An icon before the text, as large as the control's icons.
    pub icon: Option<IconSpec<'static>>,
    /// A line after the text in the muted text colour. It joins the control's description, not its name.
    pub detail: Option<Label>,
}

impl SideLabel {
    /// A text label beside its control that names it.
    pub const fn new() -> Self {
        SideLabel {
            kind: LabelKind::Text,
            layout: FieldLayout::Beside,
            align: LabelAlign::End,
            role: FieldLabelRole::Names,
            scrub: false,
            icon: None,
            detail: None,
        }
    }

    /// How the label is shown.
    #[must_use]
    pub fn kind(mut self, kind: LabelKind) -> Self {
        self.kind = kind;
        self
    }

    /// Where the label sits.
    #[must_use]
    pub const fn layout(mut self, layout: FieldLayout) -> Self {
        self.layout = layout;
        self
    }

    /// Where a beside label sits in its column: against the control by default.
    #[must_use]
    pub const fn align(mut self, align: LabelAlign) -> Self {
        self.align = align;
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
    pub fn icon(mut self, icon: impl Into<IconSpec<'static>>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Shows `detail` after the text, in the muted text colour, and adds it to the control's description.
    #[must_use]
    pub fn detail(mut self, detail: impl Into<Label>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

/// How a drag on the side label (or on the control itself) changes a control's value: the drag maths over the
/// control's number.
pub struct Scrub<V> {
    pub(crate) drag: ValueDrag,
    pub(crate) on_field: bool,
    pub(crate) read: Arc<dyn Fn(&V) -> f64 + Send + Sync>,
    pub(crate) write: Arc<dyn Fn(f64) -> V + Send + Sync>,
}

impl<V> Clone for Scrub<V> {
    fn clone(&self) -> Self {
        Scrub { drag: self.drag.clone(), on_field: self.on_field, read: Arc::clone(&self.read), write: Arc::clone(&self.write) }
    }
}

impl<V> Scrub<V> {
    /// A scrub by `drag`, reading the number a value holds with `read` and making a value of a number with `write`.
    /// Both may capture the control's decimals and step, so a scrubbed value is rounded the way the control rounds.
    pub fn new(drag: ValueDrag, read: impl Fn(&V) -> f64 + Send + Sync + 'static, write: impl Fn(f64) -> V + Send + Sync + 'static) -> Self {
        Scrub { drag, on_field: false, read: Arc::new(read), write: Arc::new(write) }
    }

    /// The drag starts on the control itself instead of its side label, for a readout with no label. The control
    /// must sense a drag.
    #[must_use]
    pub fn on_field(mut self) -> Self {
        self.on_field = true;
        self
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
    activated: bool,
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
    icon: Option<IconSpec<'static>>,
    /// A glyph shown in place of the name, icon and detail.
    glyph: Option<Glyph>,
    /// The detail, resolved in the scope's language.
    detail: Option<Label>,
    baseline: Cell<f32>,
    double_click: bool,
    dot_click: bool,
    activated: bool,
}

impl LabelLeaf {
    fn detail_run<'a>(&self, detail: &'a Label, style: &LabelStyle) -> TextRun<'a> {
        TextRun::new(detail.as_str()).color(style.muted).wrap(Wrap::Wrap)
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
            glyph: None,
            detail: None,
            baseline: Cell::new(0.0),
            double_click: false,
            dot_click: false,
            activated: false,
        }
        .label(text)
    }
}

impl Component for LabelLeaf {
    const ROLE: super::Role = super::Role::LABEL;
    const OVERLAYS: bool = false;
    // The label is no key target: Enter and Space go to its control, which a click on the label focuses.
    const KEYS: crate::runtime::focus::KeyUse =
        crate::runtime::focus::KeyUse::of(Self::INPUT).with_activate(crate::runtime::focus::ActivateKeys::Neither);
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
        let line = cx.layout(&TextRun::new(cx.resolved.label));
        let name = line.size();
        if let Some(glyph) = &self.glyph {
            let (shown, baseline) = match glyph {
                Glyph::Icon(_) => {
                    let height = cx.style.icon.max(name.y);
                    (Vec2::new(cx.style.icon, height), line.first_baseline + (height - name.y) / 2.0)
                }
                Glyph::Text(text) => {
                    let run = cx.layout(&TextRun::new(text.as_str()));
                    (run.size(), run.first_baseline)
                }
            };
            self.baseline.set(baseline);
            return Vec2::new(shown.x + self.slot, shown.y);
        }
        let (mut width, mut height) = (name.x, name.y);
        if self.icon.is_some() {
            width += cx.style.icon + cx.style.gap;
            height = height.max(cx.style.icon);
        }
        if let Some(detail) = &self.detail {
            // The detail wraps in the room the name and the gap leave.
            let room = match cx.proposal {
                craft_behavior::Proposal::Width(width) => Some(width),
                _ => cx.room,
            };
            let left = room.map_or(f32::INFINITY, |room| (room - self.slot - width - cx.style.gap).max(0.0));
            let detail = cx.layout_at(&self.detail_run(detail, cx.style), left).size();
            width += cx.style.gap + detail.x;
            height = height.max(detail.y);
        }
        // The text sits at the row's vertical centre, so an icon taller than the text moves the baseline down.
        self.baseline.set(line.first_baseline + (height - name.y) / 2.0);
        Vec2::new(width + self.slot, height)
    }

    /// A click on the label focuses its control (the scope's `FieldLabel::activated`), so every label takes clicks.
    fn sense(&self) -> Sense {
        if self.scrub { Sense::CLICK | Sense::DRAG } else { Sense::CLICK }
    }

    fn parts(&mut self, _rect: Rect, cx: &mut PartsCx<'_, Self>) {
        if self.slot > 0.0 && self.differs {
            let (slot, _) = cx.flow.take_end(self.slot);
            cx.part(DOT, slot).sense(Sense::CLICK).pointer_only("the Reset command");
        }
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        use super::Pointer as _;
        self.double_click = self.resettable && cx.press().double_pressed();
        self.dot_click = self.slot > 0.0 && self.differs && cx.part(DOT).press.phase.is_click();
        self.activated = cx.activation().is_some();
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let flow = Flow::new(cx.rect, cx.core.env.direction().is_rtl());
        let (slot, mut rest) = flow.take_end(self.slot);
        let draw = ui.draw();
        let middle = cx.rect.center().y;
        let centred = |rect: Rect, height: f32| Rect::from_min_size(egui::pos2(rect.min.x, middle - height / 2.0), Vec2::new(rect.width(), height));
        if let Some(glyph) = &self.glyph {
            match glyph {
                Glyph::Icon(icon) => {
                    draw.icon(centred(rest.take_start(cx.style.icon).0, cx.style.icon), icon);
                }
                Glyph::Text(text) => {
                    let run = TextRun::new(text.as_str());
                    let size = draw.layout(&run, rest.rect().width()).size();
                    draw.text(centred(rest.take_start(size.x).0, size.y), &run);
                }
            }
        } else {
            if let Some(icon) = &self.icon {
                let (cell, after) = rest.take_start(cx.style.icon);
                draw.icon(centred(cell, cx.style.icon), icon);
                rest = after.take_start(cx.style.gap).1;
            }
            // A name wider than its room elides; the full text stays the accessible name.
            let run = TextRun::new(cx.core.resolved.label).elide();
            let name = draw.layout(&run, rest.rect().width()).size();
            let (name_rect, after) = rest.take_start(name.x);
            draw.text(centred(name_rect, name.y), &run);
            if let Some(detail) = &self.detail {
                let run = self.detail_run(detail, cx.style);
                let (_, detail_flow) = after.take_start(cx.style.gap);
                let size = draw.layout(&run, detail_flow.rect().width()).size();
                draw.text(centred(detail_flow.rect(), size.y), &run);
            }
        }
        if self.differs && self.slot > 0.0 {
            draw.reset_mark(&cx.style.reset, slot.center());
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> LabelOutput {
        LabelOutput {
            response: cx.core.response.clone(),
            press: cx.core.press,
            double_click: self.double_click,
            dot_click: self.dot_click,
            activated: self.activated,
        }
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
    /// The help line the container's footer shows; it joins the control's description with the label's detail.
    pub help: Option<&'a Label>,
    pub enabled: bool,
    pub size: craft_theme::ControlSize,
    /// The narrowest the control can be; a row too narrow for the label column, the gap and this stacks the label above.
    pub control_min: f32,
}

/// The label fact the scope records: the name's text, and the detail and the help line (resolved) as the control's
/// description.
fn field_label(env: &Env, input: &LabelIn<'_>, id: Id, rect: Rect) -> FieldLabel {
    let label = FieldLabel::new(id, rect, input.text.as_str()).role(input.side.role);
    let described: Vec<String> =
        [input.side.detail.as_ref(), input.help].into_iter().flatten().map(|text| env.view().resolve(text).as_str().to_owned()).collect();
    if described.is_empty() { label } else { label.detail(described.join(". ")) }
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
    /// The label's rect; `Rect::NOTHING` for a hidden label.
    pub label: Rect,
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
    // The child's own `enabled(false)` disables its label too: a disabled field's label does not scrub.
    let enabled = at.enabled && child.common(super::Internal::new()).enabled.unwrap_or(true);
    let control_min = super::run::measure_under(child, &*ui, Some(at.id), craft_behavior::Proposal::Min, Some((enabled, at.size))).x;
    let input = LabelIn {
        scope,
        owner: at.owner,
        parent: at.parent,
        row: at.row,
        room: at.row.width(),
        text: at.text,
        side: at.side,
        help: None,
        enabled,
        size: at.size,
        control_min,
    };
    let placed = run(ui, env, &input);
    LeafPlaced { control: placed.control, label: placed.label, scope }
}

/// Where the label step put the label and the control.
pub(crate) struct Placed {
    /// The label's rect; `Rect::NOTHING` for a hidden label.
    pub label: Rect,
    /// The rect left for the control.
    pub control: Rect,
}

/// The `After` placement: the control's cell first, then the gap, then the label. As in `LabelColumn::place`, the
/// one with the lower baseline moves down so the two baselines line up, using the baseline `record_control_baseline`
/// stored last pass.
fn place_after(ctx: &egui::Context, scope: Id, flow: Flow, control_min: f32, gap: f32, label: Vec2, label_baseline: f32) -> Placed {
    let rtl = flow.is_rtl();
    let (control_cell, rest) = flow.take_start(control_min.min(flow.rect().width()));
    let (_, rest) = rest.take_start(gap);
    let control_baseline = layout_memory::read(ctx, sub(scope, "baseline")).map(|size| size.y);
    let drop = control_baseline.map_or(0.0, |control| control - label_baseline);
    let (label_drop, control_drop) = (drop.max(0.0), (-drop).max(0.0));
    let label_size = Vec2::new(label.x.min(rest.rect().width()), label.y);
    let label = Flow::new(rest.rect(), rtl).align(label_size, Align::Min, Align::Min).translate(Vec2::new(0.0, label_drop));
    let control = Rect::from_min_max(egui::pos2(control_cell.left(), control_cell.top() + control_drop), control_cell.max);
    Placed { label, control }
}

/// The rect a hidden label leaves for its control. In a body whose labelled rows share a label column the label
/// still takes the column, so the control starts in the control column; elsewhere, or where the row is too narrow
/// for the column, the control takes the row.
fn hidden_control(ctx: &egui::Context, input: &LabelIn<'_>, flow: Flow, gap: f32) -> Rect {
    if input.side.layout != FieldLayout::Beside {
        return input.row;
    }
    match crate::layout::ambient_width(ctx, &crate::layout::LABELS, 0.0).filter(|column| *column > 0.0) {
        Some(column) if input.room >= column + gap + input.control_min => {
            let (_, rest) = flow.take_start(column);
            rest.take_start(gap).1.rect()
        }
        _ => input.row,
    }
}

/// Runs the label step in `ui`: shows the label, opens the label scope and returns where the label and the control
/// sit. The caller closes the scope with `scope::finish` after the control ran.
pub(crate) fn run(ui: &mut Ui, env: &Env, input: &LabelIn<'_>) -> Placed {
    let ctx = ui.ctx().clone();
    let rtl = env.direction.is_rtl();
    let tokens = &env.tokens;
    let (gap, slot_side) = (tokens.space.gap.inline, tokens.size.handle.sm);
    let reset = scope::last_reset(&ctx, input.scope);
    let flow = Flow::new(input.row, rtl);
    let label_id = sub(input.scope, "label");
    if input.side.kind == LabelKind::Hidden {
        scope::begin(&ctx, input.scope, input.owner, scope::Kind::Label(field_label(env, input, label_id, Rect::NOTHING)));
        return Placed { label: Rect::NOTHING, control: hidden_control(&ctx, input, flow, gap) };
    }
    let slot = if reset.is_some() { slot_side } else { 0.0 };
    let mut leaf = LabelLeaf::new(input.text.clone(), slot);
    leaf.scrub = input.enabled && (input.side.scrub || scope::last_scrub(&ctx, input.scope));
    leaf.resettable = reset.is_some();
    leaf.differs = reset.is_some_and(|fact| fact.differs);
    match &input.side.kind {
        LabelKind::Glyph(glyph) => leaf.glyph = Some(glyph.resolved(env)),
        _ => {
            leaf.icon = input.side.icon.clone();
            leaf.detail = input.side.detail.as_ref().map(|detail| env.view().resolve(detail));
        }
    }
    let glyph = leaf.glyph.is_some();
    let kids_scope = sub(input.scope, "label_ui");
    let builder = UiBuilder::new().max_rect(input.row).layout(child::pinned_layout()).scope_id(kids_scope);
    let builder = match input.parent {
        Some(parent) => access::with_parent(builder, parent),
        None => builder,
    };
    let mut kids = alloc::child(ui, builder);
    let kid_env = super::cx::child_env(env, input.enabled, input.size);
    let installed = Env::install_for(&kids, kid_env);
    let mut natural = leaf.measure_in(&kids, craft_behavior::Proposal::Ideal);
    if input.side.layout == FieldLayout::After {
        // The label takes the room the control and the gap leave, and its detail wraps there.
        let room = (input.row.width() - input.control_min - gap).max(0.0);
        if natural.x > room {
            natural = leaf.measure_in(&kids, craft_behavior::Proposal::Width(room));
        }
    }
    let baseline = leaf.baseline.get();
    let (column, stack_below) = match input.side.layout {
        FieldLayout::Beside => {
            let own = natural.x;
            let width = crate::layout::ambient_width(&ctx, &if glyph { crate::layout::GLYPHS } else { crate::layout::LABELS }, own)
                .filter(|width| *width > 0.0)
                .unwrap_or(own);
            (width, width + gap + input.control_min)
        }
        FieldLayout::Above => (natural.x, f32::INFINITY),
        FieldLayout::After => (natural.x, 0.0),
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
    // A stacked label has the row's width at most; its name elides there.
    let column = if stacks { column.min(input.row.width()) } else { column };
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
    let placed = if input.side.layout == FieldLayout::After {
        place_after(&ctx, input.scope, flow, input.control_min, gap, natural, baseline)
    } else {
        let placed = crate::layout::place_label(
            &kids,
            input.scope,
            &LabelColumn::new(column, if stacks { f32::INFINITY } else { 0.0 }, gap),
            flow,
            natural.min(Vec2::new(column, natural.y)),
            baseline,
        );
        let mut label = placed.label;
        if !placed.stacked && input.side.align == LabelAlign::End {
            let spare = (column - label.width()).max(0.0);
            label = label.translate(Vec2::new(if rtl { -spare } else { spare }, 0.0));
        }
        Placed { label, control: placed.control }
    };
    let label_id_for_scope = sub(input.scope, "label");
    let shown = Props::id(leaf, label_id_for_scope).show_at(&mut kids, placed.label);
    Env::restore(&ctx, installed);
    if let Some(fact) = reset.filter(|_| shown.double_click || shown.dot_click) {
        keys::route_whole_reset(&ctx, fact.control);
    }
    let label = field_label(env, input, label_id_for_scope, placed.label).with_press(shown.press).activated(shown.activated);
    scope::begin(&ctx, input.scope, input.owner, scope::Kind::Label(label));
    placed
}
