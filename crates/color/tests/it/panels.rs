//! The colour panels through the fixture: the Color panel, the Swatches panel, the Color Guide and the swatch
//! palette and table.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use craft_behavior::{EditEvent, Selection};
use craft_cms::convert::CmykConverter;
use craft_color::palette::{SwatchColor, SwatchEntry, SwatchFormat, SwatchGroup, SwatchNote, SwatchReport};
use craft_color::ui::{
    ColorPanel, ColorPanelCommand, ColorPanelMenu, ColorPanelMode, ColorPanelRamp, ColorPanelState, HarmonyCommand, HarmonyMenu, HarmonyPanel,
    HarmonyPanelState, HarmonyPicked, HarmonyWheel, PanelTarget, PickedColor, SwatchAt, SwatchLibrary, SwatchPalette, SwatchTable, SwatchesCommand,
    SwatchesMenu, SwatchesPanel, SwatchesState, SwatchesWords, finite_values,
};
use craft_color::{CmykModel, GrayModel};
use craft_i18n::Format;
use craft_theme::{ColorModel, HsbModel, LabModel, ModelValues, RgbModel, Srgba};
use craft_ui::contract::{Bind, Role, Show as _};
use craft_ui::patterns::presets::{PresetFileRequest, PresetWords};
use craft_ui::runtime::commands::CommandState;
use craft_ui::runtime::env::Label;
use craft_ui::runtime::requests::ColorRequest;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers, Vec2};

const RED: Srgba = Srgba::rgb(1.0, 0.0, 0.0);
const BLUE: Srgba = Srgba::rgb(0.0, 0.0, 1.0);

/// The rooms the panels are shown in.
const COLOR_ROOM: Vec2 = Vec2::new(320.0, 600.0);
const SWATCHES_ROOM: Vec2 = Vec2::new(320.0, 320.0);
const GUIDE_ROOM: Vec2 = Vec2::new(480.0, 640.0);
const PALETTE_WIDE: Vec2 = Vec2::new(600.0, 300.0);

type Edits = Rc<RefCell<Vec<(PanelTarget, EditEvent<PickedColor>)>>>;

/// The Color panel on a state and two colours the test can change, collecting what it reports.
struct PanelRig {
    fixture: Fixture,
    state: Rc<RefCell<ColorPanelState>>,
    colors: Rc<Cell<[Srgba; 2]>>,
    edits: Edits,
    opened: Rc<RefCell<Vec<ColorRequest>>>,
    added: Rc<RefCell<Vec<PanelTarget>>>,
}

impl PanelRig {
    fn new(mode: ColorPanelMode) -> Self {
        let mut start = ColorPanelState::default();
        start.mode = mode;
        let state = Rc::new(RefCell::new(start));
        let colors = Rc::new(Cell::new([RED, BLUE]));
        let (edits, opened, added): (Edits, Rc<RefCell<Vec<ColorRequest>>>, Rc<RefCell<Vec<PanelTarget>>>) = Default::default();
        let (s, c, e, o, a) = (state.clone(), colors.clone(), edits.clone(), opened.clone(), added.clone());
        let mut fixture = Fixture::new(move |ui| {
            let mut current = *s.borrow();
            let shown = ColorPanel::new(Bind::bound(&mut current), c.get()).show(ui);
            *s.borrow_mut() = current;
            e.borrow_mut().extend(shown.edits);
            o.borrow_mut().extend(shown.pick_color);
            a.borrow_mut().extend(shown.add_to_swatches);
        });
        fixture.room(COLOR_ROOM);
        fixture.settle();
        PanelRig { fixture, state, colors, edits, opened, added }
    }

    fn set_colors(&mut self, colors: [Srgba; 2]) {
        self.colors.set(colors);
        self.fixture.settle();
    }
}

/// A click on the other well makes it the edited one; a click on the edited one asks for the picker.
#[trial]
fn wells_switch_and_open_the_picker() {
    let mut rig = PanelRig::new(ColorPanelMode::RgbSliders);
    assert!(!rig.state.borrow().background);
    let back = rig.fixture.by_label("Background color").expect("the background well");
    let at = back.rect.lerp_inside(Vec2::splat(0.95));
    rig.fixture.click_at(at);
    assert!(rig.state.borrow().background, "a click on the other well makes it the edited one");
    assert!(rig.opened.borrow().is_empty(), "and opens nothing");

    rig.fixture.click_at(at);
    let request = ColorRequest::new(PanelTarget::Background.key(), rig.colors.get()[1]).title("Color Picker (Background Color)");
    assert_eq!(*rig.opened.borrow(), [request], "a click on the edited well asks for its picker, keyed for the well");
    assert_eq!(PanelTarget::from_key(PanelTarget::Background.key()), Some(PanelTarget::Background));
}

