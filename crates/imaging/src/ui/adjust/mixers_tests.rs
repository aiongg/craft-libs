//! Tests of Black & White, Channel Mixer and Selective Color (task AD-5), and the AccessKit test over every
//! adjustment editor, which needs the bodies of tasks AD-1 to AD-5.

use egui::Ui;
use egui::accesskit::{Role, Toggled};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::hue_saturation::RANGE;
use super::mixers::{total_text, total_warns};
use super::{AdjustField, AdjustLabels};
use super::{
    AdjustSliders, BlackWhite, BlackWhiteEditor, BlackWhiteOutput, BrightnessContrast, BrightnessContrastEditor, ChannelMixer, ChannelMixerEditor,
    ColorBalance, ColorBalanceEditor, CurvesPanel, HueSaturation, HueSaturationEditor, Levels, LevelsPanel, PhotoFilter, PhotoFilterEditor,
    PhotoFilterPreset, SelectiveColor, SelectiveColorEditor, ThresholdEditor, ToneChannels, ToneCurves, ToneSpace,
};
use craft_theme::Srgba;
use craft_ui::dialog::ColorRequest;
use craft_ui::edit::EditEvent;
use craft_ui::localized::LocaleNumbers;
use craft_ui::theme::{Base, install};

/// A host of one value: applies every event and keeps them, with the colour requests.
struct Host<T> {
    value: T,
    events: Vec<EditEvent<T>>,
    requests: Vec<ColorRequest>,
}

fn builder<State>() -> egui_kittest::HarnessBuilder<State> {
    Harness::builder().with_size([420.0, 560.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default()))
}

fn rig<T: Copy + 'static>(value: T, mut show: impl FnMut(&mut Ui, &mut Host<T>) + 'static) -> Harness<'static, Host<T>> {
    let state = Host { value, events: Vec::new(), requests: Vec::new() };
    let mut h = builder().build_ui_state(move |ui, host: &mut Host<T>| show(ui, host), state);
    h.run_steps(3);
    h
}

fn record<T: Copy>(host: &mut Host<T>, event: Option<EditEvent<T>>) {
    let Some(event) = event else { return };
    if let Some(value) = event.value() {
        host.value = *value;
    }
    host.events.push(event);
}

fn black_white(value: BlackWhite) -> Harness<'static, Host<BlackWhite>> {
    rig(value, |ui, host| {
        let numbers = LocaleNumbers::from_tag("en");
        let BlackWhiteOutput { edit, pick_color } = BlackWhiteEditor::new("bw", &host.value, &numbers).show(ui);
        host.requests.extend(pick_color);
        record(host, edit.event);
    })
}

fn channel_mixer(value: ChannelMixer) -> Harness<'static, Host<ChannelMixer>> {
    rig(value, |ui, host| {
        let numbers = LocaleNumbers::from_tag("en");
        let edit = ChannelMixerEditor::new("cm", &host.value, &numbers).show(ui);
        record(host, edit.event);
    })
}

fn selective_color(value: SelectiveColor) -> Harness<'static, Host<SelectiveColor>> {
    rig(value, |ui, host| {
        let numbers = LocaleNumbers::from_tag("en");
        let edit = SelectiveColorEditor::new("sc", &host.value, &numbers).show(ui);
        record(host, edit.event);
    })
}

// Default reports one `Set` with the weights 40, 60, 40, 60, 20, 80 and the tint unchanged.
#[test]
fn black_white_default_button_restores_the_six_defaults() {
    let tint_color = Srgba::rgb(0.2, 0.4, 0.6);
    let mut h = black_white(BlackWhite { weights: [0.0; 6], tint: true, tint_color });
    h.get_by_role_and_label(Role::Button, "Default").click();
    h.run_steps(3);
    let events = &h.state().events;
    let [EditEvent::Set { value, .. }] = events.as_slice() else { panic!("one Set: {events:?}") };
    assert_eq!(value.weights, [40.0, 60.0, 40.0, 60.0, 20.0, 80.0]);
    assert!(value.tint);
    assert_eq!(value.tint_color, tint_color);
}

// A click on the tint swatch with Tint on reports a `ColorRequest` keyed `blackWhite.tint`; with Tint off the swatch is disabled.
#[test]
fn black_white_tint_swatch_asks_for_a_colour() {
    let tint_color = Srgba::rgb(0.2, 0.4, 0.6);
    let mut h = black_white(BlackWhite { tint: true, tint_color, ..BlackWhite::default() });
    h.get_by_role_and_label(Role::ColorWell, "Tint Color").click();
    h.run_steps(3);
    assert_eq!(h.state().requests, [ColorRequest::new("blackWhite.tint", tint_color).title("Tint Color")]);

    let mut h = black_white(BlackWhite { tint: false, tint_color, ..BlackWhite::default() });
    assert!(h.get_by_role_and_label(Role::ColorWell, "Tint Color").accesskit_node().is_disabled());
    h.run_steps(2);
    assert!(h.state().requests.is_empty());
}

