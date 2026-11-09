//! Find (N3.03): [`FindBar`], Acrobat's Cmd+F bar over a document (query, "3 of 12", previous and next, an options
//! menu with Whole words only and Case-sensitive, close), and [`FindReplaceDialog`], Word's Find and Replace (Find,
//! Replace and Go To tabs, Search Options, Replace and Replace All) and PowerPoint's simpler Find / Replace, on the
//! dialog stack. Both search with [`crate::find`]; the document's text and the replacing are the app's.

#[cfg(feature = "screen-testing")]
mod stories;

use std::borrow::Cow;

use craft_behavior::IconId;
use craft_behavior::{SelectionChange, keyed};
use craft_ui::buttons::{Button, IconButton};
use craft_ui::choice::Checkbox;
use craft_ui::contract::{
    Bind, BodyCx, CollectionEvent, Commands as _, Entry, Outcome as _, Props as _, Show as _, ShowCollection as _, ShowContainer as _, Trigger,
    clicked,
};
use craft_ui::dialog::{Dialog, DialogId, DialogResult, DialogStack, StackDialog, dialog_id};
use craft_ui::display::Text;
use craft_ui::layout_components::{Row, Stack};
use craft_ui::menu::Menu;
use craft_ui::pickers::Select;
use craft_ui::runtime::access::PopupKind;
use craft_ui::runtime::env::{Env, Label};
use craft_ui::runtime::focus::{self, FocusTarget};
use craft_ui::sections::{DialogRole, FooterButton};
use craft_ui::tabs::{Tab, TabKind, TabStrip};
use craft_ui::text::TextField;
use egui::{Response, Ui};

use crate::find::{FindError, FindMatches, FindOptions, MAX_COUNT, MAX_PATTERN, RegexError, WildcardError};

craft_ui::words! {
    /// The words of the find bar and the Find and Replace dialog.
    #[non_exhaustive]
    pub struct FindLabels {
        find_text: "find.findText" = "Find text";
        searching: template "find.searching" = "Searching… {s}/{p}";
        no_matches: "find.noMatches" = "No matches";
        count: template "find.count" = "{c} of {n}{more}";
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
        replace_more: "find.replaceMore" = "Replace…";
        replace_all: "find.replaceAll" = "Replace All";
        replaced: template "find.replaced" = "All done. We made {n} replacements.";
        not_found: template "find.notFound" = "{app} can't find “{text}”.";
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
        special_item: template "find.specialItem" = "{name}  {code}";
        error_too_long: template "find.errorTooLong" = "The Find what text is longer than {max} characters";
        wildcard_unclosed_brace: "find.wildcardUnclosedBrace" = "An opening brace needs a closing brace";
        wildcard_unclosed_bracket: "find.wildcardUnclosedBracket" = "An opening bracket needs a closing bracket";
        wildcard_back_reference: "find.wildcardBackReference" = "Back-references (\\1) work in Replace with, not in Find what";
        wildcard_bad_count: template "find.wildcardBadCount" = "{count} is not a valid count";
        wildcard_count_too_large: template "find.wildcardCountTooLarge" = "Counts go up to {max}";
        wildcard_empty_class: "find.wildcardEmptyClass" = "[ ] needs at least one character";
        wildcard_reversed_range: template "find.wildcardReversedRange" = "[{from}-{to}] is not in ascending order";
        wildcard_refused: "find.wildcardRefused" = "This wildcard pattern can't be searched for";
        regex_unclosed_group: "find.regexUnclosedGroup" = "A ( needs a closing )";
        regex_unopened_group: "find.regexUnopenedGroup" = "A ) has no ( before it";
        regex_unclosed_class: "find.regexUnclosedClass" = "A [ needs a closing ]";
        regex_nothing_to_repeat: "find.regexNothingToRepeat" = "Nothing before the repeat (*, +, ? or a count)";
        regex_bad_repetition: "find.regexBadRepetition" = "A repeat count is a number, a number and a comma, or two numbers with the first not above the second";
        regex_reversed_range: "find.regexReversedRange" = "A range of a [ ] class must go up, as in [a-z]";
        regex_bad_escape: "find.regexBadEscape" = "A \\ must be followed by a character it can escape";
        regex_bad_group_name: "find.regexBadGroupName" = "A group name must be a word, used once";
        regex_unsupported: "find.regexUnsupported" = "Back-references and look-around are not supported";
        regex_bad_flag: "find.regexBadFlag" = "This flag is not supported";
        regex_too_big: "find.regexTooBig" = "The expression is too big";
        regex_invalid: "find.regexInvalid" = "This is not a valid regular expression";
    }
}

