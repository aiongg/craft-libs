//! [`SymmetryTransform`]: Transform Symmetry, a cancellable session that moves a copy of the
//! symmetry's centre and turns its axes on the canvas tool host.

use egui::{Context, CursorIcon, Event, Key, Modifiers, Painter, Pos2};
use kurbo::{Point, Size};

use super::menu::SymmetryLabels;
use super::model::{Symmetry, TURN_STEP_DEGREES};
use super::overlay::{SymmetryOverlay, part_rect};
use craft_ui::behavior::nudge::NudgeKeys;
use craft_ui::canvas::{CanvasTool, Part, RulerUnits, ToolAction, ToolContext, ToolCursor, ToolKey, ToolParts, ToolPointer};
use craft_ui::edit::{CancelReason, EditEvent, GestureId};
use craft_ui::focus::key_owner::{KeyOwner, OwnerGate, owner};
use craft_ui::localized::Labels as _;

/// What Transform Symmetry reports.
#[derive(Clone, Debug, PartialEq)]
pub enum SymmetryEvent {
    /// The preview changed: a drag of the centre or the handle (Begin, Update, Commit; Cancel on
    /// Esc during the drag) or a key step (`Set`). The app draws the preview; nothing is stored.
    Preview(EditEvent<Symmetry>),
    /// Enter or ✓ ([`SymmetryTransform::commit`]): the app stores the symmetry as one undo step
    /// (PhotoCraft's `paint.setSymmetry`) and drops the session.
    Apply(Symmetry),
    /// Esc or ⊘ with no drag ([`SymmetryTransform::discard`]), or the session became invalid: the
    /// app drops the session.
    Discard,
}

/// One pass's input.
pub struct SymmetryInput<'a> {
    size: Size,
    valid: bool,
    units: Option<(&'a RulerUnits, &'a RulerUnits)>,
}

impl<'a> SymmetryInput<'a> {
    /// The pass over a document of `size`. `valid` is false once the document, the tool, the
    /// preset or the document's revision changed since [`SymmetryTransform::begin`], or the window
    /// lost focus: the pass then reports [`SymmetryEvent::Discard`] and takes nothing.
    pub fn new(size: Size, valid: bool) -> Self {
        Self { size, valid, units: None }
    }

    /// The ruler units of the centre's value ("x, y"); default: document pixels.
    pub fn units(mut self, x: &'a RulerUnits, y: &'a RulerUnits) -> Self {
        self.units = Some((x, y));
        self
    }
}

/// What a pass reported, from [`SymmetryTransformPass::finish`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SymmetryTransformOutput {
    /// What the pass reported, in order.
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
/// - A press within the hit screen points of the centre moves it; within the same reach of
///   the rotation handle ([`Symmetry::rotation_handle`]) turns the axes, ⇧ in absolute
///   [15°](super::TURN_STEP_DEGREES) steps; a press anywhere else does nothing and never paints
///   (the tool claims it).
/// - Cursor: Move over the centre, Alias over the handle, the arrow elsewhere; a brush outline
///   never shows.
/// - Keyboard: one Tab stop through the host with the parts "Center" (value "x, y") and "Rotation
///   handle" (value "angle°"). Arrows are read through
///   [`NudgeKeys::from_key`](craft_ui::behavior::nudge::NudgeKeys::from_key). While the parts
///   group has the keyboard the picked part takes them: the centre moves 1 unit, ⇧ 10; the
///   handle turns with ← and → 1°, ⇧ 15° (→ clockwise; nothing mirrors in a right-to-left
///   interface). Otherwise (the canvas has the keyboard) the arrows move the centre.
/// - Enter applies and Esc discards while the app owns the keys (nothing focused or the canvas
///   focused, no menu, popup or dialog open), so the session works as soon as the menu opens it,
///   before the canvas is ever focused (PhotoCraft's `track`); they also work while the canvas or
///   its parts group has the keyboard through the host. Esc during a drag ends the drag only.
/// - The options bar shows "Transform Symmetry" with ✓ and ⊘
///   ([`OptionsBar::commit_cancel`](craft_ui::patterns::OptionsBar::commit_cancel) with
///   [`SymmetryLabels`]' words); the app calls [`commit`](Self::commit) or
///   [`discard`](Self::discard) for them.
#[derive(Clone, Debug)]
pub struct SymmetryTransform {
    original: Symmetry,
    preview: Symmetry,
    drag: Option<Drag>,
    /// The running drag's gesture and the preview it began with.
    gesture: Option<(GestureId, Symmetry)>,
    /// Apply or Discard went out: the pass takes nothing more.
    ended: bool,
    /// 0 the centre, 1 the rotation handle.
    picked: Option<u64>,
    labels: Option<SymmetryLabels>,
}

impl SymmetryTransform {
    /// Opens the session on a copy of `symmetry`.
    pub fn begin(symmetry: Symmetry) -> Self {
        Self { original: symmetry, preview: symmetry, drag: None, gesture: None, ended: false, picked: None, labels: None }
    }

    /// The words; default: [`SymmetryLabels`] of the installed words.
    pub fn labels(&mut self, labels: SymmetryLabels) -> &mut Self {
        self.labels = Some(labels);
        self
    }

    /// The symmetry the session began with (the app compares it to the preset's for `valid`).
    pub fn original(&self) -> Symmetry {
        self.original
    }

    /// The edited copy, for [`SymmetryOverlay`](super::SymmetryOverlay).
    pub fn preview(&self) -> Symmetry {
        self.preview
    }