// Monochrome on reports a `Set` with `gray` = [40, 40, 20, 0] and the select shows Gray; off reports `gray: None`.
#[test]
fn channel_mixer_monochrome_seeds_the_gray_row_and_off_removes_it() {
    let mut h = channel_mixer(ChannelMixer::default());
    assert_eq!(h.get_by_role_and_label(Role::ComboBox, "Output Channel").accesskit_node().value().as_deref(), Some("Red"));
    h.get_by_role_and_label(Role::CheckBox, "Monochrome").click();
    h.run_steps(3);
    let events = &h.state().events;
    let [EditEvent::Set { value, .. }] = events.as_slice() else { panic!("one Set: {events:?}") };
    assert_eq!(value.gray, Some(ChannelMixer::MONOCHROME_START));
    assert_eq!(h.get_by_role_and_label(Role::ComboBox, "Output Channel").accesskit_node().value().as_deref(), Some("Gray"));

    h.get_by_role_and_label(Role::CheckBox, "Monochrome").click();
    h.run_steps(3);
    let events = &h.state().events;
    let Some(EditEvent::Set { value, .. }) = events.last() else { panic!("a Set: {events:?}") };
    assert_eq!(value.gray, None);
    assert_eq!(h.get_by_role_and_label(Role::ComboBox, "Output Channel").accesskit_node().value().as_deref(), Some("Red"));
}

/// The values of the Label nodes of the tree.
fn label_values(h: &Harness<'_, Host<ChannelMixer>>) -> Vec<String> {
    h.get_all_by_role(Role::Label).filter_map(|node| node.accesskit_node().value()).collect()
}

// A total of 120 % shows "+120%" in `status.warn.fg` and the accessible value adds "over 100 percent"; 100 % shows the normal colour.
#[test]
fn channel_mixer_total_turns_warn_above_one_hundred_percent() {
    let over = ChannelMixer { red: [60.0, 60.0, 0.0, 0.0], ..ChannelMixer::default() };
    let h = channel_mixer(over);
    assert!(label_values(&h).iter().any(|value| value == "+120%, over 100 percent"), "{:?}", label_values(&h));
    assert!(total_warns(120.0));
    assert_eq!(total_text(120.0), "+120%");

    let at = ChannelMixer { red: [60.0, 40.0, 0.0, 0.0], ..ChannelMixer::default() };
    let h = channel_mixer(at);
    let values = label_values(&h);
    assert!(values.iter().any(|value| value == "+100%"), "{values:?}");
    assert!(!values.iter().any(|value| value.contains("over 100")));
    assert!(!total_warns(100.0));
}

// Picking Neutrals and moving Black reports a value whose only change is range 7, slot 3.
#[test]
fn selective_color_edits_the_shown_range_only() {
    let mut h = selective_color(SelectiveColor::default());
    h.get_by_role_and_label(Role::ComboBox, "Colors").click();
    h.run_steps(3);
    h.get_by_label("Neutrals").click();
    h.run_steps(3);
    h.get_by_role_and_label(Role::Slider, "Black").focus();
    h.run_steps(2);
    h.key_press(egui::Key::ArrowRight);
    h.run_steps(3);
    let events = &h.state().events;
    let Some(EditEvent::Set { value, .. }) = events.last() else { panic!("a Set: {events:?}") };
    let mut want = SelectiveColor::default();
    want.ranges[7][3] = 1.0;
    assert_eq!(*value, want);
}

/// Whether a node with `role` and `name` is in the tree (a panel and its editor may share a name).
fn present(h: &Harness<'_>, role: Role, name: &str) {
    assert!(h.get_all_by_role_and_label(role, name).next().is_some(), "{role:?} {name}");
}

fn button_toggle(h: &Harness<'_>, name: &str) -> Option<Toggled> {
    h.get_all_by_role_and_label(Role::Button, name).next().and_then(|node| node.accesskit_node().toggled())
}