/// Alt on the ramp picks for the colour not edited; Shift-click cycles the ramp.
#[trial]
fn ramp_alt_targets_the_other_and_shift_cycles() {
    let mut rig = PanelRig::new(ColorPanelMode::RgbSliders);
    let bar = rig.fixture.by_label("Color Ramp").expect("the ramp");
    let at = egui::pos2(bar.rect.left() + bar.rect.width() * 0.25, bar.rect.center().y);

    rig.fixture.click_at(at);
    let picks = std::mem::take(&mut *rig.edits.borrow_mut());
    assert!(!picks.is_empty() && picks.iter().all(|(t, _)| *t == PanelTarget::Foreground), "a plain press picks for the edited colour");
    let picked = picks.iter().find_map(|(_, e)| e.value()).expect("a picked colour");
    let c = picked.color;
    assert!(c.g > c.r && c.r > c.b, "a quarter across is hue 90, yellow-green: {c:?}");

    rig.fixture.hold(Modifiers::ALT).click_at(at);
    rig.fixture.hold(Modifiers::NONE);
    let picks = std::mem::take(&mut *rig.edits.borrow_mut());
    assert!(!picks.is_empty() && picks.iter().all(|(t, _)| *t == PanelTarget::Background), "Alt picks for the other colour");

    assert_eq!(rig.state.borrow().ramp, ColorPanelRamp::RgbSpectrum);
    rig.fixture.hold(Modifiers::SHIFT).click_at(at);
    rig.fixture.hold(Modifiers::NONE);
    assert_eq!(rig.state.borrow().ramp, ColorPanelRamp::CmykSpectrum, "Shift-click cycles the ramp");
    assert!(rig.edits.borrow().is_empty(), "and picks nothing");
}

/// Copy Color as HTML and Copy Color's Hex Code give color="#rrggbb" and rrggbb; the views and Make Ramp Web Safe set the
/// state.
#[trial]
fn copy_entries_use_photoshops_formats() {
    let mut state = ColorPanelState::default();
    let orange = Srgba::rgb(1.0, 0.5, 0.0);
    assert_eq!(ColorPanelMenu::run_command(&mut state, ColorPanelCommand::CopyHtml, orange).as_deref(), Some("color=\"#ff8000\""));
    assert_eq!(ColorPanelMenu::run_command(&mut state, ColorPanelCommand::CopyHex, orange).as_deref(), Some("ff8000"));
    assert_eq!(ColorPanelMenu::run_command(&mut state, ColorPanelCommand::ColorWheel, orange), None);
    assert_eq!(state.mode, ColorPanelMode::ColorWheel);
    assert_eq!(ColorPanelMenu::run_command(&mut state, ColorPanelCommand::WebSafeRamp, orange), None);
    assert!(state.web_safe_ramp);
}

/// The Add to Swatches button under the wheel asks the app to add the edited colour.
#[trial]
fn add_to_swatches_reports_the_edited_well() {
    let mut rig = PanelRig::new(ColorPanelMode::ColorWheel);
    let add = rig.fixture.by_label("Add to Swatches").expect("the button");
    rig.fixture.click(&add);
    assert_eq!(*rig.added.borrow(), [PanelTarget::Foreground]);
}

/// The panel never takes the focus when shown.
#[trial]
fn showing_the_panel_keeps_the_focus() {
    for mode in ColorPanelMode::ALL {
        let mut rig = PanelRig::new(mode);
        rig.fixture.idle(1);
        assert_eq!(rig.fixture.focused_stop(), None, "{mode:?} takes no focus");
    }
}

/// Under the cubes the R, G and B fields sit beside the hex field, and an edit of one sets that channel of the edited
/// colour.
#[trial]
fn cube_modes_have_red_green_and_blue_fields() {
    for mode in [ColorPanelMode::HueCube, ColorPanelMode::BrightnessCube] {
        let mut rig = PanelRig::new(mode);
        let green = rig.fixture.by_label("Green").expect("the field");
        rig.fixture.click(&green).key(Key::A, Modifiers::COMMAND).type_text("128").key(Key::Enter, Modifiers::NONE);
        let edits = rig.edits.borrow();
        let picked = edits.iter().rev().find_map(|(t, e)| e.value().map(|v| (*t, v.color))).expect("an edit of the colour");
        assert_eq!(picked.0, PanelTarget::Foreground, "{mode:?}");
        assert_eq!(picked.1.to_rgba().0[..3], [255, 128, 0], "{mode:?}: only green changes");
    }
}

