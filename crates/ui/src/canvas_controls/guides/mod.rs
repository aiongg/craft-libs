//! `Guides`: the ruler guides of a canvas, a canvas control (20.6). Each unlocked guide is a part in the canvas's
//! arena, so a press on a guide stays with the guides and one elsewhere reaches the tool.

mod place;
#[cfg(feature = "testing")]
mod stories;

use super::words::GuideWords;
use crate::canvas::{CanvasSpace, Guide, GuideDrag, GuideKey, GuideLine, RulerText, canvas_space, readout};
use crate::contract::{
    Access, Bind, BindMut, Common, Component, Cx, Edit as _, EditResponse, Focus as _, MeasureCx, Name, Numeric, Outcome, PaintUi, PartsCx,
    Pointer as _, Props as _, Remember as _, Role, WordSetCx as _,
};
use crate::primitives::Rule;
use crate::runtime::env::{Label, UnitKind};
use crate::runtime::focus::KeyUse;
use crate::style::{ComponentStyle, FocusLook, RuleLook, StyleColor, StyleCx};
use craft_behavior::geometry::{coordinate_across, motion_across};
use craft_behavior::kurbo::{Point, Rect as DocRect};
use craft_behavior::{
    Axis, CancelReason, GesturePhase, Granularity, HitLevel, HitPriority, HitShape, Intent, Metrics, NavMove, NavShape, PartKey, PointerButton,
    PressPhase, Roving, RulerUnits, Sense, resize_cursor,
};
use craft_keymap::Mods;
use craft_theme::Rgba;
use egui::{Color32, Pos2, Response, Vec2};

/// The part of one guide: `GUIDE.at(key)` with the guide's `GuideKey::get`.
pub(super) const GUIDE: PartKey = PartKey::new("guide");

/// The least distance in points between two labelled ruler steps. Shift snaps a dragged guide to the minor ticks the
/// ruler draws at this pitch.
const RULER_STEP_MIN: f32 = 60.0;

/// The distance an arrow key moves a guide, in document units; Shift moves it by `NUDGE_FAR`.
const NUDGE: f64 = 1.0;
const NUDGE_FAR: f64 = 10.0;

/// The commands of the guides (11). They are app commands: the app owns the guide vector and the dialog, and runs
/// them with [`GuideCommands::apply`] and its own "New Guide" dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, craft_ui_macros::CommandSet)]
#[command(prefix = "canvas.guides")]
#[non_exhaustive]
pub enum GuideCommands {
    /// Open the dialog that places a guide by its numbers.
    #[command(label = "New Guide…")]
    New,
    /// Remove every guide.
    #[command(label = "Clear Guides")]
    Clear,
    /// Remove the selected guide.
    #[command(label = "Delete Guide")]
    Delete,
    /// Lock every guide: locked guides draw and take no pointer or key.
    #[command(label = "Lock Guides")]
    Lock,
    /// Unlock every guide.
    #[command(label = "Unlock Guides")]
    Unlock,
}

impl GuideCommands {
    /// Runs the command on `guides` with `selected` selected. Returns the selection the command asks for (`Some(None)`
    /// clears it), or `None` when it leaves the selection alone. `New` changes nothing here: the dialog that places
    /// the guide is the app's.
    pub fn apply(self, guides: &mut Vec<Guide>, selected: Option<GuideKey>) -> Option<Option<GuideKey>> {
        match self {
            GuideCommands::New => None,
            GuideCommands::Clear => {
                guides.clear();
                Some(None)
            }
            GuideCommands::Delete => {
                let key = selected.filter(|key| guides.iter().any(|guide| guide.key == *key && !guide.locked))?;
                *guides = place::removed(guides, key);
                Some(None)
            }
            GuideCommands::Lock | GuideCommands::Unlock => {
                let locked = self == GuideCommands::Lock;
                guides.iter_mut().for_each(|guide| guide.locked = locked);
                None
            }
        }
    }
}