/// Checks an editor's accessible nodes with `show`.
fn tree(size: [f32; 2], show: impl FnMut(&mut Ui) + 'static) -> Harness<'static> {
    let mut h = Harness::builder().with_size(size).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui(show);
    h.run_steps(3);
    h
}

fn has_value(h: &Harness<'_>, role: Role, name: &str) {
    let Some(node) = h.get_all_by_role_and_label(role, name).next() else { panic!("{role:?} {name}") };
    let node = node.accesskit_node();
    assert!(node.numeric_value().is_some() || node.value().is_some(), "{name} has a value");
}

// Every editor's controls have AccessKit names and values (Levels and Curves groups, picker toggles, slider rows, spectrum images, range-thumb sliders, the Tone radio group, the Total label).
#[test]
fn adjust_editors_have_accesskit_names_and_values() {
    let numbers = LocaleNumbers::from_tag("en");
    let rgb = || ToneChannels::of(ToneSpace::Rgb, false);

    // Levels and Curves: the panel groups and the three picker toggles.
    let levels = vec![Levels { in_black: 0.0, gamma: 1.0, in_white: 255.0, out_black: 0.0, out_white: 255.0 }; 4];
    let n = numbers.clone();
    let h = tree([480.0, 640.0], move |ui| {
        let _ = LevelsPanel::new("lp", &levels, rgb(), &n).show(ui);
    });
    present(&h, Role::Group, "Levels");
    for name in ["Set Black Point", "Set Neutral Gray Point", "Set White Point"] {
        assert!(button_toggle(&h, name) == Some(Toggled::False), "{name}");
    }
    has_value(&h, Role::Slider, "Input black");
    let curves = ToneCurves::identity(&rgb());
    let n = numbers.clone();
    let h = tree([480.0, 720.0], move |ui| {
        let _ = CurvesPanel::new("cp", &curves, rgb(), &n).show(ui);
    });
    present(&h, Role::Group, "Curves");
    present(&h, Role::Button, "Set White Point");

    // Slider rows, Threshold, Brightness/Contrast.
    let labels = AdjustLabels::default();
    let n = numbers.clone();
    let h = tree([420.0, 300.0], move |ui| {
        let fields = AdjustField::exposure(&labels);
        let _ = AdjustSliders::new("ex", &fields, &[0.0, 0.0, 1.0], &n).show(ui);
    });
    has_value(&h, Role::Slider, "Exposure");
    present(&h, Role::SpinButton, "Exposure");
    let n = numbers.clone();
    let h = tree([420.0, 300.0], move |ui| {
        let _ = ThresholdEditor::new("th", 128.0, &n).show(ui);
    });
    present(&h, Role::Image, "Histogram");
    has_value(&h, Role::Slider, "Threshold Level");
    let n = numbers.clone();
    let h = tree([420.0, 300.0], move |ui| {
        let _ = BrightnessContrastEditor::new("bc", &BrightnessContrast::default(), &n).show(ui);
    });
    has_value(&h, Role::Slider, "Brightness");
    present(&h, Role::CheckBox, "Use Legacy");

    // Color Balance: the Tone radio group; Photo Filter: its swatch.
    let n = numbers.clone();
    let h = tree([420.0, 420.0], move |ui| {
        let _ = ColorBalanceEditor::new("cb", &ColorBalance::default(), &n).show(ui);
    });
    present(&h, Role::RadioGroup, "Tone");
    has_value(&h, Role::Slider, "Cyan · Red");
    let n = numbers.clone();
    let h = tree([420.0, 420.0], move |ui| {
        let value = PhotoFilter { color: PhotoFilterPreset::Warming85.color(), density: 25.0, preserve_luminosity: true };
        let _ = PhotoFilterEditor::new("pf", &value, &n).show(ui);
    });
    present(&h, Role::ColorWell, "Filter Color");
    has_value(&h, Role::Slider, "Density");

    // Hue/Saturation: group, range select, spectrum images and, with a range shown, the four thumbs.
    let n = numbers.clone();
    let mut h = Harness::builder().with_size([420.0, 640.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui(move |ui| {
        let id = HueSaturationEditor::id_in(ui, "hs");
        ui.data_mut(|d| d.insert_temp(id.with(RANGE), 1usize));
        let _ = HueSaturationEditor::new("hs", &HueSaturation::default(), &n).show(ui);
    });
    h.run_steps(3);
    present(&h, Role::Group, "Hue/Saturation");
    present(&h, Role::ComboBox, "Range");
    present(&h, Role::Image, "Hue spectrum, before");
    present(&h, Role::Image, "Hue spectrum, after");
    has_value(&h, Role::Slider, "Hue");
    present(&h, Role::CheckBox, "Colorize");
    for name in ["Range start", "Fully affected start", "Fully affected end", "Range end"] {
        has_value(&h, Role::Slider, name);
    }

    // The mixers: slider rows, the selects and the Total readout.
    let n = numbers.clone();
    let h = tree([420.0, 560.0], move |ui| {
        let _ = BlackWhiteEditor::new("bw", &BlackWhite::default(), &n).show(ui);
    });
    has_value(&h, Role::Slider, "Reds");
    present(&h, Role::CheckBox, "Tint");
    present(&h, Role::ColorWell, "Tint Color");
    present(&h, Role::Button, "Default");
    let n = numbers.clone();
    let h = tree([420.0, 560.0], move |ui| {
        let _ = ChannelMixerEditor::new("cm", &ChannelMixer::default(), &n).show(ui);
    });
    assert!(h.get_by_role_and_label(Role::ComboBox, "Output Channel").accesskit_node().value().is_some());
    has_value(&h, Role::Slider, "Constant");
    assert!(h.get_all_by_role(Role::Label).any(|node| node.accesskit_node().value().as_deref() == Some("+100%")));
    let n = numbers;
    let h = tree([420.0, 560.0], move |ui| {
        let _ = SelectiveColorEditor::new("sc", &SelectiveColor::default(), &n).show(ui);
    });
    assert_eq!(h.get_by_role_and_label(Role::ComboBox, "Colors").accesskit_node().value().as_deref(), Some("Reds"));
    has_value(&h, Role::Slider, "Cyan");
    present(&h, Role::RadioButton, "Relative");
}
