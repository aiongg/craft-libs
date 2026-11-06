//! `NewGuideDialog`, `GuideLayoutDialog` and `GuideLayoutOptions::lines`.
#![allow(clippy::expect_used, reason = "tests")]

use std::sync::Arc;

use egui::accesskit::Role;
use egui::{Color32, Event, Key, Modifiers, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::*;
use crate::canvas::{Guide, Orientation};
use crate::localized::units::{Length, LengthUnit};
use crate::theme::{Base, install};

/// A 3000 × 2000 px document at 300 ppi with a centimetre ruler.
fn units() -> GuideUnits {
    GuideUnits { ruler: LengthUnit::Centimetres, width_px: 3000.0, height_px: 2000.0, pixels_per_inch: 300.0, points_per_inch: 72.0 }
}

fn pixel_units() -> GuideUnits {
    GuideUnits { ruler: LengthUnit::Pixels, width_px: 1000.0, height_px: 800.0, pixels_per_inch: 72.0, points_per_inch: 72.0 }
}

/// The dialog host, and the answers of New Guide, which the app remembers as it does.
#[derive(Default)]
struct App {
    host: DialogHost,
    answers: Vec<NewGuideOptions>,
}

impl std::ops::Deref for App {
    type Target = DialogHost;
    fn deref(&self) -> &DialogHost {
        &self.host
    }
}

impl std::ops::DerefMut for App {
    fn deref_mut(&mut self) -> &mut DialogHost {
        &mut self.host
    }
}

fn harness<D: StackDialog>(dialog: D, store: Arc<craft_host::MemorySections>) -> Harness<'static, App> {
    let mut host = DialogHost::default();
    host.open(dialog);
    let app = App { host, answers: Vec::new() };
    HarnessBuilder::default()
        .with_size(vec2(1000.0, 800.0))
        .with_context_setup(move |ctx| {
            install(ctx, &Base::suite_default());
            craft_host::Host::builder(craft_host::AppInfo { id: "test", name: "Test", version: "1.0" }).settings(store.clone()).install(ctx);
        })
        .build_ui_state(
            |ui, app: &mut App| {
                app.host.show(ui.ctx());
                if let Some(DialogResult::Ok(answer)) = app.host.take::<NewGuideDialog>() {
                    NewGuideDialog::remember(ui.ctx(), &answer);
                    app.answers.push(answer);
                }
            },
            app,
        )
}

fn open<D: StackDialog>(dialog: D) -> Harness<'static, App> {
    let mut harness = harness(dialog, Arc::default());
    harness.run();
    harness
}

fn new_guide(harness: &Harness<'_, App>) -> NewGuideOptions {
    harness.state().get::<NewGuideDialog>().expect("open").options()
}

fn layout(harness: &Harness<'_, App>) -> GuideLayoutOptions {
    harness.state().get::<GuideLayoutDialog>().expect("open").options()
}

fn type_into(harness: &mut Harness<'_, App>, field: &str, text: &str) {
    harness.get_all_by_role_and_label(Role::SpinButton, field).next().expect("the field is shown").focus();
    harness.run();
    harness.event(Event::Text(text.into()));
    harness.run();
    harness.key_press(Key::Tab);
    harness.run();
}

fn choose(harness: &mut Harness<'_, App>, list: &str, option: &str) {
    harness.get_by_role_and_label(Role::ComboBox, list).click();
    harness.run();
    harness.get_by_label(option).click();
    harness.run();
}

