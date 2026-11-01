//! Tests of the Crop options bar (task TO-C5).

use egui::accesskit::Role;
use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use kurbo::Size;

use super::{
    CropBarEvent, CropBarOutput, CropDocKey, CropDocument, CropOptionsBar, CropPresetChoice, CropRatio, CropSettings, CropSizeFields, CropUserPreset,
};
use craft_ui::patterns::OptionsBar;
use craft_ui::theme::{Base, install};

struct Host {
    settings: CropSettings,
    document: CropDocument,
    users: Vec<CropUserPreset>,
    out: CropBarOutput,
    /// Every event of every pass, in order.
    events: Vec<CropBarEvent>,
}

fn user(name: &str) -> CropUserPreset {
    CropUserPreset { name: name.to_owned(), ratio: CropRatio::Sides { width: 3.0, height: 7.0 }, size: CropSizeFields::default() }
}

fn harness(settings: CropSettings, users: Vec<CropUserPreset>) -> Harness<'static, Host> {
    let document = CropDocument::new(CropDocKey(1), Size::new(2400.0, 1500.0));
    let host = Host { settings, document, users, out: CropBarOutput::default(), events: Vec::new() };
    let mut h = Harness::builder().with_size([1800.0, 600.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, host: &mut Host| {
            let Host { settings, document, users, out, events } = host;
            OptionsBar::new("options").show(ui, |bar| {
                CropOptionsBar::new("crop", settings).document(document).user_presets(users).output(out).add_to(bar);
            });
            events.extend(out.take());
        },
        host,
    );
    h.run_steps(3);
    h
}

fn sides(width: f64, height: f64) -> CropSettings {
    CropSettings { ratio: CropRatio::Sides { width, height }, ratio_fields: [width, height], ..CropSettings::default() }
}

fn open_presets(h: &mut Harness<'_, Host>) {
    h.get_by_role_and_label(Role::ComboBox, "Crop ratio").click();
    h.run_steps(3);
}

fn pick(h: &mut Harness<'_, Host>, label: &str) {
    open_presets(h);
    h.get_by_role_and_label(Role::ListBoxOption, label).click();
    h.run_steps(3);
}

// Clicking ✓, ⊘, New Crop Preset… and Delete Crop Preset… gives their events in order; a preset pick and a ratio field edit change the settings and give `Ratio`, with `editing` true while the field has focus.
#[test]
fn the_bar_reports_commit_cancel_new_and_delete_and_edits_options() {
    let mut h = harness(sides(2.0, 3.0), vec![user("Poster")]);
    h.get_by_label("Commit current crop operation").click();
    h.run_steps(3);
    h.get_by_label("Cancel current crop operation").click();
    h.run_steps(3);
    pick(&mut h, "New Crop Preset…");
    pick(&mut h, "Delete Crop Preset…");
    assert_eq!(h.state().events, vec![CropBarEvent::Commit, CropBarEvent::Cancel, CropBarEvent::NewPreset, CropBarEvent::DeletePreset]);

    h.state_mut().events.clear();
    pick(&mut h, "5 : 7");
    assert_eq!(h.state().settings.ratio, CropRatio::Sides { width: 5.0, height: 7.0 });
    assert_eq!(h.state().events, vec![CropBarEvent::Ratio { editing: false }]);
    assert_eq!(h.state().settings.choice(&h.state().users), CropPresetChoice::Ratio(3));

    // An arrow key steps the focused width field: the edit is still under way.
    h.state_mut().events.clear();
    h.get_by_role_and_label(Role::SpinButton, "Width").focus();
    h.run_steps(3);
    h.key_press_modifiers(Modifiers::NONE, Key::ArrowUp);
    h.run_steps(3);
    assert_eq!(h.state().settings.ratio, CropRatio::Sides { width: 6.0, height: 7.0 });
    assert_eq!(h.state().events, vec![CropBarEvent::Ratio { editing: true }]);

    // ⇄ swaps the sides.
    h.state_mut().events.clear();
    h.get_by_label("Swaps height and width").click();
    h.run_steps(3);
    assert_eq!(h.state().settings.ratio, CropRatio::Sides { width: 7.0, height: 6.0 });
    assert_eq!(h.state().events, vec![CropBarEvent::Ratio { editing: false }]);
}

// W x H x Resolution shows the W, H and Resolution fields with the unit select; any ratio shows the two ratio fields with ⇄; Original shows the document's size.
#[test]
fn the_bar_shows_size_fields_in_size_mode_and_ratio_fields_otherwise() {
    let size = CropSettings {
        ratio: CropRatio::Size,
        size: CropSizeFields { width: "4 in".into(), height: "5 in".into(), resolution: "300".into(), ..CropSizeFields::default() },
        ..CropSettings::default()
    };
    let h = harness(size, Vec::new());
    assert!(h.query_by_role_and_label(Role::TextInput, "Width").is_some());
    assert!(h.query_by_role_and_label(Role::TextInput, "Height").is_some());
    let resolution = h.get_by_role_and_label(Role::SpinButton, "Resolution");
    assert!(resolution.accesskit_node().value().is_some_and(|v| v.contains("300")));
    assert!(h.query_by_role_and_label(Role::ComboBox, "Resolution unit").is_some());
    assert!(h.query_by_label("Swaps height and width").is_some());

    let h = harness(sides(16.0, 9.0), Vec::new());
    assert!(h.query_by_role_and_label(Role::SpinButton, "Resolution").is_none());
    assert!(h.get_by_role_and_label(Role::SpinButton, "Width").accesskit_node().value().is_some_and(|v| v.contains("16")));
    assert!(h.get_by_role_and_label(Role::SpinButton, "Height").accesskit_node().value().is_some_and(|v| v.contains('9')));
    assert!(h.query_by_label("Swaps height and width").is_some());

    let original = CropSettings { ratio: CropRatio::Original, ..CropSettings::default() };
    let h = harness(original, Vec::new());
    let digits = |h: &Harness<'_, Host>, label: &str| {
        h.get_by_role_and_label(Role::SpinButton, label).accesskit_node().value().map(|v| v.chars().filter(char::is_ascii_digit).collect::<String>())
    };
    assert_eq!(digits(&h, "Width").as_deref(), Some("2400"));
    assert_eq!(digits(&h, "Height").as_deref(), Some("1500"));
}

// New Crop Preset… is disabled for Original and a free ratio, Delete Crop Preset… without user presets.
#[test]
fn new_and_delete_are_disabled_with_nothing_to_save_or_delete() {
    let disabled = |h: &Harness<'_, Host>, label: &str| h.get_by_role_and_label(Role::ListBoxOption, label).accesskit_node().is_disabled();

    let mut h = harness(CropSettings { ratio: CropRatio::Original, ..CropSettings::default() }, Vec::new());
    open_presets(&mut h);
    assert!(disabled(&h, "New Crop Preset…") && disabled(&h, "Delete Crop Preset…"));

    let mut h = harness(CropSettings::default(), vec![user("Poster")]);
    open_presets(&mut h);
    assert!(disabled(&h, "New Crop Preset…") && !disabled(&h, "Delete Crop Preset…"));

    let mut h = harness(sides(2.0, 3.0), Vec::new());
    open_presets(&mut h);
    assert!(!disabled(&h, "New Crop Preset…") && disabled(&h, "Delete Crop Preset…"));
}
