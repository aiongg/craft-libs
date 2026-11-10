//! `SplitButton`: the main part runs the action and never opens the menu, the arrow and Alt+Down open it, a pick
//! reports the entry as one choice and closes the menu.

use craft_behavior::{IconId, Mixed};
use craft_theme::{Rgba, contrast_ratio, text_contrast_min};
use craft_ui::buttons::{Button, IconButton, SplitButton, ToggleButton};
use craft_ui::contract::{Bind, CommandSet, Entry, Role, Show as _, ShowContainer as _};
use craft_ui::dialog::Dialog;
use craft_ui::primitives::IconSpec;
use craft_ui::runtime::commands::{CommandInput, CommandState};
use craft_ui::runtime::focus::Initial;
use craft_ui::sections::{DialogRole, FooterButton};
use craft_ui::testing::focus::AccessActionKind;
use craft_ui::testing::{Fixture, Frames, trial};
use egui::{Id, Key, Modifiers};
use std::cell::Cell;
use std::rc::Rc;

struct Rig {
    fixture: Fixture,
    clicks: Rc<Cell<u32>>,
    chosen: Rc<Cell<Option<u8>>>,
    id: Rc<Cell<Option<Id>>>,
}

fn rig() -> Rig {
    let (clicks, chosen, id) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(None)), Rc::new(Cell::new(None)));
    let (shown_clicks, shown_chosen, shown_id) = (clicks.clone(), chosen.clone(), id.clone());
    let mut fixture = Fixture::new(move |ui| {
        let entries =
            [Entry::new(7u8, "Colour").checked(true), Entry::separator(), Entry::new(8, "Duotone").enabled(false), Entry::new(9, "Grayscale")];
        let out = SplitButton::new("Trace", &entries).show(ui);
        shown_id.set(Some(out.response.id));
        if craft_ui::testing::contract::clicked(&out.response) {
            shown_clicks.set(shown_clicks.get() + 1);
        }
        if out.chosen.is_some() {
            shown_chosen.set(out.chosen);
        }
    });
    fixture.settle();
    Rig { fixture, clicks, chosen, id }
}

impl Rig {
    fn menu_shown(&self) -> bool {
        !self.fixture.by_role(Role::MENU).is_empty()
    }

    fn press(&mut self, key: Key, modifiers: Modifiers) {
        self.fixture.key(key, modifiers);
        self.fixture.idle(2);
    }
}

/// AccessKit Expand on the arrow's sub-node opens the menu and Collapse closes it, and the sub-node reports which.
#[trial]
fn expand_and_collapse_on_the_arrow_open_and_close_the_menu() {
    let mut rig = rig();
    let id = rig.id.get().expect("the button showed");
    let arrow = |rig: &Rig| rig.fixture.part(id, craft_behavior::PartKey::new("arrow")).expect("the arrow has a node");
    assert_eq!(rig.fixture.tree().expanded_of(arrow(&rig).node), Some(false), "the closed menu's arrow is expandable");
    let target = arrow(&rig);
    rig.fixture.access(AccessActionKind::Expand, &target);
    rig.fixture.idle(4);
    assert!(rig.menu_shown(), "Expand opens the menu");
    assert_eq!(rig.fixture.tree().expanded_of(arrow(&rig).node), Some(true), "the arrow reports it open");
    let target = arrow(&rig);
    rig.fixture.access(AccessActionKind::Collapse, &target);
    rig.fixture.idle(4);
    assert!(!rig.menu_shown(), "Collapse closes it");
    assert_eq!(rig.fixture.tree().expanded_of(arrow(&rig).node), Some(false), "the arrow reports it closed");
}

/// Enter on the focused button runs the action and leaves the menu closed.
#[trial]
fn a_split_button_runs_its_action_without_opening_the_menu() {
    let mut rig = rig();
    rig.press(Key::Tab, Modifiers::NONE);
    rig.press(Key::Enter, Modifiers::NONE);
    assert_eq!(rig.clicks.get(), 1);
    assert!(!rig.menu_shown());
}

