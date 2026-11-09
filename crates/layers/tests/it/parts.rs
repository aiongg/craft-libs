//! The layers parts: the filter rule, the settings, the options dialog's answer and the property rows' locks and
//! disabled states.

use craft_layers::ui::filter::{FilterBy, FilterKind, LayerFilter};
use craft_layers::ui::options::{LayersOptions, LayersOptionsDialog, LayersSettings};
use craft_layers::ui::rows::{BlendRows, LayerPropertyEdit};
use craft_layers::ui::thumbs::ThumbContents;
use craft_layers::ui::tree::{LayerInfo, LayerKind, LayerProperties, LockKind};
use craft_ui::contract::{Entry, Role};
use craft_ui::dialog::{DialogStack, StackDialog as _};
use craft_ui::list::ThumbSize;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};
use std::cell::RefCell;
use std::rc::Rc;

fn layer(name: &str, kind: LayerKind, blend_key: &str) -> LayerInfo {
    let mut info = LayerInfo::new(name, kind);
    info.blend_key = blend_key.to_owned();
    info
}

/// Mode always filters, Name trims and ignores case, Kind matches `filter_kind`, groups match no kind, an inactive
/// filter shows every layer.
#[trial]
fn filter_rules_for_kind_name_and_mode() {
    let pixel = layer("Sky Photo", LayerKind::Pixel, "normal");
    let fill = layer("Colour", LayerKind::Fill, "multiply");
    let group = layer("Group 1", LayerKind::Group, "passThrough");

    let none = LayerFilter::default();
    assert!(!none.active());
    assert!([&pixel, &fill, &group].into_iter().all(|info| none.shows(info)));

    let mut kinds = LayerFilter::default();
    kinds.kinds = vec![FilterKind::Adjustment];
    assert!(kinds.active());
    assert!(kinds.shows(&fill) && !kinds.shows(&pixel) && !kinds.shows(&group), "fill layers are adjustments; groups match no kind");

    let mut blank = LayerFilter::default();
    blank.by = FilterBy::Name;
    blank.name = String::from("   ");
    assert!(!blank.active() && blank.shows(&group));
    let mut name = LayerFilter::default();
    name.by = FilterBy::Name;
    name.name = String::from("  sky ");
    assert!(name.shows(&pixel) && !name.shows(&fill));

    let mut mode = LayerFilter::default();
    mode.by = FilterBy::Mode;
    assert!(mode.active(), "Mode always filters, by Normal at first");
    assert!(mode.shows(&pixel) && !mode.shows(&fill));
    mode.mode = String::from("multiply");
    assert!(mode.shows(&fill) && !mode.shows(&pixel));
}

