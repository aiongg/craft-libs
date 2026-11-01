//! [`SymmetryTransform`]: Transform Symmetry, a cancellable session that moves a copy of the
//! symmetry's centre and turns its axes on the canvas tool host.

use egui::{CursorIcon, Key, Modifiers, Painter, Pos2};
use kurbo::{Point, Size};

use super::menu::SymmetryLabels;
use super::model::{Symmetry, TURN_STEP_DEGREES};
use super::overlay::part_rect;
use super::style::SymmetryStyle;
use craft_ui::canvas::{CanvasTool, Part, RulerUnits, ToolAction, ToolContext, ToolCursor, ToolKey, ToolParts, ToolPointer};
use craft_ui::edit::{CancelReason, EditEvent, GestureId};
use craft_ui::localized::Labels as _;

/// What Transform Symmetry reports.
#[derive(Clone, Debug, PartialEq)]
pub enum SymmetryEvent {
    /// The preview changed: a drag of the centre or the handle (Begin, Update, Commit; Cancel on
    /// Esc during the drag) or a key step (`Set`). The app draws the preview; nothing is stored.
    Preview(EditEvent<Symmetry>),
    /// Enter or ✓: the app stores the symmetry as one undo step (PhotoCraft's `paint.setSymmetry`)
    /// and drops the session.
    Commit(Symmetry),
    /// Esc or ⊘ with no drag, or the session became invalid: the app drops the session.
    Cancel,
}

/// One pass's input.
pub struct SymmetryInput<'a> {
    size: Size,
    valid: bool,
    units: Option<(&'a RulerUnits, &'a RulerUnits)>,
    labels: Option<SymmetryLabels>,
}

impl<'a> SymmetryInput<'a> {
    /// The pass over a document of `size`. `valid` is false once the document, the tool, the
    /// preset or the document's revision changed since [`SymmetryTransform::begin`], or the window
    /// lost focus: the pass then reports [`SymmetryEvent::Cancel`] and takes nothing.
    pub fn new(size: Size, valid: bool) -> Self {
        Self { size, valid, units: None, labels: None }
    }

    /// The ruler units of the centre's value ("x, y"); default: document pixels.
    pub fn units(mut self, x: &'a RulerUnits, y: &'a RulerUnits) -> Self {
        self.units = Some((x, y));
        self
    }

    /// The words; default: [`SymmetryLabels`] of the installed words.
    pub fn labels(mut self, labels: SymmetryLabels) -> Self {
        self.labels = Some(labels);
        self
    }
}

/// What a pass reported, from [`SymmetryTransformPass::finish`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SymmetryTransformOutput {
    pub events: Vec<SymmetryEvent>,
}

/// A drag of the session.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Drag {
    /// Moves the centre: the press and the centre then.
    Move { start: Point, center: Point },
    /// Turns the axes: the pointer's angle from the centre at the press (radians) and the rotation
    /// then.
    Turn { angle: f64, rotation: f64 },
}

/// Transform Symmetry, kept by the app while it is open (the menu's Transform Symmetry).
///
/// - A press within `symmetry.hit` screen points of the centre moves it; within the same reach of
///   the rotation handle ([`Symmetry::rotation_handle`]) turns the axes, ⇧ in absolute
///   [15°](super::TURN_STEP_DEGREES) steps; a press anywhere else does nothing and never paints
///   (the tool claims it).
/// - Cursor: Move over the centre, Alias over the handle, the arrow elsewhere; a brush outline
///   never shows.
/// - Keyboard: one Tab stop through the host with the parts "Center" (value "x, y") and "Rotation
///   handle" (value "angle°"); with the centre picked the arrows move it 1 unit, ⇧ 10; with the
///   handle picked ← and → turn 1°, ⇧ 15° (→ clockwise; nothing mirrors in a right-to-left
///   interface). Enter commits, Esc cancels.
/// - The options bar shows "Transform Symmetry" with ✓ and ⊘
///   ([`OptionsBar::commit_cancel`](craft_ui::patterns::OptionsBar::commit_cancel) with
///   [`SymmetryLabels`]' words); the app calls [`commit`](Self::commit) or
///   [`cancel`](Self::cancel) for them.
#[derive(Clone, Debug)]
pub struct SymmetryTransform {
    original: Symmetry,
    preview: Symmetry,
    drag: Option<Drag>,
    /// The running drag's gesture and the preview it began with.
    gesture: Option<(GestureId, Symmetry)>,
    /// Commit or Cancel went out: the pass takes nothing more.
    ended: bool,
    /// 0 the centre, 1 the rotation handle.
    picked: Option<u64>,
}

