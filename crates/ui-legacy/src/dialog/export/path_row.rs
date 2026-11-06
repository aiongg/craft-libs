//! [`OutputPathRow`]: where a render or export goes, as a template list and the chosen file.

use craft_host::{Host, PickId, PickRequest};
use egui::{Modifiers, Ui};

use crate::controls::{Link, Select, TextField};
use crate::edit::{EditEvent, EditResponse};
use crate::localized::SharedLabels as _;

crate::words! {
    /// The row's words, English by default.
    #[non_exhaustive]
    pub struct OutputPathLabels {
        output_to: "outputPath.outputTo" = "Output To:";
        choose: "outputPath.choose" = "Choose File…";
        edit_template: "outputPath.editTemplate" = "Alt-click to edit the template";
    }
}

/// What the row reported this pass.
#[derive(Debug)]
pub struct OutputPathResponse {
    /// The new path or template: a `Set` when the user picked a template or the save picker
    /// answered, the text field's `Commit` when the user finished editing the template text.
    pub edit: EditResponse<String>,
    /// A save picker was queued; the row fills itself when its answer arrives.
    pub browsing: Option<PickId>,
}

/// "Output To:", a list of name templates (`[Comp Name].[fileExtension]`) and the resulting file
/// name as a link button, after After Effects' Render Queue and EffectCraft's port of it.
///
/// Picking a template sets the path; a click on the file name queues `request` through
/// `host.files()` (Choose File…) and the answer replaces the path; Alt-click on the file name edits
/// the template in place (Enter commits, Esc cancels). Whether an output may be written
/// (duplicates, protected folders) is the app's check.
pub struct OutputPathRow<'a> {
    id_salt: egui::IdSalt,
    value: &'a str,
    templates: Vec<(String, String)>,
    request: PickRequest,
}

impl<'a> OutputPathRow<'a> {
    /// The row for the current path `value`; `request` is the save picker Choose File… opens.
    pub fn new(id_salt: impl egui::AsIdSalt, value: &'a str, request: PickRequest) -> Self {
        Self { id_salt: egui::IdSalt::new(id_salt), value, templates: Vec::new(), request }
    }

    /// The template list, `(template, label)`.
    pub fn templates(mut self, templates: Vec<(String, String)>) -> Self {
        self.templates = templates;
        self
    }

    // effectcraft: crates/ui-egui/src/panels/render_queue.rs:634 output_to (templates :397, Choose File… :413)
    /// Draws the row and reports a new path or template, and any save picker it queued.
    pub fn show(self, ui: &mut Ui) -> OutputPathResponse {
        let labels = OutputPathLabels::shared(ui);
        let host = Host::of(ui);
        let salt = self.id_salt;
        let scope = ui.scope_id();
        let ticket_key = scope.with((salt, "ticket"));
        let edit_key = scope.with((salt, "editing"));
        let editing = ui.data(|d| d.get_temp::<Editing>(edit_key));
        let mut event = None;
        let mut browsing = None;

        let response = ui
            .horizontal(|ui| {
                ui.label(labels.output_to.as_str());
                if !self.templates.is_empty() {
                    let mut current = self.value.to_owned();
                    let name = labels.output_to.trim_end_matches(':');
                    let mut select = Select::new((salt, "templates"), name, &mut current).fit(true);
                    for (template, label) in &self.templates {
                        select = select.option(template.clone(), label.as_str());
                    }
                    event = select.show(ui).event;
                }

                match editing {
                    Some(state) => {
                        let id = ui.make_persistent_id((salt, "template"));
                        let edit = TextField::new((salt, "template"), labels.edit_template.as_str(), self.value).id(id).show(ui);
                        if state == Editing::Pending {
                            ui.memory_mut(|m| m.request_focus(id));
                            if edit.response.has_focus() {
                                ui.data_mut(|d| d.insert_temp(edit_key, Editing::Active));
                            }
                        } else if edit.submitted {
                            if let Some(EditEvent::Commit { value, .. }) = &edit.event
                                && !value.trim().is_empty()
                            {
                                event = edit.event.clone();
                            }
                            ui.data_mut(|d| d.remove::<Editing>(edit_key));
                        } else if edit.escaped || !edit.response.has_focus() {
                            ui.data_mut(|d| d.remove::<Editing>(edit_key));
                        }
                        edit.response
                    }
                    None => {
                        let shown = if self.value.is_empty() { labels.choose.as_str() } else { self.value };
                        let link = ui.add(Link::new(shown).id_salt((salt, "file"))).on_hover_text(labels.edit_template.as_str());
                        if link.clicked() {
                            if ui.input(|i| i.modifiers.alt) {
                                ui.data_mut(|d| d.insert_temp(edit_key, Editing::Pending));
                            } else {
                                let id = host.files().pick(self.request.clone());
                                ui.data_mut(|d| d.insert_temp(ticket_key, id));
                                browsing = Some(id);
                            }
                        }
                        link
                    }
                }
            })
            .inner;

        if event.is_none()
            && let Some(ticket) = ui.data(|d| d.get_temp::<PickId>(ticket_key))
            && let Some(picked) = host.files().take_answer(ticket)
        {
            ui.data_mut(|d| d.remove::<PickId>(ticket_key));
            event = picked.first_path().map(|path| EditEvent::set(path.to_string_lossy().into_owned(), Modifiers::NONE));
        }
        OutputPathResponse { edit: EditResponse::new(response, event), browsing }
    }
}

/// The in-place template edit: asking for focus, then running until the field loses it.
#[derive(Clone, Copy, PartialEq)]
enum Editing {
    Pending,
    Active,
}