/// The host's snapping of a guide (View › Snap To): the axis the guide runs along, the pointer's position across it
/// and the modifiers held, to the position to use.
pub struct GuideSnap<'a>(Box<dyn Fn(Axis, f64, Mods) -> f64 + 'a>);

impl<'a> GuideSnap<'a> {
    /// The snapping that runs `snap`.
    pub fn new(snap: impl Fn(Axis, f64, Mods) -> f64 + 'a) -> Self {
        GuideSnap(Box::new(snap))
    }
}

impl GuideSnap<'_> {
    fn apply(&self, along: Axis, position: f64, mods: Mods) -> f64 {
        (self.0)(along, position, mods)
    }
}

/// The style of the guides.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct GuideStyle {
    #[style(metrics)]
    metrics: Metrics,
    /// A guide that has no colour of its own.
    line: StyleColor,
    /// The selected guide.
    selected: StyleColor,
    /// Under the focus lines, so they read on any image.
    halo: StyleColor,
    focus: FocusLook,
    /// The width of a guide line.
    width: f32,
    /// The width of the halo of a focus line.
    halo_width: f32,
    /// From a guide's centre line to a focus line's centre line.
    focus_gap: f32,
    dash: f32,
    gap: f32,
}

/// What the guides report beyond the edit: selection, clicks and the secondary click.
#[derive(Clone, Debug, Default)]
struct Report {
    select: Option<Option<GuideKey>>,
    clicked: Option<(GuideKey, Mods, Point)>,
    double_clicked: Option<GuideKey>,
    context: Option<GuideKey>,
}

/// The guide the focus arrived at, so a move of the focus selects.
#[derive(Clone, Default)]
struct Arrival(Option<u64>);

/// The key of the guide a ruler drag is creating, kept for the length of the drag.
#[derive(Clone, Default)]
struct PendingKey(Option<GuideKey>);

/// The ruler guides of a canvas: drawn, taken by the pointer and moved with the keyboard.
///
/// - The pointer takes an unlocked guide while the guides are [`interactive`](Guides::interactive). A drag moves it;
///   the release commits where it is, or removes the guide outside the [`bounds`](Guides::bounds) or the canvas.
///   Escape cancels the drag.
/// - Shift snaps to the ruler's ticks; otherwise the host's [`snap`](Guides::snap) places the guide and gets the
///   modifiers (Ctrl turns snapping off).
/// - The guides are one Tab stop. Page Up and Page Down select the previous or next guide (vertical guides left to
///   right, then horizontal ones top to bottom), the arrow keys nudge the selected one across its axis (Shift moves
///   it by the large step) and Delete or Backspace removes it.
/// - A press on a guide selects it, so [`GuidesOutput::select`] reports the key. A press that reaches the canvas's
///   tool clears the selection; the app reads it from `CanvasOutput::press`.
/// - [`pending`](Guides::pending) shows the guide a ruler drag is creating.
#[derive(crate::contract::Component, crate::contract::Props)]
#[component(canvas_control, content_painter)]
pub struct Guides<'a> {
    #[common]
    common: Common,
    guides: Bind<'a, Vec<Guide>>,
    /// The selected guide.
    #[prop]
    selected: Option<GuideKey>,
    /// Whether the pointer and the keyboard take guides (the Move tool, or the selection tools). Off, the guides only
    /// draw.
    #[prop]
    interactive: bool,
    /// Where guides may live, in document units; a guide released outside is removed. Without bounds, the canvas.
    #[prop]
    bounds: Option<DocRect>,
    /// The host's snapping of a guide being dragged.
    #[prop(func = "Fn(Axis, f64, Mods) -> f64")]
    snap: Option<GuideSnap<'a>>,
    /// The x ruler's units, for the readout, the accessible values and the snap to ticks.
    #[prop]
    x_units: Option<RulerText>,
    /// The y ruler's units.
    #[prop]
    y_units: Option<RulerText>,
    /// The document point the rulers count from.
    #[prop]
    zero: Point,
    /// The distance an arrow key moves a guide, in document units.
    #[prop(quantity = "document units")]
    step: f64,
    /// The distance an arrow key moves a guide with Shift held, in document units.
    #[prop(quantity = "document units")]
    big_step: f64,
    /// The guide a ruler is creating, as the ruler reports it each pass.
    #[prop]
    pending: Option<Option<GuideDrag>>,
    report: Report,
}

