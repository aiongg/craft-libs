//! `GlyphsPanel` through the fixture with a fake glyph source.

use std::cell::RefCell;
use std::rc::Rc;

use craft_font_manager::glyphs::{GlyphSource, GlyphsPanel, GlyphsPanelSettings};
use craft_ui::contract::Bind;
use craft_ui::testing::{Fixture, Room, trial};
use egui::{ColorImage, Key, Modifiers};

/// Latin capitals, the euro sign and an arrow, with white square glyphs.
struct Fake;

impl GlyphSource for Fake {
    fn charmap(&self, _family: &str, _style: &str) -> Vec<char> {
        ('A'..='Z').chain(['€', '→']).collect()
    }

    fn glyph(&self, _family: &str, _style: &str, _c: char, px: u32) -> Option<ColorImage> {
        Some(ColorImage::filled([px as usize, px as usize], egui::Color32::default()))
    }

    fn in_category(&self, c: char, category: &str) -> bool {
        match category {
            "Currency" => c == '€',
            "Arrows" => c == '→',
            "Latin" => c.is_ascii_uppercase(),
            _ => false,
        }
    }
}

struct State {
    family: String,
    style: String,
    settings: GlyphsPanelSettings,
    inserted: Vec<String>,
}

fn rig(editing: bool) -> (Fixture, Rc<RefCell<State>>) {
    let state = Rc::new(RefCell::new(State {
        family: "Fake".into(),
        style: "Regular".into(),
        settings: GlyphsPanelSettings::default(),
        inserted: Vec::new(),
    }));
    let shown = Rc::clone(&state);
    let mut fixture = Fixture::new(move |ui| {
        let mut state = shown.borrow_mut();
        let State { family, style, settings, inserted } = &mut *state;
        let out = GlyphsPanel::new("Glyphs", &Fake, Bind::bound(family), Bind::bound(style), Bind::bound(settings)).editing(editing).show(ui);
        inserted.extend(out.insert);
    });
    fixture.in_room(Room::Panel);
    fixture.settle();
    (fixture, state)
}

fn cell(c: char) -> String {
    format!("{c}, U+{:04X}", c as u32)
}

/// Picking a category shows only its characters; "Entire Font" shows the whole charmap.
#[trial]
fn category_filters_the_grid() {
    let (mut fixture, state) = rig(true);
    assert!(fixture.by_label(&cell('A')).is_some() && fixture.by_label(&cell('€')).is_some());
    let select = fixture.by_label("Category").expect("the category list");
    fixture.click(&select);
    fixture.idle(3);
    let currency = fixture.by_label("Currency").expect("the Currency option");
    fixture.click(&currency);
    fixture.idle(6);
    assert_eq!(state.borrow().settings.category, "Currency");
    assert!(fixture.by_label(&cell('€')).is_some());
    assert!(fixture.by_label(&cell('A')).is_none() && fixture.by_label(&cell('→')).is_none());
}

/// Enter on the focused cell inserts it while editing and adds it to Recently Used; not while idle.
#[trial]
fn enter_inserts_only_while_editing_and_records_recent() {
    for editing in [true, false] {
        let (mut fixture, state) = rig(editing);
        let tile = fixture.by_label(&cell('B')).expect("the B cell");
        fixture.click(&tile);
        fixture.key(Key::Enter, Modifiers::NONE);
        fixture.idle(3);
        let state = state.borrow();
        if editing {
            assert_eq!(state.inserted, ["B"]);
            assert_eq!(state.settings.recent, ["B"]);
        } else {
            assert!(state.inserted.is_empty() && state.settings.recent.is_empty());
        }
    }
}