/// Step through the matches. The bar reads them while it is shown; the chords are the keymap's to rebind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, craft_ui::contract::CommandSet)]
#[command(prefix = "find")]
#[non_exhaustive]
pub enum FindStep {
    /// Go to the next match.
    #[command(label = "Find Next", default = "Cmd+G", default = "F3")]
    Next,
    /// Go to the previous match.
    #[command(label = "Find Previous", default = "Cmd+Shift+G", default = "Shift+F3")]
    Previous,
}

/// What the bar did this pass.
#[derive(Debug)]
#[non_exhaustive]
pub struct FindBarOutput {
    /// The query or an option changed: search again ([`FindMatches::new`] from the current page).
    pub search: bool,
    /// Enter, Shift+Enter, the arrows or the [`FindStep`] chords: step through the matches (the bar already called
    /// [`FindMatches::step`]); the app shows the current match.
    pub stepped: bool,
    /// Esc in the field or the close button.
    pub closed: bool,
    /// The bar's row.
    pub response: Response,
}

impl craft_ui::contract::Outcome for FindBarOutput {
    type Value = craft_ui::contract::NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// Whether the bar has just been asked for, kept by the app beside its query ([`open`](Self::open) when Cmd+F opens
/// it, and again when it is asked for with the bar open). The bar clears it once the query field holds focus.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FindBarState {
    opened: bool,
}

impl FindBarState {
    /// The bar is being opened, or asked for again while open.
    pub fn open(&mut self) {
        self.opened = true;
    }

    /// Whether the bar was asked for and its query field has not taken focus yet.
    pub fn is_opening(&self) -> bool {
        self.opened
    }
}

/// A checked item of the bar's options menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BarOption {
    WholeWords,
    MatchCase,
}

/// The find bar, built per pass while open, over the app's query, options and matches.
///
/// - The field is a search `TextField` (placeholder "Find text"); Enter steps to the next match and keeps the
///   keyboard, Shift+Enter the previous; Esc clears the field and a second Esc closes the bar.
/// - The count reads "Searching… {s}/{p}" while pages are still being read, "No matches", or "{c} of {n}" (with "+"
///   in `{more}` when a page hit its match limit).
/// - Previous and Next are `IconButton`s bound to the [`FindStep`] commands, so their tooltips show the chords; the
///   options button opens a `Menu` of the options (check rows) and shows pressed while an option is on; changing an
///   option searches again.
#[derive(craft_ui::contract::Pattern)]
pub struct FindBar<'a> {
    query: &'a mut String,
    options: &'a mut FindOptions,
    matches: &'a mut FindMatches,
    progress: Option<(usize, usize)>,
    state: Option<&'a mut FindBarState>,
}

impl<'a> FindBar<'a> {
    /// A find bar over the app's `query` and `options`, listing `matches`.
    pub fn new(query: &'a mut String, options: &'a mut FindOptions, matches: &'a mut FindMatches) -> Self {
        FindBar { query, options, matches, progress: None, state: None }
    }

    /// The app's [`FindBarState`].
    #[must_use]
    pub fn state(mut self, state: &'a mut FindBarState) -> Self {
        self.state = Some(state);
        self
    }

    /// Pages searched so far and the page count, while the search runs.
    #[must_use]
    pub fn progress(mut self, searched: usize, pages: usize) -> Self {
        self.progress = Some((searched, pages));
        self
    }

