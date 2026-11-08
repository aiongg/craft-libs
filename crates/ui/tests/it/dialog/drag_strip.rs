//! Under a modal dialog the window still moves by the part of its title bar the modal covers, and the dialog keeps
//! the presses on itself.

use crate::windows_kit::Windows;
use craft_ui::contract::{Show as _, ShowContainer as _};
use craft_ui::dialog::Dialog;
use craft_ui::display::Text;
use craft_ui::runtime::InstallSpec;
use craft_ui::shell::{Caption, CaptionFamily, Captions, Frame, Title, TitleBar};
use craft_ui::testing::pointer::{moved, press, release};
use craft_ui::testing::trial;
use craft_ui::testing::windows::NoWindows;
use craft_window::policy::{Decision, Desktop, DesktopKind};
use egui::{Pos2, Rect, ViewportCommand, ViewportId, vec2};
use std::cell::Cell;
use std::sync::Arc;

/// A window whose app draws the frame, with the Windows captions at the end.
fn framed_window() -> Windows {
    let layout = craft_window::Layout { left: vec![], right: vec![Caption::Minimize, Caption::Maximize, Caption::Close] };
    let frame = Frame::Integrated(Captions { family: CaptionFamily::Windows, layout });
    let desktop = Desktop { kind: DesktopKind::Other, ssd: None, gtk_theme: None, icon_theme: None };
    Windows::with(InstallSpec::new(craft_host::target::ShellInfo::default(), Arc::new(NoWindows)).frame(Decision {
        frame,
        overridden: None,
        desktop,
    }))
}

/// Where the bar and the dialog lay in the last frame.
#[derive(Default)]
struct Seen {
    bar: Cell<Rect>,
    dialog: Cell<Rect>,
}

/// One frame of a window with its title bar and a modal dialog whose top-left corner is at `at`; the window
/// commands it sent.
fn frame(windows: &Windows, events: Vec<egui::Event>, at: Pos2, seen: &Seen) -> Vec<(ViewportId, ViewportCommand)> {
    windows
        .frame(events, vec![], |ui| {
            seen.bar.set(TitleBar::new().show_titled(ui, Title::new("Landscape.psd"), |_, _| {}, |_, _| {}).response.rect);
            let shown = Dialog::new("Grid options").first_place(at).show(ui, |ui, _| {
                Text::new("Snap to grid").show(ui);
            });
            seen.dialog.set(shown.output.response.rect);
        })
        .commands
}

/// A primary drag from `from` along the bar: the window commands of its frames.
fn drag(windows: &Windows, from: Pos2, at: Pos2, seen: &Seen) -> Vec<(ViewportId, ViewportCommand)> {
    let along = |by: f32| from + vec2(by, 0.0);
    let steps = [vec![moved(from)], vec![press(from)], vec![moved(along(20.0))], vec![moved(along(40.0))], vec![release(along(40.0))], vec![]];
    steps.into_iter().flat_map(|events| frame(windows, events, at, seen)).collect()
}

fn moves_window(sent: &[(ViewportId, ViewportCommand)]) -> bool {
    sent.iter().any(|(viewport, command)| *viewport == ViewportId::ROOT && matches!(command, ViewportCommand::StartDrag))
}

/// With a modal dialog open and moved into the title bar, a drag on the bar beside the dialog moves the window, and
/// a drag that starts on the dialog where it lies over the bar does not.
#[trial]
fn a_modal_leaves_the_covered_title_bar_draggable_but_not_the_dialog() {
    let windows = framed_window();
    let seen = Seen::default();
    let at = Pos2::new(300.0, 4.0);
    for _ in 0..6 {
        frame(&windows, vec![], at, &seen);
    }
    let (bar, dialog) = (seen.bar.get(), seen.dialog.get());
    assert!(dialog.intersects(bar), "the dialog lies over the bar");
    assert!(dialog.top() < bar.center().y && dialog.left() > bar.left(), "the bar is free before the dialog's start edge");

    let beside = Pos2::new((bar.left() + dialog.left()) / 2.0, bar.center().y);
    assert!(moves_window(&drag(&windows, beside, at, &seen)), "a drag on the covered bar moves the window");

    let on_dialog = Pos2::new(dialog.center().x, bar.center().y);
    assert!(!moves_window(&drag(&windows, on_dialog, at, &seen)), "a drag on the dialog stays the dialog's");
}
