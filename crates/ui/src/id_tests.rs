//! The id a control or composite shows under is the one `Interactive::id_in` (or its own
//! `id_in`) gives its salt, so a host that reads focus or state before showing it finds the
//! same widget.

use egui::{Context, Id, IdSalt, Pos2, RawInput, Rect, Ui, vec2};

use crate::behavior::RowInfo;
use crate::composites::{Curve, CurveEditor, SectionHeader};
use crate::controls::angle_dial::DialOp;
use crate::controls::{AngleDial, NumberField};
use crate::edit::parts;
use crate::list::{List, ListRow, Table, TableColumn, TableLayout};
use crate::localized::LocaleNumbers;
use crate::menu::facts_id;
use crate::primitives::Interactive;
use crate::theme::{Base, install};

fn run(show: impl FnMut(&mut Ui)) {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let mut show = show;
    for _ in 0..2 {
        let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))), ..Default::default() };
        ctx.run_ui(input, &mut show).textures_delta.clear();
    }
}

#[test]
fn id_in_is_the_shown_id() {
    let numbers = LocaleNumbers::from_tag("en");
    let curve = Curve::identity(255.0);
    run(|ui| {
        let at = |salt: &str| -> Id { Interactive::id_in(ui, salt) };
        let (dial, number, header) = (at("dial"), at("number"), at("header"));
        // The dial's response is its needle element; the control's own state is at its id.
        let _ = AngleDial::new("dial", "Angle", 10.0, &numbers).show(ui);
        assert!(ui.data(|d| d.get_temp::<parts::State<DialOp>>(dial)).is_some(), "AngleDial");
        assert_eq!(NumberField::new("number", "Width", 1.0, &numbers).show(ui).response.id, number, "NumberField");
        assert_eq!(SectionHeader::new("header", "Header").show(ui, |_| ()).response.id, header, "SectionHeader");
        let curve_id = CurveEditor::id_in(ui, "curve");
        assert_eq!(CurveEditor::new("curve", "Curve", &curve, &numbers).show(ui).response.id, curve_id, "CurveEditor");
    });
}

/// Focus given to `id` before the first pass is the control's focus in every pass when `id` is
/// the id the control reads its memory under.
fn focus_reaches(id_of: impl Fn(&Ui) -> Id, mut show: impl FnMut(&mut Ui) -> egui::Response) {
    let mut focused = Vec::new();
    run(|ui| {
        if focused.is_empty() {
            let id = id_of(ui);
            ui.memory_mut(|m| m.request_focus(id));
        }
        let response = show(ui);
        assert_eq!(response.id, id_of(ui));
        focused.push(response.has_focus());
    });
    assert_eq!(focused, [true, true]);
}

#[test]
fn list_id_in_is_the_id_its_memory_is_read_under() {
    let rows: Vec<RowInfo<usize>> = (0..3).map(RowInfo::leaf).collect();
    focus_reaches(
        |ui| List::id_in(ui, "layers"),
        |ui| List::new("layers", "Layers", rows.as_slice()).show(ui, |_, &key| ListRow::new(key, format!("Row {key}"))).response,
    );
}

#[test]
fn table_id_in_is_the_id_its_memory_is_read_under() {
    let rows: Vec<RowInfo<usize>> = (0..3).map(RowInfo::leaf).collect();
    let columns = vec![TableColumn::new("name", "Name")];
    let mut layout = TableLayout::new(&columns);
    focus_reaches(
        |ui| Table::id_in(ui, "files"),
        |ui| {
            Table::new("files", "Files", &columns, &mut layout, rows.as_slice())
                .show(ui, |ui, row, _, cx| {
                    cx.label(ui, format!("Row {row}"));
                })
                .response
        },
    );
}

#[test]
fn facts_id_is_the_same_from_the_raw_and_the_stored_salt() {
    // A panel's `show` holds the salt it was made with; its `panel_menu` gets the raw salt.
    let stored = IdSalt::new("history");
    assert_eq!(facts_id(stored), facts_id(IdSalt::new("history")));
    assert_eq!(facts_id(stored), Id::NULL.with("history"));
    assert_ne!(facts_id(stored), Id::NULL.with(stored), "a stored salt hashed again is another id");
}
