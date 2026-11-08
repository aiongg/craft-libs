//! The app shell's own rules: the screen-mode cycle, the full-screen steps, the reveal rules and the slot order.

use craft_ui::canvas::{Canvas, ToolPath};
use craft_ui::contract::{Bind, PaneEdge, Show as _, ShowContainer as _};
use craft_ui::dialog::{DialogResult, PromptChoice, StandardPrompt};
use craft_ui::display::Text;
use craft_ui::runtime::env::{RevealEdge, RootEnv, WindowKey};
use craft_ui::runtime::frame::ChromeToggle;
use craft_ui::shell::{Aim, AppShell, AppShellOutput, Reveal, ScreenMode, ScreenModeCommand, Slot, SlotSize, Tracking};
use craft_ui::testing::input::key_press;
use craft_ui::testing::pointer::{click, moved, settle};
use craft_ui::testing::{Fixture, Frames, trial};
use egui::{Key, Modifiers, Pos2, Rect, Ui, pos2};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const WINDOW: Rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(800.0, 600.0));
const BAR: Rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(800.0, 32.0));

fn aim(pointer: Pos2, frozen: bool, rtl: bool) -> Aim {
    Aim::new(Some(pointer), frozen, WINDOW, rtl)
}

fn body_only() -> Tracking {
    Tracking::new(true, true, true)
}

fn edges(reveal: &Reveal) -> [bool; 3] {
    [RevealEdge::Top, RevealEdge::Start, RevealEdge::End].map(|edge| reveal.edge(edge))
}

#[trial]
fn the_f_key_cycles_the_modes_and_shift_f_goes_back() {
    let mut mode = ScreenMode::Standard;
    for expected in [ScreenMode::FullScreenWithMenuBar, ScreenMode::FullScreen, ScreenMode::Standard] {
        mode = mode.next();
        assert_eq!(mode, expected);
    }
    for mode in [ScreenMode::Standard, ScreenMode::FullScreenWithMenuBar, ScreenMode::FullScreen] {
        assert_eq!(mode.next().previous(), mode);
        assert_eq!(ScreenModeCommand::Cycle.apply(mode), mode.next());
        assert_eq!(ScreenModeCommand::CycleBack.apply(mode), mode.previous());
        assert_eq!(ScreenModeCommand::of(mode).apply(ScreenMode::Standard), mode);
    }
}

#[trial]
fn each_mode_hides_what_it_hides() {
    assert!(ScreenMode::Standard.tabs_visible() && ScreenMode::Standard.scrollbars_visible());
    assert!(!ScreenMode::Standard.is_full_screen() && !ScreenMode::Standard.body_only());
    let with_bars = ScreenMode::FullScreenWithMenuBar;
    assert!(!with_bars.tabs_visible() && !with_bars.scrollbars_visible() && with_bars.is_full_screen() && !with_bars.body_only());
    assert!(ScreenMode::FullScreen.body_only() && ScreenMode::FullScreen.is_full_screen());
}

#[trial]
fn the_pointer_at_the_top_start_or_end_edge_reveals_that_edge_only() {
    let cases = [(pos2(400.0, 1.0), [true, false, false]), (pos2(1.0, 300.0), [false, true, false]), (pos2(799.0, 300.0), [false, false, true])];
    for (pointer, expected) in cases {
        let mut reveal = Reveal::default();
        reveal.track(body_only(), aim(pointer, false, false));
        assert_eq!(edges(&reveal), expected, "pointer at {pointer:?}");
    }
    let mut reveal = Reveal::default();
    reveal.track(body_only(), aim(pos2(400.0, 599.0), false, false));
    assert_eq!(edges(&reveal), [false; 3], "the bottom edge never reveals");
}

#[trial]
fn start_and_end_mirror_under_right_to_left() {
    let mut reveal = Reveal::default();
    reveal.track(body_only(), aim(pos2(1.0, 300.0), false, true));
    assert_eq!(edges(&reveal), [false, false, true]);
}

#[trial]
fn a_held_button_or_open_popup_freezes_the_edge_reveal() {
    let mut reveal = Reveal::default();
    reveal.track(body_only(), aim(pos2(400.0, 1.0), false, false));
    reveal.track(body_only(), aim(pos2(400.0, 300.0), true, false));
    assert!(reveal.edge(RevealEdge::Top), "frozen: the reveal does not end");
    reveal.track(body_only(), aim(pos2(400.0, 300.0), false, false));
    assert!(!reveal.edge(RevealEdge::Top));

    let mut frozen = Reveal::default();
    frozen.track(body_only(), aim(pos2(400.0, 1.0), true, false));
    assert!(!frozen.edge(RevealEdge::Top), "frozen: the reveal does not start");
}

