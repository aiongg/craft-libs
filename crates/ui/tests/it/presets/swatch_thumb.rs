//! A preset browser's thumbnail painter can paint the swatch face a `Swatch` paints.

use craft_theme::Srgba;
use craft_ui::choice::SwatchFace;
use craft_ui::contract::{Bind, Show as _};
use craft_ui::patterns::presets::{PresetBrowser, PresetRow, PresetState};
use craft_ui::testing::{Frames, trial};
use egui::{Color32, Rect, Shape};
use std::cell::Cell;

const KEY: u32 = 1;

/// The rect the thumbnail painter was given and the filled rects inside it, in paint order, for a swatch preset of
/// `color`.
fn face_of(color: Srgba) -> (Rect, Vec<Color32>) {
    let mut frames = Frames::new();
    let mut state = PresetState::<u32>::new();
    let rows = vec![PresetRow::item(KEY, "Swatch", 0)];
    let given = Cell::new(Rect::NOTHING);
    let mut shapes = Vec::new();
    for _ in 0..2 {
        shapes = frames.frame_shapes(|ui| {
            PresetBrowser::new("Swatches", Bind::bound(&mut state), &rows)
                .thumb(|cx, rect, _| {
                    given.set(rect);
                    SwatchFace::new(color).paint(cx, rect);
                })
                .show(ui);
        });
    }
    let thumb = given.get();
    let fills = shapes
        .iter()
        .filter_map(|s| if let Shape::Rect(r) = s { Some((r.rect, r.fill)) } else { None })
        .filter(|(rect, fill)| *fill != Color32::TRANSPARENT && thumb.expand(0.5).contains_rect(*rect))
        .map(|(_, fill)| fill)
        .collect();
    (thumb, fills)
}

#[trial]
fn a_swatch_preset_thumbnail_fills_the_colour_inside_its_rect() {
    let (thumb, fills) = face_of(Srgba::rgb(1.0, 0.0, 0.0));
    assert!(thumb.is_positive(), "the painter ran with a rect");
    assert_eq!(fills.last(), Some(&Color32::from_rgb(255, 0, 0)), "the colour fills the thumbnail: {fills:?}");
}

