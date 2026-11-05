//! Stories of the page widgets. Each draws the real widget over a flat-colour page source and
//! sample values, and applies the widget's events to its demo state as an app would. The line
//! under a story names the last event. Sample names are words of [`PagesStoryLabels`].
//!
//! | Story id | Shows |
//! |---|---|
//! | `patterns/page-view/continuous` | twelve pages, fit width, a text selection and margin grids on page 2 |
//! | `patterns/page-view/spreads` | the pages two-up with a cover, not continuous, showing pages 2 and 3 |
//! | `patterns/page-thumbnails/strip` | thirty thumbnails in two sections, page 4 current, 6 to 8 picked |
//! | `patterns/page-setup/dialog` | the dialog over Letter with Normal margins, in inches, layout group on |
//! | `patterns/page-range/field` | the field over pages 1 to 3 and 5 of 12, and the range options row |
//! | `patterns/page-list/artboards` | six artboards, the second active, the fourth locked, two picked |
//! | `composites/column-bar/bar` | four columns at 40, 20, 20 and 20 percent |
//! | `composites/edge-picker/cell` | outer edges on, inner off; the paragraph form beside it |
//! | `composites/table-size/grid` | the 8 by 10 size grid |
//! | `patterns/table-properties/panel` | a 4 by 3 table with two cells selected and mixed row heights |
//! | `patterns/page-preview/overlay` | page 3 of 8 in the preview over a dimmed strip |

use std::ops::Range;

use craft_ui::behavior::Selection;
use craft_ui::canvas::textures::{FakeThumbSource, Revision, ThumbRender, ThumbRequest, ThumbSource};
use craft_ui::canvas::{CanvasView, ToolHost};
use craft_ui::edit::EditEvent;
use craft_ui::localized::units::LengthUnit;
use craft_ui::localized::{Labels as _, fmt};
use craft_ui::menu::{MenuEntry, MenuItem};
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, NoArgs, Story, StoryArgs, StoryLabels, dialog_closed, dialog_shown, drive, lock, shared};
use craft_ui::theme::{Tokens, TokensOf as _};
use craft_ui::typed_story;
use egui::{Ui, vec2};
use kurbo::{Point, Rect, Size};

use super::column_bar::ColumnBar;
use super::grid::{PageGrid, PageGrids};
use super::list::{PageList, PageListEvent, PageListRow};
use super::preview::PagePreview;
use super::range::{PageRangeField, PageRangeFieldState, PageRangeLabels, PageRangeOptions};
use super::setup::PageSetupDialog;
use super::table::{BorderEdges, EdgeKey, EdgePicker, RowHeight, TableChange, TableProperties, TableSizePicker, TableValues};
use super::text_selection::{PageTextSource, PageTexts, TextSelectState, TextSelectTool, TextSelection, TextSpan};
use super::thumbnails::{PageThumbnails, ThumbnailEvent, ThumbnailSection};
use super::view::{PageSource, PageView, PageViewState};
use crate::layout::{Fit, LayoutOptions, PageDisplay};
use crate::range::{PageRange, Subset};
use crate::setup::PageSetup;