/// The cubes show the out-of-gamut chip, whose click takes the nearest printable colour for the edited well, and no
/// web-colour chip.
#[trial]
fn cube_modes_warn_when_the_colour_cannot_print() {
    const GAMUT: &str = "Out of gamut for printing: click for the nearest printable color";
    const NOT_WEB: &str = "Not a web color: click for the nearest web color";
    let green = Srgba::rgb(0.0, 1.0, 0.0);
    for mode in [ColorPanelMode::HueCube, ColorPanelMode::BrightnessCube] {
        let mut rig = PanelRig::new(mode);
        rig.set_colors([green, BLUE]);
        assert!(rig.fixture.by_label(NOT_WEB).is_none(), "{mode:?}: the web-colour chip is for Web Color");
        let chip = rig.fixture.by_label(GAMUT).unwrap_or_else(|| panic!("{mode:?}: pure green can't print"));
        rig.fixture.click(&chip);
        let picked = rig.edits.borrow().iter().find_map(|(t, e)| e.value().map(|v| (*t, v.color))).expect("the nearest colour");
        assert_eq!(picked.0, PanelTarget::Foreground, "{mode:?}");
        assert_ne!(picked.1.to_rgba(), green.to_rgba(), "{mode:?}");

        // A printable colour shows no chip.
        let mut rig = PanelRig::new(mode);
        rig.set_colors([Srgba::rgb(0.5, 0.5, 0.5), BLUE]);
        assert!(rig.fixture.by_label(GAMUT).is_none(), "{mode:?}");

        // With the background well edited, the click edits the background.
        let mut rig = PanelRig::new(mode);
        rig.state.borrow_mut().background = true;
        rig.set_colors([Srgba::rgb(0.5, 0.5, 0.5), green]);
        let chip = rig.fixture.by_label(GAMUT).expect("the background can't print");
        rig.fixture.click(&chip);
        let edits = rig.edits.borrow();
        assert!(edits.iter().any(|(t, _)| *t == PanelTarget::Background), "{mode:?}");
        assert!(edits.iter().all(|(t, _)| *t == PanelTarget::Background), "{mode:?}: the foreground is left alone");
    }
}

/// A saturated colour dragged to the cube's bottom right (black) keeps the remembered hue and saturation: the knob stays
/// in the corner and the strip keeps its hue.
#[trial]
fn dragging_to_the_bottom_right_keeps_the_knob_there() {
    let mut rig = PanelRig::new(ColorPanelMode::HueCube);
    rig.set_colors([Srgba::rgb(0.2, 0.4, 0.6), BLUE]);
    let area = rig.fixture.by_label("Saturation and brightness").expect("the area");
    rig.fixture.drag(&area, area.rect.size());
    let picked = rig.edits.borrow().iter().rev().find_map(|(_, e)| e.value().map(|v| v.color)).expect("a picked colour");
    assert_eq!(picked.to_rgba(), Srgba::rgb(0.0, 0.0, 0.0).to_rgba(), "the corner is black");
}

/// Every model the panel offers returns its colours: whole-number fields within a step, and a colour CMYK can print comes
/// back as itself once converted.
#[trial]
fn every_panel_model_round_trips_its_colours() {
    let colours =
        [Srgba::rgb(0.2, 0.6, 0.8), Srgba::rgb(1.0, 0.0, 0.0), Srgba::rgb(0.0, 0.0, 0.0), Srgba::rgb(1.0, 1.0, 1.0), Srgba::rgb(0.5, 0.25, 0.75)];
    let worst = |a: Srgba, b: Srgba| (a.r - b.r).abs().max((a.g - b.g).abs()).max((a.b - b.b).abs());
    let rounded = |model: &dyn ColorModel, c: Srgba| {
        let mut v = model.from_srgb(c);
        for (slot, ch) in v.values.iter_mut().zip(model.channels()) {
            let scale = 10f32.powi(i32::from(ch.decimals));
            *slot = (*slot * scale).round() / scale;
        }
        v
    };
    for (model, tolerance) in [(&RgbModel as &dyn ColorModel, 0.6 / 255.0), (&HsbModel, 2.5 / 255.0), (&LabModel, 0.03)] {
        for c in colours {
            let back = model.to_srgb(&rounded(model, c).values);
            assert!(worst(back, c) <= tolerance, "{} {c:?} -> {back:?}", model.id());
        }
    }
    for g in [0.0, 0.25, 0.5, 1.0] {
        let c = Srgba::rgb(g, g, g);
        let back = GrayModel.to_srgb(&rounded(&GrayModel, c).values);
        assert!(worst(back, c) < 0.01, "gray {g} -> {back:?}");
    }
    // Pure red can't print, so CMYK returns the nearest printable colour, which then is stable.
    let cmyk = CmykModel::new(CmykConverter::coated().clone());
    for c in colours {
        let once = cmyk.to_srgb(&rounded(&cmyk, c).values);
        let twice = cmyk.to_srgb(&rounded(&cmyk, once).values);
        assert!(worst(once, twice) <= 0.03, "cmyk {c:?} -> {once:?} -> {twice:?}");
    }

    // Non-finite channel values become 0; finite ones stay.
    let cleaned = finite_values(ModelValues::new("hsb", &[f32::NAN, f32::INFINITY, 40.0]));
    assert_eq!(cleaned.values[..3], [0.0, 0.0, 40.0]);
    let cleaned = finite_values(ModelValues::new("rgb", &[f32::NEG_INFINITY, 12.0, 255.0]));
    assert_eq!(cleaned.values[..3], [0.0, 12.0, 255.0]);
}

fn rgb(name: &str, c: [f32; 3]) -> SwatchEntry {
    SwatchEntry::new(name, SwatchColor::Rgb(c))
}

/// "Basics" with Red and Color Swatch 1, "Pastels" with Pink.
fn library() -> SwatchLibrary {
    SwatchLibrary::new(vec![
        SwatchGroup { name: "Basics".into(), entries: vec![rgb("Red", [1.0, 0.0, 0.0]), rgb("Color Swatch 1", [0.0, 0.0, 1.0])] },
        SwatchGroup { name: "Pastels".into(), entries: vec![rgb("Pink", [1.0, 0.8, 0.8])] },
    ])
}

