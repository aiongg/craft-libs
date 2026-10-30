//! The window: modality, close and quit refusal, the embedded body, nested dialogs and the
//! remembered geometry. The embedded path runs in kittest; the OS-window path runs against a test
//! backend that gives each immediate viewport its own pass, like eframe's.

use egui::accesskit::Role;
use egui::{Id, TextEdit, ViewportCommand, ViewportEvent, ViewportId};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::stories::FakeHost;
use super::{Closed, PreferencesOutput, PreferencesWindow};
use crate::theme::{Base, install};

struct App {
    host: FakeHost,
    window: PreferencesWindow,
    open: bool,
    behind_clicks: usize,
    blocks: bool,
    /// The focused widget at the start of each pass.
    focus_at_start: Option<Id>,
    text: String,
    closed: Vec<Closed>,
    /// `craft_quit::on_close_request`'s answer, asked after the window like the app's `logic`.
    quit_taken: Vec<bool>,
}

const FIELD: &str = "behind-field";

/// `embed_ctx`: the context embeds viewports; `force`: the window asks for the dialog.
fn app(embed_ctx: bool, force: bool, open: bool) -> Harness<'static, App> {
    let app = App {
        host: FakeHost::new(),
        window: PreferencesWindow::new("test").embedded(force),
        open,
        behind_clicks: 0,
        blocks: false,
        focus_at_start: None,
        text: String::new(),
        closed: Vec::new(),
        quit_taken: Vec::new(),
    };
    HarnessBuilder::default()
        .with_size([1100.0, 840.0])
        .with_context_setup(move |ctx| {
            install(ctx, &Base::suite_default());
            ctx.set_embed_viewports(embed_ctx);
        })
        .build_ui_state(
            |ui, app: &mut App| {
                let ctx = ui.ctx().clone();
                app.focus_at_start = ctx.memory(|m| m.focused());
                app.blocks = PreferencesWindow::blocks_main(&ctx);
                if ui.button("Behind").clicked() {
                    app.behind_clicks += 1;
                }
                let label = ui.label("Notes");
                ui.add(TextEdit::singleline(&mut app.text).id(Id::unique(FIELD))).labelled_by(label.id);
                if app.open {
                    let out: PreferencesOutput = app.window.show(&ctx, &mut app.host);
                    if let Some(closed) = out.closed {
                        app.closed.push(closed);
                        app.open = false;
                    }
                }
                app.quit_taken.push(craft_quit::on_close_request(&ctx));
            },
            app,
        )
}

fn commands(h: &Harness<'static, App>, viewport: ViewportId) -> Vec<ViewportCommand> {
    h.output().viewport_output.get(&viewport).map(|o| o.commands.clone()).unwrap_or_default()
}

fn request_root_close(h: &mut Harness<'static, App>) {
    h.input_mut().viewports.entry(ViewportId::ROOT).or_default().events.push(ViewportEvent::Close);
    h.step();
}

#[test]
fn the_embedded_window_draws_the_body_with_its_title_and_buttons() {
    let mut h = app(true, true, true);
    h.run();
    h.get_by_role_and_label(Role::Heading, "Preferences");
    h.get_by_role(Role::SearchInput);
    for label in ["OK", "Cancel", "Apply"] {
        h.get_by_label(label);
    }
    assert!(h.state().blocks, "an open window blocks the main UI");
}

#[test]
fn the_close_box_cancels_once_and_closes() {
    let mut h = app(true, true, true);
    h.run();
    h.get_by_label("Close").click();
    h.run();
    assert_eq!(h.state().host.logged("cancel"), 1);
    assert_eq!(h.state().closed, vec![Closed::Cancel]);
}