/// `patterns/page-view/continuous`, `patterns/page-view/spreads`, `patterns/page-thumbnails/strip`,
/// `patterns/page-setup/dialog`, `patterns/page-range/field`, `patterns/page-list/artboards`,
/// `composites/column-bar/bar`, `composites/edge-picker/cell`, `composites/table-size/grid`,
/// `patterns/table-properties/panel`, `patterns/page-preview/overlay`.
pub fn stories() -> Vec<Story> {
    vec![
        typed_story!("patterns/page-view/continuous", Level::Patterns, "Page view", "Continuous", NoArgs {}, page_view_continuous),
        typed_story!("patterns/page-view/spreads", Level::Patterns, "Page view", "Spreads", NoArgs {}, page_view_spreads),
        typed_story!("patterns/page-thumbnails/strip", Level::Patterns, "Page thumbnails", "Strip", NoArgs {}, thumbnails),
        typed_story!("patterns/page-setup/dialog", Level::Patterns, "Page setup", "Dialog", DialogArgs { open: true }, page_setup),
        typed_story!("patterns/page-range/field", Level::Patterns, "Page range", "Field", NoArgs {}, page_range),
        typed_story!("patterns/page-list/artboards", Level::Patterns, "Page list", "Artboards", NoArgs {}, page_list),
        typed_story!("composites/column-bar/bar", Level::Composites, "Column bar", "Bar", NoArgs {}, column_bar),
        typed_story!("composites/edge-picker/cell", Level::Composites, "Edge picker", "Cell", NoArgs {}, edge_picker),
        typed_story!("composites/table-size/grid", Level::Composites, "Table size", "Grid", NoArgs {}, table_size),
        typed_story!("patterns/table-properties/panel", Level::Patterns, "Table properties", "Panel", NoArgs {}, table_properties),
        typed_story!("patterns/page-preview/overlay", Level::Patterns, "Page preview", "Overlay", DialogArgs { open: true }, page_preview),
    ]
}

craft_ui::words! {
    /// The words the page stories add: section and artboard names.
    #[non_exhaustive]
    pub(crate) struct PagesStoryLabels {
        /// The two sections of the thumbnail strip.
        section_front: "pagesStory.sectionFront" = "Front matter";
        section_body: "pagesStory.sectionBody" = "Body";
        /// The artboards of the page list.
        artboard_cover: "pagesStory.artboardCover" = "Cover";
        artboard_welcome: "pagesStory.artboardWelcome" = "Welcome";
        artboard_features: "pagesStory.artboardFeatures" = "Features";
        artboard_pricing: "pagesStory.artboardPricing" = "Pricing";
        artboard_contact: "pagesStory.artboardContact" = "Contact";
        artboard_credits: "pagesStory.artboardCredits" = "Credits";
    }
}

/// A story that opens a dialog or an overlay.
#[derive(StoryArgs, Clone, Debug, PartialEq)]
struct DialogArgs {
    open: bool,
}

/// US Letter in points, the size of every sample page.
const LETTER: Size = Size::new(612.0, 792.0);

/// The height of a document area over its width.
const AREA_ASPECT: f32 = 0.75;

/// A page source of flat-colour pictures: `count` Letter pages.
struct Pages {
    count: usize,
    fake: FakeThumbSource<usize>,
}

impl Pages {
    fn new(count: usize) -> Self {
        Pages { count, fake: FakeThumbSource::new().aspect(612, 792) }
    }
}

impl ThumbSource<usize> for Pages {
    fn revision(&self, key: &usize, request: &ThumbRequest) -> Revision {
        self.fake.revision(key, request)
    }

    fn render(&self, key: &usize, request: &ThumbRequest) -> ThumbRender {
        self.fake.render(key, request)
    }
}

impl PageSource for Pages {
    fn page_count(&self) -> usize {
        self.count
    }

    fn page_size(&self, _page: usize) -> Size {
        LETTER
    }
}

fn last_event(ui: &mut Ui, event: Option<&str>) {
    if let Some(event) = event {
        let line = fmt(&StoryLabels::of(ui).last_event, &[("event", event)]);
        ui.add(Text::new(line).content(Content::Muted));
    }
}

/// Runs `add` in an area as wide as the story and `aspect` times as high, so a widget that fills
/// the space left has a size.
fn in_area<R>(ui: &mut Ui, aspect: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    let width = ui.available_width();
    ui.allocate_ui(vec2(width, width * aspect), add).inner
}

// --- Mock page text ---

