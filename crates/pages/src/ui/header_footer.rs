//! [`HeaderFooterZones`] (N3.10): the header and footer bands of the pages on screen, each in three
//! sections (left, centre, right), as Excel's Page Layout view shows them: the hovered section
//! says "Add header" / "Add footer" when empty, a click starts editing it, and the app's editor
//! takes the section's rectangle. What each section holds, its fields and the editor are the
//! app's.
//!
//! ```ignore
//! let r = HeaderFooterZones::new("hf", &view.pages, &bands).text(&|page, zone| doc.hf_text(page, zone)).editing(edit.map(|e| e.at)).show(ui);
//! if let Some(at) = r.begin { edit = Some(ZoneEditor::new(at, doc.hf_source(at))); }
//! if let Some(editor) = &mut edit { editor.show(ui, r.rect(at)); }   // commit on a click elsewhere
//! ```

use craft_ui::primitives::Interactive;
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _};
use egui::accesskit::Role;
use egui::{Align2, Color32, FontId, Rect, Response, Sense, Shape, Ui, pos2};
use kurbo::Point;

use super::style::HeaderFooterStyle;
use super::view::PlacedPage;

/// The page the zones sit on and the text printed on it: document colours, not interface ones.
const PAPER: Color32 = Color32::WHITE;
const INK: Color32 = Color32::BLACK;

/// The dash and gap of the hint outline, screen points.
const DASH: f32 = 3.0;
const GAP: f32 = 3.0;

/// The header and footer text size, page points (GridCraft's `hf_font` base).
pub const HEADER_TEXT: f32 = 10.0;

/// The screen sizes header and footer text is kept between at extreme zooms (GridCraft clamps
/// 4–120): limits of document text drawn at a zoom, not interface sizes.
pub const HEADER_TEXT_SCREEN: std::ops::RangeInclusive<f32> = 4.0..=120.0;

craft_ui::words! {
    /// The zones' words (GridCraft's strings).
    #[non_exhaustive]
    pub struct HeaderFooterLabels {
        add_header: "pages.addHeader" = "Add header";
        add_footer: "pages.addFooter" = "Add footer";
        header: "pages.header" = "Header";
        footer: "pages.footer" = "Footer";
        first_page: "pages.firstPageHeader" = "First Page Header";
        even_page: "pages.evenPageHeader" = "Even Page Header";
        odd_page: "pages.oddPageHeader" = "Odd Page Header";
    }
}

/// One of the six sections of a page's header and footer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Zone {
    pub footer: bool,
    /// 0 left, 1 centre, 2 right (in reading order: mirrored for a right-to-left page).
    pub section: u8,
}

/// A section of one page.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ZoneAt {
    pub page: usize,
    pub zone: Zone,
}

/// Which header and footer a page uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HeaderFooterPage {
    First,
    Odd,
    Even,
}

impl HeaderFooterPage {
    /// The kind for 1-based page `number` with Different First Page and Different Odd & Even.
    // gridcraft: crates/model/src/features/print.rs:86 `hf_page`; crates/ui-egui/src/paper_view.rs:354
    // `page_kind`.
    pub fn of(number: usize, different_first: bool, different_odd_even: bool) -> Self {
        if different_first && number == 1 {
            HeaderFooterPage::First
        } else if different_odd_even && number.is_multiple_of(2) {
            HeaderFooterPage::Even
        } else {
            HeaderFooterPage::Odd
        }
    }
}

/// Where the bands are on each page, page points: the header band from the top edge down to the
/// top margin and the footer band from the bottom margin to the bottom edge, inset left and right
/// by `sides` (GridCraft's `hf_sides`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ZoneBands {
    pub top: f64,
    pub bottom: f64,
    pub sides: [f64; 2],
}

/// What the zones did this pass.
#[derive(Debug)]
pub struct HeaderFooterResponse {
    /// A click on a section: start (or move) editing there.
    pub begin: Option<ZoneAt>,
    /// A click on a page outside the zones while one was being edited: commit it.
    pub end: bool,
    pub response: Response,
}