#[test]
fn a_nested_dialog_takes_input_alone_and_the_parents_ok_is_refused() {
    let mut h = app(true, true, true);
    h.state_mut().host.nested = true;
    h.run();
    assert!(h.state().host.logged("nested") > 0, "the child is drawn in the same pass");
    assert!(h.get_by_label("OK").accesskit_node().is_disabled());
    h.get_by_label("OK").click();
    h.run();
    assert_eq!(h.state().host.logged("apply"), 0);
    assert!(h.state().closed.is_empty());

    h.state_mut().host.nested = false;
    h.run();
    h.get_by_label("OK").click();
    h.run();
    assert_eq!(h.state().closed, vec![Closed::Ok]);
}

#[test]
fn a_close_request_while_a_nested_dialog_is_open_waits_for_it() {
    let mut h = app(true, true, true);
    h.state_mut().host.nested = true;
    h.run();
    h.get_by_label("Close").click();
    h.run();
    assert!(h.state().closed.is_empty());
    assert_eq!(h.state().host.logged("cancel"), 0);
}

#[test]
fn the_embedded_window_blocks_the_main_ui_until_it_closes() {
    let mut h = app(true, true, true);
    h.run();
    assert!(h.state().blocks);
    h.get_by_label("Behind").click();
    h.run();
    assert_eq!(h.state().behind_clicks, 0);

    h.get_by_label("Cancel").click();
    h.run();
    h.run();
    h.run();
    assert!(!h.state().blocks, "the flag follows the window");
    h.get_by_label("Behind").click();
    h.run();
    assert_eq!(h.state().behind_clicks, 1);
}

#[test]
fn a_backend_without_viewports_falls_back_to_the_dialog() {
    let mut h = app(false, false, true);
    h.run();
    h.get_by_role_and_label(Role::Heading, "Preferences");
    h.get_by_label("Cancel").click();
    h.run();
    assert_eq!(h.state().closed, vec![Closed::Cancel], "the dialog is not under the modal layer");
}

#[test]
fn a_root_close_request_is_refused_while_the_embedded_window_is_open() {
    let mut h = app(true, true, true);
    h.run();
    request_root_close(&mut h);
    assert!(commands(&h, ViewportId::ROOT).contains(&ViewportCommand::CancelClose));
    assert_eq!(h.state().host.logged("cancel"), 0, "a quit request does not close the window");
    assert_eq!(h.state().quit_taken.last(), Some(&false));

    h.get_by_label("Cancel").click();
    h.run();
    h.run();
    h.run();
    request_root_close(&mut h);
    assert!(!commands(&h, ViewportId::ROOT).contains(&ViewportCommand::CancelClose), "closed: the app quits as usual");
}

#[test]
fn opening_clears_the_main_windows_keyboard_focus() {
    let mut h = app(true, true, false);
    h.get_by_role(Role::TextInput).focus();
    h.run();
    assert!(h.state().focus_at_start.is_some(), "the field has the focus before the window opens");
    h.state_mut().open = true;
    h.step();
    h.step();
    let field = Id::unique(FIELD);
    assert_ne!(h.ctx.memory(|m| m.focused()), Some(field), "typing must not reach a main-window field");
}

#[cfg(feature = "window-restore")]
mod remember {
    use std::sync::{Arc, Mutex};

    use craft_window::restore::{Monitor, MonitorKey, Placement, Store};

    use super::super::Remember;

    fn monitors() -> Vec<Monitor> {
        vec![Monitor::new(MonitorKey::new("DP-1", [1920, 1080]), [0, 0], [1920, 1080], 1.0)]
    }

    fn store_with(placement: Option<Placement>) -> Arc<Mutex<Store>> {
        let mut store = Store::in_memory();
        if let Some(p) = placement {
            store.set("preferences", &p);
        }
        Arc::new(Mutex::new(store))
    }

    fn at(size: [f32; 2], pos: [i32; 2]) -> Placement {
        let mut p = Placement::new(size, 1.0);
        p.position_px = Some(pos);
        p.normal.pos = [pos[0] as f32, pos[1] as f32];
        p
    }

    #[test]
    fn nothing_saved_opens_at_1040_by_800() {
        let mut r = Remember::new(&store_with(None), "preferences").monitors(monitors());
        assert_eq!(r.placement().normal.size, [1040.0, 800.0]);
    }