/// Glyphs per line of the mock text, and lines per page.
const GLYPHS_PER_LINE: usize = 48;
const LINES: usize = 36;
/// A glyph box and the line pitch, in page points; the text starts one inch in.
const GLYPH_WIDTH: f64 = 8.0;
const GLYPH_HEIGHT: f64 = 11.0;
const LINE_PITCH: f64 = 16.0;
const TEXT_ORIGIN: f64 = 72.0;
/// Every sixth glyph is a space.
const WORD_LENGTH: usize = 6;

/// A page of identical lines of five-letter words.
struct MockText;

impl MockText {
    fn box_of(glyph: usize) -> Rect {
        let (line, column) = ((glyph / GLYPHS_PER_LINE) as f64, (glyph % GLYPHS_PER_LINE) as f64);
        let (x, y) = (TEXT_ORIGIN + column * GLYPH_WIDTH, TEXT_ORIGIN + line * LINE_PITCH);
        Rect::new(x, y, x + GLYPH_WIDTH, y + GLYPH_HEIGHT)
    }

    fn is_space(glyph: usize) -> bool {
        glyph % WORD_LENGTH == WORD_LENGTH - 1
    }

    fn joined(glyphs: impl Iterator<Item = usize>) -> String {
        let mut out = String::new();
        let mut line = None;
        for glyph in glyphs {
            if line.is_some_and(|l| l != glyph / GLYPHS_PER_LINE) {
                out.push('\n');
            }
            line = Some(glyph / GLYPHS_PER_LINE);
            out.push(if Self::is_space(glyph) { ' ' } else { 'x' });
        }
        out
    }
}

impl PageTextSource for MockText {
    fn glyph_count(&self) -> usize {
        GLYPHS_PER_LINE * LINES
    }

    fn glyph_at(&self, p: Point) -> Option<usize> {
        (0..self.glyph_count()).find(|glyph| Self::box_of(*glyph).contains(p))
    }

    fn line_rects(&self, glyphs: Range<usize>) -> Vec<Rect> {
        (0..LINES)
            .filter_map(|line| {
                let own: Vec<usize> = glyphs.clone().filter(|g| g / GLYPHS_PER_LINE == line).collect();
                own.first().zip(own.last()).map(|(a, b)| Self::box_of(*a).union(Self::box_of(*b)))
            })
            .collect()
    }

    fn glyphs_in(&self, rect: Rect) -> Vec<usize> {
        (0..self.glyph_count()).filter(|glyph| rect.contains_rect(Self::box_of(*glyph))).collect()
    }

    fn word_at(&self, glyph: usize) -> Option<Range<usize>> {
        if Self::is_space(glyph) {
            return None;
        }
        let start = glyph - glyph % WORD_LENGTH;
        Some(start..start + WORD_LENGTH - 1)
    }

    fn line_at(&self, glyph: usize) -> Option<Range<usize>> {
        let start = glyph / GLYPHS_PER_LINE * GLYPHS_PER_LINE;
        Some(start..start + GLYPHS_PER_LINE)
    }

    fn text_of(&self, glyphs: Range<usize>) -> String {
        Self::joined(glyphs)
    }

    fn column_text(&self, glyphs: &[usize]) -> String {
        Self::joined(glyphs.iter().copied())
    }
}

/// Every page has the mock text.
struct MockTexts;

impl PageTexts for MockTexts {
    fn page(&self, _page: usize) -> Option<&dyn PageTextSource> {
        Some(&MockText)
    }
}

// --- Page view ---

struct ViewDemo {
    state: PageViewState,
    text: TextSelectState,
    last: Option<String>,
}

fn view_demo(state: PageViewState) -> ViewDemo {
    let mut text = TextSelectState::default();
    text.selection = Some(TextSelection { page: 1, span: TextSpan::Run { anchor: 100, head: 250 } });
    ViewDemo { state, text, last: None }
}

