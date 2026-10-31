//! [`Pasteboard`]: the area around the document's pages or artboards, and the frames of the pages
//! on it. Photoshop's Preferences › Interface › Standard Screen Mode sets its colour and the page
//! border (Drop Shadow, Line, None); artboards show their names above them. WordCraft's and
//! GridCraft's page frames and craft-pages' spreads draw through it.
//!
//! ```no_run
//! use craft_ui::canvas::{CanvasView, PageFrame, Pasteboard, PasteboardColor, kurbo::Rect};
//! # fn demo(ui: &mut egui::Ui, view: &CanvasView) {
//! let pages = [PageFrame::new(Rect::new(0.0, 0.0, 612.0, 792.0)).name("Page 1")];
//! let response = Pasteboard::new("doc-1", &pages).color(PasteboardColor::Default).show(ui, view);
//! # let _ = response;
//! # }
//! ```

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{AsIdSalt, Color32, FontId, IdSalt, Pos2, Stroke, Ui, epaint::Shadow};
use kurbo::{Point, Rect};

use super::view::CanvasView;
use crate::theme::UiTokens;

/// The pasteboard's colour (Preferences › Interface › Standard Screen Mode › Color).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PasteboardColor {
    /// The theme's canvas colour (`surface.canvas`).
    #[default]
    Default,
    Black,
    /// Grey 40.
    DarkGray,
    /// Grey 83.
    MediumGray,
    /// Grey 184.
    LightGray,
    Custom(Color32),
}

impl PasteboardColor {
    /// The fill, or `None` for the theme's.
    // pc: crates/ui-egui/src/prefs/prefs_ui/appearance.rs:21 `pasteboard_color`.
    pub fn fill(self) -> Option<Color32> {
        match self {
            PasteboardColor::Default => None,
            PasteboardColor::Black => Some(Color32::BLACK),
            PasteboardColor::DarkGray => Some(Color32::from_gray(40)),
            PasteboardColor::MediumGray => Some(Color32::from_gray(83)),
            PasteboardColor::LightGray => Some(Color32::from_gray(184)),
            PasteboardColor::Custom(c) => Some(c),
        }
    }
}

/// The border around each page (Preferences › Interface › Border).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PageBorder {
    #[default]
    DropShadow,
    Line,
    None,
}

/// One page or artboard on the pasteboard, in document units.
#[derive(Clone, Debug, PartialEq)]
pub struct PageFrame {
    pub rect: Rect,
    /// Drawn above the page's top-left corner (artboards, spreads); `None` draws nothing.
    pub name: Option<String>,
    /// The page's own fill under the document (an artboard's background), or `None` for none.
    pub fill: Option<Color32>,
}

impl PageFrame {
    pub fn new(rect: Rect) -> Self {
        Self { rect, name: None, fill: None }
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn fill(mut self, fill: Color32) -> Self {
        self.fill = Some(fill);
        self
    }
}

/// The pasteboard's look, from [`Pasteboard::style`].
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct PasteboardStyle {
    /// `surface.canvas`, unless the preference sets a colour.
    pub fill: Color32,
    /// The page's drop shadow (Border: Drop Shadow).
    pub shadow: Shadow,
    /// The page's outline (Border: Line).
    pub outline: Stroke,
    /// Page names: the UI face at the caption role, in the muted text colour.
    pub name_font: FontId,
    pub name_color: Color32,
}

/// What the pasteboard sensed this pass, from the host's canvas response.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PasteboardResponse {
    /// A page name was clicked (the app selects the artboard or page).
    pub name_clicked: Option<usize>,
    /// The secondary button was pressed on the pasteboard outside every page, at this document
    /// point: the app opens Photoshop's pasteboard colour menu there.
    pub context_at: Option<Point>,
}

/// The pasteboard and page frames under a document. Draw it first, then the document, then the
/// overlays.
pub struct Pasteboard<'a> {
    id_salt: IdSalt,
    pages: &'a [PageFrame],
    color: PasteboardColor,
    border: PageBorder,
}

impl<'a> Pasteboard<'a> {
    /// A pasteboard around `pages`. A single-image document passes one page: its bounds.
    pub fn new(id_salt: impl AsIdSalt, pages: &'a [PageFrame]) -> Self {
        Self { id_salt: IdSalt::new(id_salt), pages, color: PasteboardColor::Default, border: PageBorder::DropShadow }
    }

    pub fn color(mut self, color: PasteboardColor) -> Self {
        self.color = color;
        self
    }

    pub fn border(mut self, border: PageBorder) -> Self {
        self.border = border;
        self
    }

    /// The look from the theme's tokens with the preference's colour.
    pub fn style(tokens: &UiTokens, color: PasteboardColor) -> PasteboardStyle {
        todo!()
    }

    /// Fill the canvas outside the pages, draw each page's fill, border and name through `view`,
    /// and sense name clicks and the pasteboard's secondary press. Nothing mirrors in a
    /// right-to-left interface.
    // pc: crates/ui-egui/src/canvas.rs:490 (artboards: pasteboard, outlines, names), :869 pasteboard menu.
    pub fn show(self, ui: &mut Ui, view: &CanvasView) -> PasteboardResponse {
        todo!()
    }

    /// The pasteboard as screen quads: the canvas minus the pages, each piece as its four
    /// corners (a turned view needs quads, not rects). For an app that draws the pasteboard
    /// itself (a GPU path).
    // pc: crates/ui-egui/src/artboard_ui.rs:21 `pasteboard_quads`.
    pub fn quads(view: &CanvasView, pages: &[PageFrame]) -> Vec<[Pos2; 4]> {
        todo!()
    }

    /// The page under the document point `at`, if any.
    pub fn page_at(pages: &[PageFrame], at: Point) -> Option<usize> {
        todo!()
    }
}
