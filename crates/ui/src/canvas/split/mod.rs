//! `SplitViews`: one canvas area shown as 1, 2 or 4 views of the same document. One view is active and is the full
//! interactive view (tools, overlays, keys); the others show the document from their own view and become active on a
//! click. The dividers between views move with the pointer and, when focused, with the arrow keys.
//!
//! The body shows one view per pane at the rects [`SplitLayout::rects`] gives for the split the caller binds. A
//! linked pane follows the active view's zoom and position ([`SplitViews::follow`]).

#[cfg(feature = "testing")]
mod stories;

use super::words::SplitWords;
use crate::contract::{
    Access, Bind, BindMut, Common, Component, Container, ContainerProps, Cx, Edit as _, EditResponse, Focus as _, Name, Numeric, Outcome, Pad,
    PaintUi, PartsCx, Pointer as _, Props as _, Role, Sizing, WordSetCx as _,
};
use crate::docking::Splitter;
use crate::primitives::{PaintShape, StrokeWidth};
use crate::runtime::env::Label;
use crate::runtime::focus::KeyUse;
use crate::style::{ComponentStyle, StyleColor, StyleCx};
use craft_behavior::nav::{StepResult, ValueAxes, ValueSteps, step_value};
use craft_behavior::{Axis, CanvasView, CursorIcon, HitLevel, HitPriority, Metrics, NavMove, PartKey, PressPhase, Sense};
use egui::{CornerRadius, Pos2, Rect, Response};

const DIVIDER: PartKey = PartKey::new("divider");
const PANE: PartKey = PartKey::new("pane");

/// How the canvas area divides.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SplitLayout {
    /// One view.
    #[default]
    One,
    /// Two views side by side, the main view on the right.
    Two,
    /// Four views in quarters, the main view at the bottom right.
    Four,
}

/// Where the dividers lie, as fractions of the area: `x` of its width for the vertical divider between side by side
/// views, `y` of its height for the horizontal one of four views.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SplitAt {
    /// The vertical divider's distance from the left edge, over the width.
    pub x: f32,
    /// The horizontal divider's distance from the top edge, over the height.
    pub y: f32,
}

impl SplitAt {
    /// Dividers at fractions `x` and `y` of the area.
    pub const fn new(x: f32, y: f32) -> Self {
        SplitAt { x, y }
    }
}

impl Default for SplitAt {
    fn default() -> Self {
        SplitAt::new(0.5, 0.5)
    }
}

impl SplitLayout {
    /// The number of panes.
    pub fn panes(self) -> usize {
        match self {
            SplitLayout::One => 1,
            SplitLayout::Two => 2,
            SplitLayout::Four => 4,
        }
    }

    /// The panes' rects in `full` with the dividers at `at`, the main view (pane 0) first and the others in reading
    /// order.
    pub fn rects(self, at: SplitAt, full: Rect) -> Vec<Rect> {
        let (x, y) = (full.left() + full.width() * at.x, full.top() + full.height() * at.y);
        let quad = |x0, y0, x1, y1| Rect::from_min_max(Pos2::new(x0, y0), Pos2::new(x1, y1));
        match self {
            SplitLayout::One => vec![full],
            SplitLayout::Two => vec![quad(x, full.top(), full.right(), full.bottom()), quad(full.left(), full.top(), x, full.bottom())],
            SplitLayout::Four => vec![
                quad(x, y, full.right(), full.bottom()),
                quad(full.left(), full.top(), x, y),
                quad(x, full.top(), full.right(), y),
                quad(full.left(), y, x, full.bottom()),
            ],
        }
    }

    /// The panes in reading order: top left first, row by row. The pane numbered `n` in the accessible names is the
    /// `n`th of this order.
    pub fn reading_order(self) -> Vec<usize> {
        match self {
            SplitLayout::One => vec![0],
            SplitLayout::Two => vec![1, 0],
            SplitLayout::Four => vec![1, 2, 3, 0],
        }
    }

