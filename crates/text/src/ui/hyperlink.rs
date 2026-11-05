//! [`HyperlinkDialog`] (N3.28): Insert › Link (Word) and Insert Hyperlink (PowerPoint) on the
//! dialog stack: Link to a web address, an email address or a place in the document (a slide, a
//! heading, a bookmark), the text to display, a ScreenTip, and PowerPoint's Highlight click.

use std::cell::Cell;

use craft_ui::behavior::{RowInfo, Selection, SelectionMode};
use craft_ui::composites::{Field, FieldGroup};
use craft_ui::controls::{Checkbox, RadioOption, RadioSet, TextField};
use craft_ui::dialog::{Dialog, DialogId, DialogResult, DialogStack, DialogState, Footer, InitialFocus, Outcome, StackDialog, Width};
use craft_ui::edit::EditEvent;
use craft_ui::icons::Icon;
use craft_ui::list::{List, ListRow};
use craft_ui::localized::Labels as _;
use egui::{Context, Ui};

craft_ui::words! {
    /// The dialog's words (WordCraft's and DeckCraft's strings).
    #[non_exhaustive]
    pub struct HyperlinkLabels {
        title: "hyperlink.title" = "Insert Hyperlink";
        edit_title: "hyperlink.editTitle" = "Edit Hyperlink";
        link_to: "hyperlink.linkTo" = "Link to:";
        web_page: "hyperlink.webPage" = "Existing File or Web Page";
        place: "hyperlink.place" = "Place in This Document";
        email: "hyperlink.email" = "E-mail Address";
        text_to_display: "hyperlink.textToDisplay" = "Text to display:";
        address: "hyperlink.address" = "Address:";
        email_address: "hyperlink.emailAddress" = "E-mail address:";
        subject: "hyperlink.subject" = "Subject:";
        select_place: "hyperlink.selectPlace" = "Select a place in this document:";
        top_of_document: "hyperlink.topOfDocument" = "Top of the Document";
        screen_tip: "hyperlink.screenTip" = "ScreenTip:";
        highlight_click: "hyperlink.highlightClick" = "Highlight click";
        remove: "hyperlink.remove" = "Remove Link";
    }
}

/// Where a link goes.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LinkTarget {
    /// A web address or file (`https://…`).
    Url(String),
    /// An email address and an optional subject, written as a `mailto:` link by the app.
    Email {
        /// The email address.
        address: String,
        /// The subject line, if any.
        subject: Option<String>,
    },
    /// The top of the document (Word's "Top of the Document").
    Top,
    /// A place in the document: the index into [`HyperlinkDialog::places`].
    Place(usize),
}

/// The link the dialog edits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hyperlink {
    /// Where the link goes.
    pub target: LinkTarget,
    /// The tooltip shown over the link.
    pub tooltip: String,
    /// PowerPoint's Highlight click.
    pub highlight_click: bool,
}

/// What the selection holds, which decides what OK does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CurrentLink {
    /// Nothing linked: OK sets a new link (a caret at a word boundary inserts the display text).
    None,
    /// One link covers the selected text or every selected shape.
    One(Hyperlink),
    /// Several different links: OK replaces none of them until a new target is chosen.
    Mixed,
    /// A link this dialog doesn't edit (next slide, a program): kept unless a new target is
    /// chosen.
    Other,
}

/// OK's answer.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum HyperlinkAnswer {
    /// Write this link; `text` is the display text when the selection was a bare caret (insert
    /// it) or the user changed it.
    Set {
        /// The link to write.
        link: Hyperlink,
        /// The display text, when the selection was a bare caret or the user changed it.
        text: Option<String>,
    },
    /// Remove Link (only with a link to edit).
    Remove,
    /// OK changed nothing (a mixed or other link with no new target).
    Keep,
}

/// The dialog.
///
/// - Link to: the three targets as craft-ui's `RadioSet`, a column at the left (Word's), each
///   with its icon (a globe, a bookmark, an envelope) after the label; a new link's address
///   starts at `https://` (DeckCraft). Address, Email address, Subject and Text to display are
///   craft-ui `TextField`s.
/// - Place in This Document lists Top of the Document, then [`places`](Self::places), in a
///   craft-ui [`List`] (one Tab stop, the list keys, type-ahead; headings
///   and bookmarks, or slides "1. Title"). Top is [`LinkTarget::Top`]; `places` holds only the
///   named places.
/// - With [`CurrentLink::Mixed`] or [`CurrentLink::Other`] the fields start empty and OK returns
///   [`HyperlinkAnswer::Keep`] until a target differs from the seed (DeckCraft's
///   `hyperlink_should_apply`).
/// - Keys: Enter is OK, Esc Cancel; first focus on Address.
pub struct HyperlinkDialog {
    current: CurrentLink,
    text: Option<String>,
    places: Vec<String>,
    highlight: bool,
    labels: Option<HyperlinkLabels>,
    state: DialogState,
    form: Form,
    seed: Form,
}

/// Which kind of target the dialog edits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Url,
    Email,
    Place,
}

