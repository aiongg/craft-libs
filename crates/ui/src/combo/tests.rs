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

fn sizes(ui: &mut egui::Ui, rows: usize) {
    for n in 0..rows {
        let size = SIZES.get(n).map_or_else(|| format!("Row {n}"), |s| (*s).to_owned());
        let _ = ui.selectable_label(n == 1, size);
    }
}

/// Combo boxes in the states a user sees them in, drawn with egui's combo box or ours.
#[expect(clippy::disallowed_types, reason = "the reference our combo box is compared with")]
fn scene(ui: &mut egui::Ui, ours: bool) {
    macro_rules! both {
        ($ui:ident, $rows:expr; $ctor:ident ( $($arg:expr),* ) $(. $m:ident ( $($a:expr),* ))*) => {
            if ours {
                ComboBox::$ctor($($arg),*)$(.$m($($a),*))*.show_ui($ui, |ui| sizes(ui, $rows)).response
            } else {
                egui::ComboBox::$ctor($($arg),*)$(.$m($($a),*))*.show_ui($ui, |ui| sizes(ui, $rows)).response
            }
        };
    }
    let _ = both!(ui, 3; from_label("Size").selected_text("Medium"));
    let _ = both!(ui, 3; from_id_salt("narrow").selected_text("A long selected value that is cut short").width(120.0).truncate());
    let _ = both!(ui, 3; new("wrapped", "Wrapped").selected_text("A long selected value that wraps onto a second line").width(140.0).wrap());
    let _ = ui.add_enabled_ui(false, |ui| both!(ui, 3; new("disabled", "Disabled").selected_text("Medium")));
    let _ = both!(ui, 3; new("open", "Open").selected_text("Medium").width(160.0));
    let _ = both!(ui, 40; new("long", "Long").selected_text("Row 1").height(300.0));
}

fn render(ours: bool, hover: Option<&str>, open: Option<&str>) -> image::RgbaImage {
    // Two of the combo boxes have no label: the scene compares looks, not names.
    let mut h = Harness::builder().with_size(egui::vec2(480.0, 700.0)).with_accessibility_check(false).build_ui(move |ui| scene(ui, ours));
    h.run();
    if let Some(label) = open {
        h.get_by_role_and_label(egui::Role::ComboBox, label).click();
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
    for (hover, open) in [(None, None), (Some("Size"), None), (None, Some("Open")), (None, Some("Long"))] {
        let (egui, ours) = (render(false, hover, open), render(true, hover, open));
        let differ = egui.pixels().zip(ours.pixels()).filter(|(a, b)| a != b).count();
        assert_eq!(differ, 0, "hover {hover:?}, open {open:?}: {differ} pixels differ");
    }
}

/// A combo box at `top` in a window `screen` tall, with `rows` rows (none while filtered).
struct Long {
    rows: usize,
    filtered: bool,
    to_end: bool,
    last_row: egui::Rect,
    button: egui::Rect,
    popup: egui::Id,
}

fn long_menu(top: f32, screen: f32, height: f32, rows: usize) -> Harness<'static, Long> {
    Harness::builder().with_size(egui::vec2(400.0, screen)).build_ui_state(
        move |ui, m: &mut Long| {
            ui.add_space(top);
            let inner = ComboBox::new("long", "Long").selected_text("Row 0").height(height).show_ui(ui, |ui| {
                let shown = if m.filtered { 2 } else { m.rows };
                for n in 0..shown {
                    m.last_row = ui.selectable_label(false, format!("Row {n}")).rect;
                }
                if std::mem::take(&mut m.to_end) {
                    ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                }
            });
            m.button = inner.response.rect;
            m.popup = super::popup_id(inner.response.id);
        },
        Long { rows, filtered: false, to_end: false, last_row: egui::Rect::NOTHING, button: egui::Rect::NOTHING, popup: egui::Id::NULL },
    )
}

fn menu_rect(h: &Harness<'_, Long>) -> egui::Rect {
    h.ctx.memory(|m| m.area_rect(h.state().popup)).unwrap_or(egui::Rect::NOTHING)
}

