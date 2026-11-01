//! Find (N3.03): [`FindBar`], Acrobat's ⌘F bar over a document (query, "3 of 12", previous and
//! next, an options menu with Whole words only and Case-sensitive, close), and
//! [`FindReplaceDialog`], Word's Find and Replace (Find, Replace and Go To tabs, Search Options,
//! Replace and Replace All) and PowerPoint's simpler Find / Replace, on the dialog stack. Both
//! search with [`crate::find`]; the document's text and the replacing are the app's.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::dialog::{DialogId, DialogResult, DialogStack, StackDialog};
use egui::{Context, Response, Ui};

use crate::find::{FindMatches, FindOptions};

craft_ui::words! {
    /// The bar's and dialog's words (PdfCraft's, WordCraft's and DeckCraft's strings).
    #[non_exhaustive]
    pub struct FindLabels {
        find_text: "find.findText" = "Find text";
        searching: "find.searching" = "Searching… {s}/{p}";
        no_matches: "find.noMatches" = "No matches";
        count: "find.count" = "{c} of {n}{more}";
        previous: "find.previous" = "Previous ({key})";
        next: "find.next" = "Next ({key})";
        options: "find.options" = "Find options";
        whole_words_only: "find.wholeWordsOnly" = "Whole words only";
        case_sensitive: "find.caseSensitive" = "Case-sensitive";
        close: "find.close" = "Close (Esc)";
        title: "find.title" = "Find and Replace";
        find_tab: "find.findTab" = "Find";
        replace_tab: "find.replaceTab" = "Replace";
        go_to_tab: "find.goToTab" = "Go To";
        find_what: "find.findWhat" = "Find what:";
        replace_with: "find.replaceWith" = "Replace with:";
        match_case: "find.matchCase" = "Match case";
        whole_words: "find.wholeWords" = "Whole words";
        use_wildcards: "find.useWildcards" = "Use wildcards";
        search_options: "find.searchOptions" = "Search Options";
        match_prefix: "find.matchPrefix" = "Match prefix";
        match_suffix: "find.matchSuffix" = "Match suffix";
        ignore_punctuation: "find.ignorePunctuation" = "Ignore punctuation characters";
        ignore_space: "find.ignoreSpace" = "Ignore white-space characters";
        regular_expressions: "find.regex" = "Regular expressions";
        special: "find.special" = "Special";
        find_next: "find.findNext" = "Find Next";
        replace: "find.replace" = "Replace";
        replace_all: "find.replaceAll" = "Replace All";
        replaced: "find.replaced" = "All done. We made {n} replacements.";
        not_found: "find.notFound" = "{app} can't find “{text}”.";
        go_to_what: "find.goToWhat" = "Go to what:";
        go_to: "find.goTo" = "Go To";
    }
}

/// What the bar did this pass.
#[derive(Debug)]
pub struct FindBarResponse {
    /// The query or an option changed: search again ([`FindMatches::new`] from the current page).
    pub search: bool,
    /// Enter, ⇧-Enter, the arrows or ⌘G / ⇧⌘G: step through the matches (the bar already called
    /// [`FindMatches::step`]); the app shows the current match.
    pub stepped: bool,
    /// Esc in the field or the close button.
    pub closed: bool,
    /// The bar's row.
    pub response: Response,
}

/// The find bar, built per pass while open, over the app's query, options and matches.
///
/// - The field is craft-ui's [`SearchField`](craft_ui::controls::SearchField) (placeholder "Find
///   text"); it takes the keyboard when the bar opens; Enter steps to the next match and keeps
///   the keyboard, ⇧-Enter the previous; Esc clears the field and a second Esc closes the bar (the
///   suite's search field rule).
/// - The count reads "Searching… {s}/{p}" while pages are still being read, "No matches", or
///   "{c} of {n}" (with "+" in `{more}` when a page hit its match limit).
/// - Previous and Next are craft-ui `IconButton`s with their shortcuts in the tooltips; the
///   options button opens a `PressMenu` of the options (checked items) and shows pressed while an
///   option is on; changing an option searches again.
/// - Keys: the field, Previous, Next and the options button are Tab stops in that order; ⌘G and
///   ⇧⌘G (F3 and ⇧F3 elsewhere) step while the bar is open, through `focus::key_owner`.
pub struct FindBar<'a> {
    id_salt: egui::IdSalt,
    query: &'a mut String,
    options: &'a mut FindOptions,
    matches: &'a mut FindMatches,
    progress: Option<(usize, usize)>,
    labels: Option<FindLabels>,
}

