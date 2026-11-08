//! The Crop options: the settings model, the options bar and the overlay and gear menus.

use craft_behavior::geometry::crop::{CropGuide, GuideShow};
use craft_draw::ui::crop::{
    CROP_RATIOS, CROP_SIZES, CropBar, CropBarWords, CropDocKey, CropDocument, CropFill, CropOptionsMenu, CropOverlayMenu, CropPresetChoice,
    CropRatio, CropRatioChange, CropSettings, CropSizeFields, CropUserPreset, ResolutionUnit,
};
use craft_ui::bars::{Bar, BarKind};
use craft_ui::canvas::ShieldColor;
use craft_ui::contract::{Role, ShowContainer as _};
use craft_ui::runtime::env::{Env, Units, WordSet as _};
use craft_ui::testing::{Fixture, Frames, trial};
use egui::{Key, Modifiers, Vec2};
use kurbo::Size;
use std::cell::RefCell;
use std::rc::Rc;

fn document() -> CropDocument {
    CropDocument::new(CropDocKey(1), Size::new(2400.0, 1500.0)).resolution(72.0)
}

/// Runs `check` with the units of the default scope.
fn with_units(check: impl FnOnce(&Units)) {
    let mut check = Some(check);
    Frames::new().frame(vec![], |ui| {
        if let Some(check) = check.take() {
            check(&Env::of(ui).units());
        }
    });
}

fn size_mode(width: &str, height: &str, resolution: &str) -> CropSettings {
    let mut settings = CropSettings::default();
    settings.ratio = CropRatio::Size;
    settings.size.width = width.into();
    settings.size.height = height.into();
    settings.size.resolution = resolution.into();
    settings
}

fn sides(width: f64, height: f64) -> CropSettings {
    let mut settings = CropSettings::default();
    settings.ratio = CropRatio::Sides { width, height };
    settings.ratio_fields = [width, height];
    settings
}

/// Choosing each ratio preset sets the ratio and its fields; each size preset fills W, H and the resolution in px/in
/// and turns W x H x Resolution on; `choice` shows them back.
#[trial]
fn ratio_presets_and_size_presets_fill_the_fields() {
    with_units(|units| {
        let doc = document();
        let mut settings = CropSettings::default();
        for (i, ratio) in CROP_RATIOS.iter().enumerate() {
            settings.choose(CropPresetChoice::Free, &doc, &[], units);
            settings.choose(CropPresetChoice::Ratio(i), &doc, &[], units);
            assert_eq!(settings.ratio, *ratio);
            if let CropRatio::Sides { width, height } = ratio {
                assert_eq!(settings.ratio_fields, [*width, *height]);
            }
            assert_eq!(settings.choice(&[]), CropPresetChoice::Ratio(i));
        }
        for (i, preset) in CROP_SIZES.iter().enumerate() {
            settings.size.resolution_unit = ResolutionUnit::PixelsPerCentimetre;
            assert!(settings.choose(CropPresetChoice::SizePreset(i), &doc, &[], units));
            assert_eq!(settings.ratio, CropRatio::Size);
            assert_eq!(
                (settings.size.width.as_str(), settings.size.height.as_str(), settings.size.resolution.as_str(), settings.size.resolution_unit),
                (preset.width, preset.height, preset.resolution, ResolutionUnit::PixelsPerInch)
            );
            assert_eq!(settings.choice(&[]), CropPresetChoice::SizePreset(i));
        }
        assert!(!settings.choose(CropPresetChoice::NewPreset, &doc, &[], units));
        assert!(settings.choose(CropPresetChoice::Free, &doc, &[], units));
        assert_eq!(settings.choice(&[]), CropPresetChoice::Free);

        let user = [CropUserPreset::new("Cinema", CropRatio::Sides { width: 21.0, height: 9.0 }, CropSizeFields::default())];
        assert!(settings.choose(CropPresetChoice::User(0), &doc, &user, units));
        assert_eq!(settings.ratio_fields, [21.0, 9.0]);
        assert_eq!(settings.choice(&user), CropPresetChoice::User(0));
    });
}

