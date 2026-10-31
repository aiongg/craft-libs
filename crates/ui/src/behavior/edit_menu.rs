//! [`EditMenu`]: the secondary-press menu of a text control (Undo, Cut, Copy, Paste, Delete and
//! Select All), as PhotoCraft's text fields have it.
//!
//! [`TextField`](crate::controls::TextField), [`SearchField`](crate::controls::SearchField) and
//! [`NumberField`](crate::controls::NumberField) open it on a secondary click, and on the Menu key
//! or Shift+F10 while they have the focus ([`ContextMenu`] does the gestures). The field keeps its
//! focus while the menu is open and while a command runs. The menu is shown at the start of the
//! control's pass, before its key readers, anchored on the field's response of the last pass. A
//! command is the input the keyboard shortcut would send, given to the field in the same pass
//! and taken back after its `TextEdit` ran, so the field's gesture sees ordinary typing: `Cut` and `Copy` events, a Delete key press, Cmd/Ctrl+A,
//! Cmd/Ctrl+Z, and for Paste the window's clipboard request, which arrives as a `Paste` event.

use std::sync::Arc;

use egui::plugin::Plugin;
use egui::text_edit::TextEditState;
use egui::{Context, Event, Id, Key, KeyboardShortcut, Modifiers, Popup, Pos2, Ui, ViewportCommand};

use crate::controls::Kbd;
use crate::localized::SharedLabels as _;
use crate::menu::{ContextMenu, MenuEntry, MenuItem, OpenAt};

crate::words! {
    /// The menu's labels, English by default. [`EditMenu::labels`] gives the ones installed with
    /// [`EditMenu::install`], else the installed words' translation.
    pub struct EditMenuLabels {
        undo: "edit.undo" = "Undo";
        cut: "edit.cut" = "Cut";
        copy: "edit.copy" = "Copy";
        paste: "edit.paste" = "Paste";
        delete: "edit.delete" = "Delete";
        select_all: "edit.selectAll" = "Select All";
    }
}

/// The edit menu of text controls. It has no state of its own to show: the controls open it. The
/// app installs the labels.
pub struct EditMenu;

impl EditMenu {
    /// Sets the labels the edit menus of `ctx` use. Call it with [`theme::install`](crate::theme::install)
    /// when the language changes.
    /// The context keeps them outside its [`egui::Memory`], so a memory reset leaves them installed.
    pub fn install(ctx: &Context, labels: EditMenuLabels) {
        ctx.plugin_or_default::<Installed>().lock().0 = Some(Arc::new(labels));
    }

    /// The labels the menus of `ui` show: those installed with [`EditMenu::install`], else the
    /// installed words' translation of the English.
    pub fn labels_of(ui: &Ui) -> Arc<EditMenuLabels> {
        let installed = ui.ctx().plugin_opt::<Installed>().and_then(|installed| installed.lock().0.clone());
        installed.unwrap_or_else(|| EditMenuLabels::shared(ui))
    }

    /// The labels installed with [`EditMenu::install`] in `ctx`, else the installed words'
    /// translation of the English.
    pub fn labels(ctx: &Context) -> Arc<EditMenuLabels> {
        let installed = ctx.plugin_opt::<Installed>().and_then(|installed| installed.lock().0.clone());
        installed.unwrap_or_else(|| EditMenuLabels::shared_ctx(ctx))
    }

    /// The menu for a raw `egui::TextEdit` the host draws itself (PhotoCraft's ~25 sites that are
    /// not library fields; gap 69). Call at the start of the pass, before the `TextEdit`, with the
    /// id the `TextEdit` will have; then draw it; then call [`EditMenu::after_text_edit`] with its
    /// response. The menu opens on a secondary click on the last pass's response, or on the Menu
    /// key or Shift+F10 while the edit has the focus; a chosen command reaches the `TextEdit` as
    /// the input its shortcut would send. Returns whether the field must keep its focus this pass
    /// (a click on the menu is no click away).
    // pc: crates/ui-egui/src/widgets/text_field_menu.rs (`TextFieldMenu`); here on `show` and
    // `withdraw` with the raw response in place of a library field's.
    pub fn before_text_edit(ui: &Ui, field: Id, password: bool) -> bool {
        let keeps = show(ui, field, password, None).pointer_on(ui.ctx(), field);
        ui.data_mut(|d| d.insert_temp(keeps_id(field), keeps));
        keeps
    }