#[test]
fn an_unbounded_menu_fits_on_screen_and_scrolls_to_its_last_row() {
    let mut h = long_menu(0.0, 300.0, f32::INFINITY, 100);
    h.run();
    h.get_by_role_and_label(egui::Role::ComboBox, "Long").click();
    h.run();
    let menu = menu_rect(&h);
    assert!(menu.bottom() <= 300.0 + 0.5, "the menu ends on screen: {menu:?}");
    h.state_mut().to_end = true;
    h.run();
    let last = h.state().last_row;
    assert!(last.bottom() <= menu_rect(&h).bottom() && last.top() >= menu_rect(&h).top(), "the last row {last:?} scrolls into the menu {menu:?}");
}

#[test]
fn a_menu_that_shrinks_and_grows_while_open_stays_on_its_side() {
    // Too little room below for the full list: the menu opens above. Filtered, it would fit
    // below too.
    let mut h = long_menu(480.0, 600.0, 380.0, 60);
    h.run();
    h.get_by_role_and_label(egui::Role::ComboBox, "Long").click();
    h.run();
    let above = |h: &Harness<'_, Long>| menu_rect(h).bottom() <= h.state().button.top() + 0.5;
    assert!(above(&h), "opens above: {:?}", menu_rect(&h));
    h.state_mut().filtered = true;
    h.run();
    assert!(above(&h), "stays above when it shrinks: {:?}", menu_rect(&h));
    h.state_mut().filtered = false;
    h.run();
    assert!(above(&h), "and when it grows back: {:?}", menu_rect(&h));
    assert!(menu_rect(&h).top() >= -0.5, "keeping to the room there: {:?}", menu_rect(&h));
}

#[test]
fn a_footer_stays_under_the_list() {
    let mut h = Harness::builder().with_size(egui::vec2(400.0, 600.0)).build_ui_state(
        |ui, s: &mut (bool, egui::Rect, egui::Rect)| {
            let _ = ComboBox::new("buses", "Send").selected_text("Bus 1").height(300.0).show_ui_with_header_footer(
                ui,
                |ui| ui.label("Buses"),
                |ui, _| {
                    for n in 0..50 {
                        let _ = ui.selectable_label(false, format!("Bus {n}"));
                    }
                    if std::mem::take(&mut s.0) {
                        ui.scroll_with_delta(egui::vec2(0.0, -200.0));
                    }
                    s.1 = ui.clip_rect();
                },
                |ui| s.2 = ui.button("New bus…").rect,
            );
        },
        (false, egui::Rect::NOTHING, egui::Rect::NOTHING),
    );
    h.run();
    h.get_by_role_and_label(egui::Role::ComboBox, "Send").click();
    h.run();
    let (list, footer) = (h.state().1, h.state().2);
    assert!(footer.top() >= list.bottom() - 0.5, "the footer {footer:?} is under the list {list:?}");
    h.state_mut().0 = true;
    h.run();
    assert_eq!(h.state().2, footer, "the footer stays put while the list scrolls");
    assert!(h.query_by_label("New bus…").is_some());
}

struct Typed {
    text: String,
    shown: bool,
    open_with_focus: bool,
}

#[test]
fn a_menu_under_a_text_field_leaves_it_the_focus_while_typing() {
    for open_with_focus in [false, true] {
        let mut h = Harness::new_ui_state(
            |ui, t: &mut Typed| {
                let name = ui.label("Font").id;
                let field = ui.text_edit_singleline(&mut t.text).labelled_by(name);
                let mut menu = super::Menu::new(ui, &field).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside);
                if t.open_with_focus {
                    menu = menu.open(field.has_focus());
                }
                let text = t.text.clone();
                t.shown = menu.show_ui(|ui| ui.label(format!("Fonts matching {text:?}"))).is_some();
            },
            Typed { text: String::new(), shown: false, open_with_focus },
        );
        h.run();
        h.get_by_role_and_label(egui::Role::TextInput, "Font").click();
        h.run();
        h.get_by_role_and_label(egui::Role::TextInput, "Font").type_text("ar");
        h.run();
        let field_focused = h.get_by_role_and_label(egui::Role::TextInput, "Font").is_focused();
        assert_eq!(h.state().text, "ar", "open_with_focus {open_with_focus}");
        assert!(field_focused, "the field keeps the focus (open_with_focus {open_with_focus})");
        assert!(h.state().shown, "the menu stays open (open_with_focus {open_with_focus})");
        assert!(h.query_by_label("Fonts matching \"ar\"").is_some(), "the menu follows the text");
    }
}
