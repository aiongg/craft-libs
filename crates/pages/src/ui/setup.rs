//! [`PageSetupDialog`] (N3.08): Word's Page Setup and InDesign's Document Setup on the dialog
//! stack, and [`PageLayoutMenus`]: Layout › Margins, Orientation and Size as menu items.
//!
//! ```ignore
//! dialogs.open(PageSetupDialog::new(doc.page_setup()).unit(prefs.unit).layout(true));
//! // a later pass:
//! if let Some(DialogResult::Ok(setup)) = dialogs.take::<PageSetupDialog>() {
//!     doc.apply_page_setup(setup);       // one undo step
//! }
//! // the Layout tab:
//! if let Some(choice) = PageLayoutMenus::new(ui.ctx(), &doc.page_setup(), prefs.unit).margins(ui) { doc.apply(choice) }
//! ```

use std::cell::Cell;
use std::ops::RangeInclusive;
use std::sync::Arc;

use craft_ui::composites::{FormGrid, Notice, SectionHeader, Tone};
use craft_ui::controls::expr::ExprUnits;
use craft_ui::controls::{Checkbox, NumberField, RadioGroup, Select};
use craft_ui::dialog::{Dialog, DialogId, DialogResult, DialogStack, DialogState, Footer, InitialFocus, Outcome, StackDialog, Width};
use craft_ui::edit::{DraftStep, EditEvent, GestureDraft};
use craft_ui::localized::units::{LengthUnit, UnitContext};
use craft_ui::localized::{LocaleNumbers, SharedLabels as _, fmt};
use craft_ui::menu::{MenuEntry, MenuItem};
use craft_ui::patterns::unit_number_field;
use craft_ui::primitives::Gap;
use craft_ui::theme::{Tokens, TokensOf as _};
use egui::{Context, Id, Ui};

use crate::setup::{
    MARGIN_PRESETS, MAX_PAGE_SIDE, MIN_PAGE_SIDE, MarginPreset, Orientation, PAPER_SIZES, PageSetup, PageVerticalAlign, PaperSize, paper_named,
};

/// The margin and paper fields' scrub step, page points (WordCraft's 0.05 in).
pub const MARGIN_SCRUB_STEP: f64 = 3.6;

/// The most pages Document Setup takes (InDesign's 9999).
const MAX_PAGES: u32 = 9999;

