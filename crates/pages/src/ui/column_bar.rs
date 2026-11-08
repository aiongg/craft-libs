//! `ColumnBar`: a bar split into column shares with draggable dividers, as PdfCraft's Create Table of Fields sets its
//! column widths. A divider pushes its neighbours (`craft_behavior::divider`): the columns on the far side give way in
//! turn down to their least share.

#[cfg(feature = "screen-testing")]
mod stories;

use craft_behavior::divider::{Segment, resize, travel};
use craft_behavior::nav::{StepResult, ValueAxes, ValueSteps, step_value};
use craft_behavior::{CursorIcon, HitLevel, HitPriority, Metrics, PartKey, PressPhase, Sense};
use craft_ui::contract::{
    Access, Bind, BindMut, Common, Component, Cx, Edit as _, EditResponse, Focus as _, MeasureCx, Name, Numeric, Outcome, PaintUi, PartsCx,
    Pointer as _, Props, Role, WordSetCx as _,
};
use craft_ui::primitives::{PaintShape, StrokeWidth, TextRun};
use craft_ui::runtime::env::Label;
use craft_ui::runtime::focus::KeyUse;
use craft_ui::style::{ComponentStyle, FocusLook, StyleColor, StyleCx, TypeRole};
use egui::{CornerRadius, Pos2, Rect, Response, Vec2};

/// The divider after column `i`: `DIVIDER.at(i)`.
const DIVIDER: PartKey = PartKey::new("divider");
/// Column `i`: `COLUMN.at(i)`, which names the column on hover.
const COLUMN: PartKey = PartKey::new("column");

/// The smallest share a column keeps when a divider pushes it.
pub const MIN_SHARE: f64 = 0.04;

/// The share an arrow key moves the focused divider by (PdfCraft: 0.01); Shift moves it by ten times that.
pub const KEY_STEP: f64 = 0.01;

/// The keys' large step as a multiple of the small one.
const JUMP: f32 = 10.0;

craft_ui::words! {
    /// The bar's words.
    pub struct ColumnBarWords {
        /// `{n}` is the number of the column the divider ends.
        divider: template "columnBar.divider" = "Column {n} width";
        /// `{letter}` is the column's spreadsheet letter.
        column: template "columnBar.column" = "Column {letter}";
        /// `{value}` is the column's share in percent.
        percent: template "columnBar.percent" = "{value}%";
    }
}

/// `widths` (any unit) as shares of their sum; equal shares when the sum is not positive and finite.
pub fn shares(widths: &[f64]) -> Vec<f64> {
    let total: f64 = widths.iter().sum();
    let equal = 1.0 / widths.len().max(1) as f64;
    widths.iter().map(|w| if total > 0.0 && total.is_finite() { w / total } else { equal }).collect()
}

/// The style of the column bar.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct ColumnBarStyle {
    #[style(metrics)]
    metrics: Metrics,
    #[style(focus)]
    focus: FocusLook,
    outline: StyleColor,
    divider: StyleColor,
    divider_active: StyleColor,
    fill: StyleColor,
    fill_alt: StyleColor,
    label: StyleColor,
    radius: CornerRadius,
    /// How far from a divider the pointer still grabs it.
    grab: f32,
}

/// What the column bar reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct ColumnBarOutput {
    /// The edit of the widths: a divider drag is one `Begin` to `Commit`; an arrow press is a `Set`.
    pub edit: EditResponse<Vec<f64>>,
    /// The divider (after this column, 0-based) pressed twice: the app opens its column dialog.
    pub double_clicked: Option<usize>,
}

impl Outcome for ColumnBarOutput {
    type Value = Vec<f64>;

    fn response(&self) -> &Response {
        &self.edit.response
    }
}

/// The bar over the app's `widths`.
///
/// - The bar is outlined and rounded; each divider is a line, the active one (hovered, dragged or focused) heavier.
///   A column paints its percentage when it is wide enough; its name (the app's, else "Column A") is the hover text.
/// - A divider is dragged with `craft_behavior::divider::resize`; one drag is one edit gesture and Escape cancels it. A
///   double click on a divider is reported for the app's column dialog.
/// - Each divider is a Tab stop (a splitter named "Column {n} width", valued in percent). The arrow keys move the
///   focused divider one [`KEY_STEP`] toward the side they point at, Shift ten steps; a screen reader's SetValue sets
///   the column's percentage. In a right-to-left interface the bar mirrors.
#[derive(Component, Props)]
pub struct ColumnBar<'a> {
    #[common]
    common: Common,
    widths: Bind<'a, Vec<f64>>,
    /// The columns' names (the app's field names), for the hover text.
    #[prop]
    names: &'a [String],
    double_clicked: Option<usize>,
}