    #[test]
    fn a_saved_size_and_position_come_back() {
        let mut r = Remember::new(&store_with(Some(at([900.0, 600.0], [120, 80]))), "preferences").monitors(monitors());
        let p = r.placement();
        assert_eq!(p.normal.size, [900.0, 600.0]);
        assert_eq!(p.position_px, Some([120, 80]));
    }

    #[test]
    fn a_position_on_a_monitor_that_is_gone_moves_onto_a_connected_one() {
        let mut r = Remember::new(&store_with(Some(at([900.0, 600.0], [7000, 4000]))), "preferences").monitors(monitors());
        let p = r.placement().clone();
        let [x, y] = p.position_px.unwrap_or([0, 0]);
        assert!((0..1920 - 64).contains(&x) && (0..1080 - 64).contains(&y), "{:?}", p.position_px);
    }

    #[test]
    fn the_placement_is_held_while_the_window_is_open() {
        let store = store_with(Some(at([900.0, 600.0], [120, 80])));
        let mut r = Remember::new(&store, "preferences").monitors(monitors());
        assert_eq!(r.placement().normal.size, [900.0, 600.0]);
        store.lock().unwrap_or_else(std::sync::PoisonError::into_inner).set("preferences", &at([700.0, 500.0], [0, 0]));
        assert_eq!(r.placement().normal.size, [900.0, 600.0], "the tracker's writes do not move the open window");
    }
}

/// The OS-window path: a backend like eframe's, where each immediate viewport runs its own pass.
mod native {
    use std::cell::RefCell;
    use std::collections::HashMap;

    use egui::{
        Button, Context, Event, Modifiers, PointerButton, RawInput, TextEdit, ViewportBuilder, ViewportCommand, ViewportEvent, ViewportId,
        ViewportInfo, pos2,
    };

    use super::super::{Closed, PreferencesWindow};
    use super::{Base, FakeHost, Id, install};

    thread_local! {
        /// The builders the backend received, by viewport.
        static BUILDERS: RefCell<HashMap<ViewportId, ViewportBuilder>> = RefCell::new(HashMap::new());
        /// The viewport infos handed to each viewport's pass.
        static INFOS: RefCell<egui::ViewportIdMap<ViewportInfo>> = RefCell::new(egui::ViewportIdMap::default());
        /// The viewports whose pass ran.
        static RAN: RefCell<Vec<ViewportId>> = const { RefCell::new(Vec::new()) };
        /// The commands the viewports' own passes ended with (eframe handles those outputs too).
        static CHILD_COMMANDS: RefCell<Vec<(ViewportId, ViewportCommand)>> = const { RefCell::new(Vec::new()) };
    }

    /// What a root pass and the viewport passes inside it sent out.
    struct Pass {
        cursor: egui::CursorIcon,
        commands: Vec<(ViewportId, ViewportCommand)>,
    }

    impl Pass {
        fn sent(&self, viewport: ViewportId, command: &ViewportCommand) -> bool {
            self.commands.iter().any(|(v, c)| *v == viewport && c == command)
        }
    }

    struct Shell {
        ctx: Context,
        host: FakeHost,
        window: PreferencesWindow,
        behind_clicks: usize,
        closed: Vec<Closed>,
        time: f64,
        /// Whether the root window has the OS focus.
        focused: bool,
        /// `craft_quit::on_close_request`'s answer each pass, asked last like the app's `logic`.
        quit_taken: Vec<bool>,
    }