/// The Swatches panel on a library the test can read back.
struct SwatchesRig {
    fixture: Fixture,
    library: Rc<RefCell<SwatchLibrary>>,
    used: Rc<RefCell<Vec<(SwatchAt, Srgba, bool)>>>,
}

impl SwatchesRig {
    fn new() -> Self {
        let library = Rc::new(RefCell::new(library()));
        let state = Rc::new(RefCell::new(SwatchesState::default()));
        let used: Rc<RefCell<Vec<(SwatchAt, Srgba, bool)>>> = Rc::default();
        let (l, s, u) = (library.clone(), state, used.clone());
        let mut fixture = Fixture::new(move |ui| {
            let (mut lib, mut st) = (l.borrow().clone(), s.borrow().clone());
            let shown =
                SwatchesPanel::new(Bind::bound(&mut lib), Bind::bound(&mut st), Srgba::rgb(0.2, 0.4, 0.6)).cmyk(CmykConverter::coated()).show(ui);
            *l.borrow_mut() = lib;
            *s.borrow_mut() = st;
            u.borrow_mut().extend(shown.used);
        });
        fixture.room(SWATCHES_ROOM);
        fixture.settle();
        SwatchesRig { fixture, library, used }
    }
}

/// A click uses the swatch for the foreground, Alt-click for the background.
#[trial]
fn click_and_alt_click_use_the_swatch() {
    let mut rig = SwatchesRig::new();
    let red = rig.fixture.by_label("Red, #ff0000").expect("the swatch");
    rig.fixture.click(&red);
    rig.fixture.hold(Modifiers::ALT).click(&red);
    rig.fixture.hold(Modifiers::NONE);
    let at = SwatchAt::new(0, 0);
    let color = Srgba::rgb(1.0, 0.0, 0.0);
    assert_eq!(*rig.used.borrow(), [(at, color, false), (at, color, true)]);

    // The clicked swatch reports selected through the access tree and the others do not.
    rig.fixture.settle();
    let selected = |rig: &SwatchesRig, label: &str| {
        let node = rig.fixture.by_label(label).unwrap_or_else(|| panic!("no `{label}`")).node;
        rig.fixture.tree().selected_of(node)
    };
    assert_eq!(selected(&rig, "Red, #ff0000"), Some(true), "the clicked swatch is selected");
    assert_ne!(selected(&rig, "Pink, #ffcccc"), Some(true), "the other swatch is not");
}

/// New Swatch adds the foreground to the selected swatch's group under a free name, and only for the panel whose state
/// holds the selection.
#[trial]
fn new_swatch_adds_the_foreground_with_a_free_name() {
    let words = SwatchesWords::default();
    let foreground = Srgba::rgb(0.2, 0.4, 0.6);
    let mut library = library();
    let mut state = SwatchesState::default();
    state.selected = Some(craft_color::ui::SwatchKey::Swatch("Pink".into()));
    let out = SwatchesMenu::run_command(&mut library, &mut state, SwatchesCommand::NewSwatch, foreground, &words);
    assert!(out.changed);
    assert_eq!(library.groups[0].entries.len(), 2, "the other group is untouched");
    let added = library.groups[1].entries.last().expect("the new swatch");
    assert_eq!(added.name, "Color Swatch 2", "Color Swatch 1 is taken");
    assert_eq!(added.color, SwatchColor::Rgb(foreground.to_rgb_array()));
    assert_eq!(state.selected, Some(craft_color::ui::SwatchKey::Swatch("Color Swatch 2".into())));
    assert_eq!(state.rename, state.selected, "the name opens for editing");

    let mut other = SwatchesState::default();
    let out = SwatchesMenu::run_command(&mut library, &mut other, SwatchesCommand::NewSwatch, foreground, &words);
    assert!(out.changed);
    assert_eq!(library.groups[1].entries.len(), 2, "a state with no selection");
    assert_eq!(library.groups[0].entries.len(), 3, "so the swatch goes to the first group");
}

/// Add to Swatches from the picker keeps typed CMYK inks as a CMYK swatch.
#[trial]
fn add_picked_keeps_the_typed_model() {
    let mut library = library();
    let color = Srgba::rgb(0.5, 0.5, 0.5);
    let cmyk = PickedColor::in_model(color, ModelValues::new("cmyk", &[10.0, 20.0, 30.0, 40.0]));
    let at = library.add_picked(&cmyk).expect("room");
    let entry = library.get(at).expect("added");
    assert_eq!(entry.color, SwatchColor::Cmyk([0.1, 0.2, 0.3, 0.4]));
    let plain = library.add_picked(&PickedColor::rgb(color)).expect("room");
    assert_eq!(library.get(plain).map(|e| &e.color), Some(&SwatchColor::Rgb([0.5, 0.5, 0.5])));
}

