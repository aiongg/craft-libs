//! Hosting menus on the overlay contract: one popup per part, toggling on press, a binding-free container, keep-open
//! rows, injected activations and the opening reader.

use super::kit::PlainStyle;
use craft_behavior::{HitLevel, HitPriority, PartKey, Sense};
use craft_ui::buttons::Button;
use craft_ui::contract::overlay::{Dismiss, Outside, Popup};
use craft_ui::contract::{
    Access, Common, Component, Cx, MeasureCx, Name, Overlay, OverlayUi, Overlays as _, PartsCx, Pointer as _, Props as _, Role, Show as _,
    ShowContainer as _, Trigger,
};
use craft_ui::menu::{Menu, MenuItem};
use craft_ui::runtime::commands::{FireSource, Fired};
use craft_ui::runtime::frame::take_output;
use craft_ui::runtime::memory::Remember as _;
use craft_ui::style::StyleCx;
use craft_ui::testing::input::key_press;
use craft_ui::testing::pointer::{click, moved, press, release};
use craft_ui::testing::{Frames, trial};
use egui::{Key, Modifiers, Pos2, Rect, Response, Ui, Vec2};
use std::cell::{Cell, RefCell};

const TITLES: Rect = Rect { min: Pos2::new(100.0, 100.0), max: Pos2::new(230.0, 120.0) };
const FIRST: PartKey = PartKey::new("first");
const SECOND: PartKey = PartKey::new("second");
const FIRST_AT: Pos2 = Pos2::new(130.0, 110.0);
const SECOND_AT: Pos2 = Pos2::new(200.0, 110.0);

/// What the rows of a popup saw.
#[derive(Default)]
struct Rows {
    shown: [Cell<bool>; 2],
    activations: Cell<u32>,
}

/// One component with a toggling part per popup.
#[derive(Component)]
#[component(overlay)]
struct Titles<'a> {
    #[common]
    common: Common,
    on_press: bool,
    /// The first part has a sub-node of its own.
    node: bool,
    rows: &'a Rows,
}

impl Component for Titles<'_> {
    const ROLE: Role = Role::BUTTON;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: TitlesStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        TITLES.size()
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn parts(&mut self, _: Rect, cx: &mut PartsCx<'_, Self>) {
        for (key, at, index) in [(FIRST, TITLES.min, 0), (SECOND, TITLES.min + Vec2::new(70.0, 0.0), 1)] {
            let rect = Rect::from_min_size(at, Vec2::new(60.0, 20.0));
            let part = cx.part(key, rect).sense(Sense::CLICK | Sense::HOVER).priority(HitPriority::new(HitLevel::Body, 0)).toggles_at("menu", index);
            if self.on_press {
                part.on_press();
            }
        }
    }

    fn access(&self, access: &mut Access, _: &Cx<'_, Self>) {
        if self.node {
            access.node(FIRST, Role::BUTTON).name = Name::Text("First".to_owned());
        }
    }

    fn overlay(&mut self, ui: &mut OverlayUi<'_>, cx: &mut Cx<'_, Self>) {
        for (key, index) in [(FIRST, 0), (SECOND, 1)] {
            let popup = cx.popup("menu").at(index);
            let overlay = Overlay::menu().dismiss(Dismiss::new().outside(Outside::Absorb).on_activate(true)).anchor(ui.anchor_part(key));
            let rows = self.rows;
            let shown = ui.show(popup, overlay, |ui, _| {
                if MenuItem::new("Row").show(ui).ran {
                    rows.activations.set(rows.activations.get() + 1);
                }
            });
            rows.shown[index].set(shown.is_some());
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

fn show_titles<'a>(rows: &'a Rows, on_press: bool) -> impl FnMut(&mut Ui) + 'a {
    move |ui| {
        rows.shown[0].set(false);
        rows.shown[1].set(false);
        Titles { common: Common::new(), on_press, node: false, rows }.id_salt("titles").show_at(ui, TITLES);
    }
}