    fn shell(decorate: bool) -> Shell {
        BUILDERS.with(|b| b.borrow_mut().clear());
        INFOS.with(|i| i.borrow_mut().clear());
        RAN.with(|r| r.borrow_mut().clear());
        let ctx = Context::default();
        ctx.set_embed_viewports(false);
        install(&ctx, &Base::suite_default());
        Context::set_immediate_viewport_renderer(|ctx, viewport| {
            let id = viewport.ids.this;
            BUILDERS.with(|b| b.borrow_mut().insert(id, viewport.builder.clone()));
            RAN.with(|r| r.borrow_mut().push(id));
            let mut input = RawInput { viewport_id: id, ..RawInput::default() };
            input.viewports = INFOS.with(|i| i.borrow().clone());
            input.viewports.entry(id).or_default().parent = Some(viewport.ids.parent);
            let mut cb = viewport.viewport_ui_cb;
            let mut out = ctx.run_ui(input, |ui| cb(ui));
            out.textures_delta.clear(); // not painted
            CHILD_COMMANDS.with(|c| {
                for (viewport, output) in std::mem::take(&mut out.viewport_output) {
                    c.borrow_mut().extend(output.commands.into_iter().map(|command| (viewport, command)));
                }
            });
        });
        let mut window = PreferencesWindow::new("native");
        if decorate {
            window = window.builder(|b| b.with_decorations(false));
        }
        Shell { ctx, host: FakeHost::new(), window, behind_clicks: 0, closed: Vec::new(), time: 0.0, focused: true, quit_taken: Vec::new() }
    }

    impl Shell {
        /// One root pass with the window open (or not) and `events` as the root's input.
        fn pass(&mut self, open: bool, events: Vec<Event>, root: impl FnOnce(&mut ViewportInfo)) -> Pass {
            self.time += 1.0;
            let mut input = RawInput { time: Some(self.time), events, focused: self.focused, ..RawInput::default() };
            input.viewports = INFOS.with(|i| i.borrow().clone());
            root(input.viewports.entry(ViewportId::ROOT).or_default());
            let behind = &mut self.behind_clicks;
            let (host, window, closed, quit_taken) = (&mut self.host, &mut self.window, &mut self.closed, &mut self.quit_taken);
            let mut out = self.ctx.run_ui(input, |ui| {
                if ui.add(Button::new("Behind")).clicked() {
                    *behind += 1;
                }
                let mut text = String::new();
                ui.add(TextEdit::singleline(&mut text).id(Id::unique("native-field")));
                if open {
                    let shown = window.show(ui.ctx(), host);
                    closed.extend(shown.closed);
                }
                quit_taken.push(craft_quit::on_close_request(ui.ctx()));
            });
            out.textures_delta.clear();
            let mut commands: Vec<(ViewportId, ViewportCommand)> = CHILD_COMMANDS.with(|c| c.borrow_mut().drain(..).collect());
            for (viewport, output) in &out.viewport_output {
                commands.extend(output.commands.iter().map(|command| (*viewport, command.clone())));
            }
            INFOS.with(|i| i.borrow_mut().values_mut().for_each(|v| v.events.clear()));
            Pass { cursor: out.platform_output.cursor_icon, commands }
        }
    }

    fn click_at(pos: egui::Pos2) -> Vec<Event> {
        let button = |pressed| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
        vec![Event::PointerMoved(pos), button(true), button(false)]
    }

    #[test]
    fn the_body_runs_in_its_own_viewport_with_the_builder() {
        let mut s = shell(true);
        s.pass(true, vec![], |_| {});
        let child = s.window.viewport_id();
        assert!(RAN.with(|r| r.borrow().contains(&child)));
        let builder = BUILDERS.with(|b| b.borrow().get(&child).cloned()).unwrap_or_default();
        assert_eq!(builder.title.as_deref(), Some("Preferences"));
        assert_eq!(builder.inner_size, Some(super::super::DEFAULT_SIZE));
        assert_eq!(builder.min_inner_size, Some(super::super::MIN_SIZE));
        assert_eq!(builder.resizable, Some(true));
        assert_eq!(builder.decorations, Some(false), "the app's frame decision applies");
    }