/// Alt+Down opens the menu with its first row active; Enter picks it once and closes the menu without running the
/// action.
#[trial]
fn a_split_button_opens_from_the_keyboard_and_reports_the_pick() {
    let mut rig = rig();
    rig.press(Key::Tab, Modifiers::NONE);
    rig.press(Key::ArrowDown, Modifiers::ALT);
    assert!(rig.menu_shown(), "Alt+Down opens the menu");
    rig.press(Key::Enter, Modifiers::NONE);
    assert_eq!(rig.chosen.get(), Some(7), "the first row was active");
    rig.fixture.idle(4);
    assert!(!rig.menu_shown(), "a pick closes the menu");
    assert_eq!(rig.clicks.get(), 0, "a pick is not the action");
}

/// Down from the first row skips the separator and the disabled entry, and Enter reports the value of the entry it
/// lands on.
#[trial]
fn a_split_button_menu_skips_separators_and_disabled_entries() {
    let mut rig = rig();
    rig.press(Key::Tab, Modifiers::NONE);
    rig.press(Key::ArrowDown, Modifiers::ALT);
    rig.press(Key::ArrowDown, Modifiers::NONE);
    rig.press(Key::Enter, Modifiers::NONE);
    assert_eq!(rig.chosen.get(), Some(9));
}

/// A click on a disabled entry picks nothing and leaves the menu open.
#[trial]
fn a_disabled_split_button_entry_cannot_be_picked() {
    let mut rig = rig();
    rig.press(Key::Tab, Modifiers::NONE);
    rig.press(Key::ArrowDown, Modifiers::ALT);
    let entry = rig.fixture.by_label("Duotone").expect("the disabled entry is listed");
    rig.fixture.click(&entry);
    rig.fixture.idle(2);
    assert_eq!(rig.chosen.get(), None);
    assert!(rig.menu_shown(), "the menu stays open");
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, CommandSet)]
#[command(prefix = "buttons")]
enum Panel {
    #[command(label = "Comments")]
    Comments,
}

/// How many circles an icon button bound to `Panel::Comments` paints while the command is `marked`.
fn marker_circles(marked: bool) -> usize {
    let mut frames = Frames::new();
    frames.commands(CommandInput::new().state(Panel::Comments, CommandState::new().marked(marked)));
    let mut show = |ui: &mut egui::Ui| {
        IconButton::command(IconSpec::svg(craft_behavior::IconId::new("settings")), Panel::Comments).show(ui);
    };
    frames.idle(2, &mut show);
    frames.frame_shapes(&mut show).iter().filter(|shape| matches!(shape, egui::Shape::Circle(_))).count()
}

/// A marked command shows its marker dot on the button; an unmarked one shows none.
#[trial]
fn an_icon_button_paints_its_commands_marker_only_while_marked() {
    assert_eq!(marker_circles(false), 0, "no dot without the mark");
    assert!(marker_circles(true) > 0, "the marked command paints its dot");
}

/// A tinted button reports its clicks.
#[trial]
fn a_tinted_button_reports_clicks() {
    let tint = Rgba([0xC4, 0x40, 0x40, 0xFF]);
    let clicks = Rc::new(Cell::new(0));
    let seen = clicks.clone();
    let mut fixture = Fixture::new(move |ui| {
        if craft_ui::testing::contract::clicked(&Button::new("Play").tint(tint).show(ui)) {
            seen.set(seen.get() + 1);
        }
    });
    fixture.settle();
    let button = fixture.by_label("Play").expect("the button");
    fixture.click(&button);
    assert_eq!(clicks.get(), 1, "a click on a tinted button is reported");
}

/// A tinted button's label reads on a light, a dark and a mid fill.
#[trial]
fn a_tinted_buttons_label_reads_on_light_dark_and_mid_fills() {
    let least = text_contrast_min(craft_theme::Contrast::Standard);
    for tint in [Rgba([0xF5, 0xF0, 0xC8, 0xFF]), Rgba([0x14, 0x1E, 0x46, 0xFF]), Rgba([0x80, 0x80, 0x80, 0xFF])] {
        let mut show = move |ui: &mut egui::Ui| {
            Button::new("Play").tint(tint).show(ui);
        };
        let mut frames = Frames::new();
        frames.idle(2, &mut show);
        let shapes = frames.frame_shapes(&mut show);
        let ink = shapes
            .iter()
            .find_map(|shape| match shape {
                egui::Shape::Text(text) => text.galley.job.sections.first().map(|section| section.format.color),
                _ => None,
            })
            .expect("the label is painted");
        let ratio = contrast_ratio(Rgba::from(ink), tint);
        assert!(ratio >= least, "the label reads on {tint:?}: {ratio} against {least}");
    }
}