/// A row of the places list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Spot {
    Top,
    /// An index into [`HyperlinkDialog::places`].
    Place(usize),
}

/// The dialog's fields.
#[derive(Clone, Debug, PartialEq)]
struct Form {
    mode: Mode,
    url: String,
    email: String,
    subject: String,
    place: Selection<Spot>,
    tooltip: String,
    highlight_click: bool,
    display: String,
}

const NEW_ADDRESS: &str = "https://";

impl Form {
    /// The fields a new link starts with, and the ones `current` fills in.
    fn of(current: &CurrentLink, text: Option<&str>) -> Form {
        let mut form = Form {
            mode: Mode::Url,
            url: NEW_ADDRESS.to_owned(),
            email: String::new(),
            subject: String::new(),
            place: Selection::new(),
            tooltip: String::new(),
            highlight_click: false,
            display: text.unwrap_or_default().to_owned(),
        };
        match current {
            CurrentLink::None => {}
            CurrentLink::Mixed | CurrentLink::Other => form.url.clear(),
            CurrentLink::One(link) => {
                form.tooltip.clone_from(&link.tooltip);
                form.highlight_click = link.highlight_click;
                match &link.target {
                    LinkTarget::Url(url) => form.url.clone_from(url),
                    LinkTarget::Email { address, subject } => {
                        form.mode = Mode::Email;
                        form.email.clone_from(address);
                        form.subject = subject.clone().unwrap_or_default();
                    }
                    LinkTarget::Top => {
                        form.mode = Mode::Place;
                        form.place = Selection::only(Spot::Top);
                    }
                    LinkTarget::Place(place) => {
                        form.mode = Mode::Place;
                        form.place = Selection::only(Spot::Place(*place));
                    }
                }
            }
        }
        form
    }

    fn place(&self) -> Option<Spot> {
        self.place.lead
    }
}

impl HyperlinkDialog {
    /// The dialog over what the selection holds; `text` is the selected text shown in Text to
    /// display (`None` hides the field: a shape's click action).
    pub fn new(current: CurrentLink, text: Option<String>) -> Self {
        let form = Form::of(&current, text.as_deref());
        HyperlinkDialog { current, text, places: Vec::new(), highlight: false, labels: None, state: DialogState::default(), seed: form.clone(), form }
    }

    /// The places in the document a link may go to, after Top of the Document (which the dialog
    /// always lists, as [`LinkTarget::Top`]).
    pub fn places(mut self, places: Vec<String>) -> Self {
        self.places = places;
        self
    }

    /// Show PowerPoint's Highlight click.
    pub fn highlight_click(mut self, on: bool) -> Self {
        self.highlight = on;
        self
    }

    /// Sets the translated labels; the default reads them from the interface locale.
    pub fn labels(mut self, labels: HyperlinkLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The target the fields name, if they name one.
    fn target(&self) -> Option<LinkTarget> {
        let form = &self.form;
        match form.mode {
            Mode::Url => Some(form.url.trim()).filter(|u| !u.is_empty() && *u != NEW_ADDRESS).map(|u| LinkTarget::Url(u.to_owned())),
            Mode::Email => Some(form.email.trim()).filter(|e| !e.is_empty()).map(|address| LinkTarget::Email {
                address: address.to_owned(),
                subject: Some(form.subject.trim()).filter(|s| !s.is_empty()).map(str::to_owned),
            }),
            Mode::Place => match form.place()? {
                Spot::Top => Some(LinkTarget::Top),
                Spot::Place(place) => (place < self.places.len()).then_some(LinkTarget::Place(place)),
            },
        }
    }

    /// Whether OK writes a link: always for a new link or one the dialog edits; for a link it
    /// holds as it is (several, or one it doesn't edit), only once a target differs from the seed.
    fn applies(&self) -> bool {
        if matches!(self.current, CurrentLink::None | CurrentLink::One(_)) {
            return true;
        }
        let (form, seed) = (&self.form, &self.seed);
        form.mode != seed.mode
            || match form.mode {
                Mode::Url => form.url != seed.url,
                Mode::Email => form.email != seed.email || form.subject != seed.subject,
                Mode::Place => form.place() != seed.place(),
            }
    }

    /// What OK answers; `None` while the fields name no target.
    fn answer(&self) -> Option<HyperlinkAnswer> {
        if !self.applies() {
            return Some(HyperlinkAnswer::Keep);
        }
        let link = Hyperlink { target: self.target()?, tooltip: self.form.tooltip.clone(), highlight_click: self.form.highlight_click };
        Some(HyperlinkAnswer::Set { link, text: self.changed_text() })
    }

    /// The display text when the user typed or changed it.
    fn changed_text(&self) -> Option<String> {
        let original = self.text.as_deref()?;
        (self.form.display != original && !self.form.display.is_empty()).then(|| self.form.display.clone())
    }
}

/// The text an edit event leaves in a field, if it changed it.
fn typed(event: Option<&EditEvent<String>>) -> Option<String> {
    match event? {
        EditEvent::Commit { changed: false, .. } => None,
        event => event.value().cloned(),
    }
}

impl HyperlinkDialog {
    fn address_id(&self) -> egui::Id {
        self.id().with("address")
    }

