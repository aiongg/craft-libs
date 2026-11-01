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
//! if let Some(choice) = PageLayoutMenus::new(&doc.page_setup(), prefs.unit).margins(ui) { doc.apply(choice) }
//! ```

use std::cell::Cell;
use std::ops::RangeInclusive;
use std::sync::Arc;

use craft_ui::composites::{FormGrid, Notice, SectionHeader, Tone};
use craft_ui::controls::expr::ExprUnits;
use craft_ui::controls::{Checkbox, NumberField, RadioGroup, Select};
use craft_ui::dialog::{Dialog, DialogId, DialogResult, DialogStack, DialogState, Footer, InitialFocus, Outcome, StackDialog, Width};
use craft_ui::edit::EditEvent;
use craft_ui::localized::units::{LengthUnit, UnitContext};
use craft_ui::localized::{LocaleNumbers, SharedLabels as _};
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

/// `points` as the text a menu shows: the number in `unit` without trailing zeros, then the unit
/// (inches as the double prime).
fn length_text(points: f64, unit: LengthUnit) -> String {
    let value = UnitContext::default().from_pixels(points, unit).unwrap_or(points);
    let number = format!("{:.*}", unit.decimals(), value);
    let number = if number.contains('.') { number.trim_end_matches('0').trim_end_matches('.') } else { number.as_str() };
    match unit {
        LengthUnit::Inches => format!("{number}\""),
        unit => format!("{number} {}", unit.suffix()),
    }
}

/// A length field in `unit` over `points`, which an edit changes (clamped to `range`, points).
/// A value that rounds to what the field showed changes nothing, so an untouched length keeps its
/// exact value.
#[expect(clippy::too_many_arguments, reason = "a field names every input of its control")]
fn length_field(
    ui: &mut Ui,
    salt: &str,
    caption: &str,
    points: &mut f64,
    range: RangeInclusive<f64>,
    unit: LengthUnit,
    numbers: &LocaleNumbers,
    id: Option<Id>,
) -> egui::Response {
    let context = UnitContext::default();
    let convert = |points: f64| context.from_pixels(points, unit).unwrap_or(points);
    let shown = rounded(convert(*points), unit);
    let units = ExprUnits { unit: Some(unit), suffix: unit.suffix(), context };
    let mut field = unit_number_field(salt.to_owned(), caption, shown, numbers, &units)
        .range(convert(*range.start())..=convert(*range.end()))
        .step(convert(MARGIN_SCRUB_STEP));
    if let Some(id) = id {
        field = field.with_id(id);
    }
    let edit = field.show(ui);
    let value = match edit.event.as_ref() {
        Some(EditEvent::Commit { changed: false, .. }) | None => None,
        Some(event) => event.value().copied(),
    };
    if let Some(value) = value.filter(|v| rounded(*v, unit) != shown)
        && let Some(new) = context.to_pixels(value, unit)
    {
        *points = new.clamp(*range.start(), *range.end());
    }
    edit.response
}

impl PageSetupDialog {
    fn top_id(&self) -> Id {
        self.id().with("top")
    }