    #[test]
    fn the_main_window_takes_no_press_shows_not_allowed_and_focuses_the_window() {
        let mut s = shell(false);
        s.pass(true, vec![], |_| {});
        s.pass(true, vec![], |_| {});
        assert!(PreferencesWindow::blocks_main(&s.ctx));
        let behind = pos2(20.0, 15.0);
        let out = s.pass(true, click_at(behind), |_| {});
        assert_eq!(s.behind_clicks, 0, "the layer over the main window takes the press");
        assert_eq!(out.cursor, egui::CursorIcon::NotAllowed);
        assert!(out.sent(s.window.viewport_id(), &ViewportCommand::Focus));

        // Closed: the main UI works again.
        s.pass(false, vec![], |_| {});
        s.pass(false, vec![], |_| {});
        s.pass(false, vec![], |_| {});
        assert!(!PreferencesWindow::blocks_main(&s.ctx));
        s.pass(false, click_at(behind), |_| {});
        assert_eq!(s.behind_clicks, 1);
    }

    #[test]
    fn the_root_gaining_focus_forwards_it_to_the_window() {
        let mut s = shell(false);
        s.focused = false;
        s.pass(true, vec![], |_| {});
        s.pass(true, vec![], |_| {});
        s.focused = true;
        let out = s.pass(true, vec![], |_| {});
        assert!(out.sent(s.window.viewport_id(), &ViewportCommand::Focus));
    }

    #[test]
    fn opening_clears_the_main_windows_keyboard_focus() {
        let mut s = shell(false);
        let field = Id::unique("native-field");
        s.pass(false, vec![], |_| {});
        s.ctx.memory_mut(|m| m.request_focus(field));
        s.pass(false, vec![], |_| {});
        assert_eq!(s.ctx.memory(|m| m.focused()), Some(field));
        s.pass(true, vec![], |_| {});
        assert_eq!(s.ctx.memory(|m| m.focused()), None, "typing must not reach a main-window field");
    }

    #[test]
    fn title_bar_close_dock_quit_and_cmd_q_are_refused_and_focus_the_window() {
        let mut s = shell(false);
        s.pass(true, vec![], |_| {});
        let child = s.window.viewport_id();
        // Title-bar close and the macOS Dock's Quit both arrive as a close request of the root.
        let out = s.pass(true, vec![], |root| root.events.push(ViewportEvent::Close));
        assert!(out.sent(ViewportId::ROOT, &ViewportCommand::CancelClose));
        assert!(out.sent(child, &ViewportCommand::Focus));
        assert_eq!(s.host.logged("cancel"), 0, "a quit request does not close the window");

        // Cmd+Q: the app-menu Quit item asks the root to close; the integration delivers the request.
        assert!(craft_quit::on_menu_event(&s.ctx, craft_quit::QUIT_MENU_ID));
        let out = s.pass(true, vec![], |_| {});
        assert!(out.sent(ViewportId::ROOT, &ViewportCommand::Close));
        let out = s.pass(true, vec![], |root| root.events.push(ViewportEvent::Close));
        assert!(out.sent(ViewportId::ROOT, &ViewportCommand::CancelClose));
        assert!(out.sent(child, &ViewportCommand::Focus));
        assert_eq!(s.quit_taken.last(), Some(&false), "craft_quit sees the CancelClose and leaves the window open");
    }

    #[test]
    fn a_quit_request_is_not_refused_without_the_window() {
        let mut s = shell(false);
        let out = s.pass(false, vec![], |root| root.events.push(ViewportEvent::Close));
        assert!(!out.sent(ViewportId::ROOT, &ViewportCommand::CancelClose));
    }

    #[test]
    fn the_windows_own_close_request_cancels_once() {
        let mut s = shell(false);
        s.pass(true, vec![], |_| {});
        let child = s.window.viewport_id();
        INFOS.with(|i| i.borrow_mut().entry(child).or_default().events.push(ViewportEvent::Close));
        s.pass(true, vec![], |_| {});
        s.pass(true, vec![], |_| {});
        assert_eq!(s.host.logged("cancel"), 1);
        assert_eq!(s.closed, vec![Closed::Cancel]);
    }
}
