//! The page widgets of craft-pages: the page view, thumbnails, margin handles, header and footer zones, text selection and find marks, the column bar, the reading view, frame marks, the border edge picker, the table size picker and the selected table cells. Leaves are named by part. No theme overrides them.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::color::Rgba;
use crate::values;

/// Pages tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PagesTokens {
    /// Between pages, and between rows: [`values::PAGES_GAP`].
    pub gap: f32,
    /// Above the first and below the last row: [`values::PAGES_MARGIN`].
    pub margin: f32,
    /// Kept clear at each side of the pages: [`values::PAGES_SIDE`].
    pub side: f32,
    /// What Hide White Space leaves above and below each page's text area: [`values::PAGES_WHITE_SPACE_PAD`].
    pub white_space_pad: f32,
    /// The seam Hide White Space draws between pages: [`values::PAGES_SEAM`].
    pub seam: f32,
    /// The room the page preview keeps around the page: [`values::PAGES_PREVIEW_ROOM`].
    pub preview_room: f32,
    /// The thumbnail width at zoom 1: [`values::PAGES_THUMBNAIL_USUAL_WIDTH`].
    pub thumbnail_usual_width: f32,
    /// What the thumbnails panel keeps free beside the thumbnails: [`values::PAGES_THUMBNAIL_PANEL_INSET`].
    pub thumbnail_panel_inset: f32,
    /// How far either side of a margin edge grabs it: [`values::PAGES_MARGIN_HANDLE_GRAB`].
    pub margin_handle_grab: f32,
    /// A header or footer section's inset from its band, top and bottom: [`values::PAGES_HEADER_FOOTER_INSET`].
    pub header_footer_inset: f32,
    /// The dashed outline of a hovered empty section: [`values::PAGES_HEADER_FOOTER_HINT_OUTLINE`].
    pub header_footer_hint_outline: f32,
    /// The edited section's outline and its neighbours': [`values::PAGES_HEADER_FOOTER_EDIT_OUTLINE`].
    pub header_footer_edit_outline: f32,
    /// Corner radius of the selection rects: [`values::PAGES_TEXT_SELECTION_RADIUS`].
    pub text_selection_radius: f32,
    /// The column rectangle's outline while dragging: [`values::PAGES_TEXT_SELECTION_COLUMN_OUTLINE`].
    pub text_selection_column_outline: f32,
    /// Corner radius of the current match: [`values::PAGES_FIND_CURRENT_RADIUS`].
    pub find_current_radius: f32,
    /// How much the current match grows past the others: [`values::PAGES_FIND_CURRENT_GROW`].
    pub find_current_grow: f32,
    /// The column bar's outline: [`values::PAGES_COLUMN_BAR_OUTLINE`].
    pub column_bar_outline: f32,
    /// A column divider at rest: [`values::PAGES_COLUMN_BAR_DIVIDER`].
    pub column_bar_divider: f32,
    /// A column divider hovered, dragged or focused: [`values::PAGES_COLUMN_BAR_DIVIDER_ACTIVE`].
    pub column_bar_divider_active: f32,
    /// How far either side of a column divider grabs it: [`values::PAGES_COLUMN_BAR_GRAB`].
    pub column_bar_grab: f32,
    /// The narrowest column that paints its percentage: [`values::PAGES_COLUMN_BAR_LABEL_MIN`].
    pub column_bar_label_min: f32,
    /// Between the reading view's columns: [`values::PAGES_READING_COLUMN_GAP`].
    pub reading_column_gap: f32,
    /// Room under the reading columns for the page-turn bar: [`values::PAGES_READING_NAV_BAR`].
    pub reading_nav_bar: f32,
    /// One reading column per this much width: [`values::PAGES_READING_COLUMN_TARGET`].
    pub reading_column_target: f32,
    /// The least reading column width: [`values::PAGES_READING_COLUMN_MIN`].
    pub reading_column_min: f32,
    /// A frame port's side: [`values::PAGES_FRAME_MARKS_PORT`].
    pub frame_marks_port: f32,
    /// A frame port's distance from the frame's corner: [`values::PAGES_FRAME_MARKS_PORT_INSET`].
    pub frame_marks_port_inset: f32,
    /// The overset mark's arm length: [`values::PAGES_FRAME_MARKS_OVERSET_ARM`].
    pub frame_marks_overset_arm: f32,
    /// The overset mark's stroke: [`values::PAGES_FRAME_MARKS_OVERSET_STROKE`].
    pub frame_marks_overset_stroke: f32,
    /// The least hidden-character mark size: [`values::PAGES_FRAME_MARKS_MARK_MIN`].
    pub frame_marks_mark_min: f32,
    /// The largest hidden-character mark size: [`values::PAGES_FRAME_MARKS_MARK_MAX`].
    pub frame_marks_mark_max: f32,
    /// The border edge picker's square side: [`values::PAGES_EDGE_PICKER_SIDE`].
    pub edge_picker_side: f32,
    /// The edge picker's lines' inset from the square: [`values::PAGES_EDGE_PICKER_INSET`].
    pub edge_picker_inset: f32,
    /// A chosen edge picker line: [`values::PAGES_EDGE_PICKER_LINE_ON`].
    pub edge_picker_line_on: f32,
    /// An edge picker line not chosen: [`values::PAGES_EDGE_PICKER_LINE_OFF`].
    pub edge_picker_line_off: f32,
    /// The edge picker line under the pointer: [`values::PAGES_EDGE_PICKER_LINE_HOVER`].
    pub edge_picker_line_hover: f32,
    /// A table size picker square's side: [`values::PAGES_TABLE_SIZE_CELL`].
    pub table_size_cell: f32,
    /// The space between table size picker squares: [`values::PAGES_TABLE_SIZE_GAP`].
    pub table_size_gap: f32,
    /// The selected table cells' range outline: [`values::PAGES_TABLE_SELECTION_OUTLINE`].
    pub table_selection_outline: f32,
    /// The text selection tint: `pages.text_selection_fill`, `accent.solid` at alpha [`values::PAGES_TEXT_SELECTION_FILL_ALPHA`].
    pub text_selection_fill: Rgba,
    /// A find match's tint: `pages.find_match`, `data.yellow` at alpha [`values::PAGES_FIND_MATCHED_ALPHA`].
    pub find_match: Rgba,
    /// The current find match's tint: `pages.find_current`, `status.warn.solid` at alpha [`values::PAGES_FIND_CURRENT_ALPHA`].
    pub find_current: Rgba,
    /// An even column's fill in the column bar: `pages.column_bar_fill`, `accent.solid` at alpha [`values::PAGES_COLUMN_BAR_FILL_ALPHA`].
    pub column_bar_fill: Rgba,
    /// An odd column's fill in the column bar: `pages.column_bar_fill_alt`, `accent.solid` at alpha [`values::PAGES_COLUMN_BAR_FILL_ALT_ALPHA`].
    pub column_bar_fill_alt: Rgba,
    /// The overset mark's colour: `pages.frame_marks_overset`, `status.bad.solid`.
    pub frame_marks_overset: Rgba,
    /// The edge picker's field inside the inset: `pages.edge_picker_fill`, `surface.sunken`.
    pub edge_picker_fill: Rgba,
    /// The colour of an edge picker line not chosen: `pages.edge_picker_line_off_color`, `text.faint`.
    pub edge_picker_line_off_color: Rgba,
    /// The selected table cells' tint: `pages.table_selection_fill`, `accent.solid` at alpha [`values::PAGES_TABLE_SELECTION_FILL_ALPHA`].
    pub table_selection_fill: Rgba,
    /// The paper of a page, the same in every theme: `pages.paper`, white.
    pub paper: Rgba,
    /// The ink of text on a page's paper, the same in every theme: `pages.ink`, black.
    pub ink: Rgba,
}

