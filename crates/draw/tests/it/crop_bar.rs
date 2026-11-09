//! The Crop options: the settings model, the options bar and the overlay and gear menus.

use craft_behavior::geometry::crop::{CropGuide, GuideShow};
use craft_draw::ui::crop::{
    CROP_RATIOS, CROP_SIZES, CropBar, CropBarWords, CropDocKey, CropDocument, CropFill, CropLength, CropOptionsMenu, CropOverlayMenu,
    CropPresetChoice, CropRatio, CropRatioChange, CropSettings, CropSizeFields, CropUserPreset, LENGTH_MIN, RATIO_STEP, ResolutionUnit,
};
use craft_ui::bars::{Bar, BarKind};
use craft_ui::canvas::ShieldColor;
use craft_ui::contract::{Role, ShowContainer as _};
use craft_ui::runtime::env::{Env, PointSize, RootEnv, UnitKind, Units, WordSet as _};
use craft_ui::testing::{Fixture, Frames, trial};
use egui::{Key, Modifiers, Vec2};
use kurbo::Size;
use std::cell::RefCell;
use std::rc::Rc;

fn document() -> CropDocument {
    CropDocument::new(CropDocKey::new(1), Size::new(2400.0, 1500.0), 72.0)
}

/// The units of a scope whose point size is `points`.
fn units_of(points: PointSize) -> Units {
    let mut frames = Frames::new();
    let mut root = RootEnv::default();
    root.values.set(points);
    frames.root(root);
    let mut units = None;
    frames.frame(vec![], |ui| units = Some(Env::of(ui).units()));
    units.expect("the frame ran")
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

fn inches(value: f64) -> Option<CropLength> {
    Some(CropLength::new(value, UnitKind::Inches))
}

fn pixels(value: f64) -> Option<CropLength> {
    Some(CropLength::new(value, UnitKind::Pixels))
}

fn size_mode(width: Option<CropLength>, height: Option<CropLength>, resolution: &str) -> CropSettings {
    let mut settings = CropSettings::default();
    settings.ratio = CropRatio::Size;
    settings.size.width = width;
    settings.size.height = height;
    settings.size.resolution = resolution.into();
    settings
}

fn sides(width: f64, height: f64) -> CropSettings {
    let mut settings = CropSettings::default();
    settings.ratio = CropRatio::sides(width, height);
    settings.ratio_fields = [width, height];
    settings
}

/// Choosing each ratio preset sets the ratio and its fields; each size preset fills W, H and the resolution in px/in
/// and turns W x H x Resolution on; `choice` shows them back.
#[trial]
fn ratio_presets_and_size_presets_fill_the_fields() {
    {
        let doc = document();
        let mut settings = CropSettings::default();
        for (i, ratio) in CROP_RATIOS.iter().enumerate() {
            settings.choose(CropPresetChoice::Free, &doc, &[]);
            settings.choose(CropPresetChoice::Ratio(i), &doc, &[]);
            assert_eq!(settings.ratio, *ratio);
            if let CropRatio::Sides { width, height, .. } = ratio {
                assert_eq!(settings.ratio_fields, [*width, *height]);
            }
            assert_eq!(settings.choice(&[]), CropPresetChoice::Ratio(i));
        }
        for (i, preset) in CROP_SIZES.iter().enumerate() {
            settings.size.resolution_unit = ResolutionUnit::PixelsPerCentimetre;
            assert!(settings.choose(CropPresetChoice::SizePreset(i), &doc, &[]));
            assert_eq!(settings.ratio, CropRatio::Size);
            assert_eq!(
                (settings.size.width, settings.size.height, settings.size.resolution.as_str(), settings.size.resolution_unit),
                (Some(preset.width), Some(preset.height), preset.resolution, ResolutionUnit::PixelsPerInch)
            );
            assert_eq!(settings.choice(&[]), CropPresetChoice::SizePreset(i));
        }
        assert!(!settings.choose(CropPresetChoice::NewPreset, &doc, &[]));
        assert!(settings.choose(CropPresetChoice::Free, &doc, &[]));
        assert_eq!(settings.choice(&[]), CropPresetChoice::Free);

        let user = [CropUserPreset::new("Cinema", CropRatio::sides(21.0, 9.0), CropSizeFields::default())];
        assert!(settings.choose(CropPresetChoice::User(0), &doc, &user));
        assert_eq!(settings.ratio_fields, [21.0, 9.0]);
        assert_eq!(settings.choice(&user), CropPresetChoice::User(0));
    }
}

/// `target` converts physical lengths at the typed resolution (px/cm x 2.54), else at the document's; points at the
/// document's points per inch.
#[trial]
fn target_uses_the_typed_resolution_else_the_documents() {
    {
        let doc = document().resolution(100.0);
        let at_typed = size_mode(inches(4.0), inches(6.0), "300").target(&doc, &units_of(PointSize::PostScript)).unwrap();
        assert_eq!((at_typed.width, at_typed.height, at_typed.resolution), (1200.0, 1800.0, Some(300.0)));
        let at_document = size_mode(inches(4.0), pixels(1024.0), "").target(&doc, &units_of(PointSize::PostScript)).unwrap();
        assert_eq!((at_document.width, at_document.height, at_document.resolution), (400.0, 1024.0, None));
        let mut per_cm = size_mode(Some(CropLength::new(2.54, UnitKind::Centimetres)), inches(1.0), "100");
        per_cm.size.resolution_unit = ResolutionUnit::PixelsPerCentimetre;
        let t = per_cm.target(&doc, &units_of(PointSize::PostScript)).unwrap();
        assert!((t.width - 254.0).abs() < 1e-9 && (t.height - 254.0).abs() < 1e-9 && t.resolution == Some(254.0));
        let points =
            size_mode(Some(CropLength::new(72.0, UnitKind::Points)), pixels(1.0), "").target(&doc, &units_of(PointSize::Traditional)).unwrap();
        assert!((points.width - 100.0 * 72.0 / 72.27).abs() < 1e-9);
        assert!((size_mode(inches(4.0), inches(6.0), "").ratio_value(&doc, &units_of(PointSize::PostScript)).unwrap() - 4.0 / 6.0).abs() < 1e-12);
    }
}

/// W or H empty: no target and no ratio; one ratio side typed: a free ratio that keeps the side.
#[trial]
fn empty_w_or_h_is_a_free_crop() {
    {
        let doc = document();
        let half = size_mode(inches(4.0), None, "300");
        assert_eq!((half.target(&doc, &units_of(PointSize::PostScript)), half.ratio_value(&doc, &units_of(PointSize::PostScript))), (None, None));
        let mut settings = CropSettings::default();
        settings.set_ratio_field(0, 3.0);
        assert_eq!(
            (settings.ratio, settings.ratio_fields, settings.ratio_value(&doc, &units_of(PointSize::PostScript))),
            (CropRatio::Free, [3.0, 0.0], None)
        );
        assert_eq!(settings.shown_ratio_fields(&doc), [3.0, 0.0]);
        settings.set_ratio_field(1, 1.41437);
        assert_eq!(settings.ratio, CropRatio::sides(3.0, 1.414));
        settings.set_ratio_field(0, 0.0);
        assert_eq!((settings.ratio, settings.ratio_fields), (CropRatio::Free, [0.0, 1.414]));
    }
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
    assert_eq!((partial.fill, partial.ratio), (CropFill::ContentAware, CropRatio::sides(4.0, 5.0)));
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
    fixture.room(Vec2::new(1800.0, 400.0)).settle();
    Rig { fixture, settings: state, seen }
}

impl Rig {
    fn click(&mut self, label: &str) {
        let target = self.fixture.by_label(label).unwrap_or_else(|| panic!("{label}"));
        self.fixture.click(&target).settle();
    }

    fn pick(&mut self, label: &str) {
        self.open_list_wheeled(5.0);
        self.click(label);
    }

    /// Opens the preset list and wheels it by `pages` (negative: toward the end). The list is capped to a share of the
    /// window and scrolls; the entries after Front Image are disabled when there is nothing to save or delete, so no
    /// key reaches them.
    fn open_list_wheeled(&mut self, pages: f32) {
        self.click("Crop ratio");
        let bar = self.fixture.shown().height();
        let row = self.fixture.texts().iter().find(|text| text.rect.min.y > bar).expect("the open list shows a row").rect.center();
        self.fixture.frame(vec![egui::Event::PointerMoved(row)]);
        self.fixture.frame(vec![egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Page,
            delta: Vec2::new(0.0, pages),
            modifiers: Modifiers::NONE,
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Wheel,
        }]);
        self.fixture.settle();
    }

    fn open_list_at_end(&mut self) {
        self.open_list_wheeled(-5.0);
    }

    fn pick_at_end(&mut self, label: &str) {
        self.open_list_at_end();
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
    let user = CropUserPreset::new("Poster", CropRatio::sides(3.0, 7.0), CropSizeFields::default());
    let mut rig = bar(sides(2.0, 3.0), vec![user]);
    rig.pick_at_end("New Crop Preset…");
    rig.pick_at_end("Delete Crop Preset…");
    let flags: Vec<_> = rig.seen.borrow().iter().map(|s| (s.new_preset, s.delete_preset)).collect();
    assert_eq!(flags, vec![(true, false), (false, true)]);

    rig.seen.borrow_mut().clear();
    rig.pick("5 : 7");
    let preset = CROP_RATIOS[3];
    assert_eq!(rig.settings.borrow().ratio, preset);
    assert!(rig.seen.borrow().iter().any(|s| s.ratio.is_some() && s.changed), "a preset pick edits the settings");

    // An arrow key steps the focused width field.
    rig.seen.borrow_mut().clear();
    let mut fields = rig.fixture.by_role(Role::SPIN_BUTTON);
    fields.sort_by(|a, b| a.rect.min.x.total_cmp(&b.rect.min.x));
    rig.fixture.click(&fields[0]);
    rig.fixture.key(Key::ArrowUp, Modifiers::NONE).settle();
    let stepped = rig.settings.borrow().ratio;
    let CropRatio::Sides { width: first, height: first_height, .. } = stepped else { panic!("a stepped ratio has sides") };
    assert_ne!(first, first_height, "the swap runs on unequal sides");

    rig.click("Swaps height and width");
    assert_eq!(rig.settings.borrow().ratio, CropRatio::sides(first_height, first));
    rig.click("Swaps height and width");
    assert_eq!(rig.settings.borrow().ratio, stepped);

    rig.fixture.click(&fields[0]);
    rig.fixture.key(Key::ArrowUp, Modifiers::NONE).settle();
    let twice = rig.settings.borrow().ratio;
    assert!(
        matches!(
            (preset, stepped, twice),
            (
                CropRatio::Sides { width: before, height, .. },
                CropRatio::Sides { width: after, height: kept, .. },
                CropRatio::Sides { width: again, height: still, .. },
            ) if after == before + RATIO_STEP && again == after + RATIO_STEP && kept == height && still == height
        ),
        "each arrow press adds RATIO_STEP to the width and keeps the height: {stepped:?}, {twice:?}"
    );
    assert!(rig.seen.borrow().iter().any(|s| s.ratio.is_some()));
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
    fixture.room(Vec2::new(480.0, 600.0)).settle();
    Rig { fixture, settings: state, seen }
}

/// Types `text` into the field named `label`, replacing what it holds, and commits it.
fn type_into(rig: &mut Rig, label: &str, text: &str) {
    let field = rig.fixture.by_label(label).unwrap_or_else(|| panic!("the bar shows {label}"));
    rig.fixture.click(&field);
    rig.fixture.key(Key::A, Modifiers::COMMAND);
    if text.is_empty() {
        rig.fixture.key(Key::Backspace, Modifiers::NONE);
    } else {
        rig.fixture.type_text(text);
    }
    rig.fixture.key(Key::Enter, Modifiers::NONE).settle();
}

/// Typing "2 in" into a pixel field switches the field to inches; the crop is the same length in pixels.
#[trial]
fn a_unit_typed_into_a_pixel_field_becomes_the_fields_unit() {
    let mut rig = bar(size_mode(pixels(600.0), pixels(500.0), "300"), Vec::new());
    type_into(&mut rig, "Width", "2 in");
    let width = rig.settings.borrow().size.width.expect("the width is set");
    assert_eq!(width.unit, UnitKind::Inches);
    assert!((width.value - 2.0).abs() < 1e-6, "{width:?}");
    let target = rig.settings.borrow().target(&document(), &units_of(PointSize::PostScript)).expect("a target");
    assert!((target.width - 600.0).abs() < 1e-6, "{target:?}");
}
