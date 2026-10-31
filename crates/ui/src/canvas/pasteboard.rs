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

use egui::accesskit::Role;
use egui::epaint::{Mesh, Shadow, Vertex, WHITE_UV};
use egui::{Align2, AsIdSalt, Color32, FontId, IdSalt, PointerButton, Pos2, Sense, Shape, Stroke, StrokeKind, Ui, WidgetInfo};
use kurbo::{Point, Rect};

use super::view::CanvasView;
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};
use craft_theme::TypeRole;

/// Points between a page's top edge and its name.
const NAME_GAP: f32 = 4.0;
/// The page outline's width (Border: Line).
const OUTLINE_WIDTH: f32 = 1.0;

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
        let colors = &tokens.color;
        PasteboardStyle {
            fill: color.fill().unwrap_or_else(|| colors.surface.canvas.to_egui()),
            shadow: tokens.shadow.floating,
            outline: Stroke::new(OUTLINE_WIDTH, colors.line.border.to_egui()),
            name_font: tokens.font(TypeRole::Caption),
            name_color: colors.text.muted.to_egui(),
        }
    }

    /// Fill the canvas outside the pages, draw each page's fill, border and name through `view`,
    /// and sense name clicks and the pasteboard's secondary press. Nothing mirrors in a
    /// right-to-left interface.
    // pc: crates/ui-egui/src/canvas.rs:490 (artboards: pasteboard, outlines, names), :869 pasteboard menu.
    pub fn show(self, ui: &mut Ui, view: &CanvasView) -> PasteboardResponse {
        let style = Self::style(&Tokens::of(ui), self.color);
        let painter = ui.painter_at(view.rect);
        for quad in Self::quads(view, self.pages) {
            if egui::Rect::from_points(&quad).intersects(view.rect) {
                painter.add(filled_quad(quad, style.fill));
            }
        }
        let aligned = view.axis_aligned();
        let mut response = PasteboardResponse::default();
        for (i, page) in self.pages.iter().enumerate() {
            let quad = view.quad(page.rect);
            let bounds = egui::Rect::from_points(&quad);
            if !bounds.intersects(view.rect) && page.name.is_none() {
                continue;
            }
            if self.border == PageBorder::DropShadow {
                if aligned {
                    painter.add(style.shadow.as_shape(bounds, 0.0));
                } else {
                    let offset = egui::vec2(f32::from(style.shadow.offset[0]), f32::from(style.shadow.offset[1]));
                    painter.add(filled_quad(quad.map(|p| p + offset), style.shadow.color));
                }
            }
            if let Some(fill) = page.fill {
                painter.add(filled_quad(quad, fill));
            }
            if self.border == PageBorder::Line {
                if aligned {
                    painter.rect_stroke(bounds, 0.0, style.outline, StrokeKind::Outside);
                } else {
                    painter.add(Shape::closed_line(quad.to_vec(), style.outline));
                }
            }
            let Some(name) = &page.name else { continue };
            let corner = view.to_screen(Point::new(page.rect.x0.min(page.rect.x1), page.rect.y0.min(page.rect.y1)));
            let at = Pos2::new(corner.x, corner.y - NAME_GAP);
            let galley = painter.layout_no_wrap(name.clone(), style.name_font.clone(), style.name_color);
            let rect = Align2::LEFT_BOTTOM.anchor_size(at, galley.size());
            painter.galley(rect.min, galley, style.name_color);
            if rect.intersects(view.rect) {
                let id = ui.make_persistent_id(self.id_salt).with(("name", i));
                let sensed = ui.interact(rect, id, Sense::click());
                sensed.widget_info(|| WidgetInfo::labeled(Role::Button, ui.is_enabled(), name));
                if sensed.clicked() {
                    response.name_clicked = Some(i);
                }
            }
        }
        let pressed = ui.rect_contains_pointer(view.rect) && ui.input(|i| i.pointer.button_pressed(PointerButton::Secondary));
        if let Some(pos) = ui.input(|i| i.pointer.interact_pos()).filter(|_| pressed) {
            let at = view.to_doc(pos);
            if Self::page_at(self.pages, at).is_none() {
                response.context_at = Some(at);
            }
        }
        response
    }

    /// The pasteboard as screen quads: the canvas minus the pages, each piece as its four
    /// corners (a turned view needs quads, not rects). For an app that draws the pasteboard
    /// itself (a GPU path).
    // pc: crates/ui-egui/src/artboard_ui.rs:21 `pasteboard_quads`.
    pub fn quads(view: &CanvasView, pages: &[PageFrame]) -> Vec<[Pos2; 4]> {
        let canvas = view.visible();
        if !(canvas.x1 > canvas.x0 && canvas.y1 > canvas.y0 && canvas.area().is_finite()) {
            return Vec::new();
        }
        let boards: Vec<Rect> = pages.iter().filter_map(|p| clip(p.rect.abs(), canvas)).collect();
        let mut xs = vec![canvas.x0, canvas.x1];
        for b in &boards {
            xs.extend([b.x0, b.x1]);
        }
        xs.sort_by(f64::total_cmp);
        xs.dedup();
        let mut out = Vec::new();
        for w in xs.windows(2) {
            let (x0, x1) = (w[0], w[1]);
            let mut spans: Vec<(f64, f64)> = boards.iter().filter(|b| b.x0 <= x0 && b.x1 >= x1).map(|b| (b.y0, b.y1)).collect();
            spans.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut y = canvas.y0;
            for (a, b) in spans {
                if a > y {
                    out.push(view.quad(Rect::new(x0, y, x1, a)));
                }
                y = y.max(b);
            }
            if y < canvas.y1 {
                out.push(view.quad(Rect::new(x0, y, x1, canvas.y1)));
            }
        }
        out
    }

    /// The page under the document point `at`, if any.
    pub fn page_at(pages: &[PageFrame], at: Point) -> Option<usize> {
        pages.iter().rposition(|p| p.rect.abs().contains(at))
    }
}

/// `r` clipped to `canvas`, or `None` when they share no area.
fn clip(r: Rect, canvas: Rect) -> Option<Rect> {
    let c = Rect::new(r.x0.max(canvas.x0), r.y0.max(canvas.y0), r.x1.min(canvas.x1), r.y1.min(canvas.y1));
    (c.x1 > c.x0 && c.y1 > c.y0).then_some(c)
}

/// A convex quad as a mesh, so a turned view paints the parallelogram and not its bounding box.
fn filled_quad(quad: [Pos2; 4], color: Color32) -> Shape {
    let mut mesh = Mesh::default();
    mesh.vertices.extend(quad.map(|pos| Vertex { pos, uv: WHITE_UV, color }));
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    Shape::mesh(mesh)
}