/// The zones over the pages on screen.
///
/// - Sections split the band's inner width in thirds, inset top and bottom by
///   [`HeaderFooterStyle::inset`](super::HeaderFooterStyle::inset).
/// - A hovered empty section shows the style's dashed hint outline and "Add header" / "Add
///   footer"; the section being edited ([`editing`](Self::editing)) is filled with the page
///   colour with the editing outline, its neighbours outlined (Excel).
/// - Text the app gives is drawn left, centred and right aligned in its section, at the
///   document's header size ([`HEADER_TEXT`] page points) × the page scale, between
///   [`HEADER_TEXT_SCREEN`] screen points (GridCraft's `hf_font`; document text, so not a type
///   role).
/// - Keys: each section is a Tab stop (an AccessKit Button "Add header" or its text) after the
///   page view; Enter or Space starts editing it ([`HeaderFooterResponse`]). Excel reaches the
///   zones only through Insert › Header & Footer; the suite's rule is every control from the
///   keyboard.
pub struct HeaderFooterZones<'a> {
    id_salt: egui::IdSalt,
    placed: &'a [PlacedPage],
    bands: &'a dyn Fn(usize) -> ZoneBands,
    text: Option<&'a dyn Fn(ZoneAt) -> String>,
    editing: Option<ZoneAt>,
    labels: Option<HeaderFooterLabels>,
}

impl<'a> HeaderFooterZones<'a> {
    /// The zones' sizes and strokes, from the tokens.
    pub fn style(tokens: &craft_ui::theme::UiTokens) -> HeaderFooterStyle {
        use craft_ui::theme::ToEgui as _;
        let t = &tokens.component.pages;
        let c = &tokens.color;
        HeaderFooterStyle {
            inset: t.header_footer_inset,
            hint: egui::Stroke::new(t.header_footer_hint_outline, c.line.control_edge.to_egui()),
            editing: egui::Stroke::new(t.header_footer_edit_outline, c.accent.solid.to_egui()),
            neighbour: egui::Stroke::new(t.header_footer_edit_outline, c.line.control_edge.to_egui()),
            paper: t.paper.to_egui(),
            ink: t.ink.to_egui(),
        }
    }

    pub fn new(id_salt: impl egui::AsIdSalt, placed: &'a [PlacedPage], bands: &'a dyn Fn(usize) -> ZoneBands) -> Self {
        HeaderFooterZones { id_salt: egui::IdSalt::new(id_salt), placed, bands, text: None, editing: None, labels: None }
    }

    /// Each section's text as it prints (fields expanded by the app).
    pub fn text(mut self, text: &'a dyn Fn(ZoneAt) -> String) -> Self {
        self.text = Some(text);
        self
    }

    pub fn editing(mut self, at: Option<ZoneAt>) -> Self {
        self.editing = at;
        self
    }