craft_ui::words! {
    /// The dialog's and menus' words (WordCraft's strings; DesignCraft's for the pages row).
    #[non_exhaustive]
    pub struct PageSetupLabels {
        title: "pageSetup.title" = "Page Setup";
        document_setup: "pageSetup.documentSetup" = "Document Setup";
        margins: "pageSetup.margins" = "Margins";
        top: "pageSetup.top" = "Top:";
        bottom: "pageSetup.bottom" = "Bottom:";
        left: "pageSetup.left" = "Left:";
        right: "pageSetup.right" = "Right:";
        inside: "pageSetup.inside" = "Inside:";
        outside: "pageSetup.outside" = "Outside:";
        gutter: "pageSetup.gutter" = "Gutter:";
        orientation: "pageSetup.orientation" = "Orientation";
        portrait: "pageSetup.portrait" = "Portrait";
        landscape: "pageSetup.landscape" = "Landscape";
        pages: "pageSetup.pages" = "Pages";
        mirror_margins: "pageSetup.mirrorMargins" = "Mirror margins";
        facing_pages: "pageSetup.facingPages" = "Facing Pages";
        number_of_pages: "pageSetup.numberOfPages" = "Number of Pages:";
        paper: "pageSetup.paper" = "Paper";
        paper_size: "pageSetup.paperSize" = "Paper size:";
        custom_size: "pageSetup.customSize" = "Custom size";
        width: "pageSetup.width" = "Width:";
        height: "pageSetup.height" = "Height:";
        more_paper_sizes: "pageSetup.morePaperSizes" = "More Paper Sizes…";
        vertical_alignment: "pageSetup.verticalAlignment" = "Vertical alignment:";
        align_top: "pageSetup.alignTop" = "Top";
        align_center: "pageSetup.alignCenter" = "Center";
        align_justified: "pageSetup.alignJustified" = "Justified";
        align_bottom: "pageSetup.alignBottom" = "Bottom";
        headers_and_footers: "pageSetup.headersAndFooters" = "Headers and footers";
        different_odd_even: "pageSetup.differentOddEven" = "Different Odd & Even Pages";
        different_first: "pageSetup.differentFirst" = "Different First Page";
        section_direction: "pageSetup.sectionDirection" = "Section direction:";
        left_to_right: "pageSetup.leftToRight" = "Left-to-right";
        right_to_left: "pageSetup.rightToLeft" = "Right-to-left";
        margins_dont_fit: "pageSetup.marginsDontFit" = "The margins don't fit on a page this size; make the margins smaller first.";
        uniform_preset: "pageSetup.uniformPreset" = "{name}  {size} all";
        custom_margins: "pageSetup.customMargins" = "Custom Margins…";
        /// A length in inches: the number and the double prime.
        length_inches: "pageSetup.lengthInches" = "{value}\"";
        /// A length in any other unit: the number and the unit's suffix.
        length_unit: "pageSetup.lengthUnit" = "{value} {unit}";
        /// A paper's size under its name.
        paper_dimensions: "pageSetup.paperDimensions" = "{width} \u{d7} {height}";
        paper_letter: "pageSetup.paperLetter" = "Letter";
        paper_legal: "pageSetup.paperLegal" = "Legal";
        paper_executive: "pageSetup.paperExecutive" = "Executive";
        paper_a3: "pageSetup.paperA3" = "A3";
        paper_a4: "pageSetup.paperA4" = "A4";
        paper_a5: "pageSetup.paperA5" = "A5";
        paper_b4_jis: "pageSetup.paperB4Jis" = "B4 (JIS)";
        paper_b5_jis: "pageSetup.paperB5Jis" = "B5 (JIS)";
        paper_tabloid: "pageSetup.paperTabloid" = "Tabloid";
        paper_statement: "pageSetup.paperStatement" = "Statement";
        paper_envelope_10: "pageSetup.paperEnvelope10" = "Envelope #10";
        paper_envelope_dl: "pageSetup.paperEnvelopeDl" = "Envelope DL";
        paper_a0: "pageSetup.paperA0" = "A0";
        paper_a1: "pageSetup.paperA1" = "A1";
        paper_a2: "pageSetup.paperA2" = "A2";
        paper_a6: "pageSetup.paperA6" = "A6";
        paper_b5: "pageSetup.paperB5" = "B5";
        paper_letter_half: "pageSetup.paperLetterHalf" = "Letter - Half";
        paper_legal_half: "pageSetup.paperLegalHalf" = "Legal - Half";
        paper_us_business_card: "pageSetup.paperUsBusinessCard" = "US Business Card";
        preset_normal: "pageSetup.presetNormal" = "Normal";
        preset_narrow: "pageSetup.presetNarrow" = "Narrow";
        preset_moderate: "pageSetup.presetModerate" = "Moderate";
        preset_wide: "pageSetup.presetWide" = "Wide";
        preset_mirrored: "pageSetup.presetMirrored" = "Mirrored";
        preset_office2003: "pageSetup.presetOffice2003" = "Office 2003 Default";
    }
}