/// The margin rectangle, the three columns and the baseline grid of a Letter page.
fn sample_grid() -> PageGrid {
    const MARGIN: f64 = 72.0;
    const GUTTER: f64 = 18.0;
    let margins = Rect::new(MARGIN, MARGIN, LETTER.width - MARGIN, LETTER.height - MARGIN);
    let width = (margins.width() - 2.0 * GUTTER) / 3.0;
    let columns = (0..3)
        .map(|i| {
            let x = margins.x0 + f64::from(i) * (width + GUTTER);
            Rect::new(x, margins.y0, x + width, margins.y1)
        })
        .collect();
    PageGrid { page: Rect::from_origin_size(Point::ORIGIN, LETTER), margins, columns, baseline: Some((MARGIN, LINE_PITCH)) }
}

fn show_view(ui: &mut Ui, name: &str, demo: &mut ViewDemo, options: LayoutOptions) {
    let tokens = Tokens::of(ui);
    let pages = Pages::new(12);
    let out = in_area(ui, AREA_ASPECT, |ui| PageView::new(name, &pages, &mut demo.state).options(options).name(name).show(ui));
    let mut tool = TextSelectTool::new(&out.pages, &MockTexts, &mut demo.text);
    let _ = ToolHost::new(out.canvas_id).show(ui, &out.canvas, &out.response, &mut tool);
    let selected = tool.changed();
    if selected {
        demo.last = Some(format!("{:?}", demo.text.selection));
    }
    if out.current_changed {
        demo.last = Some(format!("Current({})", demo.state.current + 1));
    }
    // The second page's grids, through a view that maps page points to its screen rectangle.
    if let Some(placed) = out.pages.iter().find(|placed| placed.page == 1) {
        let view = CanvasView::new(placed.rect, Point::new(LETTER.width / 2.0, LETTER.height / 2.0), placed.scale);
        let colors = PageGrids::style(&tokens);
        PageGrids::new(&view, &[sample_grid()], colors).paint(&ui.painter_at(out.canvas.rect));
    }
    last_event(ui, demo.last.as_deref());
}

/// A default view state with `set` applied (the state's memory is private, so no struct update).
fn view_state(set: impl FnOnce(&mut PageViewState)) -> PageViewState {
    let mut state = PageViewState::default();
    set(&mut state);
    state
}

// new: twelve Letter pages from a flat-colour mock source, continuous, fit width, page gaps,
// with a text selection layer and margin grids on page 2.
fn page_view_continuous(ui: &mut Ui, _: &mut NoArgs) {
    let demo = shared(ui, "view-continuous", || {
        view_demo(view_state(|s| {
            s.fit = Fit::Width;
            s.continuous = true;
            s.page_gaps = true;
            s.go_to = Some((1, 0.0));
        }))
    });
    let mut demo = lock(&demo);
    show_view(ui, "continuous", &mut demo, LayoutOptions::ACROBAT);
}

// new: the same pages two-up with a cover, not continuous, showing spread 2–3.
fn page_view_spreads(ui: &mut Ui, _: &mut NoArgs) {
    let demo = shared(ui, "view-spreads", || {
        view_demo(view_state(|s| {
            s.display = PageDisplay::TwoUp;
            s.cover = true;
            s.continuous = false;
            s.page_gaps = true;
            s.fit = Fit::Page;
            s.current = 1;
        }))
    });
    let mut demo = lock(&demo);
    show_view(ui, "spreads", &mut demo, LayoutOptions::ACROBAT);
}

// --- Thumbnails ---

struct ThumbnailsDemo {
    picked: Selection<usize>,
    current: usize,
    zoom: f32,
    sections: Vec<ThumbnailSection>,
    last: Option<String>,
}