#[trial]
fn the_pointer_over_a_revealed_slot_keeps_its_edge_revealed() {
    let mut reveal = Reveal::default();
    reveal.track(body_only(), aim(pos2(400.0, 1.0), false, false));
    reveal.record(vec![(RevealEdge::Top, BAR)]);
    reveal.track(body_only(), aim(pos2(400.0, 20.0), false, false));
    assert!(reveal.edge(RevealEdge::Top));
    reveal.track(body_only(), aim(pos2(400.0, 100.0), false, false));
    assert!(!reveal.edge(RevealEdge::Top));
}

#[trial]
fn a_reveal_shows_the_revealing_slots_and_never_the_status_bar() {
    let options = Slot::new("options").reveal();
    let status = Slot::new("status");
    let mut reveal = Reveal::default();
    assert!(reveal.shows(&options, PaneEdge::Top, false, true) && reveal.shows(&status, PaneEdge::Bottom, false, true), "normal mode: the flags");
    assert!(!reveal.shows(&options, PaneEdge::Top, true, true));
    reveal.track(body_only(), aim(pos2(400.0, 1.0), false, false));
    assert!(reveal.shows(&options, PaneEdge::Top, true, true));
    assert!(!reveal.shows(&options, PaneEdge::Start, true, true), "only the revealed edge's slots show");
    assert!(!reveal.shows(&options, PaneEdge::Top, true, false), "a presentation ignores the reveal");

    reveal.apply(&[ChromeToggle::Panels], true, true);
    assert!(reveal.tab());
    assert!(reveal.shows(&options, PaneEdge::Start, true, true), "the Tab reveal shows every revealing slot");
    assert!(!reveal.shows(&status, PaneEdge::Bottom, true, true));
}

#[trial]
fn tab_toggles_the_chrome_and_tab_again_shows_it() {
    let mut reveal = Reveal::default();
    reveal.apply(&[ChromeToggle::Panels], false, true);
    assert!(reveal.only(false), "Tab hides the chrome");
    reveal.apply(&[ChromeToggle::Panels], false, true);
    assert!(!reveal.only(false), "Tab again shows the chrome");
}

#[trial]
fn shift_tab_hides_the_panels_and_keeps_the_tools_in_normal_mode_only() {
    let toggles = [ChromeToggle::PanelsKeepTools];
    let mut reveal = Reveal::default();
    reveal.apply(&toggles, false, true);
    assert!(reveal.panels_hidden() && !reveal.only(false));
    let mut body_only_window = Reveal::default();
    body_only_window.apply(&toggles, true, true);
    assert!(!body_only_window.panels_hidden());
}

#[trial]
fn a_screen_mode_change_ends_a_reveal() {
    let mut reveal = Reveal::default();
    reveal.track(body_only(), aim(pos2(400.0, 1.0), false, false));
    reveal.apply(&[ChromeToggle::Panels], true, true);
    reveal.clear();
    assert!(!reveal.tab() && edges(&reveal) == [false; 3]);
}

fn rect_of(out: &AppShellOutput, key: &str) -> Rect {
    out.regions.slots.iter().find(|(shown, _)| *shown == key).map(|(_, rect)| *rect).expect("the slot showed")
}

#[trial]
fn slots_lay_out_around_the_body_on_the_first_pass() {
    let seen = RefCell::new(None);
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        let out = AppShell::new(WindowKey::main())
            .title_bar(|ui, _, _| {
                Text::new("Title").show(ui);
            })
            .bottom(Slot::new("status"), |ui, _, _| {
                Text::new("Status").show(ui);
            })
            .start(Slot::new("tools"), |ui, _, _| {
                Text::new("Tools").show(ui);
            })
            .body(|ui, _, _| {
                Text::new("Document").show(ui);
            })
            .show(ui, &mut ());
        *seen.borrow_mut() = Some(out);
    };
    frames.frame(Vec::new(), &mut show);
    let out = seen.borrow_mut().take().expect("the shell showed");
    let body = out.regions.body;
    assert!(rect_of(&out, "title").max.y <= body.min.y, "the title is above the body");
    assert!(rect_of(&out, "status").min.y >= body.max.y, "the status bar is below the body");
    assert!(rect_of(&out, "tools").max.x <= body.min.x, "the tools are at the start of the body");
    let keys: Vec<_> = out.regions.slots.iter().map(|(key, _)| *key).collect();
    assert_eq!(keys, ["title", "status", "tools"], "slots claim space in the order title, bottom, start");
}

#[trial]
fn full_screen_shows_the_body_only() {
    let seen = RefCell::new(None);
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        let mut mode = ScreenMode::FullScreen;
        let out = AppShell::new(WindowKey::main())
            .screen_mode(Bind::bound(&mut mode))
            .title_bar(|ui, _, _| {
                Text::new("Title").show(ui);
            })
            .body(|ui, _, _| {
                Text::new("Document").show(ui);
            })
            .show(ui, &mut ());
        *seen.borrow_mut() = Some(out);
    };
    frames.frame(Vec::new(), &mut show);
    let out = seen.borrow_mut().take().expect("the shell showed");
    assert!(out.regions.slots.is_empty() && !out.revealed, "full screen shows the body only");
    assert_eq!(out.screen_mode, ScreenMode::FullScreen);
}