impl PageSetupLabels {
    /// The word for `paper`, matched on its English name; a paper this table has no word for (an
    /// app's own list) keeps its own name.
    pub fn paper_name<'a>(&'a self, paper: &'a PaperSize) -> &'a str {
        match paper.name {
            "Letter" => &self.paper_letter,
            "Legal" => &self.paper_legal,
            "Executive" => &self.paper_executive,
            "A3" => &self.paper_a3,
            "A4" => &self.paper_a4,
            "A5" => &self.paper_a5,
            "B4 (JIS)" => &self.paper_b4_jis,
            "B5 (JIS)" => &self.paper_b5_jis,
            "Tabloid" => &self.paper_tabloid,
            "Statement" => &self.paper_statement,
            "Envelope #10" => &self.paper_envelope_10,
            "Envelope DL" => &self.paper_envelope_dl,
            "A0" => &self.paper_a0,
            "A1" => &self.paper_a1,
            "A2" => &self.paper_a2,
            "A6" => &self.paper_a6,
            "B5" => &self.paper_b5,
            "Letter - Half" => &self.paper_letter_half,
            "Legal - Half" => &self.paper_legal_half,
            "US Business Card" => &self.paper_us_business_card,
            other => other,
        }
    }

    /// The word for `preset`, matched on its id; a preset this table has no word for keeps its own
    /// name.
    pub fn preset_name<'a>(&'a self, preset: &'a MarginPreset) -> &'a str {
        match preset.id {
            "normal" => &self.preset_normal,
            "narrow" => &self.preset_narrow,
            "moderate" => &self.preset_moderate,
            "wide" => &self.preset_wide,
            "mirrored" => &self.preset_mirrored,
            "office2003" => &self.preset_office2003,
            _ => preset.name,
        }
    }
}

/// Which row of the paper list is chosen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PaperChoice {
    Named(usize),
    Custom,
}

/// `value` rounded to the decimals a field in `unit` shows.
fn rounded(value: f64, unit: LengthUnit) -> f64 {
    let scale = 10f64.powi(unit.decimals() as i32);
    (value * scale).round() / scale
}

/// `points` as the text a menu shows: the number in `unit` without trailing zeros, in the user's
/// digits, then the unit (inches as the double prime).
fn length_text(points: f64, unit: LengthUnit, labels: &PageSetupLabels, numbers: &LocaleNumbers) -> String {
    let value = UnitContext::default().from_pixels(points, unit).unwrap_or(points);
    let number = numbers.format(value, 0..=unit.decimals());
    match unit {
        LengthUnit::Inches => fmt(&labels.length_inches, &[("value", &number)]),
        unit => fmt(&labels.length_unit, &[("value", &number), ("unit", unit.suffix())]),
    }
}

/// The gesture a number field of the dialog is in, with the whole setup from before its `Begin`,
/// so Esc restores everything the gesture touched (a width typed past the height turns the
/// orientation too).
#[derive(Default)]
pub(super) struct FieldDraft {
    on: Option<(&'static str, GestureDraft<PageSetup>)>,
}

impl FieldDraft {
    /// Runs one event of the field `key`: `write` puts the event's value into a copy of the
    /// setup, which the draft helper previews, keeps or restores.
    pub(super) fn apply(&mut self, key: &'static str, setup: &mut PageSetup, event: EditEvent<f64>, write: impl Fn(&mut PageSetup, f64)) {
        let mut draft = match self.on.take() {
            Some((on, draft)) if on == key && draft.active() => draft,
            _ => GestureDraft::new(),
        };
        let event = event.map(|value| {
            let mut next = setup.clone();
            write(&mut next, value);
            next
        });
        if draft.apply(setup, event) == DraftStep::Previewed && draft.active() {
            self.on = Some((key, draft));
        }
    }
}

/// What the dialog's number fields share in one pass.
struct Fields<'a> {
    unit: LengthUnit,
    numbers: &'a LocaleNumbers,
    /// Set when a field's text can't become a value: OK waits until it does.
    rejected: &'a Cell<bool>,
}

/// One length of the setup: where it is and which values it takes (points).
struct Length<'a> {
    salt: &'static str,
    caption: &'a str,
    get: fn(&PageSetup) -> f64,
    set: fn(&mut PageSetup, f64),
    range: RangeInclusive<f64>,
    id: Option<Id>,
}