#[trial]
fn each_toggling_part_owns_a_popup_and_opening_one_closes_the_other() {
    let (mut frames, rows) = (Frames::new(), Rows::default());
    let mut show = show_titles(&rows, false);
    frames.idle(3, &mut show);
    click(&mut frames, FIRST_AT, &mut show);
    frames.idle(5, &mut show);
    assert_eq!((rows.shown[0].get(), rows.shown[1].get()), (true, false), "the first part opened its own popup");
    click(&mut frames, SECOND_AT, &mut show);
    frames.idle(30, &mut show);
    assert_eq!((rows.shown[0].get(), rows.shown[1].get()), (false, true), "the second popup opened and closed the first");
}

/// A part that toggles on press opens on the press; a release over a row picks it, and a release on the part leaves the popup open.
#[trial]
fn a_part_that_toggles_on_press_opens_then_a_release_picks_and_a_click_leaves_it_open() {
    let (mut frames, rows) = (Frames::new(), Rows::default());
    let mut show = show_titles(&rows, true);
    frames.idle(3, &mut show);
    frames.frame(vec![moved(FIRST_AT)], &mut show);
    frames.frame(vec![press(FIRST_AT)], &mut show);
    frames.idle(2, &mut show);
    assert!(rows.shown[0].get(), "the popup opened while the button is still down");
    // The row hangs below the part, from its start edge.
    let row = FIRST_AT + Vec2::new(0.0, 30.0);
    frames.frame(vec![moved(row)], &mut show);
    frames.frame(vec![release(row)], &mut show);
    frames.idle(30, &mut show);
    assert_eq!(rows.activations.get(), 1, "the release over the row picked it");
    click(&mut frames, FIRST_AT, &mut show);
    frames.idle(2, &mut show);
    assert!(rows.shown[0].get(), "a release on the part leaves the popup open");
    assert_eq!(rows.activations.get(), 1, "and runs no row");
}

#[trial]
fn a_menu_with_no_binding_opens_from_its_opener_and_closes_on_escape() {
    let mut frames = Frames::new();
    let shown = Cell::new(false);
    let mut show = |ui: &mut Ui| {
        let opener = Button::new("File").show_at(ui, TITLES);
        shown.set(false);
        Menu::new("File").opened_by(&opener, Trigger::Click).show(ui, |ui, _| {
            shown.set(true);
            MenuItem::new("New").show(ui);
        });
    };
    frames.idle(3, &mut show);
    assert!(!shown.get(), "closed until the opener is clicked");
    click(&mut frames, TITLES.center(), &mut show);
    frames.idle(5, &mut show);
    assert!(shown.get(), "the opener's click opened the menu");
    frames.frame(vec![key_press(Key::Escape, Modifiers::NONE)], &mut show);
    frames.idle(30, &mut show);
    assert!(!shown.get(), "Escape closed it");
}

/// A row that asks to keep the layer open, or not.
#[derive(Component)]
struct KeepRow<'a> {
    #[common]
    common: Common,
    keep: bool,
    ran: &'a Cell<u32>,
}