    /// Paints the bar and returns what the user asked for.
    pub fn show(self, ui: &mut Ui) -> FindBarOutput {
        let FindBar { query, options, matches, progress, state } = self;
        let words = Env::of(ui).words::<FindLabels>();
        let mut state = state;
        let opened = state.as_deref().is_some_and(|state| state.opened);
        let mut out = FindBarOutput { search: false, stepped: false, closed: false, response: ui.response() };
        let mut step: Option<bool> = None;
        let row = Row::new().show(ui, |ui, cx: &mut BodyCx<'_>| {
            while let Some((command, _)) = cx.take_command::<FindStep>() {
                step = Some(command == FindStep::Previous);
            }
            let edit = TextField::search(words.find_text.clone(), Bind::given(query.clone())).placeholder(words.find_text.clone()).show(ui);
            if opened {
                focus::request(ui.ctx(), FocusTarget::Id(edit.response.id));
                // Asking again each run until the field holds focus keeps the request across the measuring run.
                if edit.response.has_focus()
                    && let Some(state) = state.as_deref_mut()
                {
                    state.opened = false;
                }
            }
            if let Some(value) = edit.event.as_ref().and_then(|event| event.value()) {
                query.clone_from(value);
                out.search = true;
            }
            if edit.submitted {
                step = Some(edit.submit_mods.shift);
            }
            // The field takes the Esc that clears it; an Esc it passes on closes the bar.
            if cx.take_cancel() {
                out.closed = true;
            }
            let text = count_text(&words, query.trim().is_empty(), matches, progress);
            if !text.is_empty() {
                Text::new(text).show(ui);
            }
            if clicked(&IconButton::command(IconId::new("chevron_up"), FindStep::Previous).show(ui)) {
                step = Some(true);
            }
            if clicked(&IconButton::command(IconId::new("chevron_down"), FindStep::Next).show(ui)) {
                step = Some(false);
            }
            let active = options.match_case || options.whole_word;
            let button = IconButton::new(IconId::new("settings")).label(words.options.clone()).popup(PopupKind::Menu).selected(active).show(ui);
            let entries = [
                Entry::new(BarOption::WholeWords, words.whole_words_only.clone()).checked(options.whole_word),
                Entry::new(BarOption::MatchCase, words.case_sensitive.clone()).checked(options.match_case),
            ];
            match Menu::new(words.options.clone()).opened_by(&button, Trigger::Press).pick(ui, &entries).picked {
                Some(BarOption::WholeWords) => {
                    options.whole_word = !options.whole_word;
                    out.search = true;
                }
                Some(BarOption::MatchCase) => {
                    options.match_case = !options.match_case;
                    out.search = true;
                }
                None => {}
            }
            if clicked(&IconButton::new(IconId::new("close")).label(words.close.clone()).show(ui)) {
                out.closed = true;
            }
        });
        out.response = row.output.response().clone();
        if let Some(back) = step {
            out.stepped = matches.step(back).is_some();
        }
        out
    }
}

/// The bar's count: nothing for an empty query, "Searching… s/p" while pages are being read and nothing is found yet,
/// "No matches", else "c of n" with a "+" while pages are still being read or a page hit its match limit.
fn count_text(words: &FindLabels, empty_query: bool, matches: &FindMatches, progress: Option<(usize, usize)>) -> String {
    if empty_query {
        return String::new();
    }
    let reading = progress.filter(|(searched, pages)| searched < pages);
    if matches.is_empty() {
        return match reading {
            Some((searched, pages)) => words.searching.fill(&[("s", &searched.to_string()), ("p", &pages.to_string())]),
            None => words.no_matches.as_str().to_owned(),
        };
    }
    let current = matches.current().map_or(0, |c| c + 1);
    let more = if reading.is_some() || matches.limit_reached() { "+" } else { "" };
    {
        let (current, total) = (current.to_string(), matches.len().to_string());
        words.count.fill(&[("c", current.as_str()), ("n", total.as_str()), ("more", more)])
    }
}