impl Fields<'_> {
    /// A length field in the dialog's unit over the setup's value, which an edit changes (clamped to
    /// the range). A value that rounds to what the field showed changes nothing, so an untouched
    /// length keeps its exact value.
    fn length(&self, ui: &mut Ui, draft: &mut FieldDraft, setup: &mut PageSetup, length: Length<'_>) -> egui::Response {
        let Length { salt, caption, get, set, range, id } = length;
        let (unit, context) = (self.unit, UnitContext::default());
        let convert = |points: f64| context.from_pixels(points, unit).unwrap_or(points);
        let shown = rounded(convert(get(setup)), unit);
        let units = ExprUnits { unit: Some(unit), suffix: unit.suffix(), context };
        let mut field = unit_number_field(salt.to_owned(), caption, shown, self.numbers, &units)
            .range(convert(*range.start())..=convert(*range.end()))
            .step(convert(MARGIN_SCRUB_STEP));
        if let Some(id) = id {
            field = field.with_id(id);
        }
        let output = field.show_full(ui);
        self.rejected.set(self.rejected.get() | output.rejected);
        if let Some(event) = output.edit.event {
            // The unit conversion fails for no length unit; an event it can't convert is dropped.
            if event.value().is_none_or(|value| context.to_pixels(*value, unit).is_some()) {
                let exact = get(setup);
                let points = |value: f64| {
                    if rounded(value, unit) == shown {
                        exact
                    } else {
                        context.to_pixels(value, unit).unwrap_or(exact).clamp(*range.start(), *range.end())
                    }
                };
                draft.apply(salt, setup, event, |next, value| set(next, points(value)));
            }
        }
        output.edit.response
    }
}

impl PageSetupDialog {
    fn top_id(&self) -> Id {
        self.id().with("top")
    }

