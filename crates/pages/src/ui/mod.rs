//! The page widgets, on craft-ui (feature `ui`). Every one reads the document through a small
//! trait or plain values the app passes, and reports what the user did; none owns pages.
//!
//! - [`view`] (N3.06): [`PageView`] over the app's [`PageSource`]; overlays draw on its
//!   [`PlacedPage`]s.
//! - [`thumbnails`] (N3.01): [`PageThumbnails`], the Pages panel and slide strip.
//! - [`access`] (N3.05): [`PagesAccess`], the AccessKit description of the text on the pages.
//! - [`setup`] (N3.08): [`PageSetupDialog`] and [`PageLayoutMenus`].
//! - [`margins`] (N3.11): [`MarginHandles`] on the rulers.
//! - [`header_footer`] (N3.10): [`HeaderFooterZones`].
//! - [`range`] (N3.13): [`PageRangeField`] and [`PageRangeOptions`].
//! - [`text_selection`] (N3.14): [`TextSelectTool`], a canvas tool over the app's [`PageTexts`].
//! - [`preview`] (N3.16): [`PagePreview`].
//! - [`marks`] (N3.17): [`TextFrameMarks`], hidden characters and frame extras.
//! - [`column_bar`] (N3.19): [`ColumnBar`].
//! - [`reading`] (N3.20): [`ReadingView`].
//! - [`list`] (N3.21): [`PageList`], pages and artboards.
//! - [`sync_views`] (N3.22): [`SyncedViews`].
//! - [`grid`] (N3.26): [`PageGrids`], margins, columns and baseline grid.
//! - [`table`] (N3.07, N3.12, N3.24, N3.25): [`EdgePicker`], [`TableProperties`],
//!   [`TableSizePicker`], [`CellSelectionOverlay`].
//! - [`style`]: every widget's sizes and colours from the tokens.

pub mod access;
#[cfg(test)]
mod access_tests;
pub mod column_bar;
#[cfg(test)]
mod column_bar_tests;
pub mod grid;
#[cfg(test)]
mod grid_tests;
pub mod header_footer;
#[cfg(test)]
mod header_footer_tests;
pub mod list;
#[cfg(test)]
mod list_tests;
pub mod margins;
#[cfg(test)]
mod margins_tests;
pub mod marks;
#[cfg(test)]
mod marks_tests;
pub mod preview;
#[cfg(test)]
mod preview_tests;
pub mod range;
#[cfg(test)]
mod range_tests;
pub mod reading;
#[cfg(test)]
mod reading_tests;
pub mod setup;
#[cfg(test)]
mod setup_tests;
mod stories;
pub mod style;
pub mod sync_views;
#[cfg(test)]
mod sync_views_tests;
pub mod table;
pub mod text_selection;
#[cfg(test)]
mod text_selection_tests;
pub mod thumbnails;
#[cfg(test)]
mod thumbnails_tests;
pub mod view;
#[cfg(test)]
mod view_tests;

pub use access::{AccessBuilder, AccessDescriber, AccessNode, AccessPosition, BlockRole, PagesAccess};
pub use column_bar::{ColumnBar, ColumnBarLabels, ColumnBarResponse};
pub use grid::{PageGrid, PageGridColors, PageGrids};
pub use header_footer::{HeaderFooterLabels, HeaderFooterPage, HeaderFooterResponse, HeaderFooterZones, Zone, ZoneAt, ZoneBands};
pub use list::{PageList, PageListEvent, PageListLabels, PageListOutput, PageListRow};
pub use margins::{MarginChange, MarginEdge, MarginHandles, MarginLabels, MarginResponse, MarginRules};
pub use marks::{FrameExtras, HiddenMark, PlacedMark, TextFrameMarks};
pub use preview::{PagePreview, PagePreviewLabels, PagePreviewResponse};
pub use range::{PageRangeField, PageRangeFieldResponse, PageRangeFieldState, PageRangeLabels, PageRangeOptions};
pub use reading::{ReadingLabels, ReadingState, ReadingView, ReadingViewResponse};
pub use setup::{PageLayoutChoice, PageLayoutMenus, PageSetupDialog, PageSetupLabels};
pub use stories::stories;
pub use style::{
    CellSelectionStyle, ColumnBarStyle, EdgePickerStyle, FrameMarksStyle, HeaderFooterStyle, MarginHandlesStyle, PagePreviewStyle,
    PageThumbnailsStyle, PageViewStyle, ReadingStyle, TableSizeStyle, TextSelectStyle,
};
pub use sync_views::{Lead, SyncedViews, SyncedViewsState};
pub use table::{
    BorderEdges, BorderPreset, CellAlign, CellSelectionOverlay, EdgeKey, EdgePicker, EdgePickerLabels, EdgePickerResponse, RowHeight, TableChange,
    TableGeometry, TableProperties, TablePropertiesLabels, TablePropertiesOutput, TableSizePicker, TableSizePickerLabels, TableSizeResponse,
    TableValues, TextDirection,
};
pub use text_selection::{PageTextSource, PageTexts, TextMark, TextSelectState, TextSelectTool, TextSelection, TextSpan, paint_text_marks};
pub use thumbnails::{PAGES, PageThumbnails, PageThumbnailsOutput, ThumbnailEvent, ThumbnailLabels, ThumbnailMarks, ThumbnailSection, moved_page};
pub use view::{PageSource, PageView, PageViewResponse, PageViewState, PlacedPage};

/// The count words of the page widgets, for the facade's catalog extraction.
pub const COUNTS: &[craft_ui::localized::words::Counted] = &[PAGES];

/// Every labels table of the page widgets, for the facade's word list.
pub const WORDS: &[&[craft_ui::localized::Word]] = &[
    <ThumbnailLabels as craft_ui::localized::Labels>::WORDS,
    <HeaderFooterLabels as craft_ui::localized::Labels>::WORDS,
    <MarginLabels as craft_ui::localized::Labels>::WORDS,
    <PageSetupLabels as craft_ui::localized::Labels>::WORDS,
    <PageRangeLabels as craft_ui::localized::Labels>::WORDS,
    <PagePreviewLabels as craft_ui::localized::Labels>::WORDS,
    <PageListLabels as craft_ui::localized::Labels>::WORDS,
    <ReadingLabels as craft_ui::localized::Labels>::WORDS,
    <ColumnBarLabels as craft_ui::localized::Labels>::WORDS,
    <EdgePickerLabels as craft_ui::localized::Labels>::WORDS,
    <TablePropertiesLabels as craft_ui::localized::Labels>::WORDS,
    <TableSizePickerLabels as craft_ui::localized::Labels>::WORDS,
    <stories::PagesStoryLabels as craft_ui::localized::Labels>::WORDS,
];
