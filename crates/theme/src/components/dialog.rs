//! Dialogs: the frame every dialog shares. Sizes only (no colours): the look comes from the popup
//! family and no theme overrides these.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::tokens::TypeRole;
use crate::values;

/// Dialog tokens, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct DialogTokens {
    /// Frame inner margin: `dialog.pad`.
    pub pad: f32,
    /// Frame inner margin of the colour picker dialog, from `space.pad.dialog`: `dialog.pad_roomy`.
    pub pad_roomy: f32,
    /// Type role of the title: `dialog.title_role`.
    pub title_role: TypeRole,
    /// Between the title and the rule: `dialog.title_gap`.
    pub title_gap: f32,
    /// Between the rule and the body: `dialog.body_gap`.
    pub body_gap: f32,
    /// Between the body and the footer: `dialog.footer_gap`.
    pub footer_gap: f32,
    /// Between footer buttons: `dialog.button_gap`.
    pub button_gap: f32,
    /// Least width of a footer button: `dialog.button_min`.
    pub button_min: f32,
    /// Between the buttons of a picker's side column: `dialog.side_gap`.
    pub side_gap: f32,
    /// Least width of a side-column button: `dialog.side_min`.
    pub side_min: f32,
    /// Side of the close box (the small control height): `dialog.close_box`.
    pub close_box: f32,
    /// How far the title band reaches past the title, as `[side, up, down]`: `dialog.drag_pad`.
    pub drag_pad: [f32; 3],
    /// Greatest content width of a Compact dialog: `dialog.width_compact`.
    pub width_compact: f32,
    /// Least content width of a Standard or Wide dialog: `dialog.width_min`.
    pub width_min: f32,
    /// Greatest content width of a Standard dialog: `dialog.width_standard`.
    pub width_standard: f32,
    /// Greatest content width of a Wide dialog: `dialog.width_wide`.
    pub width_wide: f32,
    /// Content width of an alert or a prompt: `dialog.width_alert`.
    pub width_alert: f32,
    /// Default size of a secondary viewport's window: `dialog.viewport_size`.
    pub viewport_size: [f32; 2],
    /// A preview pane's least width and its height: `dialog.preview_pane`. The pane takes the
    /// dialog's width, and this least width when the dialog is narrower.
    pub preview_pane: [f32; 2],
    /// Least size of a secondary viewport's window: `dialog.viewport_min`.
    pub viewport_min: [f32; 2],
    /// Space kept between a dialog and the window edge on each side: `dialog.window_margin`.
    pub window_margin: f32,
    /// The same for a resizable dialog's greatest size: `dialog.resize_margin`.
    pub resize_margin: f32,
    /// Width of a dragged dialog that must stay on screen: `dialog.keep_visible`.
    pub keep_visible: f32,
    /// Least height of a scrolling body: `dialog.scroll_min`.
    pub scroll_min: f32,
    /// Side of one cell of Canvas Size's anchor grid: `dialog.anchor_cell`.
    pub anchor_cell: f32,
}

/// Dialogs have no overridable field: their sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DialogField {}

impl Family for DialogTokens {
    type Field = DialogField;
    const FIELDS: &'static [DialogField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { DialogTokens;
        "pad" 0.0..=64.0 => pad;
        "pad_roomy" 0.0..=32.0 => pad_roomy;
        "title_gap" 0.0..=64.0 => title_gap;
        "body_gap" 0.0..=64.0 => body_gap;
        "footer_gap" 0.0..=64.0 => footer_gap;
        "button_gap" 0.0..=64.0 => button_gap;
        "button_min" 0.0..=1024.0 => button_min;
        "side_gap" 0.0..=64.0 => side_gap;
        "side_min" 0.0..=1024.0 => side_min;
        "close_box" 0.0..=1024.0 => close_box;
        "drag_pad.0" 0.0..=64.0 => drag_pad[0];
        "drag_pad.1" 0.0..=64.0 => drag_pad[1];
        "drag_pad.2" 0.0..=64.0 => drag_pad[2];
        "width_compact" 0.0..=1024.0 => width_compact;
        "width_min" 0.0..=1024.0 => width_min;
        "width_standard" 0.0..=1024.0 => width_standard;
        "width_wide" 0.0..=2048.0 => width_wide;
        "window_margin" 0.0..=64.0 => window_margin;
        "resize_margin" 0.0..=64.0 => resize_margin;
        "keep_visible" 0.0..=1024.0 => keep_visible;
        "scroll_min" 0.0..=1024.0 => scroll_min;
        "anchor_cell" 0.0..=256.0 => anchor_cell;
        "width_alert" 0.0..=1024.0 => width_alert;
        "viewport_size.0" 0.0..=4096.0 => viewport_size[0];
        "viewport_size.1" 0.0..=4096.0 => viewport_size[1];
        "preview_pane.0" 0.0..=4096.0 => preview_pane[0];
        "preview_pane.1" 0.0..=4096.0 => preview_pane[1];
        "viewport_min.0" 0.0..=4096.0 => viewport_min[0];
        "viewport_min.1" 0.0..=4096.0 => viewport_min[1];
    };

    /// The Compact values (PhotoCraft's). Only the vertical gaps follow the control height of the
    /// size, rounded to whole points; widths, margins and the footer's horizontal values are the
    /// same in every size, as PhotoCraft's are in Pro and Studio.
    fn defaults(s: &Semantic<'_>) -> DialogTokens {
        let k = s.size.control.md / values::DIALOG_BASE_CONTROL;
        let scale = |w: f32| (w * k).round();
        DialogTokens {
            pad: values::DIALOG_PAD,
            pad_roomy: s.space.pad.dialog,
            title_role: TypeRole::Heading,
            title_gap: scale(values::DIALOG_TITLE_GAP),
            body_gap: scale(values::DIALOG_BODY_GAP),
            footer_gap: scale(values::DIALOG_FOOTER_GAP),
            button_gap: values::DIALOG_BUTTON_GAP,
            button_min: values::DIALOG_BUTTON_MIN,
            side_gap: values::DIALOG_SIDE_GAP,
            side_min: values::DIALOG_SIDE_MIN,
            close_box: s.size.control.sm,
            drag_pad: values::DIALOG_DRAG_PAD,
            width_compact: values::DIALOG_WIDTH_COMPACT,
            width_min: values::DIALOG_WIDTH_MIN,
            width_standard: values::DIALOG_WIDTH_STANDARD,
            width_wide: values::DIALOG_WIDTH_WIDE,
            width_alert: values::DIALOG_WIDTH_ALERT,
            viewport_size: values::DIALOG_VIEWPORT_SIZE,
            preview_pane: values::DIALOG_PREVIEW_PANE,
            viewport_min: values::DIALOG_VIEWPORT_MIN,
            window_margin: values::DIALOG_WINDOW_MARGIN,
            resize_margin: values::DIALOG_RESIZE_MARGIN,
            keep_visible: values::DIALOG_KEEP_VISIBLE,
            scroll_min: values::DIALOG_SCROLL_MIN,
            anchor_cell: values::DIALOG_ANCHOR_CELL,
        }
    }

    fn path(f: DialogField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<DialogField> {
        None
    }

    fn kind(f: DialogField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: DialogField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: DialogField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