// New Guide opens with the remembered orientation, colour and position in the current ruler unit;
// "2.5 cm" answers 2.5 cm and its pixels at the document's resolution, a bare "12" gets the ruler
// unit, "abc" is not taken; Enter is OK.
#[test]
fn new_guide_reads_any_unit_and_opens_with_the_last_guide() {
    let store: Arc<craft_host::MemorySections> = Arc::default();
    let mut first = harness(NewGuideDialog::new(units()), store.clone());
    first.run();
    let fresh = new_guide(&first);
    assert_eq!((fresh.orientation, fresh.position_px, fresh.color), (Orientation::Horizontal, 0.0, GuideColor::Cyan));

    type_into(&mut first, "Position", "2.5");
    let typed = new_guide(&first);
    assert_eq!(typed.position, Length { value: 2.5, unit: LengthUnit::Centimetres });
    assert!((typed.position_px - 2.5 / 2.54 * 300.0).abs() < 1e-6, "a bare number is in the ruler unit: {}", typed.position_px);
    type_into(&mut first, "Position", "abc");
    assert_eq!(new_guide(&first).position_px, typed.position_px, "text that is not a length is not taken");
    type_into(&mut first, "Position", "1in");
    assert!((new_guide(&first).position_px - 300.0).abs() < 1e-6, "another unit converts at the document's resolution");

    first.get_by_label("Vertical").click();
    first.run();
    choose(&mut first, "Color", "Green");
    first.get_by_role_and_label(Role::SpinButton, "Position").focus();
    first.run();
    first.key_press(Key::Enter);
    first.run();
    assert!(first.state().get::<NewGuideDialog>().is_none(), "Enter is OK");
    let answer = first.state().answers.last().cloned().expect("OK answers");
    assert_eq!((answer.orientation, answer.color, answer.edited), (Orientation::Vertical, GuideColor::Green, None));
    assert!((answer.position_px - 300.0).abs() < 1e-6 && answer.position.unit == LengthUnit::Centimetres);

    // The app recorded the success; the next dialog opens with it, its position in the ruler unit.
    let mut second = harness(NewGuideDialog::new(GuideUnits { ruler: LengthUnit::Inches, ..units() }), store);
    second.run();
    let again = new_guide(&second);
    assert_eq!((again.orientation, again.color, again.position_px), (Orientation::Vertical, GuideColor::Green, 300.0));
    assert!((again.position.value - 1.0).abs() < 1e-9 && again.position.unit == LengthUnit::Inches);
}

// Edit Selected Guides opens on the guide's orientation, position and colour; color_changed is
// false until the colour list changes; Custom… gives one ColorRequest keyed NEW_GUIDE_COLOR_KEY
// in that pass, set_color answers it and cancel_color restores the colour before Custom….
#[test]
fn edit_selected_guides_reports_only_a_changed_colour() {
    let guide = Guide::new(7, Orientation::Vertical, 450.0);
    let mut harness = open(NewGuideDialog::edit(&guide, GuideColor::Cyan.rgb(), units()));
    let opened = new_guide(&harness);
    assert_eq!((opened.orientation, opened.position_px, opened.color, opened.edited), (Orientation::Vertical, 450.0, GuideColor::Cyan, Some(7)));
    assert!(!opened.color_changed);
    assert!(harness.get_all_by_label("Edit Selected Guides").next().is_some());

    let take = |harness: &mut Harness<'_, App>| harness.state_mut().get_mut::<NewGuideDialog>().expect("open").pick_color();
    choose(&mut harness, "Color", "Green");
    assert!(new_guide(&harness).color_changed);
    choose(&mut harness, "Color", "Cyan");
    assert!(!new_guide(&harness).color_changed, "putting the colour back is no change");

    choose(&mut harness, "Color", "Custom…");
    let request = take(&mut harness).expect("Custom… asks for a colour");
    assert_eq!(request.key, NEW_GUIDE_COLOR_KEY);
    assert_eq!(request.title.as_deref(), Some("Color Picker (Guides)"));
    assert!(take(&mut harness).is_none(), "the request is taken once");
    harness.state_mut().get_mut::<NewGuideDialog>().expect("open").cancel_color();
    harness.run();
    assert_eq!(new_guide(&harness).color, GuideColor::Cyan, "a cancelled pick restores the colour");

    choose(&mut harness, "Color", "Custom…");
    assert!(take(&mut harness).is_some());
    let dialog = harness.state_mut().get_mut::<NewGuideDialog>().expect("open");
    dialog.set_color("other", craft_theme::Srgba::BLACK);
    dialog.set_color(NEW_GUIDE_COLOR_KEY, craft_theme::Rgba::rgb(0x12, 0x34, 0x56).into());
    harness.run();
    let custom = new_guide(&harness);
    assert_eq!(custom.color, GuideColor::Custom([0x12, 0x34, 0x56]));
    assert!(custom.color_changed);
}

