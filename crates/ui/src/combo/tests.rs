use egui_kittest::{Harness, kittest::Queryable as _};

use super::ComboBox;

struct Menu {
    filter: String,
    button: Option<egui::Id>,
}

fn popup(h: &Harness<'_, Menu>) -> egui::Id {
    super::popup_id(h.state().button.unwrap_or(egui::Id::NULL))
}

/// The menu's height as egui keeps it between passes.
fn height(h: &Harness<'_, Menu>) -> f32 {
    egui::AreaState::load(&h.ctx, popup(h)).and_then(|s| s.size).map_or(0.0, |s| s.y)
}

#[test]
fn a_filtered_menu_grows_back_when_the_filter_is_cleared() {
    let mut h = Harness::builder().with_size(egui::vec2(600.0, 800.0)).build_ui_state(
        |ui, m: &mut Menu| {
            let inner = ComboBox::new("language", "Language").selected_text("Any language").width(200.0).height(380.0).show_ui(ui, |ui| {
                let name = ui.label("Find a language").id;
                ui.text_edit_singleline(&mut m.filter).labelled_by(name);
                let rows = if m.filter.is_empty() { 60 } else { 0 };
                let row = ui.spacing().interact_size.y;
                egui::ScrollArea::vertical().max_height(300.0).auto_shrink([false, true]).show_rows(ui, row, rows, |ui, range| {
                    for n in range {
                        let _ = ui.selectable_label(false, format!("Language {n}"));
                    }
                });
            });
            m.button = Some(inner.response.id);
        },
        Menu { filter: String::new(), button: None },
    );
    h.run();
    egui::Popup::open_id(&h.ctx, popup(&h));
    h.run();
    let full = height(&h);
    assert!(full > 300.0, "the open menu shows the list: {full}");

    h.state_mut().filter = "xxxx".into();
    h.run();
    let none = height(&h);
    assert!(none < 100.0, "no matches shrink the menu: {none}");

    h.state_mut().filter.clear();
    h.run();
    let again = height(&h);
    assert!((again - full).abs() < 1.0, "cleared, the menu is {again} tall, not {full} as when it opened");
}

struct Search {
    filter: String,
    scroll: bool,
    field: egui::Rect,
    first_row: usize,
    button: Option<egui::Id>,
}

fn search_menu() -> Harness<'static, Search> {
    Harness::builder().with_size(egui::vec2(600.0, 800.0)).build_ui_state(
        |ui, m: &mut Search| {
            let row = ui.spacing().interact_size.y;
            let header = |ui: &mut egui::Ui| {
                let name = ui.label("Find a language").id;
                m.field = ui.text_edit_singleline(&mut m.filter).labelled_by(name).rect;
                if m.filter.is_empty() { 60 } else { 0 }
            };
            let rows = |ui: &mut egui::Ui, range: std::ops::Range<usize>, _: usize| {
                if std::mem::take(&mut m.scroll) {
                    ui.scroll_with_delta(egui::vec2(0.0, -200.0));
                }
                m.first_row = range.start;
                for n in range {
                    let _ = ui.selectable_label(false, format!("Language {n}"));
                }
            };
            let inner = ComboBox::new("language", "Language").selected_text("Any language").width(200.0).height(380.0).show_rows_with_header(
                ui,
                header,
                row,
                |&n| n,
                rows,
            );
            m.button = Some(inner.response.id);
        },
        Search { filter: String::new(), scroll: false, field: egui::Rect::NOTHING, first_row: 0, button: None },
    )
}

fn search_height(h: &Harness<'_, Search>) -> f32 {
    let popup = super::popup_id(h.state().button.unwrap_or(egui::Id::NULL));
    egui::AreaState::load(&h.ctx, popup).and_then(|s| s.size).map_or(0.0, |s| s.y)
}