impl<'a> ColumnBar<'a> {
    /// A bar named `label` for `widths` (one entry per column).
    pub fn new(label: impl Into<Label>, widths: Bind<'a, Vec<f64>>) -> Self {
        ColumnBar { common: Common::new(), widths, names: &[], double_clicked: None }.label(label)
    }

    fn name(&self, at: usize, words: &ColumnBarWords) -> String {
        match self.names.get(at) {
            Some(name) => name.clone(),
            None => words.column.fill(&[("letter", letter(at).as_str())]).as_str().to_owned(),
        }
    }
}

/// The distance from the bar's reading start to `x`.
fn logical(rect: Rect, rtl: bool, x: f32) -> f32 {
    if rtl { rect.right() - x } else { x - rect.left() }
}

/// The screen x of the distance `along` from the bar's reading start.
fn physical(rect: Rect, rtl: bool, along: f32) -> f32 {
    if rtl { rect.right() - along } else { rect.left() + along }
}

/// The columns as segments of the bar `width` wide, each keeping [`MIN_SHARE`].
fn row(shares: &[f64], width: f32) -> Vec<Segment> {
    shares.iter().map(|share| Segment::unbounded(*share as f32 * width, MIN_SHARE as f32 * width)).collect()
}

/// `widths` after the divider after column `at` moves to `target` from the bar's reading start, in the unit of the
/// widths.
fn pushed(widths: &[f64], at: usize, target: f32, width: f32) -> Vec<f64> {
    let total: f64 = widths.iter().sum();
    let extents = resize(&row(&shares(widths), width), at, target);
    let scale = if width > 0.0 { total / f64::from(width) } else { 0.0 };
    extents.into_iter().map(|extent| f64::from(extent) * scale).collect()
}

/// The distance of the divider after column `at` from the bar's reading start, in points.
fn position(shares: &[f64], at: usize, width: f32) -> f32 {
    shares.iter().take(at + 1).map(|share| *share as f32 * width).sum()
}