    /// The second half of [`EditMenu::before_text_edit`]: takes back the injected input and keeps
    /// the focus on the edit when the menu asked for it.
    pub fn after_text_edit(ui: &Ui, field: Id, response: &egui::Response) {
        withdraw(ui, field);
        if ui.data_mut(|d| d.remove_temp::<bool>(keeps_id(field))).unwrap_or(false) && !response.has_focus() {
            response.request_focus();
        }
    }
}

/// The labels [`EditMenu::install`] set, kept as a plugin so that a reset of the context's memory
/// does not drop them.
#[derive(Default)]
struct Installed(Option<Arc<EditMenuLabels>>);

impl Plugin for Installed {
    fn debug_name(&self) -> &'static str {
        "craft_ui::behavior::edit_menu::labels"
    }
}

/// One command of the menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Undo,
    Cut,
    Copy,
    Paste,
    Delete,
    SelectAll,
}

impl Action {
    /// The input the keyboard shortcut of this command sends.
    fn events(self) -> Vec<Event> {
        let key = |key, modifiers| Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers };
        match self {
            Action::Undo => vec![key(Key::Z, Modifiers::COMMAND)],
            Action::Cut => vec![Event::Cut],
            Action::Copy => vec![Event::Copy],
            Action::Delete => vec![key(Key::Delete, Modifiers::NONE)],
            Action::SelectAll => vec![key(Key::A, Modifiers::COMMAND)],
            // The window answers with the clipboard text as a `Paste` event.
            Action::Paste => Vec::new(),
        }
    }
}

/// Whether the menu of the text control `field` is open.
#[cfg(test)]
pub(crate) fn is_open(ctx: &Context, field: Id) -> bool {
    ctx.read_response(field).is_some_and(|r| ContextMenu::is_open(ctx, &r))
}

/// What [`show`] did this pass.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Outcome {
    /// A click chose a command of the menu, or asked to open it from a widget over the text. The
    /// menu has closed, or the press was on the covering widget, so only this tells the click is
    /// no click away from the field.
    keeps_focus: bool,
}

impl Outcome {
    /// Whether the pointer is over the open menu of `field`, or just chose its command or opened
    /// it: a click there is no click away from the field, which keeps its focus.
    pub(crate) fn pointer_on(self, ctx: &Context, field: Id) -> bool {
        self.keeps_focus || ctx.read_response(field).is_some_and(|r| ContextMenu::pointer_on(ctx, &r))
    }
}

/// What the menu of `field` gave the field this pass, to take back after its `TextEdit` ran: each
/// event with the number of equal events the input held before.
fn injected_id(field: Id) -> Id {
    field.with("craft_ui::behavior::edit_menu::injected")
}

/// Whether [`EditMenu::before_text_edit`] asked `field` to keep its focus this pass.
fn keeps_id(field: Id) -> Id {
    field.with("craft_ui::behavior::edit_menu::keeps")
}

