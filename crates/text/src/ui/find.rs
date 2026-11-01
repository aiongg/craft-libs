//! Find (N3.03): [`FindBar`], Acrobat's ⌘F bar over a document (query, "3 of 12", previous and
//! next, an options menu with Whole words only and Case-sensitive, close), and
//! [`FindReplaceDialog`], Word's Find and Replace (Find, Replace and Go To tabs, Search Options,
//! Replace and Replace All) and PowerPoint's simpler Find / Replace, on the dialog stack. Both
//! search with [`crate::find`]; the document's text and the replacing are the app's.

use craft_ui::controls::{Button, Checkbox, IconButton, SearchField, Select, TextField};
use craft_ui::dialog::{Dialog, DialogId, DialogResult, DialogStack, DialogState, Footer, Outcome, StackDialog, Width};
use craft_ui::focus::key_owner::{OwnerGate, owner};
use craft_ui::icons::Icon;
use craft_ui::localized::{Labels as _, fmt};
use craft_ui::menu::{MenuEntry, MenuItem, PressMenu};
use craft_ui::tab_strip::{Tab, TabEvent, TabKind, TabStrip};
use egui::os::OperatingSystem;
use egui::text::{CCursor, CCursorRange};
use egui::{Context, Id, Key, KeyboardShortcut, Modifiers, Response, Ui};

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
        clear: "find.clear" = "Clear";
        close_button: "find.closeButton" = "Close";
        highlight_all: "find.highlightAll" = "Highlight All";
        clear_highlight: "find.clearHighlight" = "Clear Highlighting";
        paragraph_mark: "find.paragraphMark" = "Paragraph Mark";
        tab_character: "find.tabCharacter" = "Tab Character";
        any_character: "find.anyCharacter" = "Any Character";
        any_digit: "find.anyDigit" = "Any Digit";
        any_letter: "find.anyLetter" = "Any Letter";
        caret_character: "find.caretCharacter" = "Caret Character";
        em_dash: "find.emDash" = "Em Dash";
        en_dash: "find.enDash" = "En Dash";
        nonbreaking_hyphen: "find.nonbreakingHyphen" = "Nonbreaking Hyphen";
        nonbreaking_space: "find.nonbreakingSpace" = "Nonbreaking Space";
        optional_hyphen: "find.optionalHyphen" = "Optional Hyphen";
        white_space: "find.whiteSpace" = "White Space";
        section_break: "find.sectionBreak" = "Section Break";
        line_break: "find.lineBreak" = "Manual Line Break";
        found_text: "find.foundText" = "Find What Text";
        clipboard_contents: "find.clipboardContents" = "Clipboard Contents";
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
        let FindBar { id_salt, query, options, matches, progress, labels } = self;
        let ctx = ui.ctx().clone();
        let labels = labels.unwrap_or_else(|| FindLabels::of_ctx(&ctx));
        let (next_key, previous_key) = step_shortcuts(&ctx);
        let mut response = FindBarResponse { search: false, stepped: false, closed: false, response: ui.response() };
        let mut step: Option<bool> = None;

        // The bar takes the keyboard on the pass it opens.
        let opened_id = Id::unique((id_salt, "opened"));
        let pass = ctx.cumulative_pass_nr();
        let opened = ctx.data_mut(|d| d.get_temp::<u64>(opened_id)).is_none_or(|last| last + 1 < pass);
        ctx.data_mut(|d| d.insert_temp(opened_id, pass));

        if !owner(&ctx, &OwnerGate::default()).is_exclusive() {
            if ctx.input_mut(|i| i.consume_shortcut(&previous_key)) {
                step = Some(true);
            } else if ctx.input_mut(|i| i.consume_shortcut(&next_key)) {
                step = Some(false);
            }
        }

        let row = ui.horizontal(|ui| {
            let edit = SearchField::new((id_salt, "query"), labels.find_text.as_str(), query.as_str(), labels.clear.as_str())
                .placeholder(labels.find_text.as_str())
                .keep_focus(true)
                .show(ui);
            if opened {
                edit.response.request_focus();
            }
            if let Some(value) = edit.event.as_ref().and_then(|e| e.value()) {
                query.clone_from(value);
                response.search = true;
            }
            if edit.submitted {
                step = Some(ui.input(|i| i.modifiers.shift));
                edit.response.request_focus();
            }
            if edit.escaped {
                response.closed = true;
            }

            let text = count_text(&labels, query.trim().is_empty(), matches, progress);
            if !text.is_empty() {
                ui.label(text);
            }

            let previous = fmt(&labels.previous, &[("key", &ctx.format_shortcut(&previous_key))]);
            if ui.add(IconButton::new(Icon::ChevronUp, previous).id_salt((id_salt, "previous"))).clicked() {
                step = Some(true);
            }
            let next = fmt(&labels.next, &[("key", &ctx.format_shortcut(&next_key))]);
            if ui.add(IconButton::new(Icon::ChevronDown, next).id_salt((id_salt, "next"))).clicked() {
                step = Some(false);
            }

            let active = options.match_case || options.whole_word;
            let button = ui.add(IconButton::new(Icon::Settings, labels.options.as_str()).selected(active).id_salt((id_salt, "options")));
            let entries: Vec<MenuEntry<BarOption>> = vec![
                MenuItem::new(labels.whole_words_only.as_str(), BarOption::WholeWords).check(options.whole_word).into(),
                MenuItem::new(labels.case_sensitive.as_str(), BarOption::MatchCase).check(options.match_case).into(),
            ];
            match PressMenu::new(&entries).show(ui, &button).chosen {
                Some(BarOption::WholeWords) => {
                    options.whole_word = !options.whole_word;
                    response.search = true;
                }
                Some(BarOption::MatchCase) => {
                    options.match_case = !options.match_case;
                    response.search = true;
                }
                None => {}
            }

            if ui.add(IconButton::new(Icon::Close, labels.close.as_str()).id_salt((id_salt, "close"))).clicked() {
                response.closed = true;
            }
        });
        response.response = row.response;
        if let Some(back) = step {
            response.stepped = matches.step(back).is_some();
        }
        response
    }
}

