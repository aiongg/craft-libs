//! The app shell's own rules: the screen-mode cycle, the full-screen steps, the reveal rules and the slot order.

use craft_i18n::Direction;
use craft_ui::canvas::Canvas;
use craft_ui::contract::{Bind, Show as _, ShowContainer as _};
use craft_ui::dialog::{DialogResult, PromptChoice, StandardPrompt};
use craft_ui::display::Text;
use craft_ui::runtime::env::{RevealEdge, RootEnv, WindowKey};
use craft_ui::shell::{AppShell, AppShellOutput, ScreenMode, ScreenModeCommands, Slot, SlotSize};
use craft_ui::testing::input::key_press;
use craft_ui::testing::pointer::{click, moved, press, release, settle};
use craft_ui::testing::{Fixture, Frames, trial};
use egui::{Event, Key, Modifiers, Pos2, Rect, Ui, pos2};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[trial]
fn the_f_key_cycles_the_modes_and_shift_f_goes_back() {
    let mut mode = ScreenMode::Standard;
    for expected in [ScreenMode::FullScreenWithMenuBar, ScreenMode::FullScreen, ScreenMode::Standard] {
        mode = mode.next();
        assert_eq!(mode, expected);
    }
    for mode in [ScreenMode::Standard, ScreenMode::FullScreenWithMenuBar, ScreenMode::FullScreen] {
        assert_eq!(mode.next().previous(), mode);
        assert_eq!(ScreenModeCommands::Cycle.apply(mode), mode.next());
        assert_eq!(ScreenModeCommands::CycleBack.apply(mode), mode.previous());
        assert_eq!(ScreenModeCommands::of(mode).apply(ScreenMode::Standard), mode);
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

fn rect_of(out: &AppShellOutput, key: &str) -> Rect {
    out.regions.slots.iter().find(|(shown, _)| *shown == key).map(|(_, rect)| *rect).expect("the slot showed")
}

const CENTER: Pos2 = pos2(400.0, 300.0);
const TOP_STRIP: Pos2 = pos2(400.0, 1.0);
const START_STRIP: Pos2 = pos2(1.0, 300.0);
const END_STRIP: Pos2 = pos2(799.0, 300.0);
const BOTTOM_STRIP: Pos2 = pos2(400.0, 599.0);

/// One window of a shell with a revealing slot on the top, start and end edges and a status bar that never
/// reveals, driven by the pointer and keys as a user would.
struct Window {
    frames: Frames,
    mode: Cell<ScreenMode>,
    shown: RefCell<Option<AppShellOutput>>,
    panels_hidden: Cell<bool>,
}

impl Window {
    /// A window in `mode`, its canvas focused so that Tab and F reach the shell.
    fn new(mode: ScreenMode, direction: Direction) -> Self {
        let mut frames = Frames::new();
        let mut root = RootEnv::default();
        root.tab_toggles_chrome = true;
        root.direction = direction;
        frames.root(root);
        let mut window = Window { frames, mode: Cell::new(mode), shown: RefCell::new(None), panels_hidden: Cell::new(false) };
        window.pass(Vec::new());
        window.pass(Vec::new());
        window.pass(vec![moved(CENTER)]);
        window.pass(vec![press(CENTER)]);
        window.pass(vec![release(CENTER)]);
        window
    }

    fn pass(&mut self, events: Vec<Event>) {
        let mut show = |ui: &mut Ui| {
            let mut mode = self.mode.get();
            let out = AppShell::new(WindowKey::main())
                .screen_mode(Bind::bound(&mut mode))
                .title(Slot::new("title").reveal(), |ui, _, _| {
                    Text::new("Title").show(ui);
                })
                .start(Slot::new("tools").reveal(), |ui, _, _| {
                    Text::new("Tools").show(ui);
                })
                .end(Slot::new("inspector").reveal(), |ui, _, _| {
                    Text::new("Inspector").show(ui);
                })
                .bottom(Slot::new("status"), |ui, _, _| {
                    Text::new("Status").show(ui);
                })
                .body(|ui, cx, _| {
                    self.panels_hidden.set(cx.shell().is_some_and(|shell| shell.panels_hidden()));
                    Canvas::new("Document").show(ui, |_, _| ());
                })
                .show(ui, &mut ());
            self.mode.set(mode);
            *self.shown.borrow_mut() = Some(out);
        };
        self.frames.frame(events, &mut show);
    }

    /// The keys of the slots the last pass showed.
    fn slots(&self) -> Vec<&'static str> {
        self.shown.borrow().as_ref().expect("a pass ran").regions.slots.iter().map(|(key, _)| *key).collect()
    }

    fn slot_rect(&self, key: &str) -> Rect {
        rect_of(self.shown.borrow().as_ref().expect("a pass ran"), key)
    }

    fn key(&mut self, key: Key, modifiers: Modifiers) {
        self.pass(vec![key_press(key, modifiers)]);
        self.pass(Vec::new());
    }
}

#[trial]
fn the_pointer_at_the_top_start_or_end_edge_reveals_that_edge_only() {
    let cases = [(TOP_STRIP, vec!["title"]), (START_STRIP, vec!["tools"]), (END_STRIP, vec!["inspector"]), (BOTTOM_STRIP, vec![])];
    for (pointer, expected) in cases {
        let mut window = Window::new(ScreenMode::FullScreen, Direction::Ltr);
        window.pass(vec![moved(pointer)]);
        assert_eq!(window.slots(), expected, "pointer at {pointer:?}");
    }
}

#[trial]
fn start_and_end_mirror_under_right_to_left() {
    let mut window = Window::new(ScreenMode::FullScreen, Direction::Rtl);
    window.pass(vec![moved(START_STRIP)]);
    assert_eq!(window.slots(), ["inspector"], "the left edge is the end edge");
}

#[trial]
fn a_held_button_freezes_the_edge_reveal() {
    let mut window = Window::new(ScreenMode::FullScreen, Direction::Ltr);
    window.pass(vec![moved(TOP_STRIP)]);
    window.pass(vec![press(TOP_STRIP)]);
    window.pass(vec![moved(CENTER)]);
    assert_eq!(window.slots(), ["title"], "frozen: the reveal does not end while the button is held");
    window.pass(vec![release(CENTER)]);
    window.pass(Vec::new());
    assert!(window.slots().is_empty(), "the reveal ends once the button is up and the pointer is away");

    let mut window = Window::new(ScreenMode::FullScreen, Direction::Ltr);
    window.pass(vec![moved(CENTER)]);
    window.pass(vec![press(CENTER)]);
    window.pass(vec![moved(TOP_STRIP)]);
    assert!(window.slots().is_empty(), "frozen: the reveal does not start while the button is held");
}

#[trial]
fn the_pointer_over_a_revealed_slot_keeps_its_edge_revealed() {
    let mut window = Window::new(ScreenMode::FullScreen, Direction::Ltr);
    window.pass(vec![moved(TOP_STRIP)]);
    window.pass(Vec::new());
    window.pass(vec![moved(window.slot_rect("title").center())]);
    assert_eq!(window.slots(), ["title"]);
    window.pass(vec![moved(CENTER)]);
    assert!(window.slots().is_empty());
}

#[trial]
fn tab_reveals_the_revealing_slots_and_never_the_status_bar() {
    let mut window = Window::new(ScreenMode::FullScreen, Direction::Ltr);
    assert!(window.slots().is_empty());
    window.key(Key::Tab, Modifiers::NONE);
    let mut shown = window.slots();
    shown.sort_unstable();
    assert_eq!(shown, ["inspector", "title", "tools"], "every revealing slot shows, the status bar does not");
    window.key(Key::Tab, Modifiers::NONE);
    assert!(window.slots().is_empty(), "Tab again hides them");
}

#[trial]
fn tab_hides_the_chrome_in_normal_mode_and_tab_again_shows_it() {
    let mut window = Window::new(ScreenMode::Standard, Direction::Ltr);
    let all = window.slots();
    assert_eq!(all.len(), 4, "normal mode shows every slot: {all:?}");
    window.key(Key::Tab, Modifiers::NONE);
    assert!(window.slots().is_empty(), "Tab hides the chrome");
    window.key(Key::Tab, Modifiers::NONE);
    assert_eq!(window.slots(), all, "Tab again shows it");
}

#[trial]
fn shift_tab_hides_the_panels_and_keeps_the_tools_in_normal_mode_only() {
    let mut window = Window::new(ScreenMode::Standard, Direction::Ltr);
    window.key(Key::Tab, Modifiers::SHIFT);
    assert!(window.panels_hidden.get(), "the panels are hidden");
    assert_eq!(window.slots().len(), 4, "the tools and bars stay");
    let mut only = Window::new(ScreenMode::FullScreen, Direction::Ltr);
    only.key(Key::Tab, Modifiers::SHIFT);
    assert!(!only.panels_hidden.get(), "body-only mode has no panels to hide");
}

#[trial]
fn the_f_key_cycles_the_window_and_shift_f_goes_back() {
    let mut window = Window::new(ScreenMode::Standard, Direction::Ltr);
    for expected in [ScreenMode::FullScreenWithMenuBar, ScreenMode::FullScreen, ScreenMode::Standard] {
        window.key(Key::F, Modifiers::NONE);
        assert_eq!(window.mode.get(), expected);
    }
    window.key(Key::F, Modifiers::SHIFT);
    assert_eq!(window.mode.get(), ScreenMode::FullScreen);
}

#[trial]
fn a_screen_mode_change_ends_a_reveal() {
    let mut window = Window::new(ScreenMode::FullScreen, Direction::Ltr);
    window.key(Key::Tab, Modifiers::NONE);
    assert!(!window.slots().is_empty(), "the Tab reveal shows slots");
    window.key(Key::F, Modifiers::NONE);
    window.key(Key::F, Modifiers::NONE);
    window.key(Key::F, Modifiers::NONE);
    assert_eq!(window.mode.get(), ScreenMode::FullScreen);
    assert!(window.slots().is_empty(), "back in full screen the old reveal is gone");
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
                        Canvas::new("Document").show(ui, |_, _| ());
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

/// A body that opens a delete prompt once through the window host and takes its answer; `answer` runs once the prompt
/// shows, and the prompt must be closed with its answer after it.
fn prompt_through_the_window_host(answer: impl FnOnce(&mut Fixture)) {
    let asked = Rc::new(Cell::new(false));
    let open = Rc::new(Cell::new(false));
    let choice = Rc::new(Cell::new(None));
    let (asked_in, open_in, choice_in) = (asked.clone(), open.clone(), choice.clone());
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
                        choice_in.set(Some(answered.choice));
                    }
                });
            })
            .show(ui, &mut ());
    });
    fixture.idle(OPEN_FRAMES);
    assert!(open.get() && choice.get().is_none(), "the prompt shows over the window");
    answer(&mut fixture);
    fixture.idle(2);
    assert_eq!(choice.get(), Some(PromptChoice::Proceed), "the body takes the answer in the next pass");
    assert!(!open.get(), "the default button closed the prompt");
}

#[trial]
fn a_click_on_the_default_button_of_a_window_prompt_answers_the_body() {
    prompt_through_the_window_host(|fixture| {
        let button = fixture.control("Delete").expect("the default button of the prompt");
        fixture.click(&button);
    });
}

#[trial]
fn enter_on_a_window_prompt_answers_its_default_button() {
    prompt_through_the_window_host(|fixture| {
        fixture.key(Key::Enter, Modifiers::NONE);
    });
}

#[trial]
fn a_body_cannot_reach_the_host_while_it_is_locked() {
    let seen = RefCell::new(None);
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        AppShell::new(WindowKey::main())
            .body(|_, cx, _| {
                let dialogs = cx.dialogs().expect("the body is in a shell");
                let inside = dialogs.with(|_| cx.dialogs().is_none());
                *seen.borrow_mut() = Some((inside, cx.dialogs().is_some()));
            })
            .show(ui, &mut ());
    };
    frames.frame(Vec::new(), &mut show);
    assert_eq!(*seen.borrow(), Some((true, true)), "no handle while the host is locked, a handle again after");
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