    fn body(&mut self, ui: &mut Ui, labels: &PageSetupLabels, numbers: &LocaleNumbers, valid: &Cell<bool>, rejected: &Cell<bool>) {
        let gap = Gap::Section.points(&Tokens::of(ui));
        let unit = self.unit;
        let top_id = self.top_id();
        let mirrored = self.setup.mirrored;
        let (left, right) = if mirrored { (&labels.inside, &labels.outside) } else { (&labels.left, &labels.right) };
        let side = 0.0..=MAX_PAGE_SIDE;
        let setup = &mut self.setup;
        let draft = &mut self.draft;
        let fields = Fields { unit, numbers, rejected };

        SectionHeader::new("page-setup-margins", labels.margins.as_str()).collapsible(false).show(ui, |ui| {
            FormGrid::new("page-setup-margins-grid").show(ui, |form| {
                let margin = |salt, caption, get, set, id| Length { salt, caption, get, set, range: side.clone(), id };
                form.row(labels.top.as_str(), |ui, _| {
                    let length = margin("top", &labels.top, |s: &PageSetup| s.margins.top, |s: &mut PageSetup, v| s.margins.top = v, Some(top_id));
                    fields.length(ui, draft, setup, length)
                });
                form.row(labels.bottom.as_str(), |ui, _| {
                    let length = margin("bottom", &labels.bottom, |s| s.margins.bottom, |s, v| s.margins.bottom = v, None);
                    fields.length(ui, draft, setup, length)
                });
                form.row(left.as_str(), |ui, _| {
                    fields.length(ui, draft, setup, margin("left", left, |s| s.margins.left, |s, v| s.margins.left = v, None))
                });
                form.row(right.as_str(), |ui, _| {
                    fields.length(ui, draft, setup, margin("right", right, |s| s.margins.right, |s, v| s.margins.right = v, None))
                });
                if !self.document_setup {
                    form.row(labels.gutter.as_str(), |ui, _| {
                        fields.length(ui, draft, setup, margin("gutter", &labels.gutter, |s| s.gutter, |s, v| s.gutter = v, None))
                    });
                }
            });
            let fits = setup.margins.fit(setup.width, setup.height, setup.gutter);
            valid.set(fits);
            if !fits {
                Notice::new(labels.margins_dont_fit.as_str()).tone(Tone::Bad).id_salt("page-setup-fit").show(ui);
            }
        });
        ui.add_space(gap);

        SectionHeader::new("page-setup-orientation", labels.orientation.as_str()).collapsible(false).show(ui, |ui| {
            let mut orientation = setup.orientation;
            let _ = RadioGroup::new("page-setup-orientation-group", labels.orientation.as_str(), &mut orientation)
                .item(Orientation::Portrait, labels.portrait.as_str())
                .item(Orientation::Landscape, labels.landscape.as_str())
                .show(ui);
            setup.set_orientation(orientation);
        });
        ui.add_space(gap);

        SectionHeader::new("page-setup-pages", labels.pages.as_str()).collapsible(false).show(ui, |ui| {
            if self.document_setup {
                let _ = Checkbox::bool(labels.facing_pages.as_str(), &mut setup.mirrored).show(ui);
                FormGrid::new("page-setup-pages-grid").show(ui, |form| {
                    form.row(labels.number_of_pages.as_str(), |ui, _| {
                        let shown = f64::from(setup.pages.unwrap_or(1));
                        let output = NumberField::new("pages", labels.number_of_pages.as_str(), shown, numbers)
                            .range(1.0..=f64::from(MAX_PAGES))
                            .decimals(0..=0)
                            .show_full(ui);
                        rejected.set(rejected.get() | output.rejected);
                        if let Some(event) = output.edit.event {
                            let event = event.map(|value| if value == shown { value } else { value.round() });
                            draft.apply("pages", setup, event, |next, value| {
                                if value != shown {
                                    next.pages = Some(value as u32);
                                }
                            });
                        }
                        output.edit.response
                    });
                });
            } else {
                let _ = Checkbox::bool(labels.mirror_margins.as_str(), &mut setup.mirrored).show(ui);
            }
        });
        ui.add_space(gap);

        SectionHeader::new("page-setup-paper", labels.paper.as_str()).collapsible(false).show(ui, |ui| {
            FormGrid::new("page-setup-paper-grid").show(ui, |form| {
                form.row(labels.paper_size.as_str(), |ui, _| {
                    let named = paper_named(self.papers, setup.width, setup.height).and_then(|p| self.papers.iter().position(|q| q == p));
                    let mut choice = match named {
                        Some(index) if !self.custom => PaperChoice::Named(index),
                        _ => PaperChoice::Custom,
                    };
                    let before = choice;
                    let mut select = Select::new("page-setup-paper-size", labels.paper_size.as_str(), &mut choice);
                    for (index, paper) in self.papers.iter().enumerate() {
                        select = select.option(PaperChoice::Named(index), labels.paper_name(paper));
                    }
                    let edit = select.option(PaperChoice::Custom, labels.custom_size.as_str()).show(ui);
                    if choice != before {
                        match choice {
                            PaperChoice::Named(index) => {
                                if let Some(paper) = self.papers.get(index) {
                                    let (width, height) = setup.orientation.turn((paper.width, paper.height));
                                    setup.set_size(width, height);
                                }
                                self.custom = false;
                            }
                            PaperChoice::Custom => self.custom = true,
                        }
                    }
                    edit
                });
                let sides = MIN_PAGE_SIDE..=MAX_PAGE_SIDE;
                let side = |salt, caption, get, set| Length { salt, caption, get, set, range: sides.clone(), id: None };
                form.row(labels.width.as_str(), |ui, _| {
                    fields.length(
                        ui,
                        draft,
                        setup,
                        side("width", &labels.width, |s: &PageSetup| s.width, |s: &mut PageSetup, v| s.set_size(v, s.height)),
                    )
                });
                form.row(labels.height.as_str(), |ui, _| {
                    fields.length(ui, draft, setup, side("height", &labels.height, |s| s.height, |s, v| s.set_size(s.width, v)))
                });
            });
        });

        if self.layout {
            ui.add_space(gap);
            SectionHeader::new("page-setup-layout", labels.headers_and_footers.as_str()).collapsible(false).show(ui, |ui| {
                FormGrid::new("page-setup-layout-grid").show(ui, |form| {
                    form.row(labels.vertical_alignment.as_str(), |ui, _| {
                        Select::new("page-setup-vertical", labels.vertical_alignment.as_str(), &mut setup.vertical_align)
                            .option(PageVerticalAlign::Top, labels.align_top.as_str())
                            .option(PageVerticalAlign::Center, labels.align_center.as_str())
                            .option(PageVerticalAlign::Justify, labels.align_justified.as_str())
                            .option(PageVerticalAlign::Bottom, labels.align_bottom.as_str())
                            .show(ui)
                    });
                });
                let _ = Checkbox::bool(labels.different_odd_even.as_str(), &mut setup.different_odd_even).show(ui);
                let _ = Checkbox::bool(labels.different_first.as_str(), &mut setup.different_first).show(ui);
            });
        }
        if self.direction {
            ui.add_space(gap);
            FormGrid::new("page-setup-direction-grid").show(ui, |form| {
                form.row(labels.section_direction.as_str(), |ui, _| {
                    RadioGroup::new("page-setup-direction", labels.section_direction.as_str(), &mut setup.right_to_left)
                        .item(false, labels.left_to_right.as_str())
                        .item(true, labels.right_to_left.as_str())
                        .show(ui)
                });
            });
        }
    }
}

