//! Under a modal dialog the window still moves by the part of its title bar the modal covers, and the dialog keeps
//! the presses on itself and its keys.

use crate::windows_kit::Windows;
use craft_ui::contract::{Bind, Show as _, ShowContainer as _};
use craft_ui::dialog::Dialog;
use craft_ui::runtime::InstallSpec;
use craft_ui::runtime::focus::Initial;
use craft_ui::runtime::overlay::CloseReason;
use craft_ui::runtime::windows::{WindowHit, WindowHost};
use craft_ui::sections::{DialogRole, FooterButton};
use craft_ui::shell::{Caption, CaptionFamily, Captions, Frame, Title, TitleBar};
use craft_ui::testing::input::key_press;
use craft_ui::testing::pointer::{moved, press, release};
use craft_ui::testing::trial;
use craft_ui::testing::windows::NoWindows;
use craft_ui::text::TextField;
use craft_window::policy::{Decision, Desktop, DesktopKind};
use egui::{Key, Modifiers, Pos2, Rect, ViewportCommand, ViewportId, vec2};
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

/// Where the bar and the dialog lay in the last frame, whether the dialog is open, and how it closed and which footer
/// button it picked.
struct Seen {
    bar: Cell<Rect>,
    dialog: Cell<Rect>,
    open: Cell<bool>,
    closed: Cell<Option<CloseReason>>,
    picked: Cell<Option<usize>>,
}

impl Seen {
    fn new() -> Self {
        Seen {
            bar: Cell::new(Rect::NOTHING),
            dialog: Cell::new(Rect::NOTHING),
            open: Cell::new(true),
            closed: Cell::new(None),
            picked: Cell::new(None),
        }
    }
}

/// One frame of a window with its title bar and a modal dialog whose top-left corner is at `at`: a field that takes
/// the focus first, then OK (the default button) and Cancel. Returns the window commands it sent.
fn frame(windows: &Windows, events: Vec<egui::Event>, at: Pos2, seen: &Seen) -> Vec<(ViewportId, ViewportCommand)> {
    windows
        .frame(events, vec![], |ui| {
            seen.bar.set(TitleBar::new().show_titled(ui, Title::new("Landscape.psd"), |_, _| {}, |_, _| {}).response.rect);
            let mut open = seen.open.get();
            let mut spacing = String::new();
            let shown = Dialog::new("Grid options")
                .first_place(at)
                .opened(Bind::bound(&mut open))
                .initial_focus(Initial::First)
                .close_on_enter(true)
                .buttons([FooterButton::new("OK", DialogRole::Accept), FooterButton::new("Cancel", DialogRole::Cancel)])
                .show(ui, |ui, _| {
                    TextField::new("Spacing", Bind::bound(&mut spacing)).show(ui);
                });
            seen.open.set(open);
            if open {
                seen.dialog.set(shown.output.response.rect);
            }
            if let Some(closed) = shown.output.closed {
                seen.closed.set(Some(closed));
            }
            if let Some(picked) = shown.output.picked {
                seen.picked.set(Some(picked));
            }
        })
        .commands
}

/// The window with the modal moved into its title bar, shown until the dialog's strips stand; the point on the bar
/// beside the dialog.
fn modal_over_bar(windows: &Windows, at: Pos2, seen: &Seen) -> Pos2 {
    // The strips read the last complete pass: the first publishes the bar's drag areas, the second shows the strips.
    for _ in 0..2 {
        frame(windows, vec![], at, seen);
    }
    let (bar, dialog) = (seen.bar.get(), seen.dialog.get());
    assert!(dialog.intersects(bar), "the dialog lies over the bar");
    assert!(dialog.top() < bar.center().y && dialog.left() > bar.left(), "the bar is free before the dialog's start edge");
    let beside = Pos2::new((bar.left() + dialog.left()) / 2.0, bar.center().y);
    let regions = WindowHost::of(&windows.ctx).caption_regions(ViewportId::ROOT).expect("the bar registered its parts");
    assert_eq!(regions.hit(beside), Some(WindowHit::Move), "the covered bar beside the dialog is a drag area no hit covers");
    beside
}

