//! The title bar with its menus, and the status bar.

use craft_theme::ControlSize;
use craft_ui::controls::{IconButton, NumberField};
use craft_ui::dialog::DialogHost;
use craft_ui::icons::Icon;
use craft_ui::localized::{LocaleNumbers, Unit};
use craft_ui::menu::{MenuBar, MenuEntry, MenuItem};
use craft_ui::patterns::StatusBar;
use craft_ui::title_bar::{CaptionFamily, Captions, Frame as WindowFrame, LeadingEnd, Title, TitleBar, WindowState};
use egui::Ui;

use super::dialog;
use super::dock::DockState;
use super::host_dialogs;
use super::model::Mock;
use super::panels::Panel;

/// What a menu item does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    ImageSize,
    NewLayer,
    DeleteLayer,
    DeleteLayerAsk,
    GaussianBlur,
    Rulers,
    FitOnScreen,
    Panel(Panel),
    About,
}

fn item(label: &str, command: Command) -> MenuItem<Command> {
    MenuItem::new(label, command)
}

/// An item that is drawn but has no effect in the preview.
fn inert(label: &str) -> MenuEntry<Command> {
    item(label, Command::About).enabled(false).into()
}

/// The menu bar's menus for the current state of the app.
fn menus(mock: &Mock, dock: &DockState) -> Vec<MenuItem<Command>> {
    let unlocked = mock.doc.selected.iter().any(|id| mock.doc.layer(*id).is_some_and(|l| !l.locked));
    vec![
        MenuItem::submenu("File", vec![inert("New…"), inert("Open…"), MenuEntry::Separator, inert("Close")]),
        MenuItem::submenu("Edit", vec![inert("Undo"), inert("Redo")]),
        MenuItem::submenu("Image", vec![item("Image Size…", Command::ImageSize).into()]),
        MenuItem::submenu(
            "Layer",
            vec![
                item("New Layer", Command::NewLayer).into(),
                item("Delete Layer", Command::DeleteLayer).enabled(unlocked).into(),
                item("Delete Layer\u{2026}", Command::DeleteLayerAsk).enabled(unlocked).into(),
            ],
        ),
        MenuItem::submenu("Filter", vec![item("Gaussian Blur\u{2026}", Command::GaussianBlur).into()]),
        MenuItem::submenu(
            "View",
            vec![item("Rulers", Command::Rulers).check(mock.view.rulers).into(), item("Fit on Screen", Command::FitOnScreen).into()],
        ),
        MenuItem::submenu("Window", Panel::ALL.iter().map(|p| item(p.label(), Command::Panel(*p)).check(dock.is_visible(*p)).into()).collect()),
        MenuItem::submenu("Help", vec![item("About", Command::About).into()]),
    ]
}

fn run(command: Command, mock: &mut Mock, dock: &mut DockState, dialogs: &mut DialogHost) {
    match command {
        Command::ImageSize => dialog::open_image_size(mock),
        Command::NewLayer => mock.add_layer(),
        Command::DeleteLayer => mock.delete_selected(),
        Command::DeleteLayerAsk => host_dialogs::open_delete_layer(dialogs, mock),
        Command::GaussianBlur => host_dialogs::open_blur(dialogs),
        Command::Rulers => mock.view.rulers = !mock.view.rulers,
        Command::FitOnScreen => mock.view.fit_pending = true,
        Command::Panel(panel) => dock.toggle(panel),
        Command::About => mock.status = "Craft app preview".to_owned(),
    }
}

/// The title bar as a picture of a window: the menus at the start, the document's name in the
/// middle, search at the end.
pub fn title_bar(ui: &mut Ui, mock: &mut Mock, dock: &mut DockState, dialogs: &mut DialogHost) {
    let frame = WindowFrame::Integrated(Captions { family: CaptionFamily::Generic, layout: Default::default() });
    let title = Title::new(mock.doc.name.clone()).unsaved(mock.doc.modified);
    let menu_list = menus(mock, dock);
    let menu_id = ui.scope_id().with("menus");
    let title_id = ui.scope_id().with("title");
    let mut chosen = None;
    TitleBar::new(&frame).id_salt(title_id).preview(WindowState::default()).title(title).separator(true).show_titled(
        ui,
        |ui, _| {
            chosen = MenuBar::new(&menu_list).id(menu_id).show(ui).chosen;
            LeadingEnd::from_ui(ui)
        },
        |ui, _| {
            ui.add(IconButton::new(Icon::Search, "Search"));
        },
    );
    if let Some(command) = chosen {
        run(command, mock, dock, dialogs);
    }
}

/// The status bar: zoom, colour mode, size, layer count and the message.
pub fn status_bar(ui: &mut Ui, mock: &mut Mock) {
    let numbers = LocaleNumbers::of(ui);
    let [width, height] = mock.doc.size;
    let layers = mock.doc.layers.len();
    let resolution = mock.doc.resolution;
    let mut zoom = None;
    StatusBar::new("status").show(ui, |bar| {
        bar.leading(|ui| {
            StatusBar::field(ui, |ui| {
                let edit = NumberField::new("zoom", "Zoom", mock.view.zoom * 100.0, &numbers)
                    .range(1.0..=6400.0)
                    .decimals(0..=1)
                    .unit(Unit::new("{n}%"))
                    .size(ControlSize::Sm)
                    .show(ui);
                zoom = edit.event.as_ref().and_then(|e| e.value()).copied();
            });
            StatusBar::info(ui, "RGB/8");
            StatusBar::separator(ui);
            StatusBar::info(ui, &format!("{width} x {height} px ({resolution} ppi)"));
            StatusBar::separator(ui);
            StatusBar::info(ui, &if layers == 1 { "1 layer".to_owned() } else { format!("{layers} layers") });
            StatusBar::message(ui, &mock.status, false);
        });
        bar.trailing(|ui| {
            StatusBar::warning(ui, "CPU Rendering");
        });
    });
    if let Some(percent) = zoom {
        mock.view.set_zoom(percent / 100.0);
    }
}
