//! Tests of the Crop options bar.

use egui::accesskit::Role;
use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use kurbo::Size;

use super::{
    CropBar, CropBarOutput, CropDocKey, CropDocument, CropOptionsMenu, CropPresetChoice, CropRatio, CropRatioChange, CropSettings, CropSizeFields,
    CropUserPreset, ShieldColor,
};
use craft_ui::patterns::OptionsBar;
use craft_ui::theme::{Base, install};

struct Host {
    settings: CropSettings,
    document: CropDocument,
    users: Vec<CropUserPreset>,
    out: CropBarOutput,
    /// Every pass's output that reported anything, in order.
    events: Vec<CropBarOutput>,
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
                CropBar::new("crop", settings).document(document).user_presets(users).output(out).add_to(bar);
            });
            let out = std::mem::take(out);
            if out != CropBarOutput::default() {
                events.push(out);
            }
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
    let ends: Vec<_> = h.state().events.iter().map(|o| (o.apply, o.discard, o.new_preset, o.delete_preset, o.settings_changed)).collect();
    assert_eq!(
        ends,
        vec![
            (true, false, false, false, false),
            (false, true, false, false, false),
            (false, false, true, false, false),
            (false, false, false, true, false)
        ]
    );

    h.state_mut().events.clear();
    pick(&mut h, "5 : 7");
    assert_eq!(h.state().settings.ratio, CropRatio::Sides { width: 5.0, height: 7.0 });
    assert_eq!(ratios(&h), vec![CropRatioChange { editing: false }]);
    assert!(h.state().events.iter().all(|o| o.settings_changed), "a preset pick edits the settings");
    assert_eq!(h.state().settings.choice(&h.state().users), CropPresetChoice::Ratio(3));

    // An arrow key steps the focused width field: the edit is still under way.
    h.state_mut().events.clear();
    h.get_by_role_and_label(Role::SpinButton, "Width").focus();
    h.run_steps(3);
    h.key_press_modifiers(Modifiers::NONE, Key::ArrowUp);
    h.run_steps(3);
    assert_eq!(h.state().settings.ratio, CropRatio::Sides { width: 6.0, height: 7.0 });
    assert_eq!(ratios(&h), vec![CropRatioChange { editing: true }]);

    // ⇄ swaps the sides.
    h.state_mut().events.clear();
    h.get_by_label("Swaps height and width").click();
    h.run_steps(3);
    assert_eq!(h.state().settings.ratio, CropRatio::Sides { width: 7.0, height: 6.0 });
    assert_eq!(ratios(&h), vec![CropRatioChange { editing: false }]);
}

fn ratios(h: &Harness<'_, Host>) -> Vec<CropRatioChange> {
    h.state().events.iter().filter_map(|o| o.ratio).collect()
}

// The gear menu's Custom… reports a colour request for the shield colour through the bar, starting at the stored custom colour.
#[test]
fn the_bar_reports_the_shield_color_request() {
    let mut settings = CropSettings::default();
    settings.shield.custom_color = [255, 0, 51];
    let mut h = harness(settings, Vec::new());
    h.get_by_label("Set additional Crop options").click();
    h.run_steps(3);
    h.get_by_role_and_label(Role::RadioButton, "Custom…").click();
    h.run_steps(3);
    let request = h.state().events.iter().find_map(|o| o.pick_color.clone()).expect("a colour request");
    assert_eq!(request.key, CropOptionsMenu::COLOR_KEY);
    assert_eq!(request.title.as_deref(), Some("Crop Shield color"));
    assert_eq!(request.color.to_hex().to_ascii_lowercase().trim_start_matches('#'), "ff0033");
    assert_eq!(h.state().settings.shield.color, ShieldColor::Custom);
    assert!(h.state().events.iter().any(|o| o.settings_changed));
}

// The ratio preset select is `crop.preset_width` wide and the Fill select keeps its name without a colon.
#[test]
fn the_preset_select_takes_its_token_width() {
    let h = harness(sides(2.0, 3.0), Vec::new());
    let width = h.get_by_role_and_label(Role::ComboBox, "Crop ratio").rect().width();
    assert!((width - 140.0).abs() < 1.5, "{width}");
    assert!(h.query_by_role_and_label(Role::ComboBox, "Fill").is_some());
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
