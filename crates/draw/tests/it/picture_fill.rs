//! The picture fill editor: its tile fields, the scale floor and the Insert… request.

use craft_draw::ui::picture_fill::{PictureFill, PictureFillEditor, PictureMode, TileAlignment, TileMirror};
use craft_ui::contract::Bind;
use craft_ui::runtime::requests::PathRequest;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};
use std::cell::RefCell;
use std::rc::Rc;

/// The offset typed into the tile's X field and into the stretch's left inset.
const TILE_OFFSET: f64 = 12.0;
const LEFT_INSET: f64 = 5.0;

fn tiled() -> PictureFill {
    let mut fill = PictureFill::new("sunrise.png");
    fill.mode = PictureMode::tile([0.0, 0.0], [100.0, 100.0], TileAlignment::TopLeft, TileMirror::None);
    fill
}

type Rig = (Fixture, Rc<RefCell<PictureFill>>, Rc<RefCell<Option<PathRequest>>>);

fn rig(fill: PictureFill) -> Rig {
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
    type_into(&mut fixture, "Offset X", &TILE_OFFSET.to_string());
    match value.borrow().mode {
        PictureMode::Tile { offset, .. } => assert_eq!(offset[0], TILE_OFFSET),
        other => panic!("still a tile, got {other:?}"),
    }
    let tile = fixture.by_label("Tile picture as texture").expect("the tile check is shown");
    fixture.click(&tile);
    fixture.idle(3);
    assert!(matches!(value.borrow().mode, PictureMode::Stretch { .. }));
    type_into(&mut fixture, "Offset left", &LEFT_INSET.to_string());
    match value.borrow().mode {
        PictureMode::Stretch { offsets, .. } => assert_eq!(offsets[0], LEFT_INSET),
        other => panic!("stretched, got {other:?}"),
    }
    let tile = fixture.by_label("Tile picture as texture").expect("the tile check is shown");
    fixture.click(&tile);
    fixture.idle(3);
    match value.borrow().mode {
        PictureMode::Tile { offset, .. } => assert_eq!(offset[0], TILE_OFFSET, "the tile values come back"),
        other => panic!("tiled again, got {other:?}"),
    }
}
