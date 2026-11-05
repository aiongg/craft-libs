//! [`PageRangeField`] (N3.13): Acrobat's Pages selected field: a text field taking "1-3, 5"
//! with a chevron menu of page choices, showing the selection back in the same format; and
//! [`PageRangeOptions`], the All pages / Pages from–to / Subset row of Acrobat's Page Range
//! Options.

use craft_ui::composites::Field;
use craft_ui::controls::{ControlSize, IconButton, NumberField, RadioSet, Select, TextField};
use craft_ui::edit::{EditEvent, EditPolicy};
use craft_ui::icons::Icon;
use craft_ui::localized::{Labels as _, LocaleNumbers};
use craft_ui::menu::{MenuEntry, PressMenu};
use craft_ui::theme::{Tokens, TokensOf as _};
use egui::{Response, Ui};

use crate::range::{PageRange, PageRangeError, Subset, format_pages, parse_pages};

craft_ui::words! {
    /// The field's and row's words (PdfCraft's strings).
    #[non_exhaustive]
    pub struct PageRangeLabels {
        selected_pages: "pageRange.selectedPages" = "Selected pages";
        select_pages: "pageRange.selectPages" = "Select pages";
        page_range: "pageRange.pageRange" = "Page range";
        not_a_page: "pageRange.notAPage" = "\"{page}\" is not a page number or label";
        out_of_range: "pageRange.outOfRange" = "page {n} is out of range (1–{count})";
        too_long: "pageRange.tooLong" = "Too many pages";
        all_pages: "pageRange.allPages" = "All pages";
        pages_from: "pageRange.pagesFrom" = "Pages from";
        to: "pageRange.to" = "to";
        subset: "pageRange.subset" = "Subset";
        all_in_range: "pageRange.allInRange" = "All pages in range";
        even_only: "pageRange.evenOnly" = "Even pages only";
        odd_only: "pageRange.oddOnly" = "Odd pages only";
    }
}

impl PageRangeLabels {
    /// The error in these words.
    pub fn error(&self, error: &PageRangeError, numbers: &LocaleNumbers) -> String {
        let whole = |n: usize| numbers.format(n as f64, 0..=0);
        match error {
            PageRangeError::NotAPage(token) => self.not_a_page.replace("{page}", token),
            PageRangeError::OutOfRange { page, count } => self.out_of_range.replace("{n}", &whole(*page)).replace("{count}", &whole(*count)),
            PageRangeError::TooLong => self.too_long.clone(),
        }
    }
}

/// The field's text and error between passes, kept by the app per field. The default is empty.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PageRangeFieldState {
    text: String,
    shown: Vec<usize>,
    error: Option<PageRangeError>,
}

impl PageRangeFieldState {
    /// The state with `text` typed over `selected` and read for a document of `count` pages: the
    /// field shows the text and, when it doesn't parse, the error under it (the state a story
    /// shows for the error look).
    pub fn typed(selected: &[usize], text: impl Into<String>, count: usize) -> Self {
        let text = text.into();
        let error = parse_pages(&text, count).err();
        PageRangeFieldState { text, shown: selected.to_vec(), error }
    }
}

/// What the field did this pass.
#[derive(Debug)]
pub struct PageRangeFieldResponse {
    /// The text field's response (focus, rect).
    pub response: Response,
    /// The pages to select, on the pass they were typed or picked.
    pub picked: Option<Vec<usize>>,
}

/// The field, built per pass.
///
/// - Shows `selected` formatted ([`format_pages`](crate::range::format_pages)); with nothing
///   selected, the current page as its placeholder (the page actions apply to).
/// - The field is craft-ui's [`TextField`](craft_ui::controls::TextField) (its typing run, Esc
///   and focus rules). Typed text applies on Enter or when the field loses focus: parsed text
///   returns the pages and shows them back formatted; text that doesn't parse stays, with the
///   error under the field in the field error style (`status.bad.fg`, replacing help text). Esc
///   puts the selection's text back.
/// - The chevron is a craft-ui menu button showing the app's [`menu`](Self::menu) entries (pages
///   by kind: Odd, Even, Portrait, Landscape, All) with `menu::PressMenu`; a pick returns like
///   typed pages.
/// - The field is one Tab stop (AccessKit TextInput "Selected pages"), the chevron another.
pub struct PageRangeField<'a> {
    id_salt: egui::IdSalt,
    state: &'a mut PageRangeFieldState,
    selected: &'a [usize],
    count: usize,
    current: usize,
    menu: &'a [MenuEntry<Vec<usize>>],
    labels: Option<PageRangeLabels>,
}

impl<'a> PageRangeField<'a> {
    /// The field over `selected` (0-based) in a document of `count` pages; `current` is 0-based.
    pub fn new(id_salt: impl egui::AsIdSalt, state: &'a mut PageRangeFieldState, selected: &'a [usize], count: usize, current: usize) -> Self {
        PageRangeField { id_salt: egui::IdSalt::new(id_salt), state, selected, count, current, menu: &[], labels: None }
    }

    /// The chevron's menu: the app's page choices, each item carrying its pages.
    pub fn menu(mut self, entries: &'a [MenuEntry<Vec<usize>>]) -> Self {
        self.menu = entries;
        self
    }