/// A checked item of the bar's options menu.
#[derive(Clone, Copy)]
enum BarOption {
    WholeWords,
    MatchCase,
}

/// Next and previous: ⌘G and ⇧⌘G on macOS, F3 and ⇧F3 elsewhere.
fn step_shortcuts(ctx: &Context) -> (KeyboardShortcut, KeyboardShortcut) {
    if ctx.os() == OperatingSystem::Mac {
        (KeyboardShortcut::new(Modifiers::COMMAND, Key::G), KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::SHIFT, Key::G))
    } else {
        (KeyboardShortcut::new(Modifiers::NONE, Key::F3), KeyboardShortcut::new(Modifiers::SHIFT, Key::F3))
    }
}

/// The bar's count: nothing for an empty query, "Searching… s/p" while pages are being read and
/// nothing is found yet, "No matches", else "c of n" with a "+" while pages are still being read.
fn count_text(labels: &FindLabels, empty_query: bool, matches: &FindMatches, progress: Option<(usize, usize)>) -> String {
    if empty_query {
        return String::new();
    }
    let reading = progress.filter(|(searched, pages)| searched < pages);
    if matches.is_empty() {
        return match reading {
            Some((searched, pages)) => fmt(&labels.searching, &[("s", &searched.to_string()), ("p", &pages.to_string())]),
            None => labels.no_matches.clone(),
        };
    }
    let current = matches.current().map_or(0, |c| c + 1);
    let more = if reading.is_some() { "+" } else { "" };
    fmt(&labels.count, &[("c", &current.to_string()), ("n", &matches.len().to_string()), ("more", more)])
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
    state: DialogState<()>,
    /// The field that last had the keyboard: where Special inserts.
    last_field: Field,
    go_to_kind: usize,
    go_to_target: String,
}

/// The two text fields of the Find and Replace tabs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Field {
    Find,
    With,
}