impl SymmetryTransform {
    /// Opens the session on a copy of `symmetry`.
    pub fn begin(symmetry: Symmetry) -> Self {
        Self { original: symmetry, preview: symmetry, drag: None, gesture: None, ended: false, picked: None }
    }

    /// The symmetry the session began with (the app compares it to the preset's for `valid`).
    pub fn original(&self) -> Symmetry {
        self.original
    }

    /// The edited copy, for [`SymmetryOverlay`](super::SymmetryOverlay).
    pub fn preview(&self) -> Symmetry {
        self.preview
    }

    /// The picked part, for [`SymmetryOverlay::ring`](super::SymmetryOverlay::ring).
    pub fn picked(&self) -> Option<u64> {
        self.picked
    }

    /// ✓: [`SymmetryEvent::Commit`] of the preview.
    pub fn commit(&self) -> SymmetryEvent {
        SymmetryEvent::Commit(self.preview)
    }

    /// ⊘: [`SymmetryEvent::Cancel`].
    pub fn cancel(&self) -> SymmetryEvent {
        SymmetryEvent::Cancel
    }

    /// This pass's tool for the host. The app passes it instead of the painting tool while the
    /// session is open.
    pub fn pass<'a>(&'a mut self, input: SymmetryInput<'a>) -> SymmetryTransformPass<'a> {
        SymmetryTransformPass { session: self, input, output: SymmetryTransformOutput::default() }
    }
}

/// Transform Symmetry for one pass of the host ([`SymmetryTransform::pass`]).
pub struct SymmetryTransformPass<'a> {
    session: &'a mut SymmetryTransform,
    input: SymmetryInput<'a>,
    output: SymmetryTransformOutput,
}

const CENTER: u64 = 0;
const HANDLE: u64 = 1;

impl SymmetryTransformPass<'_> {
    /// The pass's events.
    pub fn finish(self) -> SymmetryTransformOutput {
        self.output
    }

    fn push(&mut self, event: SymmetryEvent) {
        self.output.events.push(event);
    }

    /// The part under a screen position: the centre first, then the rotation handle.
    fn part_at(&self, cx: &ToolContext<'_>, at: Pos2) -> Option<u64> {
        let style = SymmetryStyle::of(cx.tokens);
        let preview = &self.session.preview;
        let handle = preview.rotation_handle(self.input.size, style.handle_share);
        [(CENTER, preview.center), (HANDLE, handle)].into_iter().find(|(_, p)| cx.view.to_screen(*p).distance(at) <= style.hit).map(|(key, _)| key)
    }

    /// Replaces the preview with `next` unless `Symmetry::new` refuses it; true when it changed.
    fn set(&mut self, center: Point, rotation: f64) -> Option<Symmetry> {
        let next = Symmetry::new(self.session.preview.mode, center, rotation)?;
        (next != self.session.preview).then(|| {
            self.session.preview = next;
            next
        })
    }

    fn labels(&self, cx: &ToolContext<'_>) -> SymmetryLabels {
        self.input.labels.clone().unwrap_or_else(|| SymmetryLabels::of_ctx(&cx.canvas.ctx))
    }

    /// The preview after a key step on the picked part, or `None` for a key it does not read.
    fn step(&self, key: &ToolKey) -> Option<Symmetry> {
        let big = if key.modifiers.matches_exact(Modifiers::SHIFT) {
            true
        } else if key.modifiers.matches_exact(Modifiers::NONE) {
            false
        } else {
            return None;
        };
        let p = self.session.preview;
        match self.session.picked? {
            CENTER => {
                let step = if big { 10.0 } else { 1.0 };
                let (dx, dy) = match key.key {
                    Key::ArrowLeft => (-step, 0.0),
                    Key::ArrowRight => (step, 0.0),
                    Key::ArrowUp => (0.0, -step),
                    Key::ArrowDown => (0.0, step),
                    _ => return None,
                };
                Some(Symmetry::new(p.mode, Point::new(p.center.x + dx, p.center.y + dy), p.rotation).unwrap_or(p))
            }
            _ => {
                let step = if big { TURN_STEP_DEGREES } else { 1.0 };
                // Right is clockwise in every language: the canvas does not mirror.
                let turn = match key.key {
                    Key::ArrowLeft => -step,
                    Key::ArrowRight => step,
                    _ => return None,
                };
                Some(Symmetry::new(p.mode, p.center, p.rotation + turn).unwrap_or(p))
            }
        }
    }
}