/// The frames of `steps`, one per entry, and the window commands they sent.
fn run(windows: &Windows, steps: Vec<Vec<egui::Event>>, at: Pos2, seen: &Seen) -> Vec<(ViewportId, ViewportCommand)> {
    steps.into_iter().flat_map(|events| frame(windows, events, at, seen)).collect()
}

/// A primary drag from `from` along the bar: the window commands of its frames.
fn drag(windows: &Windows, from: Pos2, at: Pos2, seen: &Seen) -> Vec<(ViewportId, ViewportCommand)> {
    let along = |by: f32| from + vec2(by, 0.0);
    let steps = vec![vec![moved(from)], vec![press(from)], vec![moved(along(20.0))], vec![moved(along(40.0))], vec![release(along(40.0))], vec![]];
    run(windows, steps, at, seen)
}

fn moves_window(sent: &[(ViewportId, ViewportCommand)]) -> bool {
    sent.iter().any(|(viewport, command)| *viewport == ViewportId::ROOT && matches!(command, ViewportCommand::StartDrag))
}

/// With a modal dialog open and moved into the title bar, a drag on the bar beside the dialog moves the window, and
/// a drag that starts on the dialog where it lies over the bar does not.
#[trial]
fn a_modal_leaves_the_covered_title_bar_draggable_but_not_the_dialog() {
    let windows = framed_window();
    let seen = Seen::new();
    let at = Pos2::new(300.0, 4.0);
    let beside = modal_over_bar(&windows, at, &seen);
    let on_dialog = Pos2::new(seen.dialog.get().center().x, seen.bar.get().center().y);
    let regions = WindowHost::of(&windows.ctx).caption_regions(ViewportId::ROOT).expect("the bar registered its parts");
    assert_ne!(regions.hit(on_dialog), Some(WindowHit::Move), "the dialog stays in front of the bar");

    assert!(moves_window(&drag(&windows, beside, at, &seen)), "a drag on the covered bar moves the window");
    assert!(!moves_window(&drag(&windows, on_dialog, at, &seen)), "a drag on the dialog stays the dialog's");
}

/// The strips over the bar take no keys: Escape cancels the modal under them, and Enter activates its default
/// button.
#[trial]
fn a_modal_keeps_escape_and_enter_while_its_strips_show() {
    let at = Pos2::new(300.0, 4.0);
    let escaped = (framed_window(), Seen::new());
    modal_over_bar(&escaped.0, at, &escaped.1);
    run(&escaped.0, vec![vec![key_press(Key::Escape, Modifiers::NONE)], vec![]], at, &escaped.1);
    assert!(!escaped.1.open.get(), "Escape closes the modal");
    assert_eq!(escaped.1.closed.get(), Some(CloseReason::Escape), "the modal closed by Escape");

    let entered = (framed_window(), Seen::new());
    modal_over_bar(&entered.0, at, &entered.1);
    run(&entered.0, vec![vec![key_press(Key::Enter, Modifiers::NONE)], vec![]], at, &entered.1);
    assert_eq!(entered.1.picked.get(), Some(0), "Enter picks OK, the default button");
    assert!(!entered.1.open.get(), "OK closes the modal");
}

/// A double click on the bar beside the modal maximizes the window, as one on the free title bar does.
#[trial]
fn a_double_click_beside_a_modal_maximizes_the_window() {
    let windows = framed_window();
    let seen = Seen::new();
    let at = Pos2::new(300.0, 4.0);
    let beside = modal_over_bar(&windows, at, &seen);
    let steps = vec![vec![moved(beside)], vec![press(beside)], vec![release(beside)], vec![press(beside)], vec![release(beside)], vec![]];
    let sent = run(&windows, steps, at, &seen);
    assert!(
        sent.iter().any(|(viewport, command)| *viewport == ViewportId::ROOT && matches!(command, ViewportCommand::Maximized(true))),
        "the double click maximizes the window: {sent:?}"
    );
    assert!(seen.open.get(), "the modal stays open");
}