/// A press of the dialog's side column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Press {
    Replace,
    ReplaceAll,
    Highlight(bool),
    /// PowerPoint's Replace… on the Find tab.
    ShowReplace,
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
            state: DialogState::new(),
            last_field: Field::Find,
            go_to_kind: 0,
            go_to_target: String::new(),
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
        let labels = self.labels.clone().unwrap_or_else(|| FindLabels::of_ctx(ctx));
        let salt = "find-replace";
        let find_id = Id::unique((salt, "find-what"));
        let with_id = Id::unique((salt, "replace-with"));
        let word = self.word;
        let tab = self.tab;
        let mut query = std::mem::take(&mut self.query);
        let mut with = std::mem::take(&mut self.with);
        let mut options = self.options;
        let mut more = self.more;
        let mut last_field = self.last_field;
        let mut go_kind = self.go_to_kind;
        let mut target = std::mem::take(&mut self.go_to_target);
        let mut next_tab = tab;
        let kinds = self.go_to_kinds.clone();
        let message = self.message.clone();
        let mut press: Option<Press> = None;

        let ok_label = match tab {
            FindTab::GoTo => labels.go_to.clone(),
            _ => labels.find_next.clone(),
        };
        let extras: Vec<(String, Press)> = match (tab, word) {
            (FindTab::Replace, _) => vec![(labels.replace.clone(), Press::Replace), (labels.replace_all.clone(), Press::ReplaceAll)],
            (FindTab::Find, true) => {
                vec![(labels.highlight_all.clone(), Press::Highlight(true)), (labels.clear_highlight.clone(), Press::Highlight(false))]
            }
            (FindTab::Find, false) => vec![(format!("{}…", labels.replace), Press::ShowReplace)],
            (FindTab::GoTo, _) => Vec::new(),
        };
        let ok_enabled = match tab {
            FindTab::GoTo => !kinds.is_empty(),
            _ => !query.is_empty(),
        };
        let mut footer = Footer::side().ok(ok_label).ok_stays_open(true).cancel(labels.close_button.clone()).ok_enabled(ok_enabled);
        for (label, _) in &extras {
            footer = footer.extra(label.clone());
        }

        let shown = Dialog::with_id(self.id(), labels.title.as_str()).modeless().stack(stack).width(Width::Standard).footer(footer).show(
            ctx,
            &mut self.state,
            &mut (),
            |ui, ()| {
                ui.vertical(|ui| {
                    if word {
                        let tabs = [
                            Tab::new(FindTab::Find, labels.find_tab.as_str()),
                            Tab::new(FindTab::Replace, labels.replace_tab.as_str()),
                            Tab::new(FindTab::GoTo, labels.go_to_tab.as_str()),
                        ];
                        let out = TabStrip::new((salt, "tabs"), labels.title.as_str(), TabKind::Text).selected(&tab).show(ui, &tabs);
                        for event in out.events {
                            if let TabEvent::Activate(chosen) = event {
                                next_tab = chosen;
                            }
                        }
                    }
                    if tab == FindTab::GoTo {
                        ui.label(labels.go_to_what.as_str());
                        let mut kind = go_kind.min(kinds.len().saturating_sub(1));
                        let edit = Select::new((salt, "go-to-kind"), labels.go_to_what.as_str(), &mut kind)
                            .rows(kinds.len(), |i| i, |i| kinds.get(i).cloned().unwrap_or_default())
                            .show(ui);
                        if edit.committed().is_some() {
                            go_kind = kind;
                        }
                        let edit = TextField::new((salt, "go-to-target"), labels.go_to.as_str(), &target).keep_focus(true).show(ui);
                        if let Some(value) = edit.event.as_ref().and_then(|e| e.value()) {
                            target.clone_from(value);
                        }
                        return;
                    }

                    ui.label(labels.find_what.as_str());
                    let edit = TextField::new((salt, "find-what"), labels.find_what.as_str(), &query).id(find_id).keep_focus(true).show(ui);
                    if edit.response.has_focus() {
                        last_field = Field::Find;
                    }
                    if let Some(value) = edit.event.as_ref().and_then(|e| e.value()) {
                        query.clone_from(value);
                    }
                    if tab == FindTab::Replace {
                        ui.label(labels.replace_with.as_str());
                        let edit = TextField::new((salt, "replace-with"), labels.replace_with.as_str(), &with).id(with_id).keep_focus(true).show(ui);
                        if edit.response.has_focus() {
                            last_field = Field::With;
                        }
                        if let Some(value) = edit.event.as_ref().and_then(|e| e.value()) {
                            with.clone_from(value);
                        }
                    }

                    if !word {
                        let _ = Checkbox::bool(labels.match_case.as_str(), &mut options.match_case).id_salt((salt, "match-case")).show(ui);
                        let _ = Checkbox::bool(labels.whole_words.as_str(), &mut options.whole_word).id_salt((salt, "whole-words")).show(ui);
                        return;
                    }
                    if ui.add(Button::new(labels.search_options.as_str()).id_salt((salt, "search-options"))).clicked() {
                        more = !more;
                    }
                    if !more {
                        return;
                    }
                    // Word greys the case, word and ignore options out while wildcards are on.
                    let free = !options.wildcards || options.regex;
                    let rows: [(&str, &mut bool, bool); 8] = [
                        (labels.match_case.as_str(), &mut options.match_case, free),
                        (labels.whole_words.as_str(), &mut options.whole_word, free),
                        (labels.use_wildcards.as_str(), &mut options.wildcards, !options.regex),
                        (labels.match_prefix.as_str(), &mut options.prefix, free),
                        (labels.match_suffix.as_str(), &mut options.suffix, free),
                        (labels.ignore_punctuation.as_str(), &mut options.ignore_punctuation, free),
                        (labels.ignore_space.as_str(), &mut options.ignore_space, free),
                        (labels.regular_expressions.as_str(), &mut options.regex, true),
                    ];
                    for (index, (label, value, enabled)) in rows.into_iter().enumerate() {
                        let _ = Checkbox::bool(label, value).enabled(enabled).id_salt((salt, "option", index)).show(ui);
                    }
                    let entries = special_entries(&labels, tab == FindTab::Replace);
                    let button = ui.add(Button::new(labels.special.as_str()).id_salt((salt, "special")));
                    if let Some(code) = PressMenu::new(&entries).show(ui, &button).chosen {
                        match last_field {
                            Field::With if tab == FindTab::Replace => insert_code(ui.ctx(), with_id, &mut with, code),
                            _ => insert_code(ui.ctx(), find_id, &mut query, code),
                        }
                    }
                });
                if !message.is_empty() {
                    ui.label(message.as_str());
                }
            },
        );

        self.query = query;
        self.with = with;
        self.options = options;
        self.more = more;
        self.last_field = last_field;
        self.go_to_kind = go_kind;
        self.go_to_target = target;
        self.tab = next_tab;

        if shown.outcome == Some(Outcome::Ok) {
            self.request = match tab {
                FindTab::GoTo => Some(FindRequest::GoTo { kind: self.go_to_kind, target: self.go_to_target.clone() }),
                _ if self.query.is_empty() => None,
                _ => Some(FindRequest::FindNext { query: self.query.clone(), options: self.options }),
            };
        }
        if let Some(index) = shown.extra
            && let Some((_, which)) = extras.get(index)
        {
            press = Some(*which);
        }
        if let Some(which) = press {
            let (query, with, options) = (self.query.clone(), self.with.clone(), self.options);
            match which {
                Press::ShowReplace => self.tab = FindTab::Replace,
                Press::Highlight(on) => self.request = Some(FindRequest::Highlight { query, options, on }),
                Press::Replace if !query.is_empty() => self.request = Some(FindRequest::Replace { query, with, options }),
                Press::ReplaceAll if !query.is_empty() => self.request = Some(FindRequest::ReplaceAll { query, with, options }),
                Press::Replace | Press::ReplaceAll => {}
            }
        }
        match shown.outcome {
            Some(Outcome::Cancel | Outcome::Close) => Some(DialogResult::Ok(())),
            _ => None,
        }
    }
}