impl Component for ColumnBar<'_> {
    const ROLE: Role = Role::GROUP;
    const KEYS: KeyUse = KeyUse::SPLITTER;
    const OVERLAYS: bool = false;
    type Style = ColumnBarStyle;
    type Output = ColumnBarOutput;

    fn style(cx: &StyleCx<'_>, _: ColumnBarStyleKey) -> ColumnBarStyle {
        let colors = &cx.tokens.color;
        ColumnBarStyle {
            metrics: Metrics::new().min_size(Vec2::new(0.0, cx.control().height)),
            focus: cx.focus_look(),
            outline: cx.color(colors.line.control_edge),
            divider: cx.color(colors.line.control_edge),
            divider_active: cx.color(colors.accent.solid),
            fill: cx.color(colors.accent.soft),
            fill_alt: cx.color(colors.surface.sunken),
            label: cx.color(colors.text.normal),
            radius: cx.field().radius,
            grab: cx.tokens.size.hit.slop,
        }
    }

    fn fills(&self) -> Option<craft_behavior::Axis> {
        Some(craft_behavior::Axis::X)
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(0.0, cx.style.metrics.min_size.y)
    }

    fn bind(&mut self) -> Option<BindMut<'_, Vec<f64>>> {
        Some(self.widths.reborrow())
    }

    /// A strip across each column that names it on hover, and a grab zone on each divider above them, its own Tab stop.
    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        let Some(widths) = cx.value() else { return };
        let (shares, rtl, words) = (shares(widths), cx.flow.is_rtl(), cx.words::<ColumnBarWords>());
        let mut start = 0.0;
        for (at, share) in shares.iter().enumerate() {
            let end = start + *share as f32 * rect.width();
            let (a, b) = (physical(rect, rtl, start), physical(rect, rtl, end));
            let column = Rect::from_x_y_ranges(a.min(b)..=a.max(b), rect.y_range());
            cx.part(COLUMN.at(at as u64), column).tooltip(self.name(at, &words)).pointer_only("the divider's name and value describe the column");
            start = end;
        }
        for at in 0..shares.len().saturating_sub(1) {
            let x = physical(rect, rtl, position(&shares, at, rect.width()));
            let reach = cx.style.grab.min(shares[at].min(shares[at + 1]) as f32 * rect.width() / 2.0);
            let grab = Rect::from_x_y_ranges((x - reach)..=(x + reach), rect.y_range());
            cx.part(DIVIDER.at(at as u64), grab)
                .sense(Sense::DRAG | Sense::TAKE_FOCUS)
                .priority(HitPriority::new(HitLevel::Edge, 0))
                .cursor(CursorIcon::ResizeColumn)
                .focusable();
        }
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let Some(widths) = cx.value().cloned() else { return };
        let (rect, rtl) = (cx.rect, cx.flow.is_rtl());
        let width = rect.width();
        let dividers = widths.len().saturating_sub(1);
        for at in 0..dividers {
            let press = cx.part(DIVIDER.at(at as u64)).press;
            if press.double_pressed() {
                self.double_clicked = Some(at);
            }
            match press.phase {
                PressPhase::DragStart { .. } => cx.begin(),
                PressPhase::Drag { pos, .. } => cx.propose(pushed(&widths, at, logical(rect, rtl, pos.x), width)),
                PressPhase::Release { click: false, .. } => cx.commit(),
                _ => {}
            }
        }
        let shares = shares(&widths);
        let focused = cx.active_part().filter(|part| part.key == DIVIDER).and_then(|part| part.index).map(|at| at as usize);
        if let Some(at) = focused.filter(|at| *at < dividers) {
            let segments = row(&shares, width);
            let here = position(&shares, at, width);
            let (lo, hi) = travel(&segments, at);
            let step = KEY_STEP as f32 * width;
            if let Some((mv, mods)) = cx.intents().take_move() {
                let steps = ValueSteps::new(f64::from(here), f64::from(here + lo), f64::from(here + hi), f64::from(step), f64::from(step * JUMP));
                if let StepResult::Set(value) = step_value(&steps, ValueAxes::Horizontal, mv, mods) {
                    cx.set(pushed(&widths, at, value as f32, width));
                }
            }
        }
        for at in 0..dividers {
            if let Some(percent) = cx.intents().take_set_value_at(DIVIDER.at(at as u64)) {
                let start = if at > 0 { position(&shares, at - 1, width) } else { 0.0 };
                cx.set(pushed(&widths, at, start + (percent / 100.0) as f32 * width, width));
            }
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let Some(widths) = cx.value() else { return };
        let (rect, rtl, style) = (cx.rect, cx.flow.is_rtl(), &cx.style);
        let shares = shares(widths);
        let draw = ui.draw();
        let words = cx.words::<ColumnBarWords>();
        let units = cx.env.units();
        let focused = cx.focus_visible().then(|| cx.active_part()).flatten().filter(|part| part.key == DIVIDER).and_then(|part| part.index);
        let mut start = 0.0;
        for (at, share) in shares.iter().enumerate() {
            let end = start + *share as f32 * rect.width();
            let (a, b) = (physical(rect, rtl, start), physical(rect, rtl, end));
            let column = Rect::from_x_y_ranges(a.min(b)..=a.max(b), rect.y_range());
            draw.fill(&PaintShape::Rect(column, CornerRadius::ZERO), if at % 2 == 0 { style.fill } else { style.fill_alt });
            let percent = units.number(*share * 100.0, 0..=0);
            let run = TextRun::new(words.percent.fill(&[("value", percent.as_str())]).as_str().to_owned()).role(TypeRole::Caption).color(style.label);
            let layout = draw.layout(&run, column.width());
            let size = layout.galley.size();
            if size.x <= column.width() {
                draw.galley(column.center() - size / 2.0, &layout);
            }
            start = end;
        }
        draw.stroke(&PaintShape::Rect(rect, style.radius), StrokeWidth::Thin, style.outline);
        for at in 0..shares.len().saturating_sub(1) {
            let x = physical(rect, rtl, position(&shares, at, rect.width()));
            let active =
                cx.part(DIVIDER.at(at as u64)).hovered || cx.part(DIVIDER.at(at as u64)).press.phase.is_press() || focused == Some(at as u64);
            let (color, width) = if active { (style.divider_active, StrokeWidth::Thick) } else { (style.divider, StrokeWidth::Medium) };
            draw.stroke(&PaintShape::Path(vec![Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())]), width, color);
        }
    }

    /// One splitter per divider, valued by the share of the column before it in percent.
    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
        let Some(widths) = cx.value() else { return };
        let words = cx.words::<ColumnBarWords>();
        let units = cx.env.units();
        let shares = shares(widths);
        for (at, share) in shares.iter().enumerate().take(shares.len().saturating_sub(1)) {
            let number = units.number((at + 1) as f64, 0..=0);
            let node = access.node(DIVIDER.at(at as u64), Role::SPLITTER);
            node.name = Name::Text(words.divider.fill(&[("n", number.as_str())]).as_str().to_owned());
            node.numeric = Some(Numeric::new(share * 100.0, 0.0..=100.0).step(KEY_STEP * 100.0).text(units.number(share * 100.0, 0..=0)));
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> ColumnBarOutput {
        ColumnBarOutput { edit: cx.edit_response(), double_clicked: self.double_clicked }
    }
}

/// Column `at`'s spreadsheet letter: A to Z, then AA, AB, and so on.
fn letter(at: usize) -> String {
    let mut n = at + 1;
    let mut out = Vec::new();
    while n > 0 {
        n -= 1;
        out.push(char::from(b'A' + (n % 26) as u8));
        n /= 26;
    }
    out.iter().rev().collect()
}