    pub fn labels(mut self, labels: HeaderFooterLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The section's rectangle on screen, for the app's editor; `None` when its page isn't
    /// placed.
    // gridcraft: crates/ui-egui/src/paper_view.rs:194 `section_rects`.
    pub fn rect(&self, at: ZoneAt) -> Option<Rect> {
        let placed = self.placed.iter().find(|p| p.page == at.page)?;
        Some(self.section_rect(placed, at.zone))
    }

    /// The whole section on `placed`, without the style's inset.
    fn section_rect(&self, placed: &PlacedPage, zone: Zone) -> Rect {
        let bands = (self.bands)(placed.page);
        let width = f64::from(placed.rect.width()) / placed.scale;
        let height = f64::from(placed.rect.height()) / placed.scale;
        let (top, bottom) = if zone.footer { (height - bands.bottom, height) } else { (0.0, bands.top) };
        let third = (width - bands.sides[0] - bands.sides[1]).max(0.0) / 3.0;
        let left = bands.sides[0] + third * f64::from(zone.section);
        Rect::from_min_max(placed.to_screen(Point::new(left, top)), placed.to_screen(Point::new(left + third, bottom)))
    }

    // gridcraft: crates/ui-egui/src/paper_view.rs:217 `paint_headers_footers`, :291 `interact`.
    pub fn show(self, ui: &mut Ui) -> HeaderFooterResponse {
        let tokens = Tokens::of(ui);
        let style = Self::style(&tokens);
        let labels = self.labels.clone().unwrap_or_default();
        let faint = tokens.color.text.faint.to_egui();
        let visible = ui.clip_rect();
        let mut begin = None;
        let mut zones = Vec::new();
        for placed in self.placed.iter().filter(|p| p.rect.intersects(visible)) {
            for footer in [false, true] {
                for section in 0..3 {
                    let at = ZoneAt { page: placed.page, zone: Zone { footer, section } };
                    let whole = self.section_rect(placed, at.zone);
                    zones.push(whole);
                    let rect = whole.shrink2(egui::vec2(0.0, style.inset));
                    let text = self.text.map(|text| text(at)).unwrap_or_default();
                    let editing = self.editing == Some(at);
                    let band_editing = self.editing.is_some_and(|e| e.page == at.page && e.zone.footer == footer);
                    if editing {
                        ui.painter().rect_filled(rect, 0.0, PAPER);
                        ui.painter().rect_stroke(rect, 0.0, style.editing, egui::StrokeKind::Inside);
                        continue;
                    }
                    let add = if footer { &labels.add_footer } else { &labels.add_header };
                    let name = if text.is_empty() { add.as_str() } else { text.as_str() };
                    let id = ui.make_persistent_id((self.id_salt, at.page, footer, section));
                    let response = Interactive::with_id(id, Role::Button, name)
                        .sense(Sense::click())
                        .state_layer(false)
                        .focus_ring(false)
                        .interact_at(ui, rect)
                        .finish(ui);
                    if response.clicked() {
                        begin = Some(at);
                    }
                    let font =
                        FontId::proportional((HEADER_TEXT * placed.scale as f32).clamp(*HEADER_TEXT_SCREEN.start(), *HEADER_TEXT_SCREEN.end()));
                    let painter = ui.painter();
                    if band_editing {
                        painter.rect_stroke(rect, 0.0, style.neighbour, egui::StrokeKind::Inside);
                    }
                    if text.is_empty() {
                        if response.hovered() || response.has_focus() {
                            let corners = [rect.left_top(), rect.right_top(), rect.right_bottom(), rect.left_bottom(), rect.left_top()];
                            painter.extend(Shape::dashed_line(&corners, style.hint, DASH, GAP));
                            painter.text(rect.center(), Align2::CENTER_CENTER, add, font, faint);
                        }
                        continue;
                    }
                    let galley = painter.layout(text, font, INK, rect.width());
                    let width = galley.size().x;
                    let x = match section {
                        0 => rect.left(),
                        1 => rect.center().x - width / 2.0,
                        _ => rect.right() - width,
                    };
                    let y = if footer { rect.bottom() - galley.size().y } else { rect.top() };
                    painter.galley(pos2(x, y), galley, INK);
                }
            }
        }
        let end = self.editing.is_some()
            && ui.input(|i| i.pointer.primary_clicked())
            && ui
                .input(|i| i.pointer.interact_pos())
                .is_some_and(|p| self.placed.iter().any(|page| page.rect.contains(p)) && !zones.iter().any(|z| z.contains(p)));
        let extent =
            self.placed.iter().map(|p| p.rect).reduce(Rect::union).unwrap_or_else(|| Rect::from_min_size(ui.min_rect().min, egui::Vec2::ZERO));
        let response = ui.interact(extent, ui.make_persistent_id((self.id_salt, "zones")), Sense::hover());
        HeaderFooterResponse { begin, end, response }
    }
}