    fn body(&mut self, ui: &mut Ui, labels: &PageSetupLabels, numbers: &LocaleNumbers, valid: &Cell<bool>) {
        let gap = Gap::Section.points(&Tokens::of(ui));
        let unit = self.unit;
        let top_id = self.top_id();
        let mirrored = self.setup.mirrored;
        let (left, right) = if mirrored { (&labels.inside, &labels.outside) } else { (&labels.left, &labels.right) };
        let side = 0.0..=MAX_PAGE_SIDE;
        let setup = &mut self.setup;

        SectionHeader::new("page-setup-margins", labels.margins.as_str()).collapsible(false).show(ui, |ui| {
            FormGrid::new("page-setup-margins-grid").show(ui, |form| {
                form.row(labels.top.as_str(), |ui, _| {
                    length_field(ui, "top", &labels.top, &mut setup.margins.top, side.clone(), unit, numbers, Some(top_id))
                });
                form.row(labels.bottom.as_str(), |ui, _| {
                    length_field(ui, "bottom", &labels.bottom, &mut setup.margins.bottom, side.clone(), unit, numbers, None)
                });
                form.row(left.as_str(), |ui, _| length_field(ui, "left", left, &mut setup.margins.left, side.clone(), unit, numbers, None));
                form.row(right.as_str(), |ui, _| length_field(ui, "right", right, &mut setup.margins.right, side.clone(), unit, numbers, None));
                if !self.document_setup {
                    form.row(labels.gutter.as_str(), |ui, _| {
                        length_field(ui, "gutter", &labels.gutter, &mut setup.gutter, side.clone(), unit, numbers, None)
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
                        let edit = NumberField::new("pages", labels.number_of_pages.as_str(), shown, numbers)
                            .range(1.0..=f64::from(MAX_PAGES))
                            .decimals(0..=0)
                            .show(ui);
                        if let Some(value) = edit.event.as_ref().and_then(EditEvent::value).filter(|v| **v != shown) {
                            setup.pages = Some(value.round() as u32);
                        }
                        edit
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
                        select = select.option(PaperChoice::Named(index), paper.name);
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
                let (mut width, mut height) = (setup.width, setup.height);
                form.row(labels.width.as_str(), |ui, _| length_field(ui, "width", &labels.width, &mut width, sides.clone(), unit, numbers, None));
                form.row(labels.height.as_str(), |ui, _| length_field(ui, "height", &labels.height, &mut height, sides.clone(), unit, numbers, None));
                if (width, height) != (setup.width, setup.height) {
                    setup.set_size(width, height);
                }
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
///   ([`MIN_PAGE_SIDE`](crate::setup::MIN_PAGE_SIDE)–[`MAX_PAGE_SIDE`](crate::setup::MAX_PAGE_SIDE));
///   Custom size stays chosen while the numbers match a name; a typed size wider than tall turns
///   the orientation ([`PageSetup::set_size`]).
/// - With [`layout`](Self::layout): Vertical alignment, Different Odd & Even Pages, Different
///   First Page; with [`direction`](Self::direction): Section direction.
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
    state: DialogState,
}

impl PageSetupDialog {
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

    pub fn layout(mut self, on: bool) -> Self {
        self.layout = on;
        self
    }

    pub fn direction(mut self, on: bool) -> Self {
        self.direction = on;
        self
    }

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
        let mut state = std::mem::take(&mut self.state);
        let shown = Dialog::with_id(self.id(), title.as_str())
            .stack(stack)
            .width(Width::Standard)
            .initial_focus(InitialFocus::Field(self.top_id()))
            .footer(Footer::standard().ok_enabled_with(|| valid.get()))
            .show(ctx, &mut state, &mut (), |ui, ()| self.body(ui, &labels, &numbers, &valid));
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
    labels: Option<PageSetupLabels>,
}

impl<'a> PageLayoutMenus<'a> {
    pub fn new(setup: &'a PageSetup, unit: LengthUnit) -> Self {
        PageLayoutMenus { setup, unit, presets: &MARGIN_PRESETS, papers: PAPER_SIZES, labels: None }
    }

    pub fn presets(mut self, presets: &'a [MarginPreset]) -> Self {
        self.presets = presets;
        self
    }

    pub fn papers(mut self, papers: &'a [PaperSize]) -> Self {
        self.papers = papers;
        self
    }

    pub fn labels(mut self, labels: PageSetupLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    fn words(&self) -> PageSetupLabels {
        self.labels.clone().unwrap_or_default()
    }

    /// The presets (a uniform one reads "Normal  2.54 cm all" in the unit), a separator and
    /// Custom Margins…; Mirrored is checked when the document mirrors margins.
    // wordcraft: crates/ui-egui/src/ribbon/layout.rs:8 `margins_items`.
    pub fn margins(&self) -> Vec<MenuEntry<PageLayoutChoice>> {
        let labels = self.words();
        let current = MarginPreset::matching(self.presets, &self.setup.margins, self.setup.mirrored);
        let mut entries: Vec<MenuEntry<PageLayoutChoice>> = self
            .presets
            .iter()
            .map(|preset| {
                let m = preset.margins;
                let uniform = m.top == m.bottom && m.top == m.left && m.top == m.right;
                let label = if uniform {
                    let size = length_text(m.top, self.unit);
                    labels.uniform_preset.replace("{name}", preset.name).replace("{size}", &size)
                } else {
                    preset.name.to_owned()
                };
                MenuItem::new(label, PageLayoutChoice::Margins(*preset)).check(current == Some(preset)).into()
            })
            .collect();
        entries.push(MenuEntry::Separator);
        entries.push(MenuItem::new(labels.custom_margins.as_str(), PageLayoutChoice::PageSetup).into());
        entries
    }

    // wordcraft: crates/ui-egui/src/ribbon/layout.rs:30 `orientation_items`.
    pub fn orientation(&self) -> Vec<MenuEntry<PageLayoutChoice>> {
        let labels = self.words();
        [(Orientation::Portrait, &labels.portrait), (Orientation::Landscape, &labels.landscape)]
            .into_iter()
            .map(|(orientation, label)| {
                MenuItem::new(label.as_str(), PageLayoutChoice::Orientation(orientation)).check(self.setup.orientation == orientation).into()
            })
            .collect()
    }

    /// The paper sizes with their size in the unit as a second line, a separator and More Paper
    /// Sizes….
    // wordcraft: crates/ui-egui/src/ribbon/layout.rs:51 (the Size menu button);
    // designcraft: crates/ui-egui/src/chrome/control_bar.rs:44 (presets in the control bar).
    pub fn size(&self) -> Vec<MenuEntry<PageLayoutChoice>> {
        let labels = self.words();
        let current = paper_named(self.papers, self.setup.width, self.setup.height);
        let mut entries: Vec<MenuEntry<PageLayoutChoice>> = self
            .papers
            .iter()
            .map(|paper| {
                let size = format!("{} \u{d7} {}", length_text(paper.width, self.unit), length_text(paper.height, self.unit));
                MenuItem::new(format!("{}  {size}", paper.name), PageLayoutChoice::Paper(*paper)).check(current == Some(paper)).into()
            })
            .collect();
        entries.push(MenuEntry::Separator);
        entries.push(MenuItem::new(labels.more_paper_sizes.as_str(), PageLayoutChoice::PageSetup).into());
        entries
    }
}