/// Shows the menu of the text control `field` at the start of its pass, before any of the
/// control's key readers: the menu takes its keys first, and a chosen command applies in this pass
/// (see [`withdraw`]). It is anchored on the field's response of the last pass. `open_at` opens
/// it there this pass, for a control whose text was covered by a widget of its own when the
/// press came. A control that reads keys before [`Typing`](super::Typing) shows the menu itself,
/// ahead of them, and hands the [`Outcome`] to the `Typing`
/// ([`edit_menu_shown`](super::Typing::edit_menu_shown)), which then does not show it again.
pub(crate) fn show(ui: &Ui, field: Id, password: bool, open_at: Option<Pos2>) -> Outcome {
    let ctx = ui.ctx();
    let mut outcome = Outcome { keeps_focus: open_at.is_some() };
    let Some(response) = ctx.read_response(field) else {
        return outcome;
    };
    // Esc closes only the menu: the press is taken before the field or its gesture sees it, and
    // the field keeps the focus.
    if ContextMenu::is_open(ctx, &response) && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
        Popup::close_id(ctx, ContextMenu::popup_id(&response));
        ui.memory_mut(|m| m.request_focus(field));
    }
    let selection = TextEditState::load(ctx, field).and_then(|s| s.cursor.char_range()).is_some_and(|r| !r.is_empty());
    let labels = EditMenu::labels_of(ui);
    let shortcut = |key| Kbd::new(&KeyboardShortcut::new(Modifiers::COMMAND, key));
    // A password's text stays out of the clipboard.
    let copyable = selection && !password;
    let entries = vec![
        MenuItem::new(&labels.undo, Action::Undo).shortcut(shortcut(Key::Z)).into(),
        MenuEntry::Separator,
        MenuItem::new(&labels.cut, Action::Cut).shortcut(shortcut(Key::X)).enabled(copyable).into(),
        MenuItem::new(&labels.copy, Action::Copy).shortcut(shortcut(Key::C)).enabled(copyable).into(),
        MenuItem::new(&labels.paste, Action::Paste).shortcut(shortcut(Key::V)).into(),
        MenuItem::new(&labels.delete, Action::Delete).enabled(selection).into(),
        MenuEntry::Separator,
        MenuItem::new(&labels.select_all, Action::SelectAll).shortcut(shortcut(Key::A)).into(),
    ];
    if let Some(action) = ContextMenu::new(&entries).open_at(open_at.map(OpenAt::Pointer)).show(ui, &response).chosen {
        run(ui, field, action);
        outcome.keeps_focus = true;
    }
    outcome
}

/// Gives `field` the input of the chosen command, for the `TextEdit` that runs later in this pass.
fn run(ui: &Ui, field: Id, action: Action) {
    ui.memory_mut(|m| m.request_focus(field));
    if action == Action::Paste {
        ui.send_viewport_cmd(ViewportCommand::RequestPaste);
    }
    let events = action.events();
    let given = ui.input_mut(|i| {
        let given: Vec<(Event, usize)> = events.iter().map(|e| (e.clone(), i.events.iter().filter(|x| *x == e).count())).collect();
        i.events.extend(events);
        given
    });
    ui.data_mut(|d| d.insert_temp(injected_id(field), given));
}

/// Takes back the input [`show`] gave `field`, once its `TextEdit` has run: no reader after the
/// field sees it.
pub(crate) fn withdraw(ui: &Ui, field: Id) {
    let Some(given) = ui.data_mut(|d| d.remove_temp::<Vec<(Event, usize)>>(injected_id(field))) else {
        return;
    };
    ui.input_mut(|i| {
        for (event, before) in given {
            // `TextEdit` leaves what it reads in the input; one that took ours removed the first
            // equal event, so counting what is left beyond `before` finds ours either way.
            let extra = i.events.iter().filter(|x| **x == event).count().saturating_sub(before);
            for _ in 0..extra {
                if let Some(at) = i.events.iter().rposition(|x| *x == event) {
                    i.events.remove(at);
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_labels_fall_back_to_the_installed_words() {
        let ctx = Context::default();
        assert_eq!(EditMenu::labels(&ctx).copy, "Copy");
        let (i18n, problems) = craft_i18n::I18n::new(&[("fr", "@id\tedit.copy\tCopier\n", "")], &["fr"]);
        assert!(problems.is_empty(), "{problems:?}");
        let fr = i18n.from_code("fr").unwrap();
        craft_host::Host::builder(craft_host::AppInfo { id: "test", name: "Test", version: "1.0" })
            .i18n(std::sync::Arc::new(i18n))
            .lang(fr)
            .install(&ctx);
        assert_eq!(EditMenu::labels(&ctx).copy, "Copier");
        let mine = EditMenuLabels { copy: "Mine".into(), ..EditMenuLabels::default() };
        EditMenu::install(&ctx, mine);
        assert_eq!(EditMenu::labels(&ctx).copy, "Mine");
    }
}
