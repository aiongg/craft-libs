//! The picture fill editor: its tile fields, the scale floor and the Insert… request.

use craft_draw::ui::picture_fill::{PICTURE_FILL_KEY, PictureFill, PictureFillEditor, PictureMode, TileAlignment, TileMirror};
use craft_ui::contract::{Bind, Show as _};
use craft_ui::runtime::requests::PathRequest;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};
use std::cell::RefCell;
use std::rc::Rc;

fn tiled() -> PictureFill {
    let mut fill = PictureFill::new("sunrise.png");
    fill.mode = PictureMode::Tile { offset: [0.0, 0.0], scale: [100.0, 100.0], alignment: TileAlignment::TopLeft, mirror: TileMirror::None };
    fill
}

fn rig(fill: PictureFill) -> (Fixture, Rc<RefCell<PictureFill>>, Rc<RefCell<Option<PathRequest>>>) {
    let value = Rc::new(RefCell::new(fill));
    let request = Rc::new(RefCell::new(None));
    let (held, asked) = (Rc::clone(&value), Rc::clone(&request));
    let mut fixture = Fixture::new(move |ui| {
        let mut fill = held.borrow_mut();
        let mut out = PictureFillEditor::new("Picture", Bind::bound(&mut *fill)).show(ui);
        if let Some(path) = out.requests.take_path() {
            *asked.borrow_mut() = Some(path);
        }
    });
    fixture.settle();
    (fixture, value, request)
}

fn type_into(fixture: &mut Fixture, name: &str, text: &str) {
    let field = fixture.by_label(name).unwrap_or_else(|| panic!("`{name}` is shown"));
    fixture.click(&field);
    fixture.key(Key::A, Modifiers::COMMAND);
    fixture.type_text(text);
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.idle(3);
}

/// The tile fields edit the tile mode, and switching to Stretch and back keeps each mode's values.
#[trial]
fn picture_editor_tile_fields_and_stretch_offsets() {
    let (mut fixture, value, _) = rig(tiled());
    type_into(&mut fixture, "Offset X", "12");
    match value.borrow().mode {
        PictureMode::Tile { offset, .. } => assert_eq!(offset[0], 12.0),
        other => panic!("still a tile, got {other:?}"),
    }
    let tile = fixture.by_label("Tile picture as texture").expect("the tile check is shown");
    fixture.click(&tile);
    fixture.idle(3);
    assert!(matches!(value.borrow().mode, PictureMode::Stretch { .. }));
    type_into(&mut fixture, "Offset left", "5");
    match value.borrow().mode {
        PictureMode::Stretch { offsets } => assert_eq!(offsets[0], 5.0),
        other => panic!("stretched, got {other:?}"),
    }
    let tile = fixture.by_label("Tile picture as texture").expect("the tile check is shown");
    fixture.click(&tile);
    fixture.idle(3);
    match value.borrow().mode {
        PictureMode::Tile { offset, .. } => assert_eq!(offset[0], 12.0, "the tile values come back"),
        other => panic!("tiled again, got {other:?}"),
    }
}

/// A tile is never scaled below the floor.
#[trial]
fn picture_editor_tile_scale_has_a_floor() {
    let (mut fixture, value, _) = rig(tiled());
    type_into(&mut fixture, "Scale X", "0");
    match value.borrow().mode {
        PictureMode::Tile { scale, .. } => assert!(scale[0] >= *PictureMode::SCALE.start(), "scale {scale:?}"),
        other => panic!("a tile, got {other:?}"),
    }
}

/// Insert… files a path request for images under the picture key.
#[trial]
fn picture_editor_insert_files_a_path_request_for_images() {
    let (mut fixture, _, request) = rig(PictureFill::default());
    let insert = fixture.by_label("Insert…").expect("Insert… is shown");
    fixture.click(&insert);
    fixture.idle(3);
    let asked = request.borrow();
    let asked = asked.as_ref().expect("a request was filed");
    assert_eq!(asked.key, PICTURE_FILL_KEY);
}
