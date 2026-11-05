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
use egui::{Align2, AsIdSalt, Color32, FontId, IdSalt, Key, Modifiers, PointerButton, Pos2, Sense, Shape, Stroke, StrokeKind, Ui, WidgetInfo};
use kurbo::{Point, Rect};

use super::CanvasId;
use super::view::CanvasView;
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};
use craft_theme::TypeRole;

/// The pasteboard's colour (Preferences › Interface › Standard Screen Mode › Color).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PasteboardColor {
    /// The theme's canvas colour (`surface.canvas`).
    #[default]
    Default,
    #[allow(missing_docs)]
    Black,
    /// Grey 40.
    DarkGray,
    /// Grey 83.
    MediumGray,
    /// Grey 184.
    LightGray,
    #[allow(missing_docs)]
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
    #[allow(missing_docs)]
    DropShadow,
    #[allow(missing_docs)]
    Line,
    #[allow(missing_docs)]
    None,
}

/// One page or artboard on the pasteboard, in document units.
#[derive(Clone, Debug, PartialEq)]
pub struct PageFrame {
    /// The page's rectangle in document space.
    pub rect: Rect,
    /// Drawn above the page's top-left corner (artboards, spreads); `None` draws nothing.
    pub name: Option<String>,
    /// The page's own fill under the document (an artboard's background), or `None` for none.
    pub fill: Option<Color32>,
    /// An artboard's edge: `Some(true)` for the active board (a 1 pt `accent.solid` edge, its name
    /// in `text.normal`), `Some(false)` for another board (a 1 pt `overlay.artboard` edge outside,
    /// name in `text.muted`); `None` for a plain page (only the border preference draws).
    pub active: Option<bool>,
}

impl PageFrame {
    /// A page at `rect` with no name, fill or active mark.
    pub fn new(rect: Rect) -> Self {
        Self { rect, name: None, fill: None, active: None }
    }

    /// Marks the frame as an artboard, the active one when `active` (PhotoCraft highlights the
    /// active board).
    pub fn active(mut self, active: bool) -> Self {
        self.active = Some(active);
        self
    }

    /// Sets the name drawn above the page's top-left corner.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the page's own fill under the document.
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
    #[allow(missing_docs)]
    pub name_color: Color32,
    /// Space between a page's top edge and its name: `pasteboard.name_gap`.
    pub name_gap: f32,
}

/// What the pasteboard sensed this pass, from the host's canvas response.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PasteboardResponse {
    /// A page name was clicked (the app selects the artboard or page).
    pub name_clicked: Option<usize>,
    /// The secondary button was pressed on the pasteboard outside every page, at this document
    /// point, or the Menu key or Shift+F10 was pressed on the focused [`canvas`](Pasteboard::canvas)
    /// (then the point is the view's centre): the app opens Photoshop's pasteboard colour menu
    /// there.
    pub context_at: Option<Point>,
}

/// The pasteboard and page frames under a document. Draw it first, then the document, then the
/// overlays.
pub struct Pasteboard<'a> {
    id_salt: IdSalt,
    pages: &'a [PageFrame],
    color: PasteboardColor,
    border: PageBorder,
    canvas: Option<CanvasId>,
}

impl<'a> Pasteboard<'a> {
    /// A pasteboard around `pages`. A single-image document passes one page: its bounds.
    pub fn new(id_salt: impl AsIdSalt, pages: &'a [PageFrame]) -> Self {
        Self { id_salt: IdSalt::new(id_salt), pages, color: PasteboardColor::Default, border: PageBorder::DropShadow, canvas: None }
    }

    /// Sets the pasteboard colour.
    pub fn color(mut self, color: PasteboardColor) -> Self {
        self.color = color;
        self
    }

    /// The canvas whose focus lets the Menu key and Shift+F10 open the colour menu, as a secondary
    /// press does.
    pub fn canvas(mut self, canvas: CanvasId) -> Self {
        self.canvas = Some(canvas);
        self
    }

    /// Sets how the page edge is drawn.
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
            outline: Stroke::new(tokens.component.pasteboard.outline_width, colors.line.border.to_egui()),
            name_gap: tokens.component.pasteboard.name_gap,
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
            let name_color = match page.active {
                Some(active) => artboard_edge(&painter, &quad, bounds, aligned, active, ui),
                None => style.name_color,
            };
            let Some(name) = &page.name else { continue };
            let corner = view.to_screen(Point::new(page.rect.x0.min(page.rect.x1), page.rect.y0.min(page.rect.y1)));
            let at = Pos2::new(corner.x, corner.y - style.name_gap);
            let galley = painter.layout_no_wrap(name.clone(), style.name_font.clone(), name_color);
            let rect = Align2::LEFT_BOTTOM.anchor_size(at, galley.size());
            painter.galley(rect.min, galley, name_color);
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
        let keyboard = self.canvas.is_some_and(|canvas| ui.memory(|m| m.has_focus(canvas.id())))
            && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Menu) || i.consume_key(Modifiers::SHIFT, Key::F10));
        if keyboard {
            response.context_at = Some(view.to_doc(view.rect.center()));
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

/// Draws an artboard's edge round `quad` (`bounds` when the view is axis-aligned): `pasteboard.outline_width`
/// (1) outside, in `accent.solid` when `active`, else `overlay.artboard` (black at alpha 90); and
/// returns its name's colour, `text.normal` when active, else `text.muted`.
fn artboard_edge(painter: &egui::Painter, quad: &[Pos2; 4], bounds: egui::Rect, aligned: bool, active: bool, ui: &Ui) -> Color32 {
    // pc: crates/ui-egui/src/canvas/artboard_ui.rs:98-101 `draw_frames` (accent for the active board,
    // black at alpha 90 outside otherwise; the name in `text` or `text_dim`).
    let tokens = Tokens::of(ui);
    let (edge, name) =
        if active { (tokens.color.accent.solid, tokens.color.text.normal) } else { (tokens.color.overlay.artboard, tokens.color.text.muted) };
    let stroke = Stroke::new(tokens.component.pasteboard.outline_width, edge.to_egui());
    if aligned {
        painter.rect_stroke(bounds, 0.0, stroke, StrokeKind::Outside);
    } else {
        painter.add(Shape::closed_line(quad.to_vec(), stroke));
    }
    name.to_egui()
}

/// A convex quad as a mesh, so a turned view paints the parallelogram and not its bounding box.
fn filled_quad(quad: [Pos2; 4], color: Color32) -> Shape {
    let mut mesh = Mesh::default();
    mesh.vertices.extend(quad.map(|pos| Vertex { pos, uv: WHITE_UV, color }));
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    Shape::mesh(mesh)
}