impl Component for KeepRow<'_> {
    const ROLE: Role = Role::BUTTON;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: KeepRowStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(80.0, 30.0)
    }

    fn sense(&self) -> Sense {
        Sense::CLICK | Sense::FOCUS
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        if cx.activation().is_some() {
            self.ran.set(self.ran.get() + 1);
            if self.keep {
                cx.keep_open();
            }
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// Opens a menu layer whose one row keeps it open or not, clicks the row, and reports (row ran, layer still shown).
fn click_a_row(keep: bool) -> (u32, bool) {
    let mut frames = Frames::new();
    let (ran, shown) = (Cell::new(0), Cell::new(false));
    let mut show = |ui: &mut Ui| {
        let opener = Button::new("Open").show_at(ui, TITLES);
        shown.set(false);
        Menu::new("Rows").opened_by(&opener, Trigger::Click).show(ui, |ui, _| {
            shown.set(true);
            KeepRow { common: Common::new(), keep, ran: &ran }.id_salt("row").show(ui);
        });
    };
    frames.idle(3, &mut show);
    click(&mut frames, TITLES.center(), &mut show);
    frames.idle(5, &mut show);
    let row = TITLES.left_bottom() + Vec2::new(30.0, 20.0);
    click(&mut frames, row, &mut show);
    frames.idle(30, &mut show);
    (ran.get(), shown.get())
}

#[trial]
fn a_keep_open_row_runs_and_leaves_the_layer_a_plain_row_closes_the_chain() {
    assert_eq!(click_a_row(true), (1, true), "the keep-open row ran and the layer stayed");
    assert_eq!(click_a_row(false), (1, false), "a plain row ran and closed the layer");
}

/// Whether the row injected its activation.
#[derive(Clone, Default)]
struct Injected(bool);

/// A command row that injects its own activation.
#[derive(Component)]
struct InjectRow {
    #[common]
    common: Common,
}

impl Component for InjectRow {
    const ROLE: Role = Role::BUTTON;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: InjectRowStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(80.0, 30.0)
    }

    fn sense(&self) -> Sense {
        Sense::CLICK
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let injected = cx.memory::<Injected>();
        if !injected.0 {
            injected.0 = true;
            cx.activate(FireSource::Key);
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

#[trial]
fn an_injected_activation_fires_the_bound_command_once_with_its_source() {
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        InjectRow { common: Common::new() }.command(super::kit::Go::Go).id_salt("row").show_at(ui, TITLES);
    };
    take_output(&frames.ctx);
    frames.idle(4, &mut show);
    let fired: Vec<Fired> = take_output(&frames.ctx).fired.into_iter().filter(|fired| fired.command.id == "overlayTrial.go").collect();
    assert_eq!(fired.len(), 1, "one activation fired the command once");
    assert_eq!(fired[0].source, FireSource::Key, "with the injected source");
}

/// Reports `Popup::opening` of its popup each pass.
#[derive(Component)]
#[component(overlay)]
struct Opening<'a> {
    #[common]
    common: Common,
    open: bool,
    seen: &'a RefCell<Vec<bool>>,
}

impl Component for Opening<'_> {
    const ROLE: Role = Role::BUTTON;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: OpeningStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        TITLES.size()
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let popup = cx.popup("menu");
        if self.open {
            popup.open(cx);
        }
    }

    fn overlay(&mut self, ui: &mut OverlayUi<'_>, cx: &mut Cx<'_, Self>) {
        let popup = cx.popup("menu");
        let shown = ui.show(popup, Overlay::menu(), |ui, _| {
            MenuItem::new("Row").show(ui);
        });
        if shown.is_some() {
            self.seen.borrow_mut().push(Popup::opening(popup, cx));
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

#[trial]
fn the_opening_reader_is_true_only_in_the_frame_the_layer_first_shows() {
    let mut frames = Frames::new();
    let seen = RefCell::new(Vec::new());
    let open = Cell::new(false);
    let mut show = |ui: &mut Ui| {
        Opening { common: Common::new(), open: open.get(), seen: &seen }.id_salt("opening").show_at(ui, TITLES);
    };
    frames.idle(3, &mut show);
    open.set(true);
    frames.frame(vec![], &mut show);
    open.set(false);
    frames.idle(5, &mut show);
    let seen = seen.into_inner();
    assert_eq!(seen.first(), Some(&true), "the first shown pass is the opening");
    assert!(seen.len() > 2 && seen[seen.len() - 3..].iter().all(|opening| !opening), "later frames are not");
}

/// The root's expanded state with the first part toggling, with and without a sub-node of its own.
fn root_expanded(node: bool) -> Option<bool> {
    let mut frames = Frames::new();
    let rows = Rows::default();
    let root = Cell::new(egui::Id::NULL);
    let mut show = |ui: &mut Ui| {
        root.set(Titles { common: Common::new(), on_press: false, node, rows: &rows }.id_salt("titles").show_at(ui, TITLES).id);
    };
    frames.idle(4, &mut show);
    frames.tree().expanded(root.get())
}

#[trial]
fn the_root_reports_its_popup_state_unless_a_toggling_part_has_a_sub_node_of_its_own() {
    assert_eq!(root_expanded(false), Some(false), "a toggling part with no sub-node leaves the state to the root");
    assert_eq!(root_expanded(true), None, "the part's sub-node carries the state, not the root");
}