    fn text_field(ui: &mut Ui, salt: &'static str, caption: &str, value: &mut String, id: Option<egui::Id>) {
        let edit = Field::new(caption)
            .show(ui, |ui, _| {
                let field = TextField::new(("hyperlink", salt), caption, value.as_str());
                match id {
                    Some(id) => field.id(id).show(ui),
                    None => field.show(ui),
                }
            })
            .inner;
        if let Some(text) = typed(edit.event.as_ref()) {
            *value = text;
        }
    }

    fn body(&mut self, ui: &mut Ui, labels: &HyperlinkLabels, valid: &Cell<bool>) {
        let address = self.address_id();
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.label(labels.link_to.as_str());
                let mut set = RadioSet::new(("hyperlink", "link-to"), labels.link_to.as_str(), &mut self.form.mode)
                    .item(Mode::Url)
                    .item(Mode::Email)
                    .item(Mode::Place)
                    .begin(ui);
                for (index, (label, icon)) in
                    [(&labels.web_page, Icon::Globe), (&labels.email, Icon::Mail), (&labels.place, Icon::Bookmark)].into_iter().enumerate()
                {
                    set.radio_with(ui, index, RadioOption::new(label.as_str()).icon(icon));
                }
                let _ = set.end(ui);
            });
            ui.vertical(|ui| {
                FieldGroup::new("hyperlink-fields").show(ui, |ui| {
                    if self.text.is_some() {
                        Self::text_field(ui, "display", labels.text_to_display.as_str(), &mut self.form.display, None);
                    }
                    match self.form.mode {
                        Mode::Url => Self::text_field(ui, "url", labels.address.as_str(), &mut self.form.url, Some(address)),
                        Mode::Email => {
                            Self::text_field(ui, "email", labels.email_address.as_str(), &mut self.form.email, Some(address));
                            Self::text_field(ui, "subject", labels.subject.as_str(), &mut self.form.subject, None);
                        }
                        Mode::Place => self.place_list(ui, labels),
                    }
                    Self::text_field(ui, "tip", labels.screen_tip.as_str(), &mut self.form.tooltip, None);
                });
                if self.highlight {
                    let _ = Checkbox::bool(labels.highlight_click.as_str(), &mut self.form.highlight_click).show(ui);
                }
            });
        });
        valid.set(self.answer().is_some());
    }

    fn place_list(&mut self, ui: &mut Ui, labels: &HyperlinkLabels) {
        ui.label(labels.select_place.as_str());
        let keys: Vec<RowInfo<Spot>> = std::iter::once(Spot::Top).chain((0..self.places.len()).map(Spot::Place)).map(RowInfo::leaf).collect();
        let places = &self.places;
        let _ = List::new(("hyperlink", "places"), labels.select_place.as_str(), keys.as_slice())
            .selection(&mut self.form.place)
            .mode(SelectionMode::Single)
            .type_ahead(true)
            .show(ui, |_, &key| {
                let name = match key {
                    Spot::Top => labels.top_of_document.as_str(),
                    Spot::Place(place) => places[place].as_str(),
                };
                ListRow::new(key, name)
            });
    }
}

impl StackDialog for HyperlinkDialog {
    type Result = HyperlinkAnswer;

    fn id(&self) -> DialogId {
        craft_ui::dialog::dialog_id("hyperlink")
    }

    // deckcraft: crates/ui-egui/src/dialogs/hyperlink.rs:73 `seed_from`, :161
    // `hyperlink_should_apply`, :175 `hyperlink_command`; wordcraft:
    // crates/ui-egui/src/dialogs/dispatch.rs:669 `link_dialog`.
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<HyperlinkAnswer>> {
        let labels = self.labels.clone().unwrap_or_else(|| HyperlinkLabels::of_ctx(ctx));
        let valid = Cell::new(self.answer().is_some());
        let editing = matches!(self.current, CurrentLink::One(_));
        let title = if editing { labels.edit_title.as_str() } else { labels.title.as_str() };
        let mut footer = Footer::side().ok_enabled_with(|| valid.get());
        if editing {
            footer = footer.extra(labels.remove.as_str());
        }
        let mut state = std::mem::take(&mut self.state);
        let shown = Dialog::with_id(self.id(), title)
            .stack(stack)
            .width(Width::Standard)
            .initial_focus(InitialFocus::Field(self.address_id()))
            .footer(footer)
            .show(ctx, &mut state, &mut (), |ui, ()| self.body(ui, &labels, &valid));
        self.state = state;
        if shown.extra.is_some() {
            return Some(DialogResult::Ok(HyperlinkAnswer::Remove));
        }
        match shown.outcome {
            Some(Outcome::Ok) => self.answer().map(DialogResult::Ok),
            Some(Outcome::Cancel | Outcome::Close) => Some(DialogResult::Cancel),
            _ => None,
        }
    }
}
