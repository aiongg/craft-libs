//! The Glyphs panel in a harness with a fake glyph source.

use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{ColorImage, Key};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::glyphs::*;

/// Latin capitals, the euro sign and an arrow, with white square glyphs.
struct Fake;

impl GlyphSource for Fake {
    fn charmap(&self, _family: &str, _style: &str) -> Vec<char> {
        ('A'..='Z').chain(['€', '→']).collect()
    }
    fn glyph(&self, _family: &str, _style: &str, _c: char, px: u32) -> Option<ColorImage> {
        Some(ColorImage::filled([px as usize, px as usize], egui::Color32::WHITE))
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
    editing: bool,
    inserted: Vec<String>,
}

fn harness(editing: bool) -> Harness<'static, State> {
    let state = State { family: "Fake".into(), style: "Regular".into(), settings: GlyphsPanelSettings::default(), editing, inserted: Vec::new() };
    Harness::<State>::builder().with_size([320.0, 520.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, s: &mut State| {
            let r = GlyphsPanel::new(&Fake, &mut s.family, &mut s.style, &mut s.settings).editing(s.editing).show(ui);
            s.inserted.extend(r.insert);
        },
        state,
    )
}

fn cell(c: char) -> String {
    format!("{c}, U+{:04X}", c as u32)
}

// Picking a category shows only its characters; "Entire Font" shows the whole charmap.
#[test]
fn category_filters_the_grid() {
    let mut harness = harness(true);
    harness.run();
    assert!(harness.query_by_label(&cell('A')).is_some() && harness.query_by_label(&cell('€')).is_some());

    harness.get_by_role(Role::ComboBox).click();
    harness.run();
    harness.get_by_label("Currency").click();
    harness.run();
    assert_eq!(harness.state().settings.category, "Currency");
    assert!(harness.query_by_label(&cell('€')).is_some());
    assert!(harness.query_by_label(&cell('A')).is_none() && harness.query_by_label(&cell('→')).is_none());
}

// Enter on the focused cell inserts it while editing and adds it to Recently Used; not while idle.
#[test]
fn enter_inserts_only_while_editing_and_records_recent() {
    for editing in [true, false] {
        let mut harness = harness(editing);
        harness.run();
        harness.get_by_label(&cell('B')).click();
        harness.run();
        harness.key_press(Key::Enter);
        harness.run();
        let s = harness.state();
        if editing {
            assert_eq!(s.inserted, ["B"]);
            assert_eq!(s.settings.recent, ["B"]);
            assert!(harness.query_all_by_label(&cell('B')).count() >= 2, "the recent row shows it too");
        } else {
            assert!(s.inserted.is_empty() && s.settings.recent.is_empty());
        }
    }
}

// Arrow keys move the focused cell; the grid is one Tab stop.
#[test]
fn arrows_move_the_focused_cell() {
    let mut harness = harness(true);
    harness.run();
    harness.get_by_label(&cell('A')).click();
    harness.run();
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert!(harness.get_by_label(&cell('B')).accesskit_node().is_focused());
    harness.key_press(Key::End);
    harness.run();
    assert!(harness.get_by_label(&cell('→')).accesskit_node().is_focused(), "End goes to the last cell");
    harness.key_press(Key::Home);
    harness.run();
    assert!(harness.get_by_label(&cell('A')).accesskit_node().is_focused());

    harness.key_press(Key::Tab);
    harness.run();
    let cells = ('A'..='Z').chain(['€', '→']).any(|c| harness.query_by_label(&cell(c)).is_some_and(|n| n.accesskit_node().is_focused()));
    assert!(!cells, "Tab leaves the grid");
}