impl<'a> Guides<'a> {
    /// The guides named `label` over `guides`, in document units.
    pub fn new(label: impl Into<Label>, guides: Bind<'a, Vec<Guide>>) -> Self {
        Guides {
            common: Common::new(),
            guides,
            selected: None,
            interactive: false,
            bounds: None,
            snap: None,
            x_units: None,
            y_units: None,
            zero: Point::ZERO,
            step: NUDGE,
            big_step: NUDGE_FAR,
            pending: None,
            report: Report::default(),
        }
        .label(label)
    }

    /// The units a guide running `along` is placed in: the x ruler's for a vertical guide.
    fn ruler(&self, along: Axis) -> RulerText {
        let units = match along {
            Axis::X => self.y_units,
            _ => self.x_units,
        };
        units.unwrap_or(RulerText::new(RulerUnits::pixels(), UnitKind::Pixels))
    }

    /// `position` snapped: to the ruler's minor ticks with Shift, otherwise by the host.
    fn snapped(&self, space: &CanvasSpace, along: Axis, position: f64, mods: Mods) -> f64 {
        if mods.shift {
            let zoom = place::zoom_across(&space.view, along);
            self.ruler(along).units.snap_to_tick(position, coordinate_across(along, self.zero), zoom, RULER_STEP_MIN)
        } else if let Some(snap) = &self.snap {
            snap.apply(along, position, mods)
        } else {
            position
        }
    }

    /// The positions a guide may be moved to with the keyboard.
    fn nudge_range(&self, space: &CanvasSpace, along: Axis) -> (f64, f64) {
        place::across_range(self.bounds.unwrap_or_else(|| space.view.visible()), along)
    }

    /// The positions a dragged guide follows the pointer to: the bounds, kept inside what the view shows so the
    /// guide stays under the pointer's reach.
    fn drag_range(&self, space: &CanvasSpace, along: Axis) -> (f64, f64) {
        let (view_lo, view_hi) = place::across_range(space.view.visible(), along);
        let Some(bounds) = self.bounds else { return (view_lo, view_hi) };
        let (lo, hi) = place::across_range(bounds, along);
        let (lo, hi) = (lo.max(view_lo.min(view_hi)), hi.min(view_hi.max(view_lo)));
        if lo <= hi { (lo, hi) } else { (view_lo, view_hi) }
    }

    fn inside_bounds(&self, along: Axis, position: f64) -> bool {
        self.bounds.is_none_or(|bounds| {
            let (lo, hi) = place::across_range(bounds, along);
            (lo..=hi).contains(&position)
        })
    }

    /// The position the pointer at `pos` gives a guide running `along`, before it is kept inside the view.
    fn pointer_position(&self, space: &CanvasSpace, along: Axis, pos: Pos2, mods: Mods) -> f64 {
        self.snapped(space, along, coordinate_across(along, space.view.to_doc(pos)), mods)
    }

    /// Shows the position of a dragged guide beside the pointer.
    fn show_readout(&self, cx: &Cx<'_, Self>, at: Pos2, along: Axis, position: f64) {
        let words = cx.words::<GuideWords>();
        let ruler = self.ruler(along);
        let digits = (*ruler.kind.decimals().end()).max(1);
        let value = ruler.format(&cx.env.units(), position, coordinate_across(along, self.zero), 0..=digits);
        let label = if along == Axis::Y { &words.x } else { &words.y };
        readout(cx, at, &[(Label::from(label.as_str().to_owned()), value)]);
    }