fn pixels(value: f64) -> Length {
    Length { value, unit: LengthUnit::Pixels }
}

fn document() -> GuideRect {
    GuideRect { x0: 0.0, y0: 0.0, x1: 1000.0, y1: 800.0 }
}

fn lines(options: &GuideLayoutOptions, targets: &[GuideRect], existing: &GuideLines) -> Result<GuideLayoutLines, GuideLayoutError> {
    options.lines(targets, existing, &pixel_units())
}

fn just_margin(margin: [f64; 4]) -> GuideLayoutOptions {
    let mut options = GuideLayoutOptions::default();
    options.columns.enabled = false;
    options.margin = margin.map(pixels);
    options
}

// 8 columns with a 20 px gutter over 1000 px give 16 vertical edges at PhotoCraft's positions; a
// fixed width with Center Columns centres them; margins add their four edges; rows off add none;
// a layout wider than the room is DoesNotFit, margins over the size MarginsTooLarge, all off
// Nothing, over 16 000 edges TooMany; Clear Existing keeps guides outside the artboards.
#[test]
fn layout_lines_match_photocraft() {
    let none = GuideLines::default();
    let eight = lines(&GuideLayoutOptions::default(), &[document()], &none).expect("the factory layout fits");
    assert_eq!(eight.added.vertical.len(), 16);
    assert_eq!(&eight.added.vertical[..4], [0.0, 107.5, 127.5, 235.0]);
    assert!(eight.added.horizontal.is_empty(), "rows off add none");

    let mut fixed = GuideLayoutOptions {
        columns: GuideAxis { enabled: true, number: 3, size: Some(pixels(100.0)), gutter: pixels(20.0) },
        center_columns: true,
        ..GuideLayoutOptions::default()
    };
    assert_eq!(lines(&fixed, &[document()], &none).unwrap().added.vertical, [330.0, 430.0, 450.0, 550.0, 570.0, 670.0]);
    fixed.center_columns = false;
    assert_eq!(lines(&fixed, &[document()], &none).unwrap().added.vertical[..2], [0.0, 100.0]);

    let margins = lines(&just_margin([10.0, 20.0, 30.0, 40.0]), &[document()], &none).unwrap();
    assert_eq!((margins.added.vertical, margins.added.horizontal), (vec![20.0, 960.0], vec![10.0, 770.0]));
    let mut percent = just_margin([0.0; 4]);
    percent.margin[1] = Length { value: 10.0, unit: LengthUnit::Percent };
    assert_eq!(lines(&percent, &[document()], &none).unwrap().added.vertical, [100.0, 1000.0], "percent is of the target's width");
    let mut off = just_margin([10.0; 4]);
    off.margin_enabled = false;
    assert_eq!(lines(&off, &[document()], &none), Err(GuideLayoutError::Nothing), "Margin off adds nothing");

    let mut wide = GuideLayoutOptions::default();
    wide.columns.size = Some(pixels(200.0));
    assert_eq!(lines(&wide, &[document()], &none), Err(GuideLayoutError::DoesNotFit));
    assert_eq!(lines(&just_margin([0.0, 600.0, 0.0, 600.0]), &[document()], &none), Err(GuideLayoutError::MarginsTooLarge));
    assert_eq!(lines(&just_margin([0.0; 4]), &[document()], &none), Err(GuideLayoutError::Nothing));
    let selected = GuideLayoutOptions { target: GuideTarget::SelectedArtboards, ..GuideLayoutOptions::default() };
    assert_eq!(lines(&selected, &[], &none), Err(GuideLayoutError::NoArtboards));
    let mut many = GuideLayoutOptions::default();
    many.columns.number = 1000;
    many.columns.gutter = pixels(0.0);
    assert_eq!(lines(&many, &[document(); 8], &none), Err(GuideLayoutError::TooMany));

    let existing = GuideLines { vertical: vec![50.0, 500.0, 900.0], horizontal: vec![] };
    let board = GuideRect { x0: 0.0, y0: 0.0, x1: 400.0, y1: 800.0 };
    let mut clear = just_margin([10.0; 4]);
    clear.clear_existing = true;
    assert_eq!(
        lines(&clear, &[document()], &existing).unwrap().all,
        lines(&clear, &[document()], &none).unwrap().all,
        "a cleared document keeps none"
    );
    clear.target = GuideTarget::SelectedArtboards;
    let kept = lines(&clear, &[board], &existing).unwrap().all.vertical;
    assert!(kept.contains(&500.0) && kept.contains(&900.0) && !kept.contains(&50.0), "only guides inside the artboard go: {kept:?}");
}

