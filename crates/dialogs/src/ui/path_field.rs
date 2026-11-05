//! [`PathField`]: a text field holding a path with a Browse button that opens a picker through
//! `host.files()`.

use craft_host::{Host, PickId, PickRequest};
use craft_ui::controls::{Button, ControlSize, TextField};
use craft_ui::edit::{EditEvent, EditResponse};
use craft_ui::localized::Labels;
use craft_ui::theme::{Tokens, TokensOf, role_format};
use egui::text::LayoutJob;
use egui::{AsIdSalt, IdSalt, Label, Modifiers, Ui};

craft_ui::words! {
    /// The field's words, English by default.
    #[non_exhaustive]
    pub struct PathFieldLabels {
        browse: "pathField.browse" = "Browse…";
        not_found: "pathField.notFound" = "The file was not found.";
    }
}

/// What the field reported this pass.
#[derive(Debug)]
pub struct PathFieldResponse {
    /// The field's edit: `Commit` when the user typed a path and left the field or pressed Enter;
    /// `Set` when the picker answered (read either with [`EditResponse::committed`]).
    pub edit: EditResponse<String>,
    /// Browse was pressed and a picker was queued; the field stores the ticket in egui memory and
    /// fills itself when the answer arrives.
    pub browsing: Option<PickId>,
}

/// A label, a text field with the path and a Browse button in one row. Typing commits on Enter or
/// focus loss; Browse queues `request` through `host.files()` and the answer fills the field on
/// a later pass (the field matches tickets, so several fields can browse in turn). Tab order: field,
/// then Browse. The field shows [`PathFieldLabels::not_found`] under itself in error red when
/// `must_exist` is set and the path does not exist.
///
/// ```ignore
/// let r = PathField::new("export-path", "Save to", &path, PickRequest::save("export").suggested_extension("png")).show(ui);
/// if let Some(p) = r.edit.committed() { path = p; }
/// ```
pub struct PathField<'a> {
    id_salt: IdSalt,
    label: String,
    value: &'a str,
    request: PickRequest,
    must_exist: bool,
    width: Option<f32>,
    enabled: bool,
}

impl<'a> PathField<'a> {
    /// A field with `label` showing `value`. `request` is what Browse asks the host to pick.
    /// `id_salt` keeps the field's state apart from other fields in the same `Ui`.
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, value: &'a str, request: PickRequest) -> Self {
        Self { id_salt: IdSalt::new(id_salt), label: label.into(), value, request, must_exist: false, width: None, enabled: true }
    }

    /// Show the not-found message for a path that does not exist (Open fields; not Save).
    pub fn must_exist(mut self, must_exist: bool) -> Self {
        self.must_exist = must_exist;
        self
    }

    /// The text field's width; default: fill, leaving room for Browse.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Whether the field and its Browse button react to input; default true.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Draws the row. Reads `Host::of(ui).files()`: takes the pick answer of the ticket this field
    /// stored; other tickets' answers stay in the service.
    // pc: crates/ui-egui/src/dialogs/file_ui/print.rs:246 browse_pdf (Browse beside the PDF path field); new: the field itself
    pub fn show(self, ui: &mut Ui) -> PathFieldResponse {
        let host = Host::of(ui);
        let labels = PathFieldLabels::of(ui);
        let tokens = Tokens::of(ui);
        let ticket_key = ui.scope_id().with((self.id_salt, "ticket"));
        let enabled = self.enabled && ui.is_enabled();
        let missing = self.must_exist && path_is_missing(self.value);
        let mut browsing = None;

        let mut edit = ui
            .vertical(|ui| {
                let edit = ui
                    .horizontal(|ui| {
                        ui.label(self.label.as_str());
                        let browse = Button::new(labels.browse.as_str()).id_salt((self.id_salt, "browse")).enabled(enabled);
                        let room = ui.available_width() - browse.natural_width(ui) - ui.spacing().item_spacing.x;
                        let width = self.width.unwrap_or_else(|| room.max(0.0));
                        let edit = TextField::new((self.id_salt, "field"), self.label.as_str(), self.value)
                            .width(width)
                            .enabled(enabled)
                            .invalid(missing)
                            .show(ui);
                        if ui.add(browse).clicked() {
                            let id = host.files().pick(self.request.clone());
                            ui.data_mut(|d| d.insert_temp(ticket_key, id));
                            browsing = Some(id);
                        }
                        edit
                    })
                    .inner;
                if missing {
                    let style = TextField::style(&tokens, ControlSize::Md);
                    let format = role_format(&tokens, style.role, style.invalid);
                    ui.add(Label::new(LayoutJob::single_section(labels.not_found.clone(), format)));
                }
                edit
            })
            .inner;

        // A typing event of this pass keeps the slot; the answer waits in the queue for the next one.
        if edit.event.is_none()
            && let Some(ticket) = ui.data(|d| d.get_temp::<PickId>(ticket_key))
            && let Some(picked) = host.files().take_answer(ticket)
        {
            ui.data_mut(|d| d.remove::<PickId>(ticket_key));
            if let Some(path) = picked.first_path() {
                edit.event = Some(EditEvent::set(path.to_string_lossy().into_owned(), Modifiers::NONE));
            }
        }
        PathFieldResponse { edit, browsing }
    }
}

/// Whether a non-empty `value` names nothing on disk. An empty field is not flagged, and on wasm
/// there is no file system to ask.
#[cfg(not(target_arch = "wasm32"))]
fn path_is_missing(value: &str) -> bool {
    !value.trim().is_empty() && !std::path::Path::new(value.trim()).exists()
}

#[cfg(target_arch = "wasm32")]
fn path_is_missing(_value: &str) -> bool {
    false
}