/// Page Setup (or Document Setup) as a stack dialog. Its result is the edited [`PageSetup`]; OK is
/// refused, with [`PageSetupLabels::margins_dont_fit`] under the fields, while the margins don't
/// fit the paper ([`Margins::fit`](crate::setup::Margins::fit)).
///
/// - Margins: Top, Bottom, Left (Inside with mirrored margins), Right (Outside), and Gutter, in
///   the dialog's [`LengthUnit`], each craft-ui's `unit_number_field` (typed in any unit, scrubbed,
///   two decimals, the unit's suffix), the scrub step [`MARGIN_SCRUB_STEP`] page points.
/// - Orientation: Portrait and Landscape as a craft-ui `RadioGroup`; switching turns the size
///   (and the margins) ([`PageSetup::set_orientation`]).
/// - Pages: Mirror margins (Word) or Facing Pages and Number of Pages (InDesign, with
///   [`document_setup`](Self::document_setup)).
/// - Paper: a craft-ui `Select` of [`papers`](Self::papers) plus Custom size, and Width and Height
///   `unit_number_field`s
///   ([`MIN_PAGE_SIDE`]–[`MAX_PAGE_SIDE`]);
///   Custom size stays chosen while the numbers match a name; a typed size wider than tall turns
///   the orientation ([`PageSetup::set_size`]).
/// - With [`layout`](Self::layout): Vertical alignment, Different Odd & Even Pages, Different
///   First Page; with [`direction`](Self::direction): Section direction.
/// - A length or count gesture previews in [`setup`](Self::setup) and a cancel of the gesture
///   restores the whole setup from before it (craft-ui's `GestureDraft`); a text no number can
///   come from keeps OK disabled until it is fixed.
/// - Paper names are words ([`PageSetupLabels::paper_name`]).
/// - Keys: Enter is OK, Esc Cancel; first focus on Top. Group labels are form section headers
///   (the dialog framework's), not bold text.
pub struct PageSetupDialog {
    setup: PageSetup,
    papers: &'static [PaperSize],
    unit: LengthUnit,
    document_setup: bool,
    layout: bool,
    direction: bool,
    custom: bool,
    labels: Option<PageSetupLabels>,
    draft: FieldDraft,
    state: DialogState,
}

impl PageSetupDialog {
    /// A dialog that edits `setup`, with every paper size and no layout group.
    pub fn new(setup: PageSetup) -> Self {
        PageSetupDialog {
            setup,
            papers: PAPER_SIZES,
            unit: LengthUnit::Inches,
            document_setup: false,
            layout: false,
            direction: false,
            custom: false,
            labels: None,
            draft: FieldDraft::default(),
            state: DialogState::new(),
        }
    }