impl<'a> FindBar<'a> {
    pub fn new(id_salt: impl egui::AsIdSalt, query: &'a mut String, options: &'a mut FindOptions, matches: &'a mut FindMatches) -> Self {
        FindBar { id_salt: egui::IdSalt::new(id_salt), query, options, matches, progress: None, labels: None }
    }

    /// Pages searched so far and the page count, while the search runs.
    pub fn progress(mut self, searched: usize, pages: usize) -> Self {
        self.progress = Some((searched, pages));
        self
    }

    pub fn labels(mut self, labels: FindLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // pdfcraft: crates/ui-egui/src/canvas/find.rs:208 `find_bar`, :84 `rerun_find`.
    pub fn show(self, ui: &mut Ui) -> FindBarResponse {
        todo!()
    }
}

/// Which tab of Find and Replace is open.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FindTab {
    #[default]
    Find,
    Replace,
    GoTo,
}

/// A request the dialog makes while open; the app runs it and reports back with
/// [`FindReplaceDialog::set_message`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FindRequest {
    FindNext {
        query: String,
        options: FindOptions,
    },
    /// Replace the current match, then find the next.
    Replace {
        query: String,
        with: String,
        options: FindOptions,
    },
    ReplaceAll {
        query: String,
        with: String,
        options: FindOptions,
    },
    /// Word's Reading Highlight: highlight every match (`on`) or clear the highlight.
    Highlight {
        query: String,
        options: FindOptions,
        on: bool,
    },
    /// Go To: the kind's index in [`FindReplaceDialog::go_to_kinds`] and the typed target.
    GoTo {
        kind: usize,
        target: String,
    },
}

/// Find and Replace, modeless on the dialog stack (the document stays editable under it).
///
/// - Word ([`word`](Self::word) on): Find, Replace and Go To tabs; Match case, Whole words and Use
///   wildcards (wildcards grey out the other two, as Word); Search Options with Match prefix,
///   Match suffix, Ignore punctuation, Ignore white-space, Regular expressions and the Special
///   menu of `^` codes (craft-ui `MenuEntry`s shown with `PressMenu`, inserted into the field
///   that last had the keyboard). Tabs are craft-ui's tab strip; options are `Checkbox`es.
/// - PowerPoint (off): Find what, Replace with after Replace…, Match case, Find whole words only;
///   Find Next, Replace, Replace All, Close.
/// - Enter in Find what finds the next match and keeps the keyboard there (DeckCraft #951);
///   the status line shows the app's message ("All done. We made 3 replacements.").
pub struct FindReplaceDialog {
    query: String,
    with: String,
    options: FindOptions,
    tab: FindTab,
    word: bool,
    more: bool,
    go_to_kinds: Vec<String>,
    message: String,
    request: Option<FindRequest>,
    labels: Option<FindLabels>,
}

impl FindReplaceDialog {
    pub fn new(query: impl Into<String>, tab: FindTab) -> Self {
        FindReplaceDialog {
            query: query.into(),
            with: String::new(),
            options: FindOptions::default(),
            tab,
            word: false,
            more: false,
            go_to_kinds: Vec::new(),
            message: String::new(),
            request: None,
            labels: None,
        }
    }

    pub fn word(mut self, on: bool) -> Self {
        self.word = on;
        self
    }

    pub fn options(mut self, options: FindOptions) -> Self {
        self.options = options;
        self
    }

    /// Go To's kinds (Page, Section, Line, Bookmark, Comment, …), the app's.
    pub fn go_to_kinds(mut self, kinds: Vec<String>) -> Self {
        self.go_to_kinds = kinds;
        self
    }

    pub fn labels(mut self, labels: FindLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The request the user made since the last call, if any.
    pub fn take_request(&mut self) -> Option<FindRequest> {
        self.request.take()
    }

    /// The status line under the buttons.
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }
}

impl StackDialog for FindReplaceDialog {
    type Result = ();

    fn id(&self) -> DialogId {
        craft_ui::dialog::dialog_id("find-replace")
    }

    // wordcraft: crates/ui-egui/src/dialogs/dispatch.rs:350 `find_dialog`; deckcraft:
    // crates/ui-egui/src/dialogs/find.rs:4 `find_dialog`.
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<()>> {
        todo!()
    }
}