#[trial]
fn a_slot_is_as_high_as_its_size_says_whatever_its_content() {
    let seen = RefCell::new(None);
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        let out = AppShell::new(WindowKey::main())
            .top(Slot::new("short").size(SlotSize::OptionsBar), |ui, _, _| {
                Text::new("Short").show(ui);
            })
            .top(Slot::new("tall").size(SlotSize::OptionsBar), |ui, _, _| {
                craft_ui::testing::layout::allocate(ui, egui::vec2(10.0, 300.0));
            })
            .top(Slot::new("narrow").size(SlotSize::OptionsBarNarrow), |ui, _, _| {
                Text::new("Narrow").show(ui);
            })
            .show(ui, &mut ());
        *seen.borrow_mut() = Some(out);
    };
    frames.frame(Vec::new(), &mut show);
    let out = seen.borrow_mut().take().expect("the shell showed");
    let height = |key| rect_of(&out, key).height();
    assert_eq!(height("short"), height("tall"), "the size, not the content, sets the extent");
    assert!(height("narrow") <= height("short"), "the narrow options bar is not taller than the options bar");
}

#[trial]
fn revealing_the_top_bar_in_body_only_mode_leaves_the_body_rect_alone() {
    let seen = RefCell::new(None);
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        let mut mode = ScreenMode::FullScreen;
        let out = AppShell::new(WindowKey::main())
            .screen_mode(Bind::bound(&mut mode))
            .title(Slot::new("title").reveal(), |ui, _, _| {
                Text::new("Title").show(ui);
            })
            .body(|ui, _, _| {
                Text::new("Document").show(ui);
            })
            .show(ui, &mut ());
        *seen.borrow_mut() = Some(out);
    };
    frames.frame(vec![moved(pos2(400.0, 300.0))], &mut show);
    let hidden = seen.borrow_mut().take().expect("the shell showed");
    assert!(hidden.regions.slots.is_empty());
    frames.frame(vec![moved(pos2(400.0, 1.0))], &mut show);
    let revealed = seen.borrow_mut().take().expect("the shell showed");
    assert!(revealed.revealed, "the pointer at the top edge reveals the title in the same pass");
    assert_eq!(revealed.regions.body, hidden.regions.body, "a revealed bar covers the body");
    assert!(rect_of(&revealed, "title").max.y > revealed.regions.body.min.y);
}

#[trial]
fn a_body_that_registers_a_revealed_rect_keeps_the_reveal_while_the_pointer_is_inside_it() {
    fn run(register: bool) -> bool {
        let kept = RefCell::new(false);
        let mut frames = Frames::new();
        let mut show = |ui: &mut Ui| {
            let mut mode = ScreenMode::FullScreen;
            AppShell::new(WindowKey::main())
                .screen_mode(Bind::bound(&mut mode))
                .body(|_, cx, _| {
                    let shell = cx.shell().expect("the body is in a shell");
                    if register && shell.edge_revealed(RevealEdge::Start) {
                        shell.reveal_rect(RevealEdge::Start, Rect::from_min_max(pos2(0.0, 40.0), pos2(120.0, 560.0)));
                    }
                    *kept.borrow_mut() = shell.edge_revealed(RevealEdge::Start);
                })
                .show(ui, &mut ());
        };
        frames.frame(vec![moved(pos2(300.0, 300.0))], &mut show);
        frames.frame(vec![moved(pos2(1.0, 300.0))], &mut show);
        frames.frame(vec![moved(pos2(1.0, 300.0))], &mut show);
        assert!(*kept.borrow(), "the pointer at the start edge reveals it");
        frames.frame(vec![moved(pos2(60.0, 300.0))], &mut show);
        let kept_inside = *kept.borrow();
        kept_inside
    }
    assert!(run(true), "inside the registered rect the reveal stays");
    assert!(!run(false), "without the rect the reveal ends when the pointer leaves the strip");
}

