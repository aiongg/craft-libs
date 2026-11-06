//! Input fixes egui and the platforms need before a pass: Control-click on macOS, the browser's
//! doubled touch presses, and a single-line field's drag selection.
//!
//! An app runs [`ControlClick`] and [`drop_touch_twins`] from its raw input hook, before egui
//! sees the events, and installs [`SingleLineDrag`] once with `ctx.add_plugin`. PhotoCraft runs
//! the first two in `raw_input_hook` (`crates/ui-egui/src/app.rs`) and per native viewport
//! (`panels/panel_docking.rs`); the web shell runs the touch filter in its own hook.

use egui::{Context, Event, PointerButton, Pos2, RawInput, Rect};

/// macOS Control-click as a secondary click, as every Mac app treats it.
///
/// A primary press with Control held (and neither Alt nor Command) becomes a secondary press, and
/// its release a secondary release, even when Control is let go first. Control with Alt (the
/// brush resize gesture) stays primary. A drag that leaves the window or loses focus keeps its
/// mapping so the release still matches the press; every primary press recomputes the mapping, so
/// a missed release can't remap a later plain click. Other platforms pass `macos: false` and
/// nothing changes. Keep one value per viewport.
#[derive(Clone, Debug, Default)]
pub struct ControlClick {
    /// The press that was mapped is still down.
    mapped: bool,
}

impl ControlClick {
    /// Rewrites this pass's pointer button events in place.
    // pc: crates/ui-egui/src/input/control_click.rs:8 ControlClick::normalize
    pub fn normalize(&mut self, events: &mut [Event], macos: bool) {
        if !macos {
            return;
        }
        for event in events {
            if let Event::PointerButton { button, pressed, modifiers, .. } = event
                && *button == PointerButton::Primary
            {
                if *pressed {
                    self.mapped = modifiers.ctrl && !modifiers.alt && !modifiers.command;
                }
                if self.mapped {
                    *button = PointerButton::Secondary;
                }
                if !*pressed {
                    self.mapped = false;
                }
            }
        }
    }
}

/// Seconds within which a second primary press while the button is down is the browser's twin of
/// the first.
pub const TWIN_WINDOW: f64 = 0.5;

/// In a browser a tap sends both a pointer event and a touch event, so egui sees each press and
/// release twice. This drops a primary press that arrives while the button is already down within
/// [`TWIN_WINDOW`] of the first, and a primary release that arrives while it is already up. A
/// press held longer passes. Only the web build calls it.
// pc: crates/ui-egui/src/input/touch_twins.rs:17 drop_twins
pub fn drop_touch_twins(ctx: &Context, raw: &mut RawInput) {
    let (mut down, mut pressed_at, last) = ctx.input(|i| (i.pointer.primary_down(), i.pointer.press_start_time(), i.time));
    let now = raw.time.unwrap_or(last + f64::from(raw.predicted_dt));
    raw.events.retain(|e| {
        let Event::PointerButton { button: PointerButton::Primary, pressed, .. } = e else { return true };
        if *pressed != down {
            down = *pressed;
            pressed_at = pressed.then_some(now);
            return true;
        }
        // A press while held is a twin only while fresh; a release while up always is.
        *pressed && !pressed_at.is_some_and(|t| now - t <= TWIN_WINDOW)
    });
}

/// egui's `TextEdit` selects to the text's start when a drag goes above the first row and to its
/// end below it. In a single-line field a drag that drifts up or down then jumps. While the
/// primary button is held after a press in a focused single-line field (height at most twice
/// `interact_size.y`), this plugin pins the pointer's y to the field's text row; x is unchanged.
/// Multi-line fields keep their vertical drag.
///
/// ```ignore
/// ctx.add_plugin(SingleLineDrag::default());
/// ```
#[derive(Debug, Default)]
pub struct SingleLineDrag {
    /// The text row's y of the field being dragged in, from the last pass.
    row_y: Option<f32>,
}

impl egui::plugin::Plugin for SingleLineDrag {
    fn debug_name(&self) -> &'static str {
        "craft_ui::SingleLineDrag"
    }

    /// Records the dragged single-line field's row.
    // pc: crates/ui-egui/src/input/single_line_drag.rs:30 on_end_pass, :49 dragged_row_y, :69 row_y
    fn on_end_pass(&mut self, ui: &mut egui::Ui) {
        self.row_y = dragged_row_y(ui.ctx());
    }

    /// Rewrites `PointerMoved` and `PointerButton` positions to the recorded row.
    // pc: crates/ui-egui/src/input/single_line_drag.rs:34 input_hook
    fn input_hook(&mut self, _ctx: &Context, input: &mut RawInput) {
        let Some(row_y) = self.row_y else {
            return;
        };
        for event in &mut input.events {
            if let Event::PointerMoved(pos) | Event::PointerButton { pos, .. } = event {
                *pos = Pos2::new(pos.x, row_y);
            }
        }
    }
}

/// The y to keep the pointer on: the text row of the focused single-line field the primary button
/// was pressed in, while it is still held. Read at the end of a pass, when the field's rect is
/// this pass's.
fn dragged_row_y(ctx: &Context) -> Option<f32> {
    let (down, origin) = ctx.input(|i| (i.pointer.primary_down(), i.pointer.press_origin()));
    let origin = origin.filter(|_| down)?;
    let id = ctx.memory(|m| m.focused())?;
    egui::TextEdit::load_state(ctx, id)?;
    let rect = ctx.read_response(id)?.rect;
    if !rect.contains(origin) {
        return None;
    }
    row_y(rect, ctx.global_style().spacing.interact_size.y)
}

/// A y on the text row of a single-line field laid out in `rect`, or `None` when `rect` is too
/// tall to be one (a multi-line field, whose vertical drag must keep working). A single-line
/// `TextEdit` puts its row at the top of its rect, or centred when the field asks for it; a point
/// half a widget height below the top, or the centre of a shorter rect, is on the row in both
/// cases.
fn row_y(rect: Rect, interact: f32) -> Option<f32> {
    let (h, interact) = (rect.height(), interact.max(1.0));
    if !(h.is_finite() && interact.is_finite()) || h <= 0.0 || h > 2.0 * interact {
        return None;
    }
    Some(rect.top() + (h / 2.0).min(interact / 2.0))
}