/// "4 in", "4in" and "10.5 cm" parse; a bare number keeps the field's unit (pixels when empty); zero, negative,
/// percent and unknown units do not, and normalising keeps the previous text for them.
#[trial]
fn lengths_parse_with_units_and_a_bare_number_keeps_the_field_unit() {
    with_units(|units| {
        let norm = |typed: &str, previous: &str| CropSizeFields::normalize_length(typed, previous, units);
        assert_eq!(norm("4 in", ""), "4 in");
        assert_eq!(norm("4in", ""), "4 in");
        assert_eq!(norm("10.5 cm", ""), "10.5 cm");
        assert_eq!(norm("1024", ""), "1024 px");
        assert_eq!(norm("6", "4 in"), "6 in");
        assert_eq!(norm("  ", "4 in"), "");
        for junk in ["0", "-3", "50%", "4 furlongs", "abc"] {
            assert_eq!(norm(junk, "4 in"), "4 in", "{junk}");
        }
        assert_eq!(CropSizeFields::normalize_resolution("300.5", "72"), "300.5");
        assert_eq!(CropSizeFields::normalize_resolution("0", "72"), "72");
        assert_eq!(CropSizeFields::normalize_resolution("", "72"), "");
    });
}

/// `target` converts physical lengths at the typed resolution (px/cm x 2.54), else at the document's; points at the
/// document's points per inch.
#[trial]
fn target_uses_the_typed_resolution_else_the_documents() {
    with_units(|units| {
        let doc = document().resolution(100.0);
        let at_typed = size_mode("4 in", "6 in", "300").target(&doc, units).unwrap();
        assert_eq!((at_typed.width, at_typed.height, at_typed.resolution), (1200.0, 1800.0, Some(300.0)));
        let at_document = size_mode("4 in", "1024", "").target(&doc, units).unwrap();
        assert_eq!((at_document.width, at_document.height, at_document.resolution), (400.0, 1024.0, None));
        let mut per_cm = size_mode("2.54 cm", "1 in", "100");
        per_cm.size.resolution_unit = ResolutionUnit::PixelsPerCentimetre;
        let t = per_cm.target(&doc, units).unwrap();
        assert!((t.width - 254.0).abs() < 1e-9 && (t.height - 254.0).abs() < 1e-9 && t.resolution == Some(254.0));
        let points = size_mode("72 pt", "1", "").target(&doc.points_per_inch(72.27), units).unwrap();
        assert!((points.width - 100.0 * 72.0 / 72.27).abs() < 1e-9);
        assert!((size_mode("4 in", "6 in", "").ratio_value(&doc, units).unwrap() - 4.0 / 6.0).abs() < 1e-12);
    });
}

/// W or H empty: no target and no ratio; one ratio side typed: a free ratio that keeps the side.
#[trial]
fn empty_w_or_h_is_a_free_crop() {
    with_units(|units| {
        let doc = document();
        let half = size_mode("4 in", "", "300");
        assert_eq!((half.target(&doc, units), half.ratio_value(&doc, units)), (None, None));
        let mut settings = CropSettings::default();
        settings.set_ratio_field(0, 3.0);
        assert_eq!((settings.ratio, settings.ratio_fields, settings.ratio_value(&doc, units)), (CropRatio::Free, [3.0, 0.0], None));
        assert_eq!(settings.shown_ratio_fields(&doc), [3.0, 0.0]);
        settings.set_ratio_field(1, 1.41437);
        assert_eq!(settings.ratio, CropRatio::Sides { width: 3.0, height: 1.414 });
        settings.set_ratio_field(0, 0.0);
        assert_eq!((settings.ratio, settings.ratio_fields), (CropRatio::Free, [0.0, 1.414]));
    });
}

/// Swap exchanges W and H in W x H x Resolution mode and the ratio's sides otherwise (Original turns the document's
/// size); Front Image fills the document's size in px and its resolution in the field's unit.
#[trial]
fn swap_and_front_image() {
    with_units(|units| {
        let doc = document();
        let mut settings = size_mode("4 in", "6 px", "300");
        assert!(settings.swap(&doc));
        assert_eq!((settings.size.width.as_str(), settings.size.height.as_str()), ("6 px", "4 in"));

        let mut ratio = sides(16.0, 9.0);
        assert!(ratio.swap(&doc));
        assert_eq!(ratio.ratio, CropRatio::Sides { width: 9.0, height: 16.0 });
        let mut original = CropSettings::default();
        original.ratio = CropRatio::Original;
        assert!(original.swap(&doc));
        assert_eq!(original.ratio, CropRatio::Sides { width: 1500.0, height: 2400.0 });
        assert!(!CropSettings::default().swap(&doc));

        let mut front = CropSettings::default();
        front.size.resolution_unit = ResolutionUnit::PixelsPerCentimetre;
        front.front_image(&document().resolution(254.0), units);
        assert_eq!(front.ratio, CropRatio::Size);
        assert_eq!((front.size.width.as_str(), front.size.height.as_str(), front.size.resolution.as_str()), ("2400 px", "1500 px", "100"));
    });
}