    /// The guide a ruler is dragging out: begins, follows and commits the new guide, or discards it.
    fn run_pending(&mut self, cx: &mut Cx<'_, Self>, space: &CanvasSpace, guides: &[Guide], drag: GuideDrag) {
        if drag.key.is_some() {
            return;
        }
        let position = drag.at.map(|at| self.snapped(space, drag.along, at, drag.mods));
        let outside = position.is_none_or(|position| !self.inside_bounds(drag.along, position));
        let stored = cx.keep::<PendingKey>().0;
        let key = stored.unwrap_or_else(|| place::fresh_key(guides));
        cx.keep::<PendingKey>().0 = Some(key);
        let shown = |position: Option<f64>| match position {
            Some(position) => place::with_guide(guides, Guide::new(key, drag.along, position)),
            None => place::removed(guides, key),
        };
        match drag.phase {
            GesturePhase::Begin => {
                cx.begin();
                cx.propose(shown(position));
            }
            GesturePhase::Update => cx.propose(shown(position)),
            GesturePhase::Commit => {
                if outside {
                    cx.cancel(CancelReason::Dismissed);
                } else {
                    cx.begin();
                    cx.propose(shown(position));
                    cx.commit();
                    self.report.select = Some(Some(key));
                }
                cx.keep::<PendingKey>().0 = None;
            }
            GesturePhase::Cancel => {
                cx.cancel(CancelReason::Escape);
                cx.keep::<PendingKey>().0 = None;
            }
        }
    }

    /// The pointer on the guides: select, drag, click, double click and the secondary click.
    fn run_pointer(&mut self, cx: &mut Cx<'_, Self>, space: &CanvasSpace, guides: &[Guide]) {
        let area = cx.rect;
        for guide in guides.iter().filter(|guide| !guide.locked) {
            let Some(result) = cx.part_if_declared(guide.key.part(GUIDE)) else { continue };
            let press = result.press;
            match press.phase {
                PressPhase::Down { start, .. } if start.button == PointerButton::Primary => {
                    self.report.select = Some(Some(guide.key));
                    if press.double_pressed() {
                        self.report.double_clicked = Some(guide.key);
                    }
                }
                PressPhase::DragStart { start, pos, .. } | PressPhase::Drag { start, pos, .. } if start.button == PointerButton::Primary => {
                    if matches!(press.phase, PressPhase::DragStart { .. }) {
                        cx.begin();
                    }
                    let (lo, hi) = self.drag_range(space, guide.along);
                    let position = place::clamped(self.pointer_position(space, guide.along, pos, press.mods), (lo, hi));
                    cx.propose(place::moved(guides, guide.key, position));
                    self.show_readout(cx, pos, guide.along, position);
                }
                PressPhase::Release { start, pos, click, .. } => match (start.button, click) {
                    (PointerButton::Primary, true) => {
                        self.report.clicked = Some((guide.key, press.mods, space.view.to_doc(pos)));
                        self.report.select = Some(Some(guide.key));
                    }
                    (PointerButton::Primary, false) => {
                        let position = self.pointer_position(space, guide.along, pos, press.mods);
                        if area.contains(pos) && self.inside_bounds(guide.along, position) {
                            let live = place::clamped(position, self.drag_range(space, guide.along));
                            cx.propose(place::moved(guides, guide.key, live));
                            self.report.select = Some(Some(guide.key));
                        } else {
                            cx.propose(place::removed(guides, guide.key));
                        }
                        cx.commit();
                    }
                    (PointerButton::Secondary, true) => self.report.context = Some(guide.key),
                    _ => {}
                },
                _ => {}
            }
        }
    }