impl FindLabels {
    /// What to tell the user of a Find what text the engine refused, in these words (the app shows it under the
    /// field or through [`FindReplaceDialog::set_message`]).
    pub fn error(&self, error: &FindError) -> String {
        match error {
            FindError::TooLong => self.error_too_long.fill(&[("max", &MAX_PATTERN.to_string())]),
            FindError::Wildcards(kind) => match kind {
                WildcardError::UnclosedBrace => self.wildcard_unclosed_brace.as_str().to_owned(),
                WildcardError::UnclosedBracket => self.wildcard_unclosed_bracket.as_str().to_owned(),
                WildcardError::BackReference => self.wildcard_back_reference.as_str().to_owned(),
                WildcardError::BadCount(body) => self.wildcard_bad_count.fill(&[("count", &["{", body, "}"].concat())]),
                WildcardError::CountTooLarge => self.wildcard_count_too_large.fill(&[("max", &MAX_COUNT.to_string())]),
                WildcardError::EmptyClass => self.wildcard_empty_class.as_str().to_owned(),
                WildcardError::ReversedRange { from, to } => {
                    self.wildcard_reversed_range.fill(&[("from", &from.to_string()), ("to", &to.to_string())])
                }
                _ => self.wildcard_refused.as_str().to_owned(),
            },
            FindError::Regex(kind) => match kind {
                RegexError::UnclosedGroup => self.regex_unclosed_group.as_str().to_owned(),
                RegexError::UnopenedGroup => self.regex_unopened_group.as_str().to_owned(),
                RegexError::UnclosedClass => self.regex_unclosed_class.as_str().to_owned(),
                RegexError::NothingToRepeat => self.regex_nothing_to_repeat.as_str().to_owned(),
                RegexError::BadRepetition => self.regex_bad_repetition.as_str().to_owned(),
                RegexError::ReversedRange => self.regex_reversed_range.as_str().to_owned(),
                RegexError::BadEscape => self.regex_bad_escape.as_str().to_owned(),
                RegexError::BadGroupName => self.regex_bad_group_name.as_str().to_owned(),
                RegexError::Unsupported => self.regex_unsupported.as_str().to_owned(),
                RegexError::BadFlag => self.regex_bad_flag.as_str().to_owned(),
                RegexError::TooBig => self.regex_too_big.as_str().to_owned(),
                _ => self.regex_invalid.as_str().to_owned(),
            },
        }
    }
}

/// Which tab of Find and Replace is open.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FindTab {
    /// Find.
    #[default]
    Find,
    /// Replace.
    Replace,
    /// Go To.
    GoTo,
}

/// A request the dialog makes while open; the app runs it and reports back with [`FindReplaceDialog::set_message`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FindRequest {
    /// Find the next match.
    #[non_exhaustive]
    FindNext {
        /// The Find what text.
        query: String,
        /// How the Find what text is read.
        options: FindOptions,
    },
    /// Replace the current match, then find the next.
    #[non_exhaustive]
    Replace {
        /// The Find what text.
        query: String,
        /// The Replace with text.
        with: String,
        /// How the Find what text is read.
        options: FindOptions,
    },
    /// Replace every match.
    #[non_exhaustive]
    ReplaceAll {
        /// The Find what text.
        query: String,
        /// The Replace with text.
        with: String,
        /// How the Find what text is read.
        options: FindOptions,
    },
    /// Word's Reading Highlight: highlight every match (`on`) or clear the highlight.
    #[non_exhaustive]
    Highlight {
        /// The Find what text.
        query: String,
        /// How the Find what text is read.
        options: FindOptions,
        /// `true` to highlight every match, `false` to clear the highlight.
        on: bool,
    },
    /// Go To: the kind's index in [`FindReplaceDialog::go_to_kinds`] and the typed target.
    #[non_exhaustive]
    GoTo {
        /// Index into the Go To kinds.
        kind: usize,
        /// The typed target, such as a page number.
        target: String,
    },
}

impl FindRequest {
    /// Find the next match of `query`.
    pub fn find_next(query: impl Into<String>, options: FindOptions) -> Self {
        FindRequest::FindNext { query: query.into(), options }
    }

    /// Replace the current match of `query` with `with`.
    pub fn replace(query: impl Into<String>, with: impl Into<String>, options: FindOptions) -> Self {
        FindRequest::Replace { query: query.into(), with: with.into(), options }
    }

    /// Replace every match of `query` with `with`.
    pub fn replace_all(query: impl Into<String>, with: impl Into<String>, options: FindOptions) -> Self {
        FindRequest::ReplaceAll { query: query.into(), with: with.into(), options }
    }

    /// Highlight every match of `query` (`on`) or clear the highlight.
    pub fn highlight(query: impl Into<String>, options: FindOptions, on: bool) -> Self {
        FindRequest::Highlight { query: query.into(), options, on }
    }