#[test]
fn a_menu_header_stays_put_while_the_list_scrolls_and_the_menu_grows_back() {
    let mut h = search_menu();
    h.run();
    h.get_by_role_and_label(egui::Role::ComboBox, "Language").click();
    h.run();
    let (field, full) = (h.state().field, search_height(&h));
    assert!(full > 300.0, "the open menu shows the list: {full}");
    assert!(full < 400.0, "the menu keeps to its height, plus its frame: {full}");

    h.state_mut().scroll = true;
    h.run();
    assert!(h.state().first_row > 0, "the list scrolled");
    assert_eq!(h.state().field, field, "the search field stays put");

    h.state_mut().filter = "xxxx".into();
    h.run();
    let none = search_height(&h);
    assert!(none < 100.0, "no matches shrink the menu: {none}");

    h.state_mut().filter.clear();
    h.run();
    let again = search_height(&h);
    assert!((again - full).abs() < 1.0, "cleared, the menu is {again} tall, not {full} as when it opened");
}

const SIZES: [&str; 3] = ["Small", "Medium", "Large"];

#[test]
fn show_index_picks_a_choice_and_marks_the_response_changed() {
    let mut h = Harness::new_ui_state(
        |ui, s: &mut (usize, bool)| {
            let response = ComboBox::from_label("Size").show_index(ui, &mut s.0, SIZES.len(), |i| SIZES[i]);
            s.1 |= response.changed();
        },
        (0, false),
    );
    h.run();
    h.get_by_role_and_label(egui::Role::ComboBox, "Size").click();
    h.run();
    h.get_by_label("Large").click();
    h.run();
    assert_eq!(*h.state(), (2, true));
    assert!(h.query_by_label("Medium").is_none(), "picking closes the menu");
}

/// Combo boxes in the states a user sees them in, drawn with egui's combo box or ours.
#[expect(clippy::disallowed_types, reason = "the reference our combo box is compared with")]
fn scene(ui: &mut egui::Ui, ours: bool) {
    macro_rules! both {
        ($ctor:ident ( $($arg:expr),* ) $(. $m:ident ( $($a:expr),* ))*) => {
            if ours {
                ComboBox::$ctor($($arg),*)$(.$m($($a),*))*.show_ui(ui, |ui| {
                    for size in SIZES {
                        let _ = ui.selectable_label(size == "Medium", size);
                    }
                })
            } else {
                egui::ComboBox::$ctor($($arg),*)$(.$m($($a),*))*.show_ui(ui, |ui| {
                    for size in SIZES {
                        let _ = ui.selectable_label(size == "Medium", size);
                    }
                })
            }
        };
    }
    let _ = both!(from_label("Size").selected_text("Medium"));
    let _ = both!(from_id_salt("narrow").selected_text("A long selected value that is cut short").width(120.0).truncate());
    let _ = both!(new("wrapped", "Wrapped").selected_text("A long selected value that wraps onto a second line").width(140.0).wrap());
    let _ = both!(new("open", "Open").selected_text("Medium").width(160.0));
}

fn render(ours: bool, hover: Option<&str>, open: bool) -> image::RgbaImage {
    // Two of the combo boxes have no label: the scene compares looks, not names.
    let mut h = Harness::builder().with_size(egui::vec2(480.0, 360.0)).with_accessibility_check(false).build_ui(move |ui| scene(ui, ours));
    h.run();
    if open {
        h.get_by_role_and_label(egui::Role::ComboBox, "Open").click();
        h.run();
    }
    if let Some(label) = hover {
        h.get_by_role_and_label(egui::Role::ComboBox, label).hover();
    }
    h.run();
    h.render().unwrap_or_else(|e| panic!("render: {e}"))
}

#[test]
fn the_button_and_menu_look_like_eguis() {
    for (hover, open) in [(None, false), (Some("Size"), false), (None, true)] {
        let (egui, ours) = (render(false, hover, open), render(true, hover, open));
        let differ = egui.pixels().zip(ours.pixels()).filter(|(a, b)| a != b).count();
        assert_eq!(differ, 0, "hover {hover:?}, open {open}: {differ} pixels differ");
    }
}
