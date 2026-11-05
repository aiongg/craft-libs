//! Tests of Select and Mask's panel and workspace (task WS-SM3).

use egui::accesskit::Role;
use egui::{Color32, ColorImage, Key, Modifiers};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use craft_ui::canvas::{PixelRect, RasterSource};
use craft_ui::dialog::full_window::{FullWindowLabels, PreviewTicket};
use craft_ui::edit::{CancelReason, EditEvent};
use craft_ui::localized::Labels as _;
use craft_ui::theme::{Base, install};

use super::*;
use crate::ui::mask_view::MaskViewMode;

/// A flat grey picture: the workspace tests need pixels, not the story image.
struct Flat([u32; 2]);

impl RasterSource for Flat {
    fn size(&self) -> [u32; 2] {
        self.0
    }

    fn revision(&self) -> u64 {
        1
    }

    fn read(&self, _region: PixelRect, _factor: u32, out: &mut ColorImage) {
        out.pixels.fill(Color32::GRAY);
    }
}

struct Source(Flat);

impl SelectMaskSource for Source {
    fn image(&self) -> &dyn RasterSource {
        &self.0
    }

    fn view(&self) -> &dyn RasterSource {
        &self.0
    }

    fn ants(&self) -> Option<(&dyn MaskSource, u32)> {
        None
    }
}

fn open() -> SelectMaskOpen {
    SelectMaskOpen::new([400, 300], "Photo", SelectMaskLimits { edits: 4096, edit_points: 100_000, strokes: 4096, stroke_points: 100_000 })
}

fn setup(ctx: &egui::Context) {
    install(ctx, &Base::suite_default());
}

// The panel alone, in a column.
struct PanelApp {
    settings: SelectMaskSettings,
    refine: RefineSettings,
    view: MaskViewState,
    error: Option<String>,
    preview: PreviewStatus,
}

fn panel_harness(error: Option<&str>, preview: PreviewStatus) -> Harness<'static, PanelApp> {
    let settings = SelectMaskSettings::default();
    let app = PanelApp { refine: settings.refine.clone(), view: settings.view_state(), settings, error: error.map(str::to_owned), preview };
    let mut h = HarnessBuilder::default().with_size([420.0, 1000.0]).with_context_setup(setup).build_ui_state(
        |ui, app: &mut PanelApp| {
            let PanelApp { settings, refine, view, error, preview } = app;
            SelectMaskPanel::new("panel", settings, refine, view).error(error.as_deref()).show(ui);
            preview.show(ui, &FullWindowLabels::of(ui));
        },
        app,
    );
    h.run_steps(3);
    h
}

// Object Aware is disabled and says why; Refine Hair, the bar's, has the same rule in the bar's
// own tests.
#[test]
fn select_mask_greyed_controls_explain_themselves() {
    let h = panel_harness(None, PreviewStatus::new());
    let button = h.get_by_label("Object Aware");
    assert!(button.accesskit_node().is_disabled());
    assert_eq!(button.accesskit_node().description().as_deref(), Some("Object Aware needs a segmentation model, which isn't available yet"));
    assert!(!h.get_by_label("Color Aware").accesskit_node().is_disabled());
}

// A result for an older ticket is dropped; the newest shows; a failure shows its message under the
// panel next to the limit message.
#[test]
fn select_mask_preview_tickets_drop_stale_results() {
    let mut ws = SelectMaskWorkspace::new(open());
    let (first, second): (PreviewTicket, PreviewTicket) = (ws.preview.request(), ws.preview.request());
    ws.preview_ready(first);
    assert!(ws.preview.pending(), "the older ticket does not settle the newer request");
    ws.preview_ready(second);
    assert!(!ws.preview.pending());

    let (older, newer) = (ws.preview.request(), ws.preview.request());
    ws.preview_failed(older, "stale".into());
    assert_eq!(ws.preview.error(), None, "a failure of an older ticket is dropped");
    ws.preview_failed(newer, "engine out of memory".into());
    assert_eq!(ws.preview.error(), Some("engine out of memory"));

    let h = panel_harness(Some("at most 4096 edits: undo some, or reset"), ws.preview.clone());
    assert!(h.query_by_label("Preview failed: engine out of memory").is_some());
    assert!(h.query_by_label("at most 4096 edits: undo some, or reset").is_some());
}

// Decontaminate Colors moves Output To off Selection and Layer Mask; Amount follows the check box.
#[test]
fn select_mask_decontaminate_moves_the_output_target() {
    let mut h = panel_harness(None, PreviewStatus::new());
    assert!(h.get_by_role_and_label(Role::Slider, "Amount").accesskit_node().is_disabled());
    h.get_by_label("Decontaminate Colors").click();
    h.run_steps(3);
    let refine = &h.state().refine;
    assert!(refine.decontaminate);
    assert_eq!(refine.output, OutputTo::NewLayerWithMask);
    assert!(!h.get_by_role_and_label(Role::Slider, "Amount").accesskit_node().is_disabled());
}

// The workspace in a window.
struct App {
    ws: SelectMaskWorkspace,
    source: Source,
    tickets: Vec<PreviewTicket>,
    outcomes: Vec<Outcome>,
    events: Vec<SelectMaskEvent>,
    settings_changed: usize,
}