/// Pages tokens have no overridable field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PagesField {}

impl Family for PagesTokens {
    type Field = PagesField;
    const FIELDS: &'static [PagesField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { PagesTokens;
        "gap" 0.0..=256.0 => gap;
        "margin" 0.0..=256.0 => margin;
        "side" 0.0..=512.0 => side;
        "white_space_pad" 0.0..=64.0 => white_space_pad;
        "seam" 0.0..=16.0 => seam;
        "preview_room" 0.0..=256.0 => preview_room;
        "thumbnail_usual_width" 16.0..=1024.0 => thumbnail_usual_width;
        "thumbnail_panel_inset" 0.0..=256.0 => thumbnail_panel_inset;
        "margin_handle_grab" 0.0..=32.0 => margin_handle_grab;
        "header_footer_inset" 0.0..=32.0 => header_footer_inset;
        "header_footer_hint_outline" 0.0..=8.0 => header_footer_hint_outline;
        "header_footer_edit_outline" 0.0..=8.0 => header_footer_edit_outline;
        "text_selection_radius" 0.0..=16.0 => text_selection_radius;
        "text_selection_column_outline" 0.0..=8.0 => text_selection_column_outline;
        "find_current_radius" 0.0..=16.0 => find_current_radius;
        "find_current_grow" 0.0..=16.0 => find_current_grow;
        "column_bar_outline" 0.0..=8.0 => column_bar_outline;
        "column_bar_divider" 0.0..=8.0 => column_bar_divider;
        "column_bar_divider_active" 0.0..=16.0 => column_bar_divider_active;
        "column_bar_grab" 0.0..=32.0 => column_bar_grab;
        "column_bar_label_min" 0.0..=256.0 => column_bar_label_min;
        "reading_column_gap" 0.0..=128.0 => reading_column_gap;
        "reading_nav_bar" 0.0..=256.0 => reading_nav_bar;
        "reading_column_target" 64.0..=2048.0 => reading_column_target;
        "reading_column_min" 32.0..=2048.0 => reading_column_min;
        "frame_marks_port" 2.0..=64.0 => frame_marks_port;
        "frame_marks_port_inset" 0.0..=64.0 => frame_marks_port_inset;
        "frame_marks_overset_arm" 1.0..=64.0 => frame_marks_overset_arm;
        "frame_marks_overset_stroke" 0.5..=8.0 => frame_marks_overset_stroke;
        "frame_marks_mark_min" 1.0..=64.0 => frame_marks_mark_min;
        "frame_marks_mark_max" 4.0..=256.0 => frame_marks_mark_max;
        "edge_picker_side" 16.0..=256.0 => edge_picker_side;
        "edge_picker_inset" 0.0..=64.0 => edge_picker_inset;
        "edge_picker_line_on" 0.0..=16.0 => edge_picker_line_on;
        "edge_picker_line_off" 0.0..=16.0 => edge_picker_line_off;
        "edge_picker_line_hover" 0.0..=16.0 => edge_picker_line_hover;
        "table_size_cell" 4.0..=64.0 => table_size_cell;
        "table_size_gap" 0.0..=16.0 => table_size_gap;
        "table_selection_outline" 0.0..=8.0 => table_selection_outline;
    };

    fn defaults(s: &Semantic<'_>) -> PagesTokens {
        let c = s.color;
        PagesTokens {
            gap: values::PAGES_GAP,
            margin: values::PAGES_MARGIN,
            side: values::PAGES_SIDE,
            white_space_pad: values::PAGES_WHITE_SPACE_PAD,
            seam: values::PAGES_SEAM,
            preview_room: values::PAGES_PREVIEW_ROOM,
            thumbnail_usual_width: values::PAGES_THUMBNAIL_USUAL_WIDTH,
            thumbnail_panel_inset: values::PAGES_THUMBNAIL_PANEL_INSET,
            margin_handle_grab: values::PAGES_MARGIN_HANDLE_GRAB,
            header_footer_inset: values::PAGES_HEADER_FOOTER_INSET,
            header_footer_hint_outline: values::PAGES_HEADER_FOOTER_HINT_OUTLINE,
            header_footer_edit_outline: values::PAGES_HEADER_FOOTER_EDIT_OUTLINE,
            text_selection_radius: values::PAGES_TEXT_SELECTION_RADIUS,
            text_selection_column_outline: values::PAGES_TEXT_SELECTION_COLUMN_OUTLINE,
            find_current_radius: values::PAGES_FIND_CURRENT_RADIUS,
            find_current_grow: values::PAGES_FIND_CURRENT_GROW,
            column_bar_outline: values::PAGES_COLUMN_BAR_OUTLINE,
            column_bar_divider: values::PAGES_COLUMN_BAR_DIVIDER,
            column_bar_divider_active: values::PAGES_COLUMN_BAR_DIVIDER_ACTIVE,
            column_bar_grab: values::PAGES_COLUMN_BAR_GRAB,
            column_bar_label_min: values::PAGES_COLUMN_BAR_LABEL_MIN,
            reading_column_gap: values::PAGES_READING_COLUMN_GAP,
            reading_nav_bar: values::PAGES_READING_NAV_BAR,
            reading_column_target: values::PAGES_READING_COLUMN_TARGET,
            reading_column_min: values::PAGES_READING_COLUMN_MIN,
            frame_marks_port: values::PAGES_FRAME_MARKS_PORT,
            frame_marks_port_inset: values::PAGES_FRAME_MARKS_PORT_INSET,
            frame_marks_overset_arm: values::PAGES_FRAME_MARKS_OVERSET_ARM,
            frame_marks_overset_stroke: values::PAGES_FRAME_MARKS_OVERSET_STROKE,
            frame_marks_mark_min: values::PAGES_FRAME_MARKS_MARK_MIN,
            frame_marks_mark_max: values::PAGES_FRAME_MARKS_MARK_MAX,
            edge_picker_side: values::PAGES_EDGE_PICKER_SIDE,
            edge_picker_inset: values::PAGES_EDGE_PICKER_INSET,
            edge_picker_line_on: values::PAGES_EDGE_PICKER_LINE_ON,
            edge_picker_line_off: values::PAGES_EDGE_PICKER_LINE_OFF,
            edge_picker_line_hover: values::PAGES_EDGE_PICKER_LINE_HOVER,
            table_size_cell: values::PAGES_TABLE_SIZE_CELL,
            table_size_gap: values::PAGES_TABLE_SIZE_GAP,
            table_selection_outline: values::PAGES_TABLE_SELECTION_OUTLINE,
            text_selection_fill: c.accent.solid.with_alpha(values::PAGES_TEXT_SELECTION_FILL_ALPHA),
            find_match: c.data.yellow.with_alpha(values::PAGES_FIND_MATCHED_ALPHA),
            find_current: c.status.warn.solid.with_alpha(values::PAGES_FIND_CURRENT_ALPHA),
            column_bar_fill: c.accent.solid.with_alpha(values::PAGES_COLUMN_BAR_FILL_ALPHA),
            column_bar_fill_alt: c.accent.solid.with_alpha(values::PAGES_COLUMN_BAR_FILL_ALT_ALPHA),
            frame_marks_overset: c.status.bad.solid,
            edge_picker_fill: c.surface.sunken,
            edge_picker_line_off_color: c.text.faint,
            table_selection_fill: c.accent.solid.with_alpha(values::PAGES_TABLE_SELECTION_FILL_ALPHA),
            paper: Rgba::WHITE,
            ink: Rgba::BLACK,
        }
    }

    fn path(f: PagesField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<PagesField> {
        None
    }

    fn kind(f: PagesField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: PagesField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: PagesField, _: OverrideValue) {
        match f {}
    }

    // Marks and tints drawn over surfaces other families check.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