    /// The order the body shows the panes in: reading order, with pane `first` moved to the front. Panes keep their
    /// position and number; the widgets inside them register in this order, so it sets their tab order.
    pub fn draw_order(self, first: usize) -> Vec<usize> {
        let mut order = self.reading_order();
        if let Some(at) = order.iter().position(|&pane| pane == first) {
            order[..=at].rotate_right(1);
        }
        order
    }
}

/// One pane's view: its own, or linked to the main view's zoom and position.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SplitPane {
    /// The pane's own view; ignored for centre and zoom while `linked`.
    pub view: CanvasView,
    /// The pane follows the main view's centre and zoom (its rotation stays its own).
    pub linked: bool,
}

impl SplitPane {
    /// A pane showing `view`.
    pub const fn new(view: CanvasView, linked: bool) -> Self {
        SplitPane { view, linked }
    }
}

/// A divider: the axis the panes it separates follow, its thickness-wide bar, its coordinate across the bar and the
/// span the coordinate moves in.
struct Divider {
    axis: Axis,
    bar: Rect,
    at: f32,
    lo: f32,
    hi: f32,
}

/// The dividers of `layout` in `rect` at `at`: the vertical one first, then the horizontal one of four views. Each
/// can move until a pane is `least` points across.
fn dividers(layout: SplitLayout, at: SplitAt, rect: Rect, reach: f32, least: f32) -> Vec<Divider> {
    let (x, y) = (rect.left() + rect.width() * at.x, rect.top() + rect.height() * at.y);
    let span = |from: f32, to: f32| (from + least, (to - least).max(from + least));
    let vertical = || {
        let (lo, hi) = span(rect.left(), rect.right());
        Divider {
            axis: Axis::X,
            bar: Rect::from_min_max(Pos2::new(x - reach / 2.0, rect.top()), Pos2::new(x + reach / 2.0, rect.bottom())),
            at: x,
            lo,
            hi,
        }
    };
    let horizontal = || {
        let (lo, hi) = span(rect.top(), rect.bottom());
        Divider {
            axis: Axis::Y,
            bar: Rect::from_min_max(Pos2::new(rect.left(), y - reach / 2.0), Pos2::new(rect.right(), y + reach / 2.0)),
            at: y,
            lo,
            hi,
        }
    };
    match layout {
        SplitLayout::One => Vec::new(),
        SplitLayout::Two => vec![vertical()],
        SplitLayout::Four => vec![vertical(), horizontal()],
    }
}

/// `at` with divider `divider` moved to the coordinate `coord` of `rect`.
fn moved(at: SplitAt, divider: &Divider, coord: f32, rect: Rect) -> SplitAt {
    let coord = coord.clamp(divider.lo, divider.hi);
    match divider.axis {
        Axis::X => SplitAt { x: (coord - rect.left()) / rect.width(), ..at },
        _ => SplitAt { y: (coord - rect.top()) / rect.height(), ..at },
    }
}

/// A vertical move read as the move of a value that grows downwards.
fn along_y(mv: NavMove) -> NavMove {
    match mv {
        NavMove::Up => NavMove::Down,
        NavMove::Down => NavMove::Up,
        NavMove::PageUp => NavMove::PageDown,
        NavMove::PageDown => NavMove::PageUp,
        NavMove::Forward => NavMove::Up,
        NavMove::Back => NavMove::Down,
        other => other,
    }
}

/// The style of split views: the sizes the dividers and their keys read, and the edge of the active view.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct SplitStyle {
    #[style(metrics)]
    metrics: Metrics,
    /// How thick a divider is across its boundary.
    reach: f32,
    /// The least extent of a pane along a divider's axis.
    least: f32,
    /// One arrow press.
    step: f32,
    /// One Shift-arrow or Page press.
    jump: f32,
    /// The edge of the active pane.
    active_edge: StyleColor,
    /// The width of that edge.
    edge: f32,
}

/// What split views report: the dividers' edits and the pane the user chose.
#[derive(Debug)]
#[non_exhaustive]
pub struct SplitOutput {
    /// The divider edits and the area's response.
    pub edit: EditResponse<SplitAt>,
    /// The pane clicked, or activated by key or assistive technology, this pass that was not the active one: the app
    /// makes it active.
    pub activated: Option<usize>,
}