/// The Special menu: the `^` codes of Find what, or of Replace with.
fn special_entries(labels: &FindLabels, replacing: bool) -> Vec<MenuEntry<&'static str>> {
    let find: [(&str, &'static str); 14] = [
        (&labels.paragraph_mark, "^p"),
        (&labels.tab_character, "^t"),
        (&labels.any_character, "^?"),
        (&labels.any_digit, "^#"),
        (&labels.any_letter, "^$"),
        (&labels.caret_character, "^^"),
        (&labels.em_dash, "^+"),
        (&labels.en_dash, "^="),
        (&labels.nonbreaking_hyphen, "^~"),
        (&labels.nonbreaking_space, "^s"),
        (&labels.optional_hyphen, "^-"),
        (&labels.white_space, "^w"),
        (&labels.section_break, "^b"),
        (&labels.line_break, "^l"),
    ];
    let replace: [(&str, &'static str); 11] = [
        (&labels.paragraph_mark, "^p"),
        (&labels.tab_character, "^t"),
        (&labels.caret_character, "^^"),
        (&labels.em_dash, "^+"),
        (&labels.en_dash, "^="),
        (&labels.nonbreaking_hyphen, "^~"),
        (&labels.nonbreaking_space, "^s"),
        (&labels.optional_hyphen, "^-"),
        (&labels.line_break, "^l"),
        (&labels.found_text, "^&"),
        (&labels.clipboard_contents, "^c"),
    ];
    let items: &[(&str, &'static str)] = if replacing { &replace } else { &find };
    items.iter().map(|(label, code)| MenuItem::new(format!("{label}  {code}"), *code).into()).collect()
}

/// Insert `code` into `text` at the field's cursor (replacing its selection), else at the end,
/// and put the cursor after it.
fn insert_code(ctx: &Context, field: Id, text: &mut String, code: &str) {
    let state = egui::TextEdit::load_state(ctx, field);
    let range = state.as_ref().and_then(|s| s.cursor.char_range());
    let length = text.chars().count();
    let (start, end) = range.map_or((length, length), |r| {
        let (a, b) = (r.primary.index.0.min(length), r.secondary.index.0.min(length));
        (a.min(b), a.max(b))
    });
    let byte = |chars: usize| text.char_indices().nth(chars).map_or(text.len(), |(b, _)| b);
    text.replace_range(byte(start)..byte(end), code);
    if let Some(mut state) = state {
        state.cursor.set_char_range(Some(CCursorRange::one(CCursor::new(start + code.chars().count()))));
        egui::TextEdit::store_state(ctx, field, state);
    }
}