    /// Go To the `target` of the Go To kind at index `kind`.
    pub fn go_to(kind: usize, target: impl Into<String>) -> Self {
        FindRequest::GoTo { kind, target: target.into() }
    }
}

/// The two text fields of the Find and Replace tabs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Field {
    Find,
    With,
}

/// A press of the dialog's footer buttons other than Find Next and Close.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Press {
    Replace,
    ReplaceAll,
    Highlight(bool),
    /// PowerPoint's Replace… on the Find tab.
    ShowReplace,
}

/// Find and Replace, modeless on the dialog stack (the document stays editable under it).
///
/// - Word ([`word`](Self::word) on): Find, Replace and Go To tabs; Match case, Whole words and Use wildcards
///   (wildcards grey out the other two, as Word); Search Options with Match prefix, Match suffix, Ignore
///   punctuation, Ignore white-space, Regular expressions and the Special menu of `^` codes, inserted into the field
///   that last had the keyboard.
/// - PowerPoint (off): Find what, Replace with after Replace…, Match case, Find whole words only; Find Next,
///   Replace, Replace All, Close.
/// - Enter in Find what finds the next match and keeps the keyboard there; the status line shows the app's message
///   ("All done. We made 3 replacements.").
#[derive(Debug, craft_ui::contract::Pattern)]
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
    /// The field that last had the keyboard: where Special inserts.
    last_field: Field,
    /// A code the Special menu chose, written into its field in the next pass.
    insert: Option<(Field, String)>,
    go_to_kind: usize,
    go_to_target: String,
}