/// "4 x 5 in 300 ppi", "2 : 3", mixed units, a px/cm resolution; `None` for Original and a free ratio.
#[trial]
fn preset_name_is_made_from_the_values() {
    with_units(|units| {
        let words = CropBarWords::english();
        let name = |settings: &CropSettings| settings.preset_name(&words, units);
        assert_eq!(name(&size_mode("4 in", "5 in", "300")).as_deref(), Some("4 x 5 in 300 ppi"));
        assert_eq!(name(&size_mode("8.5 in", "11 in", "")).as_deref(), Some("8.5 x 11 in"));
        assert_eq!(name(&size_mode("4 in", "300", "")).as_deref(), Some("4 in x 300 px"));
        let mut per_cm = size_mode("1024", "768", "35.5");
        per_cm.size.resolution_unit = ResolutionUnit::PixelsPerCentimetre;
        assert_eq!(name(&per_cm).as_deref(), Some("1024 x 768 px 35.5 px/cm"));
        assert_eq!(name(&size_mode("4 in", "", "300")), None);
        assert_eq!(name(&sides(2.0, 3.0)).as_deref(), Some("2 : 3"));
        let mut original = CropSettings::default();
        original.ratio = CropRatio::Original;
        assert_eq!((name(&original), name(&CropSettings::default())), (None, None));
    });
}