    /// The picked part (0 the centre, 1 the rotation handle).
    pub fn picked(&self) -> Option<u64> {
        self.picked
    }

    /// Enter or ✓: ends the session and answers [`SymmetryEvent::Apply`] of the preview. The app
    /// stores it and drops the session.
    pub fn commit(&mut self) -> SymmetryEvent {
        self.ended = true;
        self.drag = None;
        self.gesture = None;
        SymmetryEvent::Apply(self.preview)
    }

    /// Esc or ⊘: ends the session, puts back the preview a running drag began on and answers
    /// [`SymmetryEvent::Discard`]. The app drops the session.
    pub fn discard(&mut self) -> SymmetryEvent {
        self.ended = true;
        self.drag = None;
        if let Some((_, before)) = self.gesture.take() {
            self.preview = before;
        }
        SymmetryEvent::Discard
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
        let style = SymmetryOverlay::style(cx.tokens);
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
        self.session.labels.clone().unwrap_or_else(|| SymmetryLabels::of_ctx(&cx.canvas.ctx))
    }

    /// The preview after a key step, or `None` for a key it does not read. With the parts group
    /// focused the picked part takes the step; otherwise the arrows move the centre.
    fn step(&self, key: &ToolKey, parts_focused: bool) -> Option<Symmetry> {
        let nudge = NudgeKeys::new().from_key(key)?;
        let p = self.session.preview;
        match self.session.picked.filter(|_| parts_focused).unwrap_or(CENTER) {
            CENTER => Some(Symmetry::new(p.mode, p.center + nudge.by, p.rotation).unwrap_or(p)),
            _ => {
                // Right is clockwise in every language: the canvas does not mirror.
                if nudge.by.x == 0.0 {
                    return None;
                }
                let step = if nudge.shift { TURN_STEP_DEGREES } else { 1.0 };
                Some(Symmetry::new(p.mode, p.center, p.rotation + step * nudge.by.x.signum()).unwrap_or(p))
            }
        }
    }

    /// Enter and Esc at the context level while the app owns the keys (PhotoCraft `track`): the
    /// session opens from a menu, so the canvas has no focus. Nothing is read during a drag
    /// (Esc then ends the drag in the host) or while a menu, popup, dialog or text field has the
    /// keyboard.
    fn read_end_keys(&mut self, ctx: &Context) {
        // pc: crates/ui-egui/src/tools/symmetry_ui.rs:131 `track` (Enter, Esc through `consume_key`).
        if self.session.drag.is_some() || owner(ctx, &OwnerGate::default()) != KeyOwner::App {
            return;
        }
        let pressed = ctx.input_mut(|i| {
            let at = i.events.iter().position(|e| {
                matches!(e, Event::Key { key: Key::Enter | Key::Escape, pressed: true, modifiers, .. } if modifiers.matches_exact(Modifiers::NONE))
            })?;
            match i.events.remove(at) {
                Event::Key { key, .. } => Some(key),
                _ => None,
            }
        });
        match pressed {
            Some(Key::Enter) => self.end(true),
            Some(_) => self.end(false),
            None => {}
        }
    }

    /// Ends the session with [`SymmetryTransform::commit`] or [`SymmetryTransform::discard`].
    fn end(&mut self, apply: bool) {
        let event = if apply { self.session.commit() } else { self.session.discard() };
        self.push(event);
    }
}

impl CanvasTool for SymmetryTransformPass<'_> {
    /// An invalid session reports Discard once; a valid one reads Enter and Esc while the app owns
    /// the keys.
    fn each_pass(&mut self, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/symmetry_ui.rs:131 `track`, :124 `valid`.
        if self.session.ended {
            return;
        }
        if self.input.valid {
            self.read_end_keys(&cx.canvas.ctx);
            return;
        }
        let gesture = self.session.gesture;
        let event = self.session.discard();
        if let Some((id, _)) = gesture {
            self.push(SymmetryEvent::Preview(EditEvent::cancel(id, CancelReason::DocumentSwitched)));
        }
        self.push(event);
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

    fn cancel(&mut self, reason: CancelReason, _cx: &mut ToolContext<'_>) {
        // new: the drag's Cancel; the preview goes back to the press.
        self.session.drag = None;
        if let Some((id, before)) = self.session.gesture.take() {
            self.session.preview = before;
            self.push(SymmetryEvent::Preview(EditEvent::cancel(id, reason)));
        }
    }

    /// Enter applies, Esc discards (while the canvas or its parts group has the keyboard; the
    /// context-level read of [`each_pass`](CanvasTool::each_pass) covers the rest); arrows move
    /// the centre, or act on the picked part while the parts group has the keyboard.
    fn key(&mut self, key: &ToolKey, cx: &mut ToolContext<'_>) -> bool {
        // pc: crates/ui-egui/src/tools/symmetry_ui.rs:131 `track` (Enter, Esc); the arrows are new
        // (lead call 24).
        if !key.pressed || self.session.ended || self.session.drag.is_some() {
            return false;
        }
        if key.modifiers.matches_exact(Modifiers::NONE) && matches!(key.key, Key::Enter | Key::Escape) {
            self.end(key.key == Key::Enter);
            return true;
        }
        let Some(next) = self.step(key, cx.parts_focused) else { return false };
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
        let style = SymmetryOverlay::style(cx.tokens);
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
            selected: self.session.picked == Some(key),
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
