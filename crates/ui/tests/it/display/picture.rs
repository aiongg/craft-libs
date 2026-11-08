//! `Picture`: the image node of a named picture and the texture built once per key and revision.

use craft_ui::contract::{Role, Show as _};
use craft_ui::display::Picture;
use craft_ui::primitives::Revision;
use craft_ui::testing::{Fixture, trial};
use egui::{Color32, ColorImage};
use std::cell::Cell;
use std::rc::Rc;

fn image() -> ColorImage {
    ColorImage::new([4, 4], vec![Color32::RED; 16])
}

/// A named picture is an image node and an unnamed one is decoration.
#[trial]
fn a_named_picture_is_an_image_node_and_an_unnamed_one_has_none() {
    let mut named = Fixture::new(|ui| {
        Picture::new("Cover", 1, &image).show(ui);
    });
    named.settle();
    assert_eq!(named.by_role(Role::IMAGE).len(), 1);
    assert!(named.by_label("Cover").is_some());
    let mut unnamed = Fixture::new(|ui| {
        Picture::new("", 1, &image).show(ui);
    });
    unnamed.settle();
    assert!(unnamed.by_role(Role::IMAGE).is_empty());
}

/// The image is made once for a key and revision, and again when the revision changes.
#[trial]
fn the_texture_is_built_once_per_key_and_revision() {
    let calls = Rc::new(Cell::new(0));
    let revision = Rc::new(Cell::new(1));
    let (counted, current) = (calls.clone(), revision.clone());
    let mut fixture = Fixture::new(move |ui| {
        let make = || {
            counted.set(counted.get() + 1);
            image()
        };
        Picture::new("Cover", 1, &make).revision(Revision::new(current.get())).show(ui);
    });
    fixture.settle();
    fixture.idle(3);
    assert_eq!(calls.get(), 1);
    revision.set(2);
    fixture.idle(3);
    assert_eq!(calls.get(), 2);
}