fn workspace_harness() -> Harness<'static, App> {
    let app = App {
        ws: SelectMaskWorkspace::new(open()),
        source: Source(Flat([400, 300])),
        tickets: Vec::new(),
        outcomes: Vec::new(),
        events: Vec::new(),
        settings_changed: 0,
    };
    let mut h = HarnessBuilder::default().with_size([1200.0, 1400.0]).with_context_setup(setup).build_ui_state(
        |ui, app: &mut App| {
            let out = app.ws.show(ui.ctx(), &app.source);
            app.tickets.extend(out.preview_wanted);
            app.outcomes.extend(out.outcome);
            app.events.extend(out.events);
            app.settings_changed += usize::from(out.settings_changed);
        },
        app,
    );
    h.run_steps(3);
    h
}

// O M V A T K Y pick the views, J and P toggle, and a view change asks for a preview; the settings
// carry the view and Remember Settings.
#[test]
fn select_mask_view_keys_and_remember_settings() {
    let mut h = workspace_harness();
    assert_eq!(h.state().tickets.len(), 1, "the first pass asks for a preview");
    for mode in MaskViewMode::ALL {
        let asked = h.state().tickets.len();
        h.key_press(mode.key());
        h.run_steps(2);
        assert_eq!(h.state().ws.recipe().mode, mode);
        assert_eq!(h.state().ws.settings().view, mode);
        assert!(h.state().tickets.len() > asked, "{mode:?} asks for a preview");
    }
    let asked = h.state().tickets.len();
    h.key_press(Key::J);
    h.run_steps(2);
    assert!(h.state().ws.settings().show_edge);
    assert!(h.state().tickets.len() > asked);

    h.get_by_label("Remember Settings").click();
    h.run_steps(2);
    assert!(h.state().ws.settings().remember);
    assert!(h.state().settings_changed > 0);
}

// OK reports `Outcome::Ok` once; Cancel reports `Outcome::Cancel` and the session is unchanged.
#[test]
fn select_mask_ok_is_one_command_and_cancel_changes_nothing() {
    let mut h = workspace_harness();
    h.get_by_label("Clear Selection").click();
    h.run_steps(3);
    assert_eq!(h.state().ws.session().edits, [SelectMaskEdit::Clear]);
    let session = h.state().ws.session().clone();

    h.get_by_label("Cancel").click();
    h.run_steps(3);
    assert_eq!(h.state().outcomes, [Outcome::Cancel]);
    assert_eq!(h.state().ws.session(), &session);

    let mut h = workspace_harness();
    h.get_by_label("Invert").click();
    h.run_steps(3);
    h.get_by_label("OK").click();
    h.run_steps(3);
    assert_eq!(h.state().outcomes, [Outcome::Ok]);
    assert_eq!(h.state().ws.command().edits, [SelectMaskEdit::Invert]);
}

/// Presses the primary button over the canvas and drags a little: a Quick Selection stroke is open.
fn start_stroke(h: &mut Harness<'static, App>) {
    let at = egui::pos2(450.0, 750.0);
    h.event(egui::Event::PointerMoved(at));
    h.step();
    h.event(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.step();
    h.event(egui::Event::PointerMoved(at + egui::vec2(40.0, 10.0)));
    h.run_steps(2);
    assert!(h.state().ws.canvas.busy(), "the stroke is open");
}

// A tool letter pressed mid-stroke ends the stroke: the old tool reports `Cancel` (Removed), the
// canvas is idle and the history no longer merges.
#[test]
fn select_mask_tool_switch_mid_stroke_cancels_the_stroke() {
    let mut h = workspace_harness();
    start_stroke(&mut h);
    h.key_press(Key::R);
    h.run_steps(3);
    assert_eq!(h.state().ws.tool, SelectMaskTool::RefineEdge);
    assert!(!h.state().ws.canvas.busy());
    let cancelled = h.state().events.iter().any(|e| matches!(e, SelectMaskEvent::Edit(EditEvent::Cancel { reason: CancelReason::Removed, .. })));
    assert!(cancelled, "{:?}", h.state().events);
    assert!(h.state().ws.session().edits.is_empty());
}

// ⌘Z mid-stroke cancels the stroke with `UndoDuringGesture` before the history steps.
#[test]
fn select_mask_undo_mid_stroke_cancels_the_stroke() {
    let mut h = workspace_harness();
    start_stroke(&mut h);
    h.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    h.run_steps(3);
    assert!(!h.state().ws.canvas.busy());
    let cancelled =
        h.state().events.iter().any(|e| matches!(e, SelectMaskEvent::Edit(EditEvent::Cancel { reason: CancelReason::UndoDuringGesture, .. })));
    assert!(cancelled, "{:?}", h.state().events);
}

// Holding X hides the view and asks for no preview; Sample All Layers asks for one, + and − do not.
#[test]
fn select_mask_preview_is_asked_for_only_when_it_changes() {
    let mut h = workspace_harness();
    let asked = h.state().tickets.len();
    h.key_down(Key::X);
    h.run_steps(3);
    assert!(h.state().ws.recipe().hidden);
    h.key_up(Key::X);
    h.run_steps(3);
    assert!(!h.state().ws.recipe().hidden);
    assert_eq!(h.state().tickets.len(), asked, "X shows the image without a job");

    h.get_by_label("Subtract").click();
    h.run_steps(3);
    assert!(!h.state().ws.options.add);
    assert_eq!(h.state().tickets.len(), asked, "the mode is not a preview option");

    h.get_by_label("Sample All Layers").click();
    h.run_steps(3);
    assert!(h.state().ws.options.sample_all_layers);
    assert_eq!(h.state().tickets.len(), asked + 1);
}