    /// The paper sizes listed (all of [`PAPER_SIZES`] by default).
    pub fn papers(mut self, papers: &'static [PaperSize]) -> Self {
        self.papers = papers;
        self
    }

    /// The unit the fields show (the app's measurement unit preference).
    pub fn unit(mut self, unit: LengthUnit) -> Self {
        self.unit = unit;
        self
    }

    /// InDesign's File › Document Setup: titled so, with Number of Pages and Facing Pages, no
    /// gutter and no layout group.
    pub fn document_setup(mut self, on: bool) -> Self {
        self.document_setup = on;
        self
    }

    /// Shows the Layout group (vertical alignment, odd and even pages, first page) when `on`.
    pub fn layout(mut self, on: bool) -> Self {
        self.layout = on;
        self
    }

    /// Shows the section direction field when `on`.
    pub fn direction(mut self, on: bool) -> Self {
        self.direction = on;
        self
    }

    /// Sets the translated labels; the default reads them from the interface locale.
    pub fn labels(mut self, labels: PageSetupLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The setup as edited so far (for a live preview of the page).
    pub fn setup(&self) -> &PageSetup {
        &self.setup
    }
}

impl StackDialog for PageSetupDialog {
    type Result = PageSetup;

    fn id(&self) -> DialogId {
        craft_ui::dialog::dialog_id("page-setup")
    }

    // wordcraft: crates/ui-egui/src/dialogs/dispatch.rs:552 `page_setup_dialog`,
    // crates/ui-egui/src/dialogs/page_setup.rs:82 `page_paper_group`, :126 `page_valign_combo`;
    // designcraft: crates/ui-egui/src/panels/properties/document.rs:212 (pages and facing pages).
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<PageSetup>> {
        let labels = match &self.labels {
            Some(labels) => Arc::new(labels.clone()),
            None => PageSetupLabels::shared_ctx(ctx),
        };
        let numbers = LocaleNumbers::of_ctx(ctx);
        let title = if self.document_setup { &labels.document_setup } else { &labels.title };
        let valid = Cell::new(true);
        let rejected = Cell::new(false);
        let mut state = std::mem::take(&mut self.state);
        let shown = Dialog::with_id(self.id(), title.as_str())
            .stack(stack)
            .width(Width::Standard)
            .initial_focus(InitialFocus::Field(self.top_id()))
            .footer(Footer::standard().ok_enabled_with(|| valid.get() && !rejected.get()))
            .show(ctx, &mut state, &mut (), |ui, ()| self.body(ui, &labels, &numbers, &valid, &rejected));
        self.state = state;
        match shown.outcome {
            Some(Outcome::Ok) => Some(DialogResult::Ok(self.setup.clone())),
            Some(Outcome::Cancel | Outcome::Close) => Some(DialogResult::Cancel),
            _ => None,
        }
    }
}

/// A choice from one of the Layout menus.
#[derive(Clone, Debug, PartialEq)]
pub enum PageLayoutChoice {
    /// A margins preset (its margins and whether it mirrors).
    Margins(MarginPreset),
    /// The orientation to apply.
    Orientation(Orientation),
    /// A paper size, in the current orientation.
    Paper(PaperSize),
    /// Custom Margins… or More Paper Sizes…: open Page Setup.
    PageSetup,
}

/// Layout › Margins, Orientation and Size as craft-ui menu entries (`MenuEntry<PageLayoutChoice>`,
/// no `Ui`), each item checked when it matches the page setup at the caret (Word's check marks).
/// The app shows them with `menu::PressMenu` from a ribbon's menu button, in its menu bar or a
/// context menu, and applies the chosen item's value.
pub struct PageLayoutMenus<'a> {
    setup: &'a PageSetup,
    unit: LengthUnit,
    presets: &'a [MarginPreset],
    papers: &'a [PaperSize],
    labels: Arc<PageSetupLabels>,
    numbers: Arc<LocaleNumbers>,
}

