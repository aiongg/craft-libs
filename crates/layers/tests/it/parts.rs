//! The layers parts: the filter rule, the settings, the options dialog's answer and the property rows' locks and
//! disabled states.

use craft_layers::ui::options::LayersSettings;
use craft_layers::ui::rows::{BlendRows, LayerPropertyEdit};
use craft_layers::ui::thumbs::ThumbContents;
use craft_layers::ui::tree::{LayerProperties, LockKind};
use craft_ui::contract::{Entry, Role};
use craft_ui::list::ThumbSize;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};
use std::cell::RefCell;
use std::rc::Rc;

/// `LayersSettings::default()` is Medium, Entire Document, filters shown, the first four switches on, badges off; a
/// section with missing fields loads with those defaults; a JSON round trip keeps every field.
#[trial]
fn settings_defaults_missing_fields_and_round_trip() {
    let defaults = LayersSettings::default();
    assert_eq!(defaults.options.thumbnail_size, ThumbSize::Medium);
    assert_eq!(defaults.options.thumbnail_contents, ThumbContents::EntireDocument);
    assert!(defaults.show_filters);
    let o = &defaults.options;
    assert!(
        o.use_default_masks_on_fill_layers && o.expand_new_effects && o.add_copy_to_copied_layers_and_groups && o.preview_on_canvas_when_reordering
    );
    assert!(!o.show_layer_mask_badges);

    let partial: LayersSettings =
        serde_json::from_str(r#"{"options":{"thumbnail_size":"Large","expand_new_effects":false}}"#).expect("partial section loads");
    assert_eq!(partial.options.thumbnail_size, ThumbSize::Large);
    assert!(!partial.options.expand_new_effects);
    assert!(partial.show_filters && partial.options.use_default_masks_on_fill_layers, "missing fields take their defaults");
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

fn click(fixture: &mut Fixture, label: &str) {
    let target = fixture.by_label(label).unwrap_or_else(|| {
        let names: Vec<_> = fixture.tree().keys().into_iter().filter_map(|key| fixture.tree().name_of(key)).collect();
        panic!("no node named {label:?}; the tree has {names:?}")
    });
    fixture.click(&target);
    fixture.idle(2);
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