impl Outcome for SplitOutput {
    type Value = SplitAt;

    fn response(&self) -> &Response {
        &self.edit.response
    }

    #[cfg(feature = "testing")]
    fn story_facts(&self) -> crate::testing::StoryOutput {
        self.edit.story_facts()
    }
}

/// One canvas area divided into 1, 2 or 4 panes. The active pane is the app's: it routes tools and keys there. An
/// inactive pane is a Tab stop over its view: a click, Enter or Space on it reports it in [`SplitOutput::activated`]
/// and its view gets no press.
#[derive(crate::contract::Component, crate::contract::Props)]
#[component(container)]
pub struct SplitViews<'a> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    layout: SplitLayout,
    at: Bind<'a, SplitAt>,
    #[prop]
    active: usize,
    activated: Option<usize>,
}

impl<'a> SplitViews<'a> {
    /// Split views named `label` with `layout`, the dividers at `at`. A mixed split shows no dividers. Pane 0 is
    /// active until [`active`](Self::active) says otherwise.
    pub fn new(label: impl Into<Label>, layout: SplitLayout, at: Bind<'a, SplitAt>) -> Self {
        let props = ContainerProps::new().arena(true).sizing(Sizing::Fill).pad(Pad::None);
        SplitViews { common: Common::new(), props, layout, at, active: 0, activated: None }.label(label)
    }

    /// Copies the main view's centre and zoom into the linked panes, each keeping its own rect and rotation.
    pub fn follow(main: &CanvasView, panes: &mut [SplitPane]) {
        for pane in panes.iter_mut().filter(|pane| pane.linked) {
            pane.view.center = main.center;
            pane.view.zoom = main.zoom;
        }
    }

    fn active_pane(&self) -> usize {
        self.active.min(self.layout.panes() - 1)
    }
}

/// Dragging a divider moves its boundary: one gesture from the drag's start to its release.
fn resize_by_pointer(cx: &mut Cx<'_, SplitViews<'_>>, at: SplitAt, dividers: &[Divider]) {
    let rect = cx.rect;
    for (index, divider) in dividers.iter().enumerate() {
        match cx.part(DIVIDER.at(index as u64)).press.phase {
            PressPhase::DragStart { .. } => cx.begin(),
            PressPhase::Drag { pos, .. } => {
                let coord = if divider.axis == Axis::X { pos.x } else { pos.y };
                cx.propose(moved(at, divider, coord, rect));
            }
            PressPhase::Release { .. } => cx.commit(),
            _ => {}
        }
    }
}

/// The arrow keys move the focused divider by a step, Shift or Page by a jump; assistive technology sets it to a value.
fn resize_by_keys(cx: &mut Cx<'_, SplitViews<'_>>, at: SplitAt, dividers: &[Divider]) {
    let style = cx.style;
    let Some(index) = cx.active_part().filter(|part| part.key == DIVIDER).and_then(|part| part.index) else { return };
    let Some(divider) = dividers.get(index as usize) else { return };
    let coord = if let Some(set) = cx.intents().take_set_value() {
        set.clamp(f64::from(divider.lo), f64::from(divider.hi))
    } else if let Some((mv, mods)) = cx.intents().take_move() {
        let (axes, mv) = match divider.axis {
            Axis::X => (ValueAxes::Horizontal, mv),
            _ => (ValueAxes::Vertical, along_y(mv)),
        };
        let steps =
            ValueSteps::new(f64::from(divider.at), f64::from(divider.lo), f64::from(divider.hi), f64::from(style.step), f64::from(style.jump));
        match step_value(&steps, axes, mv, mods) {
            StepResult::Set(value) => value,
            _ => return,
        }
    } else {
        return;
    };
    let next = moved(at, divider, coord as f32, cx.rect);
    if next != at {
        cx.set(next);
    }
}