/// Import and Replace ask for a swatch file; the answer is read in `apply_file`.
#[trial]
fn import_and_replace_read_the_picked_file() {
    let words = SwatchesWords::default();
    let (presets, format) = (PresetWords::default(), Format::from_tag("en"));
    for (command, replace) in [(SwatchesCommand::Import, false), (SwatchesCommand::Replace, true)] {
        let (mut library, mut state) = (library(), SwatchesState::default());
        let out = SwatchesMenu::run_command(&mut library, &mut state, command, Srgba::rgb(0.2, 0.4, 0.6), &words);
        assert!(!out.changed, "the library changes when the file is read");
        let request = out.file.expect("a file request");
        assert!(matches!(&request, PresetFileRequest::Open { purpose, replace: r, .. } if purpose == "swatches" && *r == replace));
        let event = request.loaded("brand.gpl", b"GIMP Palette\nName: Brand\n#\n  0 128 255 Sky\n".to_vec()).expect("a load event");
        let done = SwatchesPanel::apply_file(&mut library, &mut state, &event, &words, &presets, &format);
        assert!(done.changed && done.report.is_some());
        let names: Vec<&str> = library.groups.iter().flat_map(|g| g.entries.iter().map(|e| e.name.as_str())).collect();
        assert!(names.contains(&"Sky"), "{names:?}");
        assert_eq!(names.contains(&"Red"), !replace, "{names:?}");
    }

    // A file that is not a swatch file is a toast, not a change.
    let (mut lib, mut state) = (library(), SwatchesState::default());
    let out = SwatchesMenu::run_command(&mut lib, &mut state, SwatchesCommand::Import, Srgba::rgb(0.2, 0.4, 0.6), &words);
    let event = out.file.expect("a request").loaded("junk.bin", b"nothing".to_vec()).expect("a load event");
    let done = SwatchesPanel::apply_file(&mut lib, &mut state, &event, &words, &presets, &format);
    assert!(!done.changed);
    let toast = done.toast.expect("a toast");
    assert_eq!(toast.tone, craft_theme::variant::Tone::Bad);
    assert_eq!(lib, library());
}

/// Export is disabled with no swatches, and otherwise gives a save request with the bytes of the chosen format.
#[trial]
fn export_writes_the_chosen_format() {
    // The export entries are disabled with no swatches.
    let enabled = |library: &SwatchLibrary| {
        let states = SwatchesMenu::states(library);
        let state = |command| states.iter().find(|(c, _)| *c == command).map(|(_, s)| s.clone());
        (state(SwatchesCommand::Export), state(SwatchesCommand::ExportExchange), state(SwatchesCommand::Import))
    };
    let on = Some(CommandState::new().enabled(true));
    let off = Some(CommandState::new().enabled(false));
    let empty = SwatchLibrary::new(vec![SwatchGroup { name: "Empty".into(), entries: Vec::new() }]);
    assert_eq!(enabled(&empty), (off.clone(), off, on.clone()));
    assert_eq!(enabled(&library()), (on.clone(), on.clone(), on));

    let words = SwatchesWords::default();
    let (mut lib, mut state) = (library(), SwatchesState::default());
    let out = SwatchesMenu::run_command(&mut lib, &mut state, SwatchesCommand::ExportExchange, Srgba::rgb(0.2, 0.4, 0.6), &words);
    let Some(PresetFileRequest::Save { extensions, file_name, bytes, .. }) = out.file else { panic!("a save request") };
    assert_eq!((extensions.as_slice(), file_name.as_str()), (["ase".to_owned()].as_slice(), "Swatches.ase"));
    assert_eq!(craft_color::palette::SwatchFormat::sniff(&bytes, "x.ase"), Some(SwatchFormat::Ase));
}

/// Each report note makes one note line.
#[trial]
fn each_note_makes_a_line() {
    let (words, format) = (SwatchesWords::default(), Format::from_tag("en"));
    let report = SwatchReport { notes: vec![SwatchNote::BookColors(1), SwatchNote::ConvertedToRgb(3), SwatchNote::GroupsFlattened] };
    assert_eq!(SwatchesPanel::note_lines(&report, true, &words, &format).len(), report.notes.len());
    assert_eq!(SwatchesPanel::note_lines(&SwatchReport { notes: vec![SwatchNote::BookColors(2)] }, false, &words, &format).len(), 1);
}

/// Dragging keeps names unique and the library whole.
#[trial]
fn import_and_moves_keep_names_unique() {
    use craft_color::ui::SwatchKey;
    let mut library = library();
    library.import(vec![SwatchGroup { name: "Basics".into(), entries: vec![rgb("Red", [0.5, 0.0, 0.0])] }], false);
    assert_eq!((library.groups[2].name.as_str(), library.groups[2].entries[0].name.as_str()), ("Basics 1", "Red 2"));
    let swatch = |name: &str| SwatchKey::Swatch(name.into());
    // Rows: Basics, Red, Color Swatch 1, Pastels, Pink, Basics 1, Red 2. Red lands after Pink (position 3 of the
    // rows that remain: Basics, Color Swatch 1, Pastels, Pink).
    assert!(library.apply_reorder(&craft_ui::contract::ReorderMove::new(swatch("Red"), vec![swatch("Red")], 4, false)));
    assert_eq!(library.groups[1].entries.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(), ["Pink", "Red"]);
    assert_eq!(library.total(), 4);
}