// Preview gives the full set while valid, keeps the last valid set while a field is invalid, and
// is None with Preview off; a typed edit turns a named preset to Custom, a target change doesn't;
// Alt+Cancel resets to the default layout; the swatch gives one ColorRequest keyed
// GUIDE_LAYOUT_COLOR_KEY and set_color recolours the preview in the same pass.
#[test]
fn layout_dialog_previews_live_and_tracks_the_preset() {
    let existing = GuideLines { vertical: vec![5.0], horizontal: vec![] };
    let dialog = GuideLayoutDialog::new(document(), existing, pixel_units(), GuideColor::Cyan.rgb())
        .artboards(vec![GuideRect { x0: 0.0, y0: 0.0, x1: 500.0, y1: 400.0 }])
        .initial(GuideLayoutOptions::default());
    let mut harness = open(dialog);
    let preview = |harness: &Harness<'_, App>| harness.state().get::<GuideLayoutDialog>().expect("open").preview();
    let first = preview(&harness).expect("a valid layout previews");
    assert_eq!(first.lines.added.vertical.len(), 16);
    assert!(first.lines.all.vertical.contains(&5.0), "the preview is the full set: the existing guides too");
    assert_eq!(first.color, Color32::from_rgb(0x4a, 0xff, 0xff));

    choose(&mut harness, "Preset", "12 Columns");
    let twelve = layout(&harness);
    assert_eq!((twelve.preset, twelve.columns.number, twelve.columns.gutter.value), (GuideLayoutPreset::TwelveColumns, 12, 24.0));
    choose(&mut harness, "Target", "Selected Artboards");
    assert_eq!(layout(&harness).preset, GuideLayoutPreset::TwelveColumns, "a target change keeps the preset");
    type_into(&mut harness, "Gutter", "10");
    assert_eq!(layout(&harness).preset, GuideLayoutPreset::Custom, "a typed edit makes the preset Custom");
    let valid = preview(&harness).expect("still valid");
    assert_eq!(valid.lines.added.vertical.len(), 24);

    type_into(&mut harness, "Width", "2000");
    assert!(harness.query_by_label("Invalid guide layout").is_some());
    assert_eq!(preview(&harness), Some(valid.clone()), "an invalid field keeps the last valid preview");
    assert!(harness.get_by_label("OK").accesskit_node().is_disabled(), "OK waits for a valid layout");

    harness.get_by_label("Preview").click();
    harness.run();
    assert!(preview(&harness).is_none(), "Preview off draws nothing");
    harness.get_by_label("Preview").click();
    harness.run();

    let dialog = harness.state_mut().get_mut::<GuideLayoutDialog>().expect("open");
    assert!(dialog.pick_color().is_none());
    dialog.set_color("other", craft_theme::Srgba::BLACK);
    dialog.set_color(GUIDE_LAYOUT_COLOR_KEY, craft_theme::Rgba::rgb(1, 2, 3).into());
    assert_eq!(dialog.preview().map(|p| p.color), Some(Color32::from_rgb(1, 2, 3)), "the preview takes the colour at once");
    harness.get_by_label("Guide color").click();
    harness.run();
    let request = harness.state_mut().get_mut::<GuideLayoutDialog>().expect("open").pick_color().expect("the swatch asks");
    assert_eq!(request.key, GUIDE_LAYOUT_COLOR_KEY);

    harness.event(Event::ModifiersChanged(Modifiers::ALT));
    harness.run();
    harness.get_by_label("Reset").focus();
    harness.run();
    harness.key_press_modifiers(Modifiers::ALT, Key::Space);
    harness.run();
    let reset = layout(&harness);
    assert_eq!(reset, GuideLayoutOptions::default(), "Alt+Cancel restores the factory layout");
    assert!(harness.state().get::<GuideLayoutDialog>().is_some(), "Reset keeps the dialog");
}