    pub fn labels(mut self, labels: PageRangeLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // pdfcraft: crates/ui-egui/src/panels/page_selection_ui.rs:64 `PageSelectionInput::show`.
    pub fn show(self, ui: &mut Ui) -> PageRangeFieldResponse {
        let PageRangeField { id_salt, state, selected, count, current, menu, labels } = self;
        let labels = labels.unwrap_or_else(|| PageRangeLabels::of(ui));
        let selected_text = format_pages(selected);
        if state.shown != selected {
            state.text.clone_from(&selected_text);
            state.shown = selected.to_vec();
            state.error = None;
        }
        let numbers = LocaleNumbers::of_ctx(ui.ctx());
        let placeholder = numbers.format((current + 1) as f64, 0..=0);
        let error = state.error.as_ref().map(|error| labels.error(error, &numbers));
        let mut chosen = None;
        let tokens = Tokens::of(ui);
        let chevron = IconButton::style(&tokens, ControlSize::Md).side;
        let edit = Field::unlabelled()
            .error(error)
            .show(ui, |ui, invalid| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    let room = if menu.is_empty() { 0.0 } else { chevron };
                    let edit = TextField::new((id_salt, "text"), labels.selected_pages.as_str(), &state.text)
                        .placeholder(placeholder)
                        .policy(EditPolicy::default().on_release_only(true))
                        .invalid(invalid)
                        .width((ui.available_width() - room).max(0.0))
                        .show(ui);
                    if !menu.is_empty() {
                        let button = ui.add(IconButton::new(Icon::ChevronDown, labels.select_pages.as_str()).id_salt((id_salt, "menu")));
                        chosen = PressMenu::new(menu).show(ui, &button).chosen;
                    }
                    edit
                })
                .inner
            })
            .inner;
        let mut picked = None;
        let escaped = edit.escaped;
        match edit.event {
            Some(EditEvent::Commit { value, changed: true, .. }) => {
                state.text = value;
                match parse_pages(&state.text, count) {
                    Ok(pages) => picked = Some(pages),
                    Err(error) => state.error = Some(error),
                }
            }
            Some(EditEvent::Cancel { .. }) => {
                state.text = selected_text;
                state.error = None;
            }
            _ if escaped => {
                state.text = selected_text;
                state.error = None;
            }
            _ => {}
        }
        if let Some(pages) = chosen {
            picked = Some(pages);
        }
        if let Some(pages) = &picked {
            state.text = format_pages(pages);
            state.shown.clone_from(pages);
            state.error = None;
        }
        PageRangeFieldResponse { response: edit.response, picked }
    }
}

/// The All pages / Pages from [ ] to [ ] / Subset [ ] row, editing a dialog's [`PageRange`] draft
/// (dialog values, not the document): a craft-ui `RadioSet` for All and From/To, `NumberField`s
/// for From and To (1 to `count`, whole numbers, disabled with All pages), a `Select` for Subset.
/// Returns the row's response; `changed()` is set when the range changed.
pub struct PageRangeOptions<'a> {
    id_salt: egui::IdSalt,
    range: &'a mut PageRange,
    count: usize,
    labels: Option<PageRangeLabels>,
}

impl<'a> PageRangeOptions<'a> {
    pub fn new(id_salt: impl egui::AsIdSalt, range: &'a mut PageRange, count: usize) -> Self {
        PageRangeOptions { id_salt: egui::IdSalt::new(id_salt), range, count, labels: None }
    }

    pub fn labels(mut self, labels: PageRangeLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // pdfcraft: crates/ui-egui/src/dialogs/marks_ui.rs:58 `PageRange::ui`.
    pub fn show(self, ui: &mut Ui) -> Response {
        let PageRangeOptions { id_salt, range, count, labels } = self;
        let labels = labels.unwrap_or_else(|| PageRangeLabels::of(ui));
        let numbers = LocaleNumbers::of_ctx(ui.ctx());
        let before = *range;
        let last = count.max(1);
        let mut all = range.all;
        let mut set = RadioSet::new((id_salt, "which"), labels.page_range.as_str(), &mut all).item(true).item(false).begin(ui);
        set.radio(ui, 0, labels.all_pages.as_str());
        let from_to = ui
            .horizontal(|ui| {
                set.radio(ui, 1, labels.pages_from.as_str());
                let enabled = !all;
                let shown_from = range.from.clamp(1, last);
                let from = NumberField::new((id_salt, "from"), labels.pages_from.as_str(), shown_from as f64, &numbers)
                    .range(1.0..=last as f64)
                    .decimals(0..=0)
                    .enabled(enabled)
                    .show(ui);
                ui.label(labels.to.as_str());
                let shown_to = range.to.map_or(last, |to| to.clamp(1, last));
                let to = NumberField::new((id_salt, "to"), labels.to.as_str(), shown_to as f64, &numbers)
                    .range(1.0..=last as f64)
                    .decimals(0..=0)
                    .enabled(enabled)
                    .show(ui);
                (from, to, shown_from, shown_to)
            })
            .inner;
        let mut response = set.end(ui).response;
        range.all = all;
        let (from, to, shown_from, shown_to) = from_to;
        if let Some(value) = from.event.as_ref().and_then(EditEvent::value).map(|v| (v.round() as usize).clamp(1, last)).filter(|v| *v != shown_from)
        {
            range.from = value;
            range.to = range.to.map(|to| to.max(value));
        }
        if let Some(value) = to.event.as_ref().and_then(EditEvent::value).map(|v| (v.round() as usize).clamp(1, last)).filter(|v| *v != shown_to) {
            range.to = Some(value);
            range.from = range.from.min(value);
        }
        response |= from.response;
        response |= to.response;
        let subset = ui
            .horizontal(|ui| {
                ui.label(labels.subset.as_str());
                Select::new((id_salt, "subset"), labels.subset.as_str(), &mut range.subset)
                    .option(Subset::All, labels.all_in_range.as_str())
                    .option(Subset::Even, labels.even_only.as_str())
                    .option(Subset::Odd, labels.odd_only.as_str())
                    .show(ui)
            })
            .inner;
        response |= subset.response;
        if *range != before {
            response.mark_changed();
        }
        response
    }
}