// new: thirty mock thumbnails, page 4 current, 6–8 picked, size row on, two sections.
fn thumbnails(ui: &mut Ui, _: &mut NoArgs) {
    let labels = PagesStoryLabels::of(ui);
    let demo = shared(ui, "thumbnails", || ThumbnailsDemo {
        picked: Selection { items: vec![5, 6, 7], lead: Some(7), anchor: Some(5) },
        current: 3,
        zoom: 1.0,
        sections: vec![
            ThumbnailSection { name: labels.section_front.clone(), first: 0, count: 8, open: true },
            ThumbnailSection { name: labels.section_body.clone(), first: 8, count: 22, open: true },
        ],
        last: None,
    });
    let mut demo = lock(&demo);
    let demo = &mut *demo;
    let pages = Pages::new(30);
    let out = in_area(ui, AREA_ASPECT, |ui| {
        PageThumbnails::new("thumbnails", &pages, &mut demo.picked, demo.current)
            .editable(true)
            .size(&mut demo.zoom)
            .sections(&demo.sections)
            .new_page_row(true)
            .show(ui)
    });
    for event in out.events {
        demo.last = Some(format!("{event:?}"));
        if let ThumbnailEvent::Go(page) = event {
            demo.current = page;
        }
    }
    last_event(ui, demo.last.as_deref());
}

// --- Page setup ---

// new: the dialog over Letter with Normal margins, inches, layout group on.
fn page_setup(ui: &mut Ui, args: &mut DialogArgs) {
    drive(
        ui,
        "page-setup",
        &mut args.open,
        (),
        || PageSetupDialog::new(PageSetup::default()).unit(LengthUnit::Inches).layout(true),
        |_| {},
        |setup| format!("{} × {} pt", setup.width, setup.height),
    );
}

// --- Page range ---

struct RangeDemo {
    field: PageRangeFieldState,
    selected: Vec<usize>,
    range: PageRange,
    last: Option<String>,
}

/// Twelve pages: the count the field and the options work over.
const RANGE_PAGES: usize = 12;

// new: the field over pages 1–3 and 5 of 12, then with "1-x" typed (error under it).
fn page_range(ui: &mut Ui, _: &mut NoArgs) {
    let labels = PageRangeLabels::of(ui);
    let demo = shared(ui, "page-range", || RangeDemo {
        field: PageRangeFieldState::default(),
        selected: vec![0, 1, 2, 4],
        range: PageRange { all: false, from: 3, to: 9, subset: Subset::Odd },
        last: None,
    });
    let mut demo = lock(&demo);
    let demo = &mut *demo;
    let menu: Vec<MenuEntry<Vec<usize>>> = vec![
        MenuItem::new(labels.all_pages.clone(), (0..RANGE_PAGES).collect()).into(),
        MenuItem::new(labels.odd_only.clone(), (0..RANGE_PAGES).step_by(2).collect()).into(),
        MenuItem::new(labels.even_only.clone(), (1..RANGE_PAGES).step_by(2).collect()).into(),
    ];
    let out = PageRangeField::new("field", &mut demo.field, &demo.selected, RANGE_PAGES, 0).menu(&menu).show(ui);
    if let Some(picked) = out.picked {
        demo.last = Some(format!("{picked:?}"));
        demo.selected = picked;
    }
    let before = demo.range;
    PageRangeOptions::new("options", &mut demo.range, RANGE_PAGES).show(ui);
    if demo.range != before {
        demo.last = Some(format!("{:?}", demo.range));
    }
    last_event(ui, demo.last.as_deref());
}

// --- Page list ---

struct ListDemo {
    rows: Vec<PageListRow>,
    picked: Selection<usize>,
    active: usize,
    last: Option<String>,
}

