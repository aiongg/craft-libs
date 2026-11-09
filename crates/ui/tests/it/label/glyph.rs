//! Glyph labels: the glyph is shown, the node keeps the full name.

use super::group::Group;
use super::kit::{Dial, Seen};
use craft_ui::contract::{GlyphLabel, LabelKind, Props as _, ShowContainer as _, SideLabel};
use craft_ui::testing::{Frames, trial};
use egui::{Pos2, Rect};
use std::cell::RefCell;

const ROW: Rect = Rect { min: Pos2::new(10.0, 10.0), max: Pos2::new(310.0, 40.0) };

#[trial]
fn glyph_labels_share_a_column_apart_from_the_text_labels() {
    let mut frames = Frames::new();
    let (wide, narrow, text) = (RefCell::new(Seen::default()), RefCell::new(Seen::default()), RefCell::new(Seen::default()));
    let mut value = 1.0;
    frames.idle(3, |ui| {
        Group::column().id_salt("glyphs").show_at(ui, ROW, |ui, cx| {
            let glyph = |shown: &'static str| SideLabel::new().kind(LabelKind::Glyph(GlyphLabel::text(shown)));
            cx.labelled(ui, "a", "Width".into(), glyph("WW"), Dial::new(&mut value, &wide));
            cx.labelled(ui, "b", "Height".into(), glyph("H"), Dial::new(&mut 1.0, &narrow));
            cx.labelled(ui, "c", "Opacity".into(), SideLabel::new(), Dial::new(&mut 1.0, &text));
        });
    });
    let start = |seen: &RefCell<Seen>| seen.borrow().rect.min.x.round();
    assert_eq!(start(&wide), start(&narrow), "glyph labels share the widest glyph");
    assert_ne!(start(&wide), start(&text), "text labels have their own column");
}
