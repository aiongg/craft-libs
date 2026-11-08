//! `Section` keeps its open state in the caller's binding or a remembered slot and shows its body only while open; a
//! `TitleRow` too narrow for both texts drops the subtitle.

use craft_ui::choice::Checkbox;
use craft_ui::contract::{Bind, Props as _, Role, Show as _, ShowContainer as _};
use craft_ui::menu::MenuItem;
use craft_ui::runtime::focus::{FocusTarget, request};
use craft_ui::sections::{HeaderAction, Section, TitleRow};
use craft_ui::testing::input::key_press;
use craft_ui::testing::pointer::click;
use craft_ui::testing::{Fixture, Frames, trial};
use egui::{Key, Modifiers, Pos2, Rect, Ui};
use std::cell::Cell;

const AREA: Rect = Rect { min: Pos2::new(10.0, 10.0), max: Pos2::new(310.0, 110.0) };

/// Longer than a collapse takes to close.
const COLLAPSE_SECONDS: f64 = 1.0;

/// The point on the disclosure at the start of a section's header.
fn disclosure() -> Pos2 {
    Pos2::new(AREA.left() + 15.0, AREA.top() + 14.0)
}

#[trial]
fn a_section_shows_its_body_while_its_binding_is_open_and_the_disclosure_flips_the_binding() {
    let mut frames = Frames::new();
    let (mut open, ran) = (true, Cell::new(false));
    let show = |ui: &mut Ui, open: &mut bool| {
        ran.set(false);
        Section::new("Options").open(Bind::bound(open)).id_salt("section").show_at(ui, AREA, |_, _| ran.set(true));
    };
    frames.idle(3, |ui| show(ui, &mut open));
    assert!(ran.get(), "an open section runs its body");
    click(&mut frames, disclosure(), &mut |ui| show(ui, &mut open));
    // The collapse animates: the body stays until the close has run its course.
    frames.advance(COLLAPSE_SECONDS);
    frames.idle(2, |ui| show(ui, &mut open));
    assert!(!open && !ran.get(), "the click writes the binding and the body stops running");
}

#[trial]
fn a_remembered_section_keeps_its_state_without_a_binding_and_a_fixed_section_is_always_open() {
    let mut frames = Frames::new();
    let ran = Cell::new(false);
    let show = |ui: &mut Ui, fixed: bool| {
        ran.set(false);
        let section = if fixed {
            Section::fixed("Options")
        } else {
            Section::new("Options").remember(craft_ui::contract::SaveKey::new("craft_ui_tests.section_open"))
        };
        section.id_salt(("section", fixed)).show_at(ui, AREA, |_, _| ran.set(true));
    };
    frames.idle(3, |ui| show(ui, false));
    assert!(ran.get(), "a remembered section starts open");
    click(&mut frames, disclosure(), &mut |ui| show(ui, false));
    frames.advance(COLLAPSE_SECONDS);
    frames.idle(2, |ui| show(ui, false));
    assert!(!ran.get(), "the saved slot holds the closed state");
    frames.idle(3, |ui| show(ui, true));
    assert!(ran.get(), "a fixed section has no state to close");
}

#[trial]
fn a_title_row_drops_its_subtitle_before_it_elides_the_title() {
    let texts = |width: f32| {
        let area = Rect::from_min_size(Pos2::new(10.0, 10.0), egui::vec2(width, 30.0));
        let mut fixture = Fixture::new(move |ui| {
            TitleRow::new("Options").subtitle("A subtitle that is long").id_salt("row").show_at(ui, area);
        });
        fixture.settle();
        fixture.texts().iter().map(|painted| painted.text.clone()).collect::<Vec<_>>()
    };
    let (wide, narrow) = (400.0, 90.0);
    assert_eq!(texts(wide).len(), 2, "a wide row shows title and subtitle");
    assert_eq!(texts(narrow), ["Options"], "a narrow row keeps the whole title and drops the subtitle");
}

/// Escape with focus inside an expanded section's body leaves it expanded: a collapse is the user's own toggle.
#[trial]
fn escape_in_a_section_body_leaves_it_expanded() {
    let mut frames = Frames::new();
    let (mut open, ran, check) = (true, Cell::new(false), Cell::new(None));
    let show = |ui: &mut Ui, open: &mut bool| {
        ran.set(false);
        Section::new("Options").open(Bind::bound(open)).id_salt("section").show_at(ui, AREA, |ui, _| {
            ran.set(true);
            check.set(Some(Checkbox::new(Bind::given(false)).label("Snap").show(ui).response.id));
        });
    };
    frames.idle(3, |ui| show(ui, &mut open));
    request(&frames.ctx, FocusTarget::Id(check.get().expect("the body ran")));
    frames.idle(2, |ui| show(ui, &mut open));
    frames.frame(vec![key_press(Key::Escape, Modifiers::NONE)], |ui| show(ui, &mut open));
    frames.idle(2, |ui| show(ui, &mut open));
    assert!(open && ran.get(), "Escape leaves the section expanded and its body shown");
}

/// The section menu opens from the menu key like any other container menu, not only from a button.
#[trial]
fn shift_f10_on_a_focused_section_opens_its_menu() {
    let mut fixture = Fixture::new(|ui| {
        Section::new("Options")
            .actions([HeaderAction::Menu])
            .menu(|ui, _| {
                MenuItem::new("Rename").show(ui);
            })
            .show(ui, |_, _| {});
    });
    fixture.settle();
    assert!(fixture.by_role(Role::MENU).is_empty(), "the menu starts closed");
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.key(Key::F10, Modifiers::SHIFT);
    fixture.settle();
    assert!(!fixture.by_role(Role::MENU).is_empty(), "Shift+F10 on the focused section opens its menu");
}

struct Rows(usize);

impl craft_behavior::DisplayOrder<usize> for Rows {
    fn len(&self) -> usize {
        self.0
    }

    fn key(&self, at: usize) -> usize {
        at
    }
}

impl craft_behavior::ItemSource<usize> for Rows {}

/// A scrolling list takes its room from the space the section is offered in this pass, so its rows show; the section's
/// own height before its first pass is zero.
#[trial]
fn a_scrolling_list_in_a_fixed_section_shows_its_rows() {
    use craft_ui::contract::ShowCollection as _;
    use craft_ui::list::{List, ListRow};
    let mut fixture = Fixture::new(|ui| {
        Section::fixed("Recent").show(ui, |ui, _| {
            let _ = List::new("Items").show_items(ui, &Rows(3), &mut |key, _| ListRow::new(format!("Row {key}")));
        });
    });
    fixture.settle();
    for key in 0..3 {
        assert!(fixture.by_label(&format!("Row {key}")).is_some(), "row {key} is shown");
    }
}