// new: six artboards, the second active, the fourth locked, two picked.
fn page_list(ui: &mut Ui, _: &mut NoArgs) {
    let labels = PagesStoryLabels::of(ui);
    let demo = shared(ui, "page-list", || {
        let names = [
            &labels.artboard_cover,
            &labels.artboard_welcome,
            &labels.artboard_features,
            &labels.artboard_pricing,
            &labels.artboard_contact,
            &labels.artboard_credits,
        ];
        let rows = names.iter().enumerate().map(|(i, name)| PageListRow { name: (*name).clone(), locked: i == 3 }).collect();
        ListDemo { rows, picked: Selection { items: vec![1, 4], lead: Some(1), anchor: Some(1) }, active: 1, last: None }
    });
    let mut demo = lock(&demo);
    let demo = &mut *demo;
    let out = in_area(ui, AREA_ASPECT, |ui| PageList::new("artboards", &demo.rows, &mut demo.picked, demo.active).show(ui));
    for event in out.events {
        demo.last = Some(format!("{event:?}"));
        match event {
            PageListEvent::Go(row) => demo.active = row,
            PageListEvent::Rename { row, text } => {
                if let Some(r) = demo.rows.get_mut(row) {
                    r.name = text;
                }
            }
            PageListEvent::Lock(edit) => {
                if let EditEvent::Commit { value, .. } | EditEvent::Set { value, .. } = edit
                    && let Some(r) = demo.rows.get_mut(value.row)
                {
                    r.locked = value.on;
                }
            }
            _ => {}
        }
    }
    last_event(ui, demo.last.as_deref());
}

// --- Column bar ---

struct BarDemo {
    widths: Vec<f64>,
    last: Option<String>,
}

// new: four columns at 40/20/20/20.
fn column_bar(ui: &mut Ui, _: &mut NoArgs) {
    let demo = shared(ui, "column-bar", || BarDemo { widths: vec![40.0, 20.0, 20.0, 20.0], last: None });
    let mut demo = lock(&demo);
    let demo = &mut *demo;
    let out = ColumnBar::new("bar", &demo.widths).show(ui);
    if let Some(edit) = out.edit {
        demo.last = Some(format!("{edit:?}"));
        if let EditEvent::Commit { value, .. } | EditEvent::Set { value, .. } = edit {
            demo.widths = value;
        }
    }
    last_event(ui, demo.last.as_deref());
}

// --- Edge picker ---

struct EdgesDemo {
    cell: BorderEdges,
    paragraph: BorderEdges,
    last: Option<String>,
}

// new: outer edges on, inner off; the paragraph form beside it.
fn edge_picker(ui: &mut Ui, _: &mut NoArgs) {
    let demo = shared(ui, "edge-picker", || {
        let mut outer = BorderEdges::default();
        for edge in [EdgeKey::Top, EdgeKey::Bottom, EdgeKey::Left, EdgeKey::Right] {
            outer.set(edge, true);
        }
        EdgesDemo { cell: outer, paragraph: outer, last: None }
    });
    let mut demo = lock(&demo);
    let demo = &mut *demo;
    let (cell, paragraph) = ui
        .horizontal(|ui| {
            let cell = EdgePicker::new("cell", &mut demo.cell).show(ui);
            let paragraph = EdgePicker::new("paragraph", &mut demo.paragraph).paragraph(true).show(ui);
            (cell.changed, paragraph.changed)
        })
        .inner;
    if cell {
        demo.last = Some(format!("{:?}", demo.cell));
    }
    if paragraph {
        demo.last = Some(format!("{:?}", demo.paragraph));
    }
    last_event(ui, demo.last.as_deref());
}

// --- Table size ---

// new: the 8 × 10 grid with 3 × 4 hovered.
fn table_size(ui: &mut Ui, _: &mut NoArgs) {
    let out = TableSizePicker::new("size").show(ui);
    let demo = shared(ui, "table-size", || None::<String>);
    let mut demo = lock(&demo);
    if let Some((rows, columns)) = out.picked {
        *demo = Some(format!("Picked({rows}, {columns})"));
    }
    last_event(ui, demo.as_deref());
}

// --- Table properties ---

struct TableDemo {
    values: TableValues,
    last: Option<String>,
}