impl CanvasTool for SymmetryTransformPass<'_> {
    /// An invalid session reports Cancel once.
    fn each_pass(&mut self, _cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/symmetry_ui.rs:131 `track` (the validity half), :124 `valid`.
        if self.input.valid || self.session.ended {
            return;
        }
        self.session.ended = true;
        self.session.drag = None;
        if let Some((id, before)) = self.session.gesture.take() {
            self.session.preview = before;
            self.push(SymmetryEvent::Preview(EditEvent::cancel(id, CancelReason::DocumentSwitched)));
        }
        self.push(SymmetryEvent::Cancel);
    }

    fn cursor(&self, cx: &ToolContext<'_>) -> ToolCursor {
        // pc: crates/ui-egui/src/tools/symmetry_ui.rs:111 `cursor`.
        let icon = match self.session.drag {
            Some(Drag::Move { .. }) => CursorIcon::Move,
            Some(Drag::Turn { .. }) => CursorIcon::Alias,
            None => match cx.pointer.filter(|_| !self.session.ended).and_then(|p| self.part_at(cx, p.screen)) {
                Some(CENTER) => CursorIcon::Move,
                Some(_) => CursorIcon::Alias,
                None => CursorIcon::Default,
            },
        };
        ToolCursor::System(icon)
    }

    /// Takes every press: the centre, the handle, or nothing (claimed, so nothing paints).
    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        // pc: crates/ui-egui/src/tools/symmetry_ui.rs:233 `pointer` (Down).
        if self.session.ended {
            return ToolAction::Ignored;
        }
        let preview = self.session.preview;
        let drag = match self.part_at(cx, pointer.screen) {
            Some(CENTER) => Drag::Move { start: pointer.doc, center: preview.center },
            Some(_) => {
                let from = pointer.doc - preview.center;
                Drag::Turn { angle: from.y.atan2(from.x), rotation: preview.rotation }
            }
            None => return ToolAction::Claimed,
        };
        self.session.picked = Some(if matches!(drag, Drag::Move { .. }) { CENTER } else { HANDLE });
        self.session.drag = Some(drag);
        let id = GestureId::next();
        self.session.gesture = Some((id, preview));
        self.push(SymmetryEvent::Preview(EditEvent::begin(id)));
        ToolAction::Claimed
    }

    fn drag(&mut self, pointer: &ToolPointer, _cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/symmetry_ui.rs:233 `pointer` (Move; ⇧ rounds the rotation to
        // 15°; a value `Symmetry::new` refuses keeps the last preview).
        let Some(drag) = self.session.drag else { return };
        let preview = self.session.preview;
        let (center, rotation) = match drag {
            Drag::Move { start, center } => (center + (pointer.doc - start), preview.rotation),
            Drag::Turn { angle, rotation } => {
                let from = pointer.doc - preview.center;
                let mut turned = rotation + (from.y.atan2(from.x) - angle).to_degrees();
                if pointer.modifiers.shift {
                    turned = (turned / TURN_STEP_DEGREES).round() * TURN_STEP_DEGREES;
                }
                (preview.center, turned)
            }
        };
        if let (Some(next), Some((id, _))) = (self.set(center, rotation), self.session.gesture) {
            self.push(SymmetryEvent::Preview(EditEvent::update(id, next, pointer.modifiers)));
        }
    }

    fn release(&mut self, pointer: &ToolPointer, _cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/symmetry_ui.rs:233 `pointer` (Up).
        self.session.drag = None;
        if let Some((id, before)) = self.session.gesture.take() {
            let preview = self.session.preview;
            self.push(SymmetryEvent::Preview(EditEvent::commit(id, preview, preview != before, pointer.modifiers)));
        }
    }

    fn cancel(&mut self, _cx: &mut ToolContext<'_>) {
        // new: the drag's Cancel; the preview goes back to the press.
        self.session.drag = None;
        if let Some((id, before)) = self.session.gesture.take() {
            self.session.preview = before;
            self.push(SymmetryEvent::Preview(EditEvent::cancel(id, CancelReason::Escape)));
        }
    }

    /// Enter commits, Esc cancels; arrows act on the picked part.
    fn key(&mut self, key: &ToolKey, _cx: &mut ToolContext<'_>) -> bool {
        // pc: crates/ui-egui/src/tools/symmetry_ui.rs:131 `track` (Enter, Esc); the arrows are new
        // (lead call 24).
        if !key.pressed || self.session.ended || self.session.drag.is_some() {
            return false;
        }
        if key.modifiers.matches_exact(Modifiers::NONE) {
            match key.key {
                Key::Enter => {
                    self.session.ended = true;
                    let event = self.session.commit();
                    self.push(event);
                    return true;
                }
                Key::Escape => {
                    self.session.ended = true;
                    let event = self.session.cancel();
                    self.push(event);
                    return true;
                }
                _ => {}
            }
        }
        let Some(next) = self.step(key) else { return false };
        if next != self.session.preview {
            self.session.preview = next;
            self.push(SymmetryEvent::Preview(EditEvent::set(next, key.modifiers)));
        }
        true
    }

    /// Nothing: the app draws [`SymmetryOverlay`](super::SymmetryOverlay) with `editing` on, so
    /// the guides show whichever tool is picked.
    fn paint(&self, _painter: &Painter, _cx: &ToolContext<'_>) {}

    fn parts(&self, cx: &ToolContext<'_>) -> Option<ToolParts> {
        // new: lead call 33; the group "Symmetry" with "Center" and "Rotation handle".
        let style = SymmetryStyle::of(cx.tokens);
        let labels = self.labels(cx);
        let preview = &self.session.preview;
        let (x, y) = match self.input.units {
            Some((x, y)) => (x.clone(), y.clone()),
            None => {
                let pixels = RulerUnits::pixels(cx.numbers.clone());
                (pixels.clone(), pixels)
            }
        };
        let center = format!("{}, {}", x.format_value(preview.center.x, 0.0), y.format_value(preview.center.y, 0.0));
        let angle = format!("{}\u{b0}", cx.numbers.format(preview.rotation, 0..=1));
        let handle = preview.rotation_handle(self.input.size, style.handle_share);
        let part = |key, name: String, value, at: Point| Part {
            key,
            name,
            value: Some(value),
            rect: part_rect(cx.view.to_screen(at), &style),
            selected: false,
        };
        Some(ToolParts::new(
            labels.group.clone(),
            vec![part(CENTER, labels.center.clone(), center, preview.center), part(HANDLE, labels.rotation_handle.clone(), angle, handle)],
        ))
    }

    fn picked(&self) -> Option<u64> {
        self.session.picked
    }

    fn pick(&mut self, key: Option<u64>, _cx: &mut ToolContext<'_>) {
        self.session.picked = key.filter(|k| *k < 2);
    }
}