/// A swatch added without a group goes to the first group; a full library or group list refuses the next one.
#[trial]
fn add_defaults_to_the_first_group_and_the_library_has_caps() {
    let mut library = library();
    let at = library.add(SwatchColor::Rgb([0.0; 3]), None, None).expect("room");
    assert_eq!(at.group, 0);
    assert_eq!(library.get(at).map(|e| e.name.as_str()), Some("Color Swatch 2"));
    library.groups[1].entries.resize(craft_color::ui::MAX_SWATCHES, SwatchEntry::new("x", SwatchColor::Rgb([0.0; 3])));
    assert!(library.add(SwatchColor::Rgb([0.0; 3]), None, None).is_none(), "full");
    library.groups.resize(craft_color::ui::MAX_GROUPS, SwatchGroup { name: "g".into(), entries: Vec::new() });
    assert!(library.new_group("More").is_none());
}

/// Dragging a marker reports the move of that colour; pressing selects it.
#[trial]
fn dragging_a_marker_moves_its_colour() {
    type Moves = Rc<RefCell<Vec<(usize, EditEvent<[Option<f32>; 3]>)>>>;
    let colors = [Srgba::rgb(0.9, 0.2, 0.2), Srgba::rgb(0.2, 0.7, 0.4)];
    let (pressed, moved): (Rc<RefCell<Vec<usize>>>, Moves) = Default::default();
    let (p, m) = (pressed.clone(), moved.clone());
    let mut fixture = Fixture::new(move |ui| {
        let shown = HarmonyWheel::new("Harmony", &colors).selected_marker(Some(0)).show(ui);
        p.borrow_mut().extend(shown.pressed);
        m.borrow_mut().extend(shown.edits);
    });
    fixture.settle();
    let marker = fixture.by_label("#33b366").expect("the marker");
    fixture.drag(&marker, Vec2::X * -marker.rect.width());
    assert!(pressed.borrow().contains(&1), "{:?}", pressed.borrow());
    let moved = moved.borrow();
    assert!(!moved.is_empty() && moved.iter().all(|(i, _)| *i == 1), "{moved:?}");
    assert!(moved.iter().filter_map(|(_, e)| e.value()).all(|[h, s, b]| h.is_some() && s.is_some() && b.is_none()), "{moved:?}");
    assert!(matches!(moved.first(), Some((_, EditEvent::Begin { .. }))), "the drag is a gesture: {moved:?}");
    assert!(matches!(moved.last(), Some((_, EditEvent::Commit { .. }))), "{moved:?}");
}

/// A click on a cell of the guide chooses its colour.
#[trial]
fn a_grid_click_chooses_its_colour() {
    let base = Srgba::rgb(0.9, 0.4, 0.1);
    let options = HarmonyPanelState::default().options;
    let chosen: Rc<RefCell<Vec<Srgba>>> = Rc::default();
    let c = chosen.clone();
    let mut start = HarmonyPanelState::default();
    start.rule = craft_color::harmony::Harmony::Complementary;
    let state = Rc::new(RefCell::new(start));
    let picked = Rc::new(RefCell::new(HarmonyPicked::default()));
    let (s, p) = (state.clone(), picked.clone());
    let mut fixture = Fixture::new(move |ui| {
        let (mut st, mut pk) = (*s.borrow(), p.borrow().clone());
        let shown = HarmonyPanel::new(Bind::bound(&mut st), Bind::bound(&mut pk), base).show(ui);
        *s.borrow_mut() = st;
        *p.borrow_mut() = pk;
        c.borrow_mut().extend(shown.chosen);
    });
    fixture.room(GUIDE_ROOM);
    fixture.settle();
    let guide = craft_color::harmony::Guide::new(base, craft_color::harmony::Harmony::Complementary, &options);
    let centre = guide.grid[1][guide.centre()];
    let cells = fixture.by_role(Role::LIST_BOX_OPTION);
    let hex = centre.to_hex();
    let target = cells.into_iter().filter(|cell| fixture.tree().name_of(cell.node) == Some(hex.as_str())).last().expect("a cell");
    fixture.click(&target);
    assert_eq!(*chosen.borrow(), [centre]);
    assert_eq!(
        HarmonyMenu::states(&state.borrow(), &picked.borrow()).iter().find(|(c, _)| *c == HarmonyCommand::SaveAsSwatches).map(|(_, s)| s.clone()),
        Some(CommandState::new().enabled(true)),
        "the clicked cell is chosen for Save Colors as Swatches"
    );
    assert_eq!(picked.borrow().colors(), [centre]);
}

/// A table of six entries, indexes 1 and 3 the same colour.
struct TableRig {
    fixture: Fixture,
    selected: Rc<RefCell<Selection<usize>>>,
    opened: Rc<RefCell<Vec<usize>>>,
}