    /// The keys of the focused guide: selection follows the focus, the arrows nudge, Delete removes.
    fn run_keys(&mut self, cx: &mut Cx<'_, Self>, space: &CanvasSpace, guides: &[Guide]) {
        let focused = cx.active_part().filter(|part| part.key == GUIDE).and_then(|part| part.index);
        let arrived = {
            let slot = cx.memory::<Arrival>();
            let changed = slot.0 != focused;
            slot.0 = focused;
            changed
        };
        let Some(guide) = focused.and_then(|index| guides.iter().find(|guide| guide.key.get() == index)) else { return };
        if arrived && self.report.select.is_none() {
            self.report.select = Some(Some(guide.key));
        }
        let nudge_range = self.nudge_range(space, guide.along);
        let mut position = guide.position;
        if let Some((direction, mods)) = cx.intents().take_nudge() {
            let step = if mods.shift { self.big_step } else { self.step };
            position += place::nudge_sign(&space.view, guide.along, direction) * step;
        }
        while let Some((mv, _)) = cx.intents().take_move() {
            match mv {
                NavMove::Forward => position += self.step,
                NavMove::Back => position -= self.step,
                _ => {}
            }
        }
        let position = place::clamped(position, nudge_range);
        if position != guide.position {
            cx.set(place::moved(guides, guide.key, position));
        }
        let forward = cx.intents().take_exact(&Intent::delete(true, Granularity::Unit));
        let backward = cx.intents().take_exact(&Intent::delete(false, Granularity::Unit));
        if forward || backward {
            let next = place::after_removal(guides, &place::keyboard_order(guides), guide.key);
            cx.set(place::removed(guides, guide.key));
            self.report.select = Some(next);
            if let Some(next) = next {
                cx.focus_part(next.part(GUIDE));
            }
        }
    }
}

/// What the guides report: the edit of the vector and what the guides ask the app to do.
#[derive(Debug)]
#[non_exhaustive]
pub struct GuidesOutput {
    /// The edit of the guide vector: a drag is one `Begin` to `Commit`; a nudge or a delete is a `Set`.
    pub edit: EditResponse<Vec<Guide>>,
    /// The selection the guides ask for: `Some(None)` deselects.
    pub select: Option<Option<GuideKey>>,
    /// A guide released where it was pressed: its key, the modifiers held (Alt-click turns a guide) and the document
    /// point.
    pub clicked: Option<(GuideKey, Mods, Point)>,
    /// A guide pressed twice, for the guide dialog.
    pub double_clicked: Option<GuideKey>,
    /// A guide clicked with the secondary button, for the app's guide menu.
    pub context: Option<GuideKey>,
}

impl Outcome for GuidesOutput {
    type Value = Vec<Guide>;

    fn response(&self) -> &Response {
        &self.edit.response
    }

    #[cfg(feature = "testing")]
    fn story_facts(&self) -> crate::testing::StoryOutput {
        self.edit.story_facts()
    }
}

fn data_color(color: Rgba) -> Color32 {
    let [r, g, b, a] = color.0;
    Color32::from_rgba_unmultiplied(r, g, b, a)
}