/// A section with missing fields loads with the defaults; a JSON round trip keeps every field.
#[trial]
fn partial_settings_load_with_defaults_and_round_trip() {
    let defaults = LayersSettings::default();
    let partial: LayersSettings =
        serde_json::from_str(r#"{"options":{"thumbnail_size":"Large","expand_new_effects":false}}"#).expect("partial section loads");
    assert_eq!(partial.options.thumbnail_size, ThumbSize::Large);
    assert!(!partial.options.expand_new_effects);
    assert_eq!(partial.show_filters, defaults.show_filters, "missing fields take their defaults");
    assert_eq!(partial.options.use_default_masks_on_fill_layers, defaults.options.use_default_masks_on_fill_layers);
    assert_eq!(serde_json::from_str::<LayersSettings>("{}").expect("empty section loads"), defaults);

    let mut changed = LayersSettings::default();
    changed.options.thumbnail_size = ThumbSize::None;
    changed.options.thumbnail_contents = ThumbContents::LayerBounds;
    changed.options.use_default_masks_on_fill_layers = false;
    changed.options.expand_new_effects = false;
    changed.options.add_copy_to_copied_layers_and_groups = false;
    changed.options.preview_on_canvas_when_reordering = false;
    changed.options.show_layer_mask_badges = true;
    changed.show_filters = false;
    let json = serde_json::to_string(&changed).expect("serializes");
    assert_eq!(serde_json::from_str::<LayersSettings>(&json).expect("round trip"), changed);
}

/// The answers the dialog gave: `Some(options)` for OK, `None` for Cancel.
type Answers = Rc<RefCell<Vec<Option<LayersOptions>>>>;

/// The room of a dialog: the whole 800 by 600 screen of a fixture, so no row of its body is clipped away.
const SCREEN: egui::Vec2 = by(800.0, 600.0);

/// A size of `width` by `height` points.
const fn by(width: f32, height: f32) -> egui::Vec2 {
    egui::Vec2::new(width, height)
}

fn open_dialog() -> (Fixture, Answers) {
    let answers: Answers = Rc::default();
    let seen = answers.clone();
    let mut dialog = LayersOptionsDialog::new(LayersOptions::default());
    let stack = DialogStack::default();
    let mut fixture = Fixture::new(move |ui| {
        if let Some(answer) = dialog.show(ui, &stack) {
            seen.borrow_mut().push(answer.ok());
        }
    });
    fixture.room(SCREEN);
    fixture.settle();
    (fixture, answers)
}

fn click(fixture: &mut Fixture, label: &str) {
    let target = fixture.by_label(label).unwrap_or_else(|| {
        let names: Vec<_> = fixture.tree().keys().into_iter().filter_map(|key| fixture.tree().name_of(key)).collect();
        panic!("no node named {label:?}; the tree has {names:?}")
    });
    fixture.click(&target);
    fixture.idle(2);
}

/// OK returns the edited `LayersOptions`; Cancel returns no options.
#[trial]
fn options_dialog_ok_returns_the_edited_options() {
    let (mut fixture, answers) = open_dialog();
    click(&mut fixture, "Layer Bounds");
    click(&mut fixture, "Show layer mask badges");
    click(&mut fixture, "Expand New Effects");
    click(&mut fixture, "Small");
    click(&mut fixture, "OK");
    let answers = answers.borrow();
    let Some(Some(options)) = answers.last() else { panic!("OK answers with the options") };
    assert_eq!(options.thumbnail_contents, ThumbContents::LayerBounds);
    assert_eq!(options.thumbnail_size, ThumbSize::Small);
    assert!(options.show_layer_mask_badges);
    assert!(!options.expand_new_effects);

    let (mut fixture, answers) = open_dialog();
    click(&mut fixture, "Show layer mask badges");
    click(&mut fixture, "Cancel");
    assert_eq!(answers.borrow().as_slice(), [None], "Cancel gives no options");
}

type Events = Rc<RefCell<Vec<LayerPropertyEdit>>>;

/// The rows over `properties`; the events they report collect in the returned list.
fn rows(properties: Option<LayerProperties>) -> (Fixture, Events) {
    let events: Events = Rc::default();
    let seen = events.clone();
    let modes = [Entry::new(String::from("normal"), "Normal"), Entry::new(String::from("multiply"), "Multiply")];
    let mut fixture = Fixture::new(move |ui| {
        let out = BlendRows::new(properties.as_ref(), &modes).show(ui);
        seen.borrow_mut().extend(out.events);
    });
    fixture.settle();
    (fixture, events)
}

/// Lock image pixels disables Lock transparent pixels; Lock all shows the others on and disabled; the other locks
/// still report.
#[trial]
fn lock_row_image_pixels_disables_transparent_pixels() {
    let mut properties = LayerProperties::new("normal");
    properties.locks.pixels = true;
    let (mut fixture, events) = rows(Some(properties));
    click(&mut fixture, "Lock transparent pixels");
    assert!(events.borrow().is_empty(), "a disabled lock reports nothing");
    click(&mut fixture, "Lock position");
    assert!(matches!(events.borrow().as_slice(), [LayerPropertyEdit::Lock { kind: LockKind::Position, on: true, .. }]));
    click(&mut fixture, "Lock image pixels");
    assert!(matches!(events.borrow().last(), Some(LayerPropertyEdit::Lock { kind: LockKind::Pixels, on: false, .. })));

    let mut all = LayerProperties::new("normal");
    all.locks.all = true;
    let (mut fixture, events) = rows(Some(all));
    for name in ["Lock transparent pixels", "Lock image pixels", "Lock position"] {
        click(&mut fixture, name);
    }
    assert!(events.borrow().is_empty(), "Lock all includes the other locks");
    click(&mut fixture, "Lock all");
    assert!(matches!(events.borrow().as_slice(), [LayerPropertyEdit::Lock { kind: LockKind::All, on: false, .. }]));
}

/// On the Background the blend select, Opacity and Fill change nothing but the locks still report; with no layer
/// nothing reports.
#[trial]
fn background_and_no_layer_grey_the_property_rows() {
    let mut background = LayerProperties::new("normal");
    background.editable = false;
    let (mut fixture, events) = rows(Some(background));
    for target in fixture.by_role(Role::COMBO_BOX).into_iter().chain(fixture.by_role(Role::SPIN_BUTTON)) {
        fixture.click(&target);
        fixture.key(Key::ArrowUp, Modifiers::NONE);
        fixture.idle(2);
    }
    assert!(events.borrow().is_empty(), "the property rows are disabled on the Background");
    click(&mut fixture, "Lock all");
    assert_eq!(events.borrow().len(), 1, "the locks stay");

    let (mut fixture, events) = rows(None);
    for target in fixture.by_role(Role::COMBO_BOX).into_iter().chain(fixture.by_role(Role::SPIN_BUTTON)) {
        fixture.click(&target);
        fixture.key(Key::ArrowUp, Modifiers::NONE);
        fixture.idle(2);
    }
    for name in ["Lock image pixels", "Lock position", "Lock all"] {
        click(&mut fixture, name);
    }
    assert!(events.borrow().is_empty(), "no layer, nothing to edit");
}