impl Component for SplitViews<'_> {
    const ROLE: Role = Role::GROUP;
    const KEYS: KeyUse = KeyUse::CANVAS;
    type Style = SplitStyle;
    type Output = SplitOutput;

    fn style(cx: &StyleCx<'_>, _: SplitStyleKey) -> SplitStyle {
        let control = cx.control();
        SplitStyle {
            metrics: Metrics::new(),
            reach: cx.tokens.size.hit.target,
            least: cx.tokens.size.panel_header,
            step: control.gap,
            jump: control.height,
            active_edge: cx.color(cx.tokens.color.line.focus),
            edge: cx.tokens.stroke.thin,
        }
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn bind(&mut self) -> Option<BindMut<'_, SplitAt>> {
        Some(self.at.reborrow())
    }

    /// The panes and the dividers. An inactive pane is a click and Tab stop above its view; the active one is an inert
    /// part under its view, so every pane has an accessible node. A divider is an edge part above the panes, its own
    /// Tab stop that the arrow keys move.
    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        let at = cx.value().copied().unwrap_or_default();
        let active = self.active_pane();
        for (pane, pane_rect) in self.layout.rects(at, rect).into_iter().enumerate() {
            if pane == active {
                cx.part(PANE.at(pane as u64), pane_rect).priority(HitPriority::new(HitLevel::Background, 0));
            } else {
                cx.part(PANE.at(pane as u64), pane_rect)
                    .sense(Sense::CLICK | Sense::TAKE_FOCUS)
                    .priority(HitPriority::new(HitLevel::Body, 0))
                    .focusable()
                    .over();
            }
        }
        if cx.value().is_none() {
            return;
        }
        for (index, divider) in dividers(self.layout, at, rect, cx.style.reach, cx.style.least).iter().enumerate() {
            let cursor = if divider.axis == Axis::X { CursorIcon::ResizeHorizontal } else { CursorIcon::ResizeVertical };
            cx.part(DIVIDER.at(index as u64), divider.bar)
                .sense(Sense::DRAG | Sense::TAKE_FOCUS)
                .priority(HitPriority::new(HitLevel::Edge, 0))
                .cursor(cursor)
                .focusable()
                .over();
        }
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        if let Some(at) = cx.value().copied() {
            let dividers = dividers(self.layout, at, cx.rect, cx.style.reach, cx.style.least);
            resize_by_pointer(cx, at, &dividers);
            resize_by_keys(cx, at, &dividers);
        }
        let active = self.active_pane();
        self.activated = (0..self.layout.panes()).filter(|&pane| pane != active).find(|&pane| cx.part_activated(PANE.at(pane as u64)).is_some());
    }

    fn paint_over(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let at = cx.value().copied().unwrap_or_default();
        if cx.value().is_some() {
            for (index, divider) in dividers(self.layout, at, cx.rect, cx.style.reach, cx.style.least).iter().enumerate() {
                ui.face(DIVIDER.at(index as u64), Splitter::new(divider.axis));
            }
        }
        if self.layout.panes() > 1
            && let Some(rect) = self.layout.rects(at, cx.rect).get(self.active_pane())
        {
            ui.draw().stroke(&PaintShape::Rect(*rect, CornerRadius::ZERO), StrokeWidth::Thin, cx.style.active_edge);
        }
    }

    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
        let words = cx.words::<SplitWords>();
        for (number, pane) in self.layout.reading_order().into_iter().enumerate() {
            let number = (number + 1).to_string();
            let name = words.view.fill(&[("n", number.as_str())]);
            access.node(PANE.at(pane as u64), Role::GROUP).name = Name::Text(name);
        }
        let at = cx.value().copied().unwrap_or_default();
        let shown = if cx.value().is_some() { dividers(self.layout, at, cx.rect, cx.style.reach, cx.style.least) } else { Vec::new() };
        for (index, divider) in shown.iter().enumerate() {
            let node = access.node(DIVIDER.at(index as u64), Role::SPLITTER);
            node.name = Name::Text(words.resize.as_str().to_owned());
            node.numeric = Some(Numeric::new(f64::from(divider.at), f64::from(divider.lo)..=f64::from(divider.hi)).step(f64::from(cx.style.step)));
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> SplitOutput {
        SplitOutput { edit: cx.edit_response(), activated: self.activated }
    }
}

impl Container for SplitViews<'_> {}
