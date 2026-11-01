//! [`HyperlinkDialog`] (N3.28): Insert › Link (Word) and Insert Hyperlink (PowerPoint) on the
//! dialog stack: Link to a web address, an email address or a place in the document (a slide, a
//! heading, a bookmark), the text to display, a ScreenTip, and PowerPoint's Highlight click.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::dialog::{DialogId, DialogResult, DialogStack, StackDialog};
use egui::Context;

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
        select_place: "hyperlink.selectPlace" = "Select a place in this document:";
        top_of_document: "hyperlink.topOfDocument" = "Top of the Document";
        screen_tip: "hyperlink.screenTip" = "ScreenTip:";
        highlight_click: "hyperlink.highlightClick" = "Highlight click";
        remove: "hyperlink.remove" = "Remove Link";
    }
}

/// Where a link goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinkTarget {
    /// A web address or file (`https://…`).
    Url(String),
    /// An email address, written `mailto:` by the app.
    Email(String),
    /// A place in the document: the index into [`HyperlinkDialog::places`].
    Place(usize),
}

/// The link the dialog edits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hyperlink {
    pub target: LinkTarget,
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
pub enum HyperlinkAnswer {
    /// Write this link; `text` is the display text when the selection was a bare caret (insert
    /// it) or the user changed it.
    Set { link: Hyperlink, text: Option<String> },
    /// Remove Link (only with a link to edit).
    Remove,
    /// OK changed nothing (a mixed or other link with no new target).
    Keep,
}

/// The dialog.
///
/// - Link to: the three targets as craft-ui's `RadioSet` with icons, a column at the left
///   (Word's); a new link's address starts at `https://` (DeckCraft). Address, Email address,
///   Subject and Text to display are craft-ui `TextField`s.
/// - Place in This Document lists [`places`](Self::places) in a craft-ui
///   [`List`](craft_ui::list::List) (one Tab stop, the list keys, type-ahead; Top of the Document
///   first; headings and bookmarks, or slides "1. Title").
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
}

impl HyperlinkDialog {
    /// The dialog over what the selection holds; `text` is the selected text shown in Text to
    /// display (`None` hides the field: a shape's click action).
    pub fn new(current: CurrentLink, text: Option<String>) -> Self {
        HyperlinkDialog { current, text, places: Vec::new(), highlight: false, labels: None }
    }

    /// The places in the document a link may go to.
    pub fn places(mut self, places: Vec<String>) -> Self {
        self.places = places;
        self
    }

    /// Show PowerPoint's Highlight click.
    pub fn highlight_click(mut self, on: bool) -> Self {
        self.highlight = on;
        self
    }

    pub fn labels(mut self, labels: HyperlinkLabels) -> Self {
        self.labels = Some(labels);
        self
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
        todo!()
    }
}