/// An empty or partial serialised `CropSettings` loads with every other field at its default (Delete Cropped Pixels
/// on, Rule of Thirds, Auto).
#[trial]
fn older_settings_load_with_defaults() {
    let empty: CropSettings = serde_json::from_str("{}").unwrap();
    assert_eq!(empty, CropSettings::default());
    assert!(empty.delete_pixels);
    assert_eq!((empty.guide, empty.guide_show), (CropGuide::Thirds, GuideShow::Auto));
    let partial: CropSettings = serde_json::from_str(r#"{"fill":"contentAware","ratio":{"sides":{"width":4.0,"height":5.0}}}"#).unwrap();
    assert_eq!((partial.fill, partial.ratio), (CropFill::ContentAware, CropRatio::Sides { width: 4.0, height: 5.0 }));
    assert!(partial.delete_pixels);
    let round_trip: CropSettings = serde_json::from_str(&serde_json::to_string(&partial).unwrap()).unwrap();
    assert_eq!(round_trip, partial);
}

/// What one pass of the bar reported.
#[derive(Debug, PartialEq)]
struct Seen {
    ratio: Option<CropRatioChange>,
    new_preset: bool,
    delete_preset: bool,
    changed: bool,
    colour: bool,
}

struct Rig {
    fixture: Fixture,
    settings: Rc<RefCell<CropSettings>>,
    seen: Rc<RefCell<Vec<Seen>>>,
}

fn bar(settings: CropSettings, users: Vec<CropUserPreset>) -> Rig {
    let state = Rc::new(RefCell::new(settings));
    let seen = Rc::new(RefCell::new(Vec::new()));
    let (shared, log) = (Rc::clone(&state), Rc::clone(&seen));
    let mut fixture = Fixture::new(move |ui| {
        let mut settings = shared.borrow().clone();
        let doc = document();
        Bar::new("Options", BarKind::Options).show(ui, |ui, _| {
            let out = CropBar::new(&mut settings, &doc).user_presets(&users).show(ui);
            if out.ratio.is_some() || out.new_preset || out.delete_preset || out.settings_changed || out.pick_color.is_some() {
                log.borrow_mut().push(Seen {
                    ratio: out.ratio,
                    new_preset: out.new_preset,
                    delete_preset: out.delete_preset,
                    changed: out.settings_changed,
                    colour: out.pick_color.is_some(),
                });
            }
        });
        *shared.borrow_mut() = settings;
    });
    fixture.room(Vec2::new(1800.0, 1400.0)).settle();
    Rig { fixture, settings: state, seen }
}

impl Rig {
    fn click(&mut self, label: &str) {
        let target = self.fixture.by_label(label).unwrap_or_else(|| panic!("{label}"));
        self.fixture.click(&target).settle();
    }

    fn pick(&mut self, label: &str) {
        self.click("Crop ratio");
        self.click(label);
    }

    fn disabled(&self, label: &str) -> bool {
        let target = self.fixture.by_label(label).unwrap_or_else(|| {
            let texts: Vec<_> = self.fixture.texts().iter().map(|text| text.text.clone()).collect();
            panic!("{label} is not shown; the texts are {texts:?}")
        });
        self.fixture.tree().is_disabled_key(target.node)
    }
}

/// New Crop Preset… and Delete Crop Preset… report their events; a preset pick and a ratio field step change the
/// settings and give `ratio`; the swap button exchanges the sides.
#[trial]
fn the_bar_reports_new_and_delete_and_edits_options() {
    let user = CropUserPreset::new("Poster", CropRatio::Sides { width: 3.0, height: 7.0 }, CropSizeFields::default());
    let mut rig = bar(sides(2.0, 3.0), vec![user]);
    rig.pick("New Crop Preset…");
    rig.pick("Delete Crop Preset…");
    let flags: Vec<_> = rig.seen.borrow().iter().map(|s| (s.new_preset, s.delete_preset)).collect();
    assert_eq!(flags, vec![(true, false), (false, true)]);

    rig.seen.borrow_mut().clear();
    rig.pick("5 : 7");
    assert_eq!(rig.settings.borrow().ratio, CropRatio::Sides { width: 5.0, height: 7.0 });
    assert!(rig.seen.borrow().iter().any(|s| s.ratio.is_some() && s.changed), "a preset pick edits the settings");

    // An arrow key steps the focused width field.
    rig.seen.borrow_mut().clear();
    let mut fields = rig.fixture.by_role(Role::SPIN_BUTTON);
    fields.sort_by(|a, b| a.rect.min.x.total_cmp(&b.rect.min.x));
    rig.fixture.click(&fields[0]);
    rig.fixture.key(Key::ArrowUp, Modifiers::NONE).settle();
    assert_eq!(rig.settings.borrow().ratio, CropRatio::Sides { width: 6.0, height: 7.0 });
    assert!(rig.seen.borrow().iter().any(|s| s.ratio.is_some()));

    rig.click("Swaps height and width");
    assert_eq!(rig.settings.borrow().ratio, CropRatio::Sides { width: 7.0, height: 6.0 });
}

/// The gear menu's Custom… reports a colour request for the shield colour through the bar.
#[trial]
fn the_bar_reports_the_shield_color_request() {
    let mut settings = CropSettings::default();
    settings.shield.custom_color = [255, 0, 51];
    let mut rig = bar(settings, Vec::new());
    rig.click("Set additional Crop options");
    rig.click("Custom…");
    assert!(rig.seen.borrow().iter().any(|s| s.colour), "a colour request");
    assert_eq!(rig.settings.borrow().shield.color, ShieldColor::Custom);
    assert!(rig.seen.borrow().iter().any(|s| s.changed));
    assert_eq!(CropOptionsMenu::COLOR_KEY, "crop-shield-color");
}

/// W x H x Resolution shows the W, H and Resolution fields with the unit select; any ratio shows the two ratio fields
/// with the swap button; Original shows the document's size.
#[trial]
fn the_bar_shows_size_fields_in_size_mode_and_ratio_fields_otherwise() {
    let rig = bar(size_mode("4 in", "5 in", "300"), Vec::new());
    assert!(rig.fixture.by_label("Width").is_some() && rig.fixture.by_label("Height").is_some());
    let resolution = rig.fixture.by_label("Resolution").expect("a resolution field");
    assert!(rig.fixture.tree().value_of(resolution.node).is_some_and(|v| v.contains("300")));
    assert!(rig.fixture.by_label("Resolution unit").is_some());
    assert!(rig.fixture.by_label("Swaps height and width").is_some());

    let rig = bar(sides(16.0, 9.0), Vec::new());
    assert!(rig.fixture.by_label("Resolution").is_none());
    assert!(rig.fixture.by_label("Swaps height and width").is_some());

    let mut original = CropSettings::default();
    original.ratio = CropRatio::Original;
    let rig = bar(original, Vec::new());
    let digits = |label: &str| {
        let field = rig.fixture.by_label(label)?;
        rig.fixture.tree().value_of(field.node).map(|v| v.chars().filter(char::is_ascii_digit).collect::<String>())
    };
    assert_eq!(digits("Width").as_deref(), Some("2400"));
    assert_eq!(digits("Height").as_deref(), Some("1500"));
}

/// New Crop Preset… is disabled for Original and a free ratio, Delete Crop Preset… without user presets.
#[trial]
fn new_and_delete_are_disabled_with_nothing_to_save_or_delete() {
    let mut original = CropSettings::default();
    original.ratio = CropRatio::Original;
    let mut rig = bar(original, Vec::new());
    rig.click("Crop ratio");
    assert!(rig.disabled("New Crop Preset…") && rig.disabled("Delete Crop Preset…"));

    let user = CropUserPreset::new("Poster", CropRatio::Sides { width: 3.0, height: 7.0 }, CropSizeFields::default());
    let mut rig = bar(CropSettings::default(), vec![user]);
    rig.click("Crop ratio");
    assert!(rig.disabled("New Crop Preset…") && !rig.disabled("Delete Crop Preset…"));

    let mut rig = bar(sides(2.0, 3.0), Vec::new());
    rig.click("Crop ratio");
    assert!(!rig.disabled("New Crop Preset…") && rig.disabled("Delete Crop Preset…"));
}

enum Menu {
    Overlay,
    Gear,
}

fn menu(settings: CropSettings, which: Menu) -> Rig {
    let state = Rc::new(RefCell::new(settings));
    let seen = Rc::new(RefCell::new(Vec::new()));
    let (shared, log) = (Rc::clone(&state), Rc::clone(&seen));
    let mut fixture = Fixture::new(move |ui| {
        let mut settings = shared.borrow().clone();
        match which {
            Menu::Overlay => {
                CropOverlayMenu::new(&mut settings).show(ui);
            }
            Menu::Gear => {
                let out = CropOptionsMenu::new(&mut settings).show(ui);
                log.borrow_mut().push(Seen {
                    ratio: None,
                    new_preset: false,
                    delete_preset: false,
                    changed: out.changed,
                    colour: out.pick_color.is_some(),
                });
            }
        }
        *shared.borrow_mut() = settings;
    });
    fixture.room(Vec2::new(480.0, 1400.0)).settle();
    Rig { fixture, settings: state, seen }
}

/// A row of the overlay menu sets the overlay or the show rule and closes the menu; Cycle Orientation is disabled for
/// Rule of Thirds and enabled for Triangle.
#[trial]
fn overlay_menu_sets_the_overlay_and_show_rule() {
    let mut rig = menu(CropSettings::default(), Menu::Overlay);
    rig.click("Set the overlay options for the Crop tool");
    assert!(rig.disabled("Cycle Orientation"));
    rig.click("Always Show Overlay");
    assert_eq!(rig.settings.borrow().guide_show, GuideShow::Always);
    assert!(rig.fixture.by_label("Cycle Overlay").is_none(), "a row closes the menu");

    rig.click("Set the overlay options for the Crop tool");
    rig.click("Triangle");
    assert_eq!(rig.settings.borrow().guide, CropGuide::Triangle);

    rig.click("Set the overlay options for the Crop tool");
    assert!(!rig.disabled("Cycle Orientation"));
    rig.click("Cycle Orientation");
    assert_eq!(rig.settings.borrow().guide_orientation, 1);
}

/// With Classic Mode on, Show Cropped Area and Auto Center Preview are disabled; Custom… reports a colour request and
/// the popover stays open.
#[trial]
fn gear_menu_greys_two_rows_in_classic_mode() {
    let mut settings = CropSettings::default();
    settings.shield.classic_mode = true;
    let mut rig = menu(settings, Menu::Gear);
    rig.click("Set additional Crop options");
    assert!(rig.disabled("Show Cropped Area"), "Show Cropped Area");
    assert!(rig.disabled("Auto Center Preview"), "Auto Center Preview");
    assert!(!rig.disabled("Use Classic Mode"));
    assert!(!rig.seen.borrow().iter().any(|s| s.colour));

    rig.click("Custom…");
    assert!(rig.seen.borrow().iter().any(|s| s.colour));
    assert!(rig.fixture.by_label("Use Classic Mode").is_some(), "the popover stays open on a click inside");
}
