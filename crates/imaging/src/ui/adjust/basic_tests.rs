//! Tests of Brightness/Contrast, Threshold, Color Balance and Photo Filter (task AD-3).

use egui::accesskit::Role;
use egui::{Id, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use super::{
    AdjustStyle, BrightnessContrast, BrightnessContrastEditor, ColorBalance, ColorBalanceEditor, ColorBalanceTone, PhotoFilter, PhotoFilterEditor,
    PhotoFilterPreset, ThresholdEditor,
};
use craft_theme::Srgba;
use craft_ui::dialog::ColorRequest;
use craft_ui::edit::EditEvent;
use craft_ui::localized::LocaleNumbers;
use craft_ui::theme::{Base, Tokens, TokensOf as _, install};

fn builder<State>() -> egui_kittest::HarnessBuilder<State> {
    Harness::builder().with_size([360.0, 420.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default()))
}

fn applied<T: Copy>(value: &mut T, event: &EditEvent<T>) {
    if let Some(v) = event.value() {
        *value = *v;
    }
}

// Turning Use Legacy off with Contrast at −80 reports one `Set` with Contrast −50 and legacy false.
#[test]
fn brightness_contrast_use_legacy_off_clamps_contrast_to_minus_fifty() {
    struct Host {
        value: BrightnessContrast,
        events: Vec<EditEvent<BrightnessContrast>>,
    }
    let state = Host { value: BrightnessContrast { brightness: 10.0, contrast: -80.0, legacy: true }, events: Vec::new() };
    let mut h = builder().build_ui_state(
        |ui, host: &mut Host| {
            let numbers = LocaleNumbers::from_tag("en");
            if let Some(event) = BrightnessContrastEditor::new("bc", &host.value, &numbers).show(ui).event {
                applied(&mut host.value, &event);
                host.events.push(event);
            }
        },
        state,
    );
    h.run_steps(3);
    h.get_by_role_and_label(Role::CheckBox, "Use Legacy").click();
    h.run_steps(3);
    let want = BrightnessContrast { brightness: 10.0, contrast: -50.0, legacy: false };
    assert_eq!(h.state().events, [EditEvent::set(want, Modifiers::NONE)]);
}

// Threshold's marker sits at `level / 255` across the histogram and the row reports the level.
#[test]
fn threshold_marker_follows_the_level() {
    struct Host {
        level: f64,
        events: Vec<EditEvent<f64>>,
        height: f32,
        inset: f32,
    }
    let state = Host { level: 64.0, events: Vec::new(), height: 0.0, inset: 0.0 };
    let counts = [3u32; 256];
    let mut h = builder().build_ui_state(
        |ui, host: &mut Host| {
            let numbers = LocaleNumbers::from_tag("en");
            let style = AdjustStyle::of(&Tokens::of(ui));
            (host.height, host.inset) = (style.threshold_histogram, style.threshold_inset);
            if let Some(event) = ThresholdEditor::new("th", host.level, &numbers).histogram(&counts).show(ui).event {
                applied(&mut host.level, &event);
                host.events.push(event);
            }
        },
        state,
    );
    h.run_steps(3);
    let plot = h.get_by_role_and_label(Role::Image, "Histogram").rect();
    assert!((plot.height() - h.state().height).abs() < 0.5, "the plot is the style's height: {plot:?}");
    assert!(plot.width() <= 300.0 - 2.0 * h.state().inset + 0.5, "at most the style's width less the inset at each side: {plot:?}");
    assert!((super::basic::marker_x(plot, 255.0) - plot.right()).abs() < 1e-3);
    assert!((super::basic::marker_x(plot, 64.0) - (plot.left() + plot.width() * 64.0 / 255.0)).abs() < 1e-3);

    // The row reports the level.
    h.get_by_role_and_label(Role::Slider, "Threshold Level").focus();
    h.run_steps(2);
    h.key_press(egui::Key::ArrowRight);
    h.run_steps(3);
    assert_eq!(h.state().level, 65.0);
    assert!(matches!(h.state().events.last(), Some(EditEvent::Set { .. })));
}

// The tone picked (Shadows) survives a value change from outside; `reset_tone` shows Midtones again; a double-click zeroes only that tone's row.
#[test]
fn color_balance_tone_choice_survives_a_value_change_and_resets_to_midtones() {
    struct Host {
        value: ColorBalance,
        id: Option<Id>,
    }
    let value = ColorBalance { shadows: [10.0, 20.0, 30.0], midtones: [-5.0, -6.0, -7.0], ..ColorBalance::default() };
    let mut h = builder().build_ui_state(
        |ui, host: &mut Host| {
            let numbers = LocaleNumbers::from_tag("en");
            host.id = Some(ColorBalanceEditor::id_in(ui, "cb"));
            if let Some(event) = ColorBalanceEditor::new("cb", &host.value, &numbers).show(ui).event {
                applied(&mut host.value, &event);
            }
        },
        Host { value, id: None },
    );
    h.run_steps(3);
    let Some(editor) = h.state().id else { panic!("the editor was shown") };
    assert_eq!(ColorBalanceEditor::tone(&h.ctx, editor), ColorBalanceTone::Midtones);
    h.get_by_role_and_label(Role::RadioButton, "Shadows").click();
    h.run_steps(3);
    assert_eq!(ColorBalanceEditor::tone(&h.ctx, editor), ColorBalanceTone::Shadows);

    // A change from outside keeps the tone, and the rows show the shadows.
    h.state_mut().value.highlights = [1.0, 2.0, 3.0];
    h.run_steps(3);
    assert_eq!(ColorBalanceEditor::tone(&h.ctx, editor), ColorBalanceTone::Shadows);
    assert_eq!(h.get_by_role_and_label(Role::SpinButton, "Cyan · Red").value().as_deref(), Some("10"));

    // A double-click on the first row's slider zeroes that row of the shown tone only.
    let at = h.get_by_role_and_label(Role::Slider, "Cyan · Red").rect().center();
    h.event(egui::Event::PointerMoved(at));
    h.run_steps(2);
    for pressed in [true, false, true, false] {
        h.event(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
        h.run_steps(2);
    }
    assert_eq!(h.state().value.shadows, [0.0, 20.0, 30.0]);
    assert_eq!(h.state().value.midtones, [-5.0, -6.0, -7.0]);

    ColorBalanceEditor::reset_tone(&h.ctx, editor);
    h.run_steps(3);
    assert_eq!(ColorBalanceEditor::tone(&h.ctx, editor), ColorBalanceTone::Midtones);
    assert_eq!(h.get_by_role_and_label(Role::SpinButton, "Cyan · Red").value().as_deref(), Some("-5"));
}

// Clicking Preserve Luminosity reports one `Set` with it toggled.
#[test]
fn color_balance_preserve_luminosity_reports_a_set() {
    struct Host {
        value: ColorBalance,
        events: Vec<EditEvent<ColorBalance>>,
    }
    let mut h = builder().build_ui_state(
        |ui, host: &mut Host| {
            let numbers = LocaleNumbers::from_tag("en");
            if let Some(event) = ColorBalanceEditor::new("cb", &host.value, &numbers).show(ui).event {
                applied(&mut host.value, &event);
                host.events.push(event);
            }
        },
        Host { value: ColorBalance::default(), events: Vec::new() },
    );
    h.run_steps(3);
    h.get_by_role_and_label(Role::CheckBox, "Preserve Luminosity").click();
    h.run_steps(3);
    let want = ColorBalance { preserve_luminosity: false, ..ColorBalance::default() };
    assert_eq!(h.state().events, [EditEvent::set(want, Modifiers::NONE)]);
}

// Picking Cooling Filter (80) reports a `Set` of (0, 109, 255); a click on the swatch reports a `ColorRequest` keyed `photoFilter.color`.
#[test]
fn photo_filter_preset_sets_its_colour_and_the_swatch_asks_for_one() {
    struct Host {
        value: PhotoFilter,
        events: Vec<EditEvent<PhotoFilter>>,
        requests: Vec<ColorRequest>,
    }
    let value = PhotoFilter { color: PhotoFilterPreset::Warming85.color(), density: 25.0, preserve_luminosity: true };
    let mut h = builder().build_ui_state(
        |ui, host: &mut Host| {
            let numbers = LocaleNumbers::from_tag("en");
            let out = PhotoFilterEditor::new("pf", &host.value, &numbers).show(ui);
            if let Some(event) = out.edit.event {
                applied(&mut host.value, &event);
                host.events.push(event);
            }
            host.requests.extend(out.pick_color);
        },
        Host { value, events: Vec::new(), requests: Vec::new() },
    );
    h.run_steps(3);
    // A preset colour starts on the Filter row: its select is enabled.
    h.get_by_role_and_label(Role::ComboBox, "Filter").click();
    h.run_steps(3);
    h.get_by_label("Cooling Filter (80)").click();
    h.run_steps(3);
    let cooling = Srgba::rgb(0.0, 109.0 / 255.0, 1.0);
    let want = PhotoFilter { color: cooling, density: 25.0, preserve_luminosity: true };
    // The select's own gesture (it opens, then picks): one commit of the preset's colour.
    let commits: Vec<_> =
        h.state().events.iter().filter_map(|e| if let EditEvent::Commit { value, changed: true, .. } = e { Some(*value) } else { None }).collect();
    assert_eq!(commits, [want]);
    assert_eq!(PhotoFilterPreset::of(cooling), Some(PhotoFilterPreset::Cooling80));

    // The Color radio enables the swatch, and a click on it asks for a colour.
    h.get_by_role_and_label(Role::RadioButton, "Color").click();
    h.run_steps(3);
    h.get_by_role_and_label(Role::ColorWell, "Filter Color").click();
    h.run_steps(3);
    let request = ColorRequest::new("photoFilter.color", cooling).title("Filter Color");
    assert_eq!(h.state().requests, [request]);
}
