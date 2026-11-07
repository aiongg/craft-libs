//! The own logic of the watermark rows: which insets apply to a position, and that switching the style keeps the
//! text and the image path apart.

use craft_ui::contract::Bind;
use craft_ui::patterns::form_dialogs::{Stash, Watermark, WatermarkSettings, WatermarkStyle};
use craft_ui::pickers::Anchor9;
use craft_ui::testing::{Fixture, trial};
use std::cell::RefCell;
use std::rc::Rc;

fn at(position: Anchor9) -> WatermarkSettings {
    WatermarkSettings { position, ..WatermarkSettings::default() }
}

/// An inset along an axis the position centres on has nothing to offset.
#[trial]
fn insets_apply_only_along_the_axes_the_position_does_not_centre() {
    assert!(at(Anchor9::BottomRight).insets_across() && at(Anchor9::BottomRight).insets_down());
    assert!(!at(Anchor9::Top).insets_across() && at(Anchor9::Top).insets_down());
    assert!(at(Anchor9::Left).insets_across() && !at(Anchor9::Left).insets_down());
    assert!(!at(Anchor9::Centre).insets_across() && !at(Anchor9::Centre).insets_down());
}

/// The stash gives back what each style held when the style was switched away from it.
#[trial]
fn the_stash_keeps_the_text_and_the_image_path_apart() {
    let mut stash = Stash::default();
    stash.keep(&WatermarkStyle::Text("Studio".to_owned()));
    stash.keep(&WatermarkStyle::Graphic("logo.png".to_owned()));
    assert_eq!(stash.style(false), WatermarkStyle::Text("Studio".to_owned()));
    assert_eq!(stash.style(true), WatermarkStyle::Graphic("logo.png".to_owned()));
}

/// Picking Graphic and then Text again brings the typed text back.
#[trial]
fn switching_the_style_restores_the_text() {
    let value = Rc::new(RefCell::new(WatermarkSettings { style: WatermarkStyle::Text("Studio".to_owned()), ..WatermarkSettings::default() }));
    let held = Rc::clone(&value);
    let mut fixture = Fixture::new(move |ui| {
        let mut value = held.borrow_mut();
        Watermark::new("Watermark", Bind::bound(&mut *value)).show(ui);
    });
    fixture.settle();
    let graphic = fixture.by_label("Graphic").expect("the style tabs are shown");
    fixture.click(&graphic);
    fixture.idle(3);
    assert_eq!(value.borrow().style, WatermarkStyle::Graphic(String::new()));
    let text = fixture.by_label("Text").expect("the Text tab is shown");
    fixture.click(&text);
    fixture.idle(3);
    assert_eq!(value.borrow().style, WatermarkStyle::Text("Studio".to_owned()));
}