impl TableRig {
    fn new() -> Self {
        let duplicate = Srgba::rgb(0.2, 0.4, 0.8);
        let colors =
            [Srgba::rgb(1.0, 0.0, 0.0), duplicate, Srgba::rgb(0.0, 1.0, 0.0), duplicate, Srgba::rgb(0.0, 0.0, 0.0), Srgba::rgb(1.0, 1.0, 1.0)];
        let entries: Vec<(Label, Srgba)> = colors.into_iter().enumerate().map(|(i, c)| (Label::from(format!("Index {i}")), c)).collect();
        let selected = Rc::new(RefCell::new(Selection::default()));
        let opened: Rc<RefCell<Vec<usize>>> = Rc::default();
        let (s, o) = (selected.clone(), opened.clone());
        let mut fixture = Fixture::new(move |ui| {
            let mut current = s.borrow().clone();
            let shown = SwatchTable::new("Color Table", &entries).selection(Bind::bound(&mut current)).show_all(ui);
            *s.borrow_mut() = current;
            o.borrow_mut().extend(shown.open);
        });
        fixture.settle();
        TableRig { fixture, selected, opened }
    }
}

/// A click selects the clicked index among duplicate colours; a double click and Enter ask to open the index; Space
/// selects the focused one.
#[trial]
fn the_table_click_double_click_enter_and_space() {
    let mut rig = TableRig::new();
    let cell = rig.fixture.by_label("Index 3").expect("the cell");
    rig.fixture.click(&cell);
    assert_eq!(rig.selected.borrow().items, [3]);
    assert!(rig.opened.borrow().is_empty(), "a click edits nothing");

    let mut rig = TableRig::new();
    let cell = rig.fixture.by_label("Index 2").expect("the cell");
    rig.fixture.click(&cell).click(&cell);
    assert_eq!(*rig.opened.borrow(), [2], "the second click of a double click opens");

    rig.fixture.key(Key::Tab, Modifiers::NONE).key(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(rig.selected.borrow().items, [2], "the arrows select nothing");
    rig.fixture.key(Key::Space, Modifiers::NONE);
    assert_eq!(rig.selected.borrow().items, [3], "Space selects the focused cell");
    rig.fixture.key(Key::ArrowRight, Modifiers::NONE);
    rig.fixture.key(Key::Enter, Modifiers::NONE);
    assert_eq!(*rig.opened.borrow(), [2, 4], "Enter opens the focused index");
    assert_eq!(rig.selected.borrow().items, [3], "Enter selects nothing");
}

/// Enter on a cell in a dialog opens the entry without answering OK: the table takes the key.
#[trial]
fn enter_on_a_cell_in_a_dialog_opens_the_entry_without_answering_ok() {
    use craft_ui::contract::ShowContainer as _;
    use craft_ui::dialog::Dialog;
    let entries: Vec<(Label, Srgba)> = (0..4).map(|i| (Label::from(format!("Index {i}")), Srgba::rgb(i as f32 / 4.0, 0.2, 0.5))).collect();
    let opened: Rc<RefCell<Vec<usize>>> = Rc::default();
    let answers: Rc<RefCell<Vec<craft_ui::runtime::overlay::CloseReason>>> = Rc::default();
    let (o, a) = (opened.clone(), answers.clone());
    let mut fixture = Fixture::new(move |ui| {
        let shown = Dialog::new("Colors").show(ui, |ui, _| SwatchTable::new("Color Table", &entries).show_all(ui));
        o.borrow_mut().extend(shown.inner.and_then(|table| table.open));
        a.borrow_mut().extend(shown.output.closed);
    });
    fixture.settle();
    let cell = fixture.by_label("Index 1").expect("the cell");
    fixture.click(&cell).key(Key::Enter, Modifiers::NONE);
    assert_eq!(*opened.borrow(), [1], "Enter opens the focused entry");
    assert!(answers.borrow().is_empty(), "the dialog gets no Enter");
}

const SWATCHES: [(&str, [f32; 3]); 6] = [
    ("Red", [0.90, 0.28, 0.30]),
    ("Orange", [0.97, 0.42, 0.08]),
    ("Green", [0.19, 0.64, 0.42]),
    ("Teal", [0.07, 0.65, 0.58]),
    ("Blue", [0.24, 0.39, 0.87]),
    ("Violet", [0.56, 0.31, 0.78]),
];

/// The hex of the `SWATCHES` colour named `name`.
fn swatch_hex(name: &str) -> String {
    let (_, [r, g, b]) = SWATCHES.iter().find(|(n, _)| *n == name).expect("a listed swatch");
    Srgba::rgb(*r, *g, *b).to_hex()
}

/// A palette over `SWATCHES` choosing the colour the test sets.
struct PaletteRig {
    fixture: Fixture,
    color: Rc<Cell<Srgba>>,
    events: Rc<Cell<usize>>,
}

impl PaletteRig {
    fn new(start: Srgba, room: Vec2) -> Self {
        let swatches: Vec<(Label, Srgba)> = SWATCHES.iter().map(|(name, [r, g, b])| (Label::from(*name), Srgba::rgb(*r, *g, *b))).collect();
        let color = Rc::new(Cell::new(start));
        let events = Rc::new(Cell::new(0));
        let (c, e) = (color.clone(), events.clone());
        let mut fixture = Fixture::new(move |ui| {
            let mut current = c.get();
            let shown = SwatchPalette::new("Swatches", Bind::bound(&mut current), &swatches).show(ui);
            // A second Tab stop for the Tab that leaves the palette to land on.
            let _ = craft_ui::buttons::Button::new("After").show(ui);
            c.set(current);
            e.set(e.get() + usize::from(shown.edit.is_some()));
        });
        fixture.room(room);
        fixture.settle();
        PaletteRig { fixture, color, events }
    }

    fn focused(&self) -> Option<String> {
        let tree = self.fixture.tree();
        tree.name_of(craft_ui::testing::access::focused_key(tree)).map(str::to_owned)
    }
}

/// The palette is a pick collection: a click, Space or Enter chooses, the arrows only move the focus.
#[trial]
fn the_palette_chooses_by_click_space_or_enter_and_moves_focus_by_arrows() {
    let green = Srgba::rgb(0.19, 0.64, 0.42);
    let mut rig = PaletteRig::new(green, PALETTE_WIDE);
    assert_eq!(rig.fixture.by_role(Role::LIST_BOX_OPTION).len(), SWATCHES.len());

    // One Tab stop: Tab enters on the colour in use and the next Tab leaves.
    rig.fixture.key(Key::Tab, Modifiers::NONE);
    assert_eq!(rig.focused().as_deref(), Some("Green"));
    rig.fixture.key(Key::Tab, Modifiers::NONE);
    assert_ne!(rig.focused().as_deref(), Some("Green"), "Tab leaves the palette");
    rig.fixture.key(Key::Tab, Modifiers::SHIFT);
    assert_eq!(rig.focused().as_deref(), Some("Green"), "Shift+Tab comes back to the selected swatch");

    // Arrows move through the swatches without selecting; Home and End go to the ends.
    rig.fixture.key(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(rig.focused().as_deref(), Some("Teal"));
    rig.fixture.key(Key::Home, Modifiers::NONE);
    assert_eq!(rig.focused().as_deref(), Some("Red"));
    rig.fixture.key(Key::End, Modifiers::NONE);
    assert_eq!(rig.focused().as_deref(), Some("Violet"));
    rig.fixture.key(Key::ArrowLeft, Modifiers::NONE);
    assert_eq!(rig.focused().as_deref(), Some("Blue"));
    assert_eq!(rig.events.get(), 0, "no selection by moving");
    rig.fixture.key(Key::Space, Modifiers::NONE);
    assert_eq!(rig.events.get(), 1, "Space selects the focused swatch: one Set");

    let orange = rig.fixture.by_label("Orange").expect("the swatch");
    rig.fixture.click(&orange);
    assert_eq!(rig.events.get(), 2, "a click is one Set");
    assert_eq!(rig.color.get().to_hex(), swatch_hex("Orange"));
    rig.fixture.key(Key::ArrowRight, Modifiers::NONE);
    rig.fixture.key(Key::Enter, Modifiers::NONE);
    assert_eq!(rig.events.get(), 3, "Enter chooses the focused swatch: one Set");
    assert_eq!(rig.color.get().to_hex(), swatch_hex("Green"));

    // A screen reader's focus on a swatch moves the palette's stop there and selects nothing.
    use craft_ui::testing::focus::AccessActionKind;
    let mut rig = PaletteRig::new(Srgba::rgb(0.9, 0.28, 0.3), PALETTE_WIDE);
    let teal = rig.fixture.by_label("Teal").expect("the swatch");
    rig.fixture.access(AccessActionKind::Focus, &teal);
    assert_eq!(rig.focused().as_deref(), Some("Teal"));
    assert_eq!(rig.events.get(), 0, "focusing selects nothing");
    rig.fixture.key(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(rig.focused().as_deref(), Some("Blue"), "the arrows go on from there");
}

/// Up and Down walk the wrapped rows of the palette to the swatch in the same column.
#[trial]
fn the_palette_arrows_walk_the_wrapped_grid() {
    // Three to a row: Red Orange Green / Teal Blue Violet. The room is three cells and the gaps the wide rig lays out.
    let wide = PaletteRig::new(Srgba::rgb(0.19, 0.64, 0.42), PALETTE_WIDE);
    let (first, third) = (wide.fixture.by_label("Red").expect("red").rect, wide.fixture.by_label("Green").expect("green").rect);
    let narrow = Vec2::new(third.max.x + first.min.x, PALETTE_WIDE.y);
    let mut rig = PaletteRig::new(Srgba::rgb(0.19, 0.64, 0.42), narrow);
    let (red, teal) = (rig.fixture.by_label("Red").expect("red").rect, rig.fixture.by_label("Teal").expect("teal").rect);
    assert!(teal.min.y > red.min.y, "the palette wrapped");
    rig.fixture.key(Key::Tab, Modifiers::NONE);
    let steps = [
        (Key::ArrowDown, "Violet"),
        (Key::ArrowUp, "Green"),
        (Key::ArrowUp, "Green"),
        (Key::ArrowRight, "Teal"),
        (Key::ArrowLeft, "Green"),
        (Key::ArrowLeft, "Orange"),
        (Key::ArrowDown, "Blue"),
        (Key::ArrowRight, "Violet"),
        (Key::ArrowRight, "Violet"),
    ];
    for (key, expected) in steps {
        rig.fixture.key(key, Modifiers::NONE);
        assert_eq!(rig.focused().as_deref(), Some(expected), "{key:?}");
    }
    assert_eq!(rig.events.get(), 0);
}