fn apply_table(values: &mut TableValues, change: &TableChange) {
    match change {
        TableChange::Rows(n) => values.rows = *n,
        TableChange::Columns(n) => values.columns = *n,
        TableChange::RowHeightMode(mode) => values.row_height_mode = Some(*mode),
        TableChange::RowHeight(h) => values.row_height = Some(*h),
        TableChange::ColumnWidth(w) => values.column_width = Some(*w),
        TableChange::CellAlign(align) => values.cell_align = Some(*align),
        TableChange::Inset(i, v) => {
            if let Some(slot) = values.insets.get_mut(*i) {
                *slot = Some(*v);
            }
        }
        TableChange::Direction(direction) => values.direction = *direction,
        TableChange::AlternatingFills(on) => values.alternating_fills = *on,
        TableChange::RepeatHeader(on) => values.repeat_header = *on,
        TableChange::HeaderRows(n) => values.header_rows = *n,
        TableChange::FooterRows(n) => values.footer_rows = *n,
        TableChange::InsertRow => values.rows += 1,
        TableChange::DeleteRow => values.rows = values.rows.saturating_sub(1).max(1),
        TableChange::InsertColumn => values.columns += 1,
        TableChange::DeleteColumn => values.columns = values.columns.saturating_sub(1).max(1),
        TableChange::Merge => values.merged = true,
        TableChange::Unmerge => values.merged = false,
        TableChange::DistributeRows => values.row_height_mode = Some(RowHeight::Exactly),
    }
}

// new: a 4 × 3 table, two cells selected with mixed row heights.
fn table_properties(ui: &mut Ui, _: &mut NoArgs) {
    let demo = shared(ui, "table-properties", || TableDemo {
        values: TableValues {
            rows: 4,
            columns: 3,
            row_height_mode: Some(RowHeight::AtLeast),
            row_height: None,
            column_width: Some(120.0),
            cell_align: None,
            insets: [Some(3.6); 4],
            several_cells: true,
            ..TableValues::default()
        },
        last: None,
    });
    let mut demo = lock(&demo);
    let demo = &mut *demo;
    let out = TableProperties::new("table", &demo.values, LengthUnit::Points).show(ui);
    for edit in out.edits {
        demo.last = Some(format!("{edit:?}"));
        if let EditEvent::Commit { value, .. } | EditEvent::Set { value, .. } = &edit {
            apply_table(&mut demo.values, value);
        }
    }
    last_event(ui, demo.last.as_deref());
}

// --- Page preview ---

struct PreviewDemo {
    picked: Selection<usize>,
    at: usize,
}

// new: page 3 of 8 in the preview over a dimmed story area.
fn page_preview(ui: &mut Ui, args: &mut DialogArgs) {
    const PREVIEW_PAGES: usize = 8;
    let demo = shared(ui, "page-preview", || PreviewDemo { picked: Selection::new(), at: 2 });
    let mut demo = lock(&demo);
    let demo = &mut *demo;
    let pages = Pages::new(PREVIEW_PAGES);
    in_area(ui, AREA_ASPECT, |ui| {
        let _ = PageThumbnails::new("behind", &pages, &mut demo.picked, demo.at).show(ui);
    });
    if dialog_shown(ui, args.open) {
        let shown: Vec<usize> = (0..PREVIEW_PAGES).collect();
        let out = PagePreview::new("preview", &pages, &shown, &mut demo.at).show(ui.ctx());
        if out.closed {
            dialog_closed(ui, &mut args.open);
        }
    }
}

#[cfg(test)]
mod tests {
    use craft_ui::story::Args;
    use craft_ui::theme::{Base, install};
    use egui_kittest::Harness;

    // every story draws its widget over its sample data without panicking.
    #[test]
    fn every_story_draws() {
        for story in super::stories() {
            let mut harness = Harness::<Args>::builder()
                .with_size([720.0, 520.0])
                .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
                .build_ui_state(move |ui, args| (story.show)(ui, args), story.defaults());
            harness.run_steps(4);
        }
    }
}