fn tab_hides(shells: &[&'static str]) -> Vec<(&'static str, bool)> {
    let seen = RefCell::new(Vec::new());
    let window = RefCell::new(Rect::NOTHING);
    let mut frames = Frames::new();
    let mut root = RootEnv::default();
    root.tab_toggles_chrome = true;
    frames.root(root);
    let mut show = |ui: &mut Ui| {
        seen.borrow_mut().clear();
        let room = ui.max_rect();
        *window.borrow_mut() = room;
        for (at, key) in shells.iter().enumerate() {
            let width = room.width() / shells.len() as f32;
            let rect = Rect::from_min_size(room.min + egui::vec2(width * at as f32, 0.0), egui::vec2(width, room.height()));
            craft_ui::testing::layout::in_region(ui, rect, |ui| {
                let out = AppShell::new(WindowKey::new(*key))
                    .title_bar(|ui, _, _| {
                        Text::new("Title").show(ui);
                    })
                    .body(|ui, _, _| {
                        Canvas::new("Document", ToolPath::Freehand).show(ui, |_, _| ());
                    })
                    .show(ui, &mut ());
                seen.borrow_mut().push((*key, !out.regions.slots.is_empty()));
            });
        }
    };
    settle(&mut frames, &mut show);
    let room = *window.borrow();
    click(&mut frames, pos2(room.max.x - room.width() / (2.0 * shells.len() as f32), room.center().y), &mut show);
    settle(&mut frames, &mut show);
    frames.frame(vec![key_press(Key::Tab, Modifiers::NONE)], &mut show);
    let result = seen.borrow().clone();
    result
}

#[trial]
fn tab_with_a_canvas_focused_hides_that_windows_panels_in_the_same_pass() {
    assert_eq!(tab_hides(&["main"]), [("main", false)]);
}

#[trial]
fn two_shells_in_one_viewport_toggle_apart() {
    assert_eq!(tab_hides(&["first", "second"]), [("first", true), ("second", false)], "Tab goes to the window that holds focus");
}

#[trial]
fn the_slots_and_the_body_edit_one_state_in_turn() {
    let seen = RefCell::new(Vec::new());
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        let mut order: Vec<&'static str> = Vec::new();
        AppShell::new(WindowKey::main())
            .title_bar(|_, _, order: &mut Vec<&'static str>| order.push("title"))
            .bottom(Slot::new("status"), |_, _, order| order.push("status"))
            .body(|_, _, order| order.push("body"))
            .show(ui, &mut order);
        *seen.borrow_mut() = order;
    };
    frames.frame(Vec::new(), &mut show);
    assert_eq!(*seen.borrow(), ["title", "status", "body"], "one state, handed to each region in the order they run");
}

const OPEN_FRAMES: usize = 30;

#[trial]
fn a_body_opens_a_prompt_through_the_window_host_and_takes_the_answer_in_a_later_pass() {
    let asked = Rc::new(Cell::new(false));
    let open = Rc::new(Cell::new(false));
    let answer = Rc::new(Cell::new(None));
    let (asked_in, open_in, answer_in) = (asked.clone(), open.clone(), answer.clone());
    let mut fixture = Fixture::new(move |ui| {
        AppShell::new(WindowKey::main())
            .body(|_, cx, _| {
                let dialogs = cx.dialogs().expect("the body is in a shell");
                dialogs.with(|host| {
                    if !asked_in.replace(true) {
                        host.open(StandardPrompt::confirm_delete(["Layer"], 1));
                    }
                    open_in.set(host.is_open::<StandardPrompt>());
                    if let Some(answered) = host.take::<StandardPrompt>().and_then(DialogResult::ok) {
                        answer_in.set(Some(answered.choice));
                    }
                });
            })
            .show(ui, &mut ());
    });
    fixture.idle(OPEN_FRAMES);
    assert!(open.get() && answer.get().is_none(), "the prompt shows over the window");
    fixture.key(Key::Enter, Modifiers::NONE).idle(2);
    assert_eq!(answer.get(), Some(PromptChoice::Proceed), "the body takes the answer in the next pass");
    assert!(!open.get(), "the default button closed the prompt");
}

#[trial]
fn two_windows_keep_separate_dialog_hosts() {
    let open = RefCell::new(Vec::new());
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        open.borrow_mut().clear();
        let room = ui.max_rect();
        for (at, key) in ["first", "second"].into_iter().enumerate() {
            let rect = Rect::from_min_size(room.min + egui::vec2(room.width() / 2.0 * at as f32, 0.0), egui::vec2(room.width() / 2.0, room.height()));
            craft_ui::testing::layout::in_region(ui, rect, |ui| {
                AppShell::new(WindowKey::new(key))
                    .body(|_, cx, _| {
                        let dialogs = cx.dialogs().expect("the body is in a shell");
                        dialogs.with(|host| {
                            if key == "first" && !host.is_open::<StandardPrompt>() {
                                host.open(StandardPrompt::confirm_delete(["Layer"], 1));
                            }
                            open.borrow_mut().push((key, host.is_open::<StandardPrompt>()));
                        });
                    })
                    .show(ui, &mut ());
            });
        }
    };
    frames.frame(Vec::new(), &mut show);
    frames.frame(Vec::new(), &mut show);
    assert_eq!(*open.borrow(), [("first", true), ("second", false)]);
}