impl Component for Guides<'_> {
    const ROLE: Role = Role::GROUP;
    const OVERLAYS: bool = false;
    const KEYS: KeyUse = KeyUse { chrome_key: false, ..KeyUse::CANVAS.with_moves() };
    type Style = GuideStyle;
    type Output = GuidesOutput;

    fn style(cx: &StyleCx<'_>, _: GuidesStyleKey) -> GuideStyle {
        let tokens = cx.tokens;
        let focus = cx.focus_look();
        GuideStyle {
            metrics: Metrics::new(),
            line: cx.color(tokens.color.overlay.guide),
            selected: cx.color(tokens.color.accent.solid),
            halo: cx.color(tokens.color.overlay.halo),
            focus,
            width: tokens.stroke.thin,
            halo_width: focus.stroke.width + 2.0 * tokens.stroke.thin,
            focus_gap: tokens.stroke.thin / 2.0 + focus.offset + focus.stroke.width / 2.0,
            dash: tokens.stroke.dash,
            gap: tokens.stroke.dash_gap,
        }
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::ZERO
    }

    fn bind(&mut self) -> Option<BindMut<'_, Vec<Guide>>> {
        Some(self.guides.reborrow())
    }

    /// The unlocked guides in view: edge-level drag parts in one roving group, in keyboard order. Page Up and Page
    /// Down move the group's arrival; the arrows are the guide's nudge.
    fn parts(&mut self, rect: egui::Rect, cx: &mut PartsCx<'_, Self>) {
        let (Some(space), Some(guides), true) = (canvas_space(&cx.env), cx.value(), self.interactive) else { return };
        let mut any = false;
        for &at in &place::keyboard_order(guides) {
            let guide = &guides[at];
            let Some([a, b]) = place::segment(&space.view, guide.along, guide.position, guide.span, rect) else { continue };
            let part = cx
                .part(guide.key.part(GUIDE), HitShape::segment(a, b))
                .sense(Sense::DRAG | Sense::CLICK | Sense::TAKE_FOCUS)
                .priority(HitPriority::new(HitLevel::Edge, 0))
                .cursor(resize_cursor(&space.view, motion_across(guide.along)))
                .focusable();
            if self.selected == Some(guide.key) {
                part.selected();
            }
            any = true;
        }
        if any {
            cx.roving(Roving::new(NavShape::Flat).page(Some(1)));
        }
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let Some(space) = canvas_space(&cx.env) else { return };
        let guides = cx.value().cloned().unwrap_or_default();
        if let Some(Some(drag)) = self.pending {
            self.run_pending(cx, &space, &guides, drag);
        }
        if self.interactive {
            self.run_pointer(cx, &space, &guides);
            self.run_keys(cx, &space, &guides);
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let (Some(space), Some(guides)) = (canvas_space(&cx.env), cx.value()) else { return };
        let painter = ui.content_painter(cx);
        let (style, ppp) = (&cx.style, painter.pixels_per_point());
        let focused = cx.focus_visible().then(|| cx.active_part()).flatten().filter(|part| part.key == GUIDE).and_then(|part| part.index);
        for guide in guides {
            let Some([a, b]) = place::segment(&space.view, guide.along, guide.position, guide.span, cx.rect) else { continue };
            let color = if self.selected == Some(guide.key) { style.selected.egui() } else { guide.color.map_or(style.line.egui(), data_color) };
            match guide.line {
                GuideLine::Solid => painter.painter.extend(Rule::shapes(&RuleLook { color, width: style.width }, a, b, ppp)),
                GuideLine::Dashed => painter.dashed(&[a, b], style.width, style.dash, style.gap, 0.0, color),
                GuideLine::Dots => painter.dashed(&[a, b], style.width, style.width, style.gap, 0.0, color),
            }
            if focused == Some(guide.key.get()) && (b - a).length() > 0.0 {
                let along = (b - a).normalized();
                let normal = Vec2::new(-along.y, along.x) * style.focus_gap;
                for side in [normal, -normal] {
                    let halo = RuleLook { color: style.halo.egui(), width: style.halo_width };
                    painter.painter.extend(Rule::shapes(&halo, a + side, b + side, ppp));
                    painter.painter.extend(Rule::shapes(
                        &RuleLook { color: style.focus.stroke.color, width: style.focus.stroke.width },
                        a + side,
                        b + side,
                        ppp,
                    ));
                }
            }
        }
    }

    /// One node per guide, named by its axis and valued by its position in the ruler's units.
    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
        let (Some(space), Some(guides)) = (canvas_space(&cx.env), cx.value()) else { return };
        let words = cx.words::<GuideWords>();
        let units = cx.env.units();
        for guide in guides {
            let ruler = self.ruler(guide.along);
            let (lo, hi) = self.nudge_range(&space, guide.along);
            let range = lo.min(hi).min(guide.position)..=hi.max(lo).max(guide.position);
            let text = ruler.format_value(&units, guide.position, coordinate_across(guide.along, self.zero));
            let mut numeric = Numeric::new(guide.position, range).text(text);
            if self.interactive && !guide.locked {
                numeric = numeric.step(self.step);
            }
            let node = access.node(guide.key.part(GUIDE), Role::SLIDER);
            node.name = Name::Text((if guide.along == Axis::Y { &words.vertical } else { &words.horizontal }).as_str().to_owned());
            node.numeric = Some(numeric);
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> GuidesOutput {
        let Report { select, clicked, double_clicked, context } = self.report;
        GuidesOutput { edit: cx.edit_response(), select, clicked, double_clicked, context }
    }
}
