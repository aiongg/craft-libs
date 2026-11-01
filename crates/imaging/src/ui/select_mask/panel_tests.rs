//! Tests of Select and Mask's panel and workspace (task WS-SM3).

use egui::accesskit::Role;
use egui::{Color32, ColorImage, Key};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use craft_ui::canvas::{PixelRect, RasterSource};
use craft_ui::dialog::full_window::{FullWindowLabels, PreviewTicket};
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
            let labels = SelectMaskLabels::of(ui);
            let PanelApp { settings, refine, view, error, preview } = app;
            SelectMaskPanel::new("panel", settings, refine, view, &labels).error(error.as_deref()).show(ui);
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
    settings_changed: usize,
}

fn workspace_harness() -> Harness<'static, App> {
    let app = App {
        ws: SelectMaskWorkspace::new(open()),
        source: Source(Flat([400, 300])),
        tickets: Vec::new(),
        outcomes: Vec::new(),
        settings_changed: 0,
    };
    let mut h = HarnessBuilder::default().with_size([1200.0, 1400.0]).with_context_setup(setup).build_ui_state(
        |ui, app: &mut App| {
            let out = app.ws.show(ui.ctx(), &app.source);
            app.tickets.extend(out.preview_wanted);
            app.outcomes.extend(out.outcome);
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
#[ignore = "needs WS-SM2"]
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
#[ignore = "needs WS-SM2"]
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