impl FindReplaceDialog {
    /// A dialog opened on `tab` with `query` in Find what.
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
            last_field: Field::Find,
            insert: None,
            go_to_kind: 0,
            go_to_target: String::new(),
        }
    }

    /// Selects the Word layout (Find, Replace and Go To tabs, more search options) when `on`, or the PowerPoint layout
    /// when off.
    #[must_use]
    pub fn word(mut self, on: bool) -> Self {
        self.word = on;
        self
    }

    /// Sets the starting options for Match case, Whole words and the rest.
    #[must_use]
    pub fn options(mut self, options: FindOptions) -> Self {
        self.options = options;
        self
    }

    /// Go To's kinds (Page, Section, Line, Bookmark, Comment, …), the app's.
    #[must_use]
    pub fn go_to_kinds(mut self, kinds: Vec<String>) -> Self {
        self.go_to_kinds = kinds;
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
        dialog_id("find-replace")
    }

    fn show(&mut self, ui: &mut Ui, _stack: &DialogStack) -> Option<DialogResult<()>> {
        let words = Env::of(ui).words::<FindLabels>();
        let (word, tab) = (self.word, self.tab);
        let mut query = std::mem::take(&mut self.query);
        let mut with = std::mem::take(&mut self.with);
        let mut options = self.options;
        let mut more = self.more;
        let mut last_field = self.last_field;
        let mut go_kind = self.go_to_kind;
        let mut target = std::mem::take(&mut self.go_to_target);
        let mut next_tab = tab;
        let mut picked_code: Option<(Field, String)> = None;
        let mut submitted = false;
        let pending = self.insert.take();
        let kinds = self.go_to_kinds.clone();
        let message = self.message.clone();

        let accept_label = if tab == FindTab::GoTo { words.go_to.clone() } else { words.find_next.clone() };
        let extras: Vec<(Label, Press)> = match (tab, word) {
            (FindTab::Replace, _) => vec![(words.replace.clone(), Press::Replace), (words.replace_all.clone(), Press::ReplaceAll)],
            (FindTab::Find, true) => {
                vec![(words.highlight_all.clone(), Press::Highlight(true)), (words.clear_highlight.clone(), Press::Highlight(false))]
            }
            (FindTab::Find, false) => vec![(words.replace_more.clone(), Press::ShowReplace)],
            (FindTab::GoTo, _) => Vec::new(),
        };
        let accept_enabled = if tab == FindTab::GoTo { !kinds.is_empty() } else { !query.is_empty() };
        let mut buttons = vec![FooterButton::new(accept_label, DialogRole::Accept).stays_open(true).enabled(accept_enabled)];
        buttons.extend(extras.iter().map(|(label, _)| FooterButton::new(label.clone(), DialogRole::Other).stays_open(true)));
        buttons.push(FooterButton::new(words.close_button.clone(), DialogRole::Cancel));
        let close_at = buttons.len() - 1;

        let shown = Dialog::new(words.title.clone()).modeless().buttons(buttons).show(ui, |ui, _| {
            Stack::new().show(ui, |ui, _| {
                if word {
                    let tabs = [
                        (FindTab::Find, words.find_tab.as_str().to_owned()),
                        (FindTab::Replace, words.replace_tab.as_str().to_owned()),
                        (FindTab::GoTo, words.go_to_tab.as_str().to_owned()),
                    ];
                    let source = keyed(&tabs[..], |tab: &(FindTab, String)| tab.0).with_text(|tab| Cow::Borrowed(tab.1.as_str()));
                    let strip = TabStrip::new(words.title.clone()).kind(TabKind::Text).shown(tab).natural_height(true);
                    let out = strip.show_items(ui, &source, &mut |key, _| {
                        Tab::new(tabs.iter().find(|tab| tab.0 == *key).map_or_else(String::new, |tab| tab.1.clone()))
                    });
                    for event in out.response.events {
                        if let CollectionEvent::Selection(SelectionChange::Set(keys)) = event
                            && let Some(chosen) = keys.last()
                        {
                            next_tab = *chosen;
                        }
                    }
                }
                if tab == FindTab::GoTo {
                    Text::new(words.go_to_what.clone()).show(ui);
                    let entries: Vec<Entry<usize>> = kinds.iter().enumerate().map(|(at, kind)| Entry::new(at, kind.clone())).collect();
                    let edit = Select::new(words.go_to_what.clone(), Bind::given(go_kind.min(kinds.len().saturating_sub(1))), &entries).show(ui);
                    if let Some(value) = edit.event.as_ref().and_then(|event| event.value()) {
                        go_kind = *value;
                    }
                    let edit = TextField::new(words.go_to.clone(), Bind::given(target.clone())).show(ui);
                    if let Some(value) = edit.event.as_ref().and_then(|event| event.value()) {
                        target.clone_from(value);
                    }
                    return;
                }

                Text::new(words.find_what.clone()).show(ui);
                let mut field = TextField::new(words.find_what.clone(), Bind::given(query.clone()));
                if let Some((Field::Find, code)) = &pending {
                    field = field.replace_selection(code.clone());
                }
                let edit = field.show(ui);
                if edit.response.has_focus() {
                    last_field = Field::Find;
                }
                if let Some(value) = edit.event.as_ref().and_then(|event| event.value()) {
                    query.clone_from(value);
                }
                submitted |= edit.submitted;
                if tab == FindTab::Replace {
                    Text::new(words.replace_with.clone()).show(ui);
                    let mut field = TextField::new(words.replace_with.clone(), Bind::given(with.clone()));
                    if let Some((Field::With, code)) = &pending {
                        field = field.replace_selection(code.clone());
                    }
                    let edit = field.show(ui);
                    if edit.response.has_focus() {
                        last_field = Field::With;
                    }
                    if let Some(value) = edit.event.as_ref().and_then(|event| event.value()) {
                        with.clone_from(value);
                    }
                }

                if !message.is_empty() {
                    Text::new(message.clone()).show(ui);
                }
                let check = |ui: &mut Ui, label: &Label, value: &mut bool, enabled: bool| {
                    Checkbox::new(Bind::bound(value)).label(label.clone()).enabled(enabled).show(ui);
                };
                if !word {
                    check(ui, &words.match_case.clone(), &mut options.match_case, true);
                    check(ui, &words.whole_words.clone(), &mut options.whole_word, true);
                    return;
                }
                if clicked(&Button::new(words.search_options.clone()).show(ui)) {
                    more = !more;
                }
                if !more {
                    return;
                }
                // Word greys the case, word and ignore options out while wildcards are on.
                let free = !options.wildcards || options.regex;
                let no_regex = !options.regex;
                check(ui, &words.match_case.clone(), &mut options.match_case, free);
                check(ui, &words.whole_words.clone(), &mut options.whole_word, free);
                check(ui, &words.use_wildcards.clone(), &mut options.wildcards, no_regex);
                check(ui, &words.match_prefix.clone(), &mut options.prefix, free);
                check(ui, &words.match_suffix.clone(), &mut options.suffix, free);
                check(ui, &words.ignore_punctuation.clone(), &mut options.ignore_punctuation, free);
                check(ui, &words.ignore_space.clone(), &mut options.ignore_space, free);
                check(ui, &words.regular_expressions.clone(), &mut options.regex, true);
                let entries = special_entries(&words, tab == FindTab::Replace);
                let button = Button::new(words.special.clone()).show(ui);
                if let Some(code) = Menu::new(words.special.clone()).opened_by(&button, Trigger::Press).pick(ui, &entries).picked {
                    let field = if last_field == Field::With && tab == FindTab::Replace { Field::With } else { Field::Find };
                    picked_code = Some((field, code.to_owned()));
                }
            });
        });

        self.query = query;
        self.with = with;
        self.options = options;
        self.more = more;
        self.last_field = last_field;
        self.go_to_kind = go_kind;
        self.go_to_target = target;
        self.tab = next_tab;
        self.insert = picked_code;

        let picked = shown.output.picked;
        if picked == Some(0) || (submitted && tab != FindTab::GoTo) {
            self.request = match tab {
                FindTab::GoTo => Some(FindRequest::GoTo { kind: self.go_to_kind, target: self.go_to_target.clone() }),
                _ if self.query.is_empty() => None,
                _ => Some(FindRequest::FindNext { query: self.query.clone(), options: self.options }),
            };
        }
        if let Some(index) = picked.and_then(|at| at.checked_sub(1))
            && let Some((_, which)) = extras.get(index)
        {
            let (query, with, options) = (self.query.clone(), self.with.clone(), self.options);
            match which {
                Press::ShowReplace => self.tab = FindTab::Replace,
                Press::Highlight(on) => self.request = Some(FindRequest::Highlight { query, options, on: *on }),
                Press::Replace if !query.is_empty() => self.request = Some(FindRequest::Replace { query, with, options }),
                Press::ReplaceAll if !query.is_empty() => self.request = Some(FindRequest::ReplaceAll { query, with, options }),
                Press::Replace | Press::ReplaceAll => {}
            }
        }
        (picked == Some(close_at) || shown.output.closed.is_some()).then_some(DialogResult::Ok(()))
    }
}

