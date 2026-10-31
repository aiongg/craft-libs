#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! Input fixes egui and the platforms need before a pass: Control-click on macOS, the browser's
//! doubled touch presses, and a single-line field's drag selection.
//!
//! An app runs [`ControlClick`] and [`drop_touch_twins`] from its raw input hook, before egui
//! sees the events, and installs [`SingleLineDrag`] once with `ctx.add_plugin`. PhotoCraft runs
//! the first two in `raw_input_hook` (`crates/ui-egui/src/app.rs`) and per native viewport
//! (`panels/panel_docking.rs`); the web shell runs the touch filter in its own hook.

use egui::{Context, Event, RawInput};

/// macOS Control-click as a secondary click, as every Mac app treats it.
///
/// A primary press with Control held (and neither Alt nor Command) becomes a secondary press, and
/// its release a secondary release, even when Control is let go first. A drag that leaves the
/// window and comes back keeps its mapping; focus loss clears it, so the next plain click stays
/// primary. Other platforms pass `macos: false` and nothing changes. Keep one value per viewport.
#[derive(Clone, Debug, Default)]
pub struct ControlClick {
    /// The press that was mapped is still down.
    mapped: bool,
}

impl ControlClick {
    /// Rewrites this pass's pointer button events in place.
    // pc: crates/ui-egui/src/input/control_click.rs:8 ControlClick::normalize
    pub fn normalize(&mut self, events: &mut [Event], macos: bool) {
        todo!()
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
    todo!()
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
        todo!()
    }

    /// Rewrites `PointerMoved` and `PointerButton` positions to the recorded row.
    // pc: crates/ui-egui/src/input/single_line_drag.rs:34 input_hook
    fn input_hook(&mut self, ctx: &Context, input: &mut RawInput) {
        todo!()
    }
}