impl<'a> PageLayoutMenus<'a> {
    /// The menus over `setup`, with the words and digits installed in `ctx` and lengths in `unit`.
    pub fn new(ctx: &Context, setup: &'a PageSetup, unit: LengthUnit) -> Self {
        PageLayoutMenus {
            setup,
            unit,
            presets: &MARGIN_PRESETS,
            papers: PAPER_SIZES,
            labels: PageSetupLabels::shared_ctx(ctx),
            numbers: LocaleNumbers::of_ctx(ctx),
        }
    }

    /// Sets the margin presets the Margins menu lists.
    pub fn presets(mut self, presets: &'a [MarginPreset]) -> Self {
        self.presets = presets;
        self
    }

    /// Sets the paper sizes the Size menu lists.
    pub fn papers(mut self, papers: &'a [PaperSize]) -> Self {
        self.papers = papers;
        self
    }

    /// Sets the translated labels.
    pub fn labels(mut self, labels: PageSetupLabels) -> Self {
        self.labels = Arc::new(labels);
        self
    }

    fn length(&self, points: f64) -> String {
        length_text(points, self.unit, &self.labels, &self.numbers)
    }

    /// The presets (a uniform one reads "Normal  2.54 cm all" in the unit), a separator and
    /// Custom Margins…; Mirrored is checked when the document mirrors margins.
    // wordcraft: crates/ui-egui/src/ribbon/layout.rs:8 `margins_items`.
    pub fn margins(&self) -> Vec<MenuEntry<PageLayoutChoice>> {
        let labels = &self.labels;
        let current = MarginPreset::matching(self.presets, &self.setup.margins, self.setup.mirrored);
        let mut entries: Vec<MenuEntry<PageLayoutChoice>> = self
            .presets
            .iter()
            .map(|preset| {
                let m = preset.margins;
                let uniform = m.top == m.bottom && m.top == m.left && m.top == m.right;
                let name = labels.preset_name(preset);
                let label = if uniform { fmt(&labels.uniform_preset, &[("name", name), ("size", &self.length(m.top))]) } else { name.to_owned() };
                MenuItem::new(label, PageLayoutChoice::Margins(*preset)).check(current == Some(preset)).into()
            })
            .collect();
        entries.push(MenuEntry::Separator);
        entries.push(MenuItem::new(labels.custom_margins.as_str(), PageLayoutChoice::PageSetup).into());
        entries
    }

    // wordcraft: crates/ui-egui/src/ribbon/layout.rs:30 `orientation_items`.
    /// The Orientation menu entries: Portrait and Landscape.
    pub fn orientation(&self) -> Vec<MenuEntry<PageLayoutChoice>> {
        let labels = &self.labels;
        [(Orientation::Portrait, &labels.portrait), (Orientation::Landscape, &labels.landscape)]
            .into_iter()
            .map(|(orientation, label)| {
                MenuItem::new(label.as_str(), PageLayoutChoice::Orientation(orientation)).check(self.setup.orientation == orientation).into()
            })
            .collect()
    }

    /// The paper sizes with their size in the unit on the detail line under the name, a separator
    /// and More Paper Sizes….
    // wordcraft: crates/ui-egui/src/ribbon/layout.rs:51 (the Size menu button);
    // designcraft: crates/ui-egui/src/chrome/control_bar.rs:44 (presets in the control bar).
    pub fn size(&self) -> Vec<MenuEntry<PageLayoutChoice>> {
        let labels = &self.labels;
        let current = paper_named(self.papers, self.setup.width, self.setup.height);
        let mut entries: Vec<MenuEntry<PageLayoutChoice>> = self
            .papers
            .iter()
            .map(|paper| {
                let size = fmt(&labels.paper_dimensions, &[("width", &self.length(paper.width)), ("height", &self.length(paper.height))]);
                MenuItem::new(labels.paper_name(paper), PageLayoutChoice::Paper(*paper)).detail(size).check(current == Some(paper)).into()
            })
            .collect();
        entries.push(MenuEntry::Separator);
        entries.push(MenuItem::new(labels.more_paper_sizes.as_str(), PageLayoutChoice::PageSetup).into());
        entries
    }
}