/// A selection with both states: the button reports the mixed toggled state, and a click makes it on.
#[trial]
fn a_mixed_toggle_reports_mixed_and_a_click_turns_it_on() {
    let value = Rc::new(Cell::new(Mixed::Mixed));
    let shown = value.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut current = shown.get();
        ToggleButton::new("Bold", Bind::bound_mixed(&mut current)).show(ui);
        shown.set(current);
    });
    fixture.settle();
    let button = fixture.by_label("Bold").expect("the button");
    assert_eq!(fixture.tree().toggled_of(button.node), Some(Mixed::Mixed), "a mixed value reads as mixed, not on or off");
    fixture.click(&button);
    fixture.idle(2);
    assert_eq!(value.get(), Mixed::Same(true), "a click on a mixed toggle makes it on");
    let button = fixture.by_label("Bold").expect("the button");
    assert_eq!(fixture.tree().toggled_of(button.node), Some(Mixed::Same(true)));
}

/// A mixed toggle paints the off look, fill and edge, not a blend of off and on; labelled or icon-only.
#[trial]
fn a_mixed_toggle_paints_the_off_look() {
    for icon_only in [false, true] {
        let look_of = |value: Mixed<bool>| {
            let mut show = move |ui: &mut egui::Ui| {
                let mut current = value;
                let button = ToggleButton::new("Bold", Bind::bound_mixed(&mut current)).icon(IconId::new("check"));
                if icon_only { button.icon_only() } else { button }.show(ui);
            };
            let mut frames = Frames::new();
            frames.idle(2, &mut show);
            let rects: Vec<_> = frames
                .frame_shapes(&mut show)
                .iter()
                .filter_map(|shape| match shape {
                    egui::Shape::Rect(rect) => Some((rect.fill, rect.stroke.color)),
                    _ => None,
                })
                .collect();
            assert!(!rects.is_empty(), "the toggle paints a surface");
            rects
        };
        assert_eq!(look_of(Mixed::Mixed), look_of(Mixed::Same(false)), "a mixed toggle paints the off look (icon only: {icon_only})");
        assert_ne!(look_of(Mixed::Mixed), look_of(Mixed::Same(true)), "and not the on look (icon only: {icon_only})");
    }
}

/// An armed toggle that disarms with Escape turns off on the first Escape and leaves the dialog open; the second
/// Escape, with nothing armed, closes the dialog.
#[trial]
fn escape_turns_off_an_armed_toggle_before_it_closes_the_dialog() {
    let armed = Rc::new(Cell::new(true));
    let open = Rc::new(Cell::new(true));
    let (shown_armed, shown_open) = (armed.clone(), open.clone());
    let mut fixture = Fixture::new(move |ui| {
        let (mut is_open, mut on) = (shown_open.get(), shown_armed.get());
        Dialog::new("Adjust")
            .opened(Bind::bound(&mut is_open))
            .initial_focus(Initial::First)
            .buttons([FooterButton::new("OK", DialogRole::Accept), FooterButton::new("Cancel", DialogRole::Cancel)])
            .show(ui, |ui, _| {
                ToggleButton::new("Pick", Bind::bound(&mut on)).escape_disarms().show(ui);
            });
        shown_open.set(is_open);
        shown_armed.set(on);
    });
    fixture.settle();
    fixture.key(Key::Escape, Modifiers::NONE).settle();
    assert!(!armed.get(), "Escape turned the toggle off");
    assert!(open.get(), "the first Escape leaves the dialog open");
    fixture.key(Key::Escape, Modifiers::NONE).settle();
    assert!(!open.get(), "the second Escape closes the dialog");
}
