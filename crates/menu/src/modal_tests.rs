//! Per-command modal rules.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use craft_keymap::CommandRef;

use crate::{Backend, Builtin, Dispatch, Event, Gate, Item, Layout, Menu, MenuBar, ModalRules, NAVIGATION_COMMANDS, NativeMenu, PendingRule};

fn cmd(id: &str) -> CommandRef {
    CommandRef::new(id)
}

fn dialog() -> Gate {
    Gate { modal: true, ..Gate::default() }
}

#[test]
// With Gate::modal and NAVIGATION_COMMANDS, view.zoomIn is allowed and image.crop is not; the built-in Quit always is.
fn navigation_commands_run_while_a_dialog_is_open() {
    let rules = ModalRules::default().in_dialogs(NAVIGATION_COMMANDS);
    let gate = dialog();
    assert!(gate.allows_under(&rules, &cmd("view.zoomIn")));
    assert!(!gate.allows_under(&rules, &cmd("image.crop")));
    assert!(gate.allows_under(&rules, &Builtin::Quit.command()));
    assert!(gate.allows_item_under(&rules, &Item::new("view.actualPixels", "100%")));
    assert!(!dialog().allows_under(&ModalRules::default(), &cmd("view.zoomIn")), "no rules: the gate alone");
    assert!(Gate::default().allows_under(&rules, &cmd("image.crop")), "no dialog: everything runs");
    let recording = Gate { recording_shortcut: true, ..Gate::default() };
    assert!(!recording.allows_under(&rules, &cmd("view.zoomIn")), "recording a shortcut stops every app command");
}

#[test]
// With ModalRules::workspace, even the in-dialog commands are refused; Hide and Minimize still run.
fn a_workspace_blocks_every_app_command() {
    let rules = ModalRules::default().in_dialogs(NAVIGATION_COMMANDS).workspace(true);
    for gate in [Gate::default(), dialog()] {
        assert!(!gate.allows_under(&rules, &cmd("view.zoomIn")));
        assert!(!gate.allows_under(&rules, &cmd("file.save")));
        assert!(gate.allows_under(&rules, &Builtin::Hide.command()));
        assert!(gate.allows_under(&rules, &Builtin::Minimize.command()));
    }
}

#[derive(Default)]
struct Fake {
    synced: Rc<RefCell<Vec<MenuBar>>>,
    events: Rc<RefCell<Vec<Event>>>,
}

impl Backend for Fake {
    fn sync(&mut self, bar: &MenuBar) {
        self.synced.borrow_mut().push(bar.clone());
    }
    fn drain(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events.borrow_mut())
    }
    fn attach(&mut self, _: Dispatch) {}
}

#[test]
// With a pending rule allowing view.*, the native menu greys image.* and drops its clicks; view.zoomIn stays enabled.
fn a_pending_crop_greys_what_its_rule_refuses() {
    let synced = Rc::new(RefCell::new(Vec::new()));
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut menu = NativeMenu::new(Box::new(Fake { synced: synced.clone(), events: events.clone() }));
    let layout = || Layout {
        bar: MenuBar {
            menus: vec![
                Menu::new("Image", vec![Item::new("image.crop", "Crop").into()]),
                Menu::new("View", vec![Item::new("view.zoomIn", "Zoom In").into()]),
            ],
            documents: Vec::new(),
        },
        clashes: Vec::new(),
    };
    menu.sync(layout());
    let enabled = |bar: &MenuBar, id: &str| bar.find(&cmd(id)).unwrap().enabled;
    assert!(enabled(&synced.borrow()[0], "image.crop"));

    let pending: PendingRule = Arc::new(|c: &CommandRef| c.id.starts_with("view."));
    menu.set_modal_rules(ModalRules::default().pending(Some(pending)));
    menu.sync(layout());
    {
        let synced = synced.borrow();
        assert_eq!(synced.len(), 2, "new rules sync the menus again");
        assert!(!enabled(&synced[1], "image.crop") && enabled(&synced[1], "view.zoomIn"));
    }

    events.borrow_mut().extend([Event::Click(cmd("image.crop")), Event::Click(cmd("view.zoomIn"))]);
    menu.raw_input(&mut egui::RawInput::default());
    let ctx = egui::Context::default();
    let mut clicks = Vec::new();
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| clicks = menu.take_clicks(ui.ctx()));
    out.textures_delta.clear();
    assert_eq!(clicks, [cmd("view.zoomIn")]);

    menu.set_modal_rules(ModalRules::default());
    menu.sync(layout());
    assert!(enabled(&synced.borrow()[2], "image.crop"), "clearing the rule enables the rows again");
}