/// The Special menu: the `^` codes of Find what, or of Replace with.
fn special_entries(words: &FindLabels, replacing: bool) -> Vec<Entry<&'static str>> {
    let find: [(&str, &'static str); 14] = [
        (words.paragraph_mark.as_str(), "^p"),
        (words.tab_character.as_str(), "^t"),
        (words.any_character.as_str(), "^?"),
        (words.any_digit.as_str(), "^#"),
        (words.any_letter.as_str(), "^$"),
        (words.caret_character.as_str(), "^^"),
        (words.em_dash.as_str(), "^+"),
        (words.en_dash.as_str(), "^="),
        (words.nonbreaking_hyphen.as_str(), "^~"),
        (words.nonbreaking_space.as_str(), "^s"),
        (words.optional_hyphen.as_str(), "^-"),
        (words.white_space.as_str(), "^w"),
        (words.section_break.as_str(), "^b"),
        (words.line_break.as_str(), "^l"),
    ];
    let replace: [(&str, &'static str); 11] = [
        (words.paragraph_mark.as_str(), "^p"),
        (words.tab_character.as_str(), "^t"),
        (words.caret_character.as_str(), "^^"),
        (words.em_dash.as_str(), "^+"),
        (words.en_dash.as_str(), "^="),
        (words.nonbreaking_hyphen.as_str(), "^~"),
        (words.nonbreaking_space.as_str(), "^s"),
        (words.optional_hyphen.as_str(), "^-"),
        (words.line_break.as_str(), "^l"),
        (words.found_text.as_str(), "^&"),
        (words.clipboard_contents.as_str(), "^c"),
    ];
    let items: &[(&str, &'static str)] = if replacing { &replace } else { &find };
    items.iter().map(|(label, code)| Entry::new(*code, words.special_item.fill(&[("name", *label), ("code", *code)]))).collect()
}
