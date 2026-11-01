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

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::dialog::{DialogId, DialogResult, DialogStack, StackDialog};
use craft_ui::localized::units::LengthUnit;
use craft_ui::menu::MenuEntry;
use egui::Context;

use crate::setup::{MARGIN_PRESETS, MarginPreset, Orientation, PAPER_SIZES, PageSetup, PaperSize};

/// The margin and paper fields' scrub step, page points (WordCraft's 0.05 in).
pub const MARGIN_SCRUB_STEP: f64 = 3.6;

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
    message: Option<String>,
    labels: Option<PageSetupLabels>,
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
            message: None,
            labels: None,
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
        todo!()
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

    /// The presets (a uniform one reads "Normal  2.54 cm all" in the unit), a separator and
    /// Custom Margins…; Mirrored is checked when the document mirrors margins.
    // wordcraft: crates/ui-egui/src/ribbon/layout.rs:8 `margins_items`.
    pub fn margins(&self) -> Vec<MenuEntry<PageLayoutChoice>> {
        todo!()
    }

    // wordcraft: crates/ui-egui/src/ribbon/layout.rs:30 `orientation_items`.
    pub fn orientation(&self) -> Vec<MenuEntry<PageLayoutChoice>> {
        todo!()
    }

    /// The paper sizes with their size in the unit as a second line, a separator and More Paper
    /// Sizes….
    // wordcraft: crates/ui-egui/src/ribbon/layout.rs:51 (the Size menu button);
    // designcraft: crates/ui-egui/src/chrome/control_bar.rs:44 (presets in the control bar).
    pub fn size(&self) -> Vec<MenuEntry<PageLayoutChoice>> {
        todo!()
    }
}