// New Guide opens with Position focused and its value selected, as Photoshop and PhotoCraft do:
// the first digit typed replaces the value.
#[test]
fn new_guide_opens_with_position_focused() {
    let initial = NewGuideOptions {
        orientation: Orientation::Horizontal,
        position: Length { value: 5.0, unit: LengthUnit::Centimetres },
        position_px: 5.0 / 2.54 * 300.0,
        color: GuideColor::Cyan,
        color_changed: false,
        edited: None,
    };
    let mut harness = open(NewGuideDialog::new(units()).initial(initial));
    assert!(harness.get_by_role_and_label(Role::SpinButton, "Position").accesskit_node().is_focused());
    harness.event(Event::Text("7".into()));
    harness.run();
    harness.key_press(Key::Tab);
    harness.run();
    assert_eq!(new_guide(&harness).position, Length { value: 7.0, unit: LengthUnit::Centimetres });
}

// A typed percentage stays a percentage and resolves per artboard: 10 % of a 500 px and of a
// 1000 px wide artboard are 50 px and 100 px in from each one's left edge.
#[test]
fn typed_percent_resolves_per_artboard() {
    let boards = vec![GuideRect { x0: 0.0, y0: 0.0, x1: 500.0, y1: 400.0 }, GuideRect { x0: 600.0, y0: 0.0, x1: 1600.0, y1: 800.0 }];
    let mut options = GuideLayoutOptions::default();
    options.columns.enabled = false;
    options.rows.enabled = false;
    options.margin_enabled = true;
    options.margin[1] = Length { value: 10.0, unit: LengthUnit::Percent };
    options.target = GuideTarget::SelectedArtboards;
    let made = lines(&options, &boards, &GuideLines::default()).expect("valid").added;
    assert_eq!(made.vertical, [50.0, 500.0, 700.0, 1600.0], "left edge in by each artboard's own 10 %");

    let dialog = GuideLayoutDialog::new(document(), GuideLines::default(), pixel_units(), GuideColor::Cyan.rgb()).artboards(boards);
    let mut harness = open(dialog);
    choose(&mut harness, "Target", "Selected Artboards");
    type_into(&mut harness, "Gutter", "10%");
    assert_eq!(layout(&harness).columns.gutter, Length { value: 10.0, unit: LengthUnit::Percent }, "the dialog keeps the percentage");
    type_into(&mut harness, "Gutter", "12");
    assert_eq!(layout(&harness).columns.gutter, Length { value: 12.0, unit: LengthUnit::Pixels });
}

// An invalid layout shows its reason as text, not only as a tooltip.
#[test]
fn invalid_layout_shows_its_reason() {
    let dialog = GuideLayoutDialog::new(document(), GuideLines::default(), pixel_units(), GuideColor::Cyan.rgb());
    let mut harness = open(dialog);
    type_into(&mut harness, "Width", "5000");
    assert!(harness.query_by_label("Invalid guide layout").is_some());
    assert!(harness.query_by_label(GuidesLabels::default().does_not_fit.as_str()).is_some());
}

// Field widths come from the values they show: a length with its unit is wider than a count.
#[test]
fn guide_fields_are_as_wide_as_their_values() {
    let width = |harness: &Harness<'_, App>, role, label| harness.get_all_by_role_and_label(role, label).next().expect("shown").rect().width();
    let position = width(&open(NewGuideDialog::new(units())), Role::SpinButton, "Position");
    let harness = open(GuideLayoutDialog::new(document(), GuideLines::default(), pixel_units(), GuideColor::Cyan.rgb()));
    let number = width(&harness, Role::SpinButton, "Number");
    assert!(position > number, "{position} vs {number}");
    assert!(width(&harness, Role::SpinButton, "Top:") > number);
}
