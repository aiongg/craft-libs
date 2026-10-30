//! [`DialogFrame`]: the look every dialog shares, without behaviour.

use craft_theme::{ControlSize, Stroke as Hairline, TypeRole};
use egui::accesskit::Role;
use egui::{Color32, Frame, IdSalt, Label, Margin, Rect, Resize, Response, Sense, Stroke, Ui, UiBuilder, Vec2, WidgetInfo, vec2};

use crate::behavior::FieldTab;
use crate::controls::IconButton;
use crate::direction;
use crate::icons::Icon;
use crate::primitives::{Content, Divider, Surface, SurfaceKind, SurfaceStyle};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens, role_format};

/// What a dialog frame draws and measures, from [`DialogFrame::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct DialogFrameStyle {
    /// The surface: [`SurfaceKind::Dialog`], the popup family's look.
    pub surface: SurfaceStyle,
    /// Frame inner margin: `dialog.pad`.
    pub pad: f32,
    /// Frame inner margin of a roomy dialog: `dialog.pad_roomy`.
    pub pad_roomy: f32,
    /// Between the title and the rule: `dialog.title_gap`.
    pub title_gap: f32,
    /// Between the rule and the body: `dialog.body_gap`.
    pub body_gap: f32,
    /// Between the body and the footer: `dialog.footer_gap`.
    pub footer_gap: f32,
    /// Side of the close box: `dialog.close_box`.
    pub close_box: f32,
    /// How far the title band reaches past the title, as `[side, up, down]`: `dialog.drag_pad`.
    pub drag_pad: [f32; 3],
    /// The title's type role: `title`.
    pub title_role: TypeRole,
    /// The title's line height, from the role.
    pub title_line: f32,
    /// The title: `text.normal`.
    pub title: Color32,
    /// The rule under the title: `line.divider`, one physical pixel.
    pub rule: Stroke,
}

/// What [`DialogFrame::show`] measured and sensed.
#[derive(Debug)]
#[non_exhaustive]
pub struct DialogFrameResponse {
    /// The frame, with its margin and edge.
    pub rect: Rect,
    /// The title row's height: the least height of the part of the frame that stays on screen.
    pub title_height: f32,
    /// The title band, which senses drags (not focus): the dialog moves from it.
    pub drag: Response,
    /// The close box.
    pub close: Option<Response>,
    /// The size the content area was given at the start of the pass (a resizable dialog's current size).
    pub content_size: Vec2,
    /// Height of the frame outside the body: margin, title row, rules, gaps and footer.
    pub chrome: f32,
}

/// The frame of a dialog: the raised popup-look surface with its shadow, a title row (the title,
/// a draggable band and a close box), a hairline, the body and an optional footer area. It holds
/// no state and takes no keys; [`Dialog`](super::Dialog) adds placement, size and outcomes.
///
/// - The title is one line, cut with an ellipsis. The close box is an [`IconButton`] named by
///   `close_label` at the end of the title row; the band behind the title senses drags and is not
///   a Tab stop.
/// - [`width`](DialogFrame::width) sets the content's least and greatest width (the title rule
///   and the footer fill it); [`resize`](DialogFrame::resize) wraps the content in an egui
///   [`Resize`] with no stroke, so the contents fill the area the grip gives.
/// - The body and footer sit in a [`FieldTab`] scope when asked for, so Tab visits fields only.
/// - **AccessKit:** the frame is a `Dialog` named by the title; the title is a level 1 heading.
#[must_use = "show the frame with `show`"]
pub struct DialogFrame {
    id_salt: IdSalt,
    title: String,
    close_label: Option<String>,
    roomy: bool,
    width: Option<(f32, f32)>,
    resize: Option<Resize>,
    field_tab: bool,
}

impl DialogFrame {
    /// A frame titled `title`. `id_salt` keeps the band's and close box's state when the title is
    /// translated.
    pub fn new(id_salt: impl egui::AsIdSalt, title: impl Into<String>) -> Self {
        Self { id_salt: IdSalt::new(id_salt), title: title.into(), close_label: None, roomy: false, width: None, resize: None, field_tab: false }
    }

    /// Shows a close box with this accessible name.
    pub fn close_label(mut self, label: impl Into<String>) -> Self {
        self.close_label = Some(label.into());
        self
    }

    /// The larger inner margin, `dialog.pad_roomy` (the colour picker dialog).
    pub fn roomy(mut self, roomy: bool) -> Self {
        self.roomy = roomy;
        self
    }

    /// The content's least and greatest width, in points.
    pub fn width(mut self, min: f32, max: f32) -> Self {
        self.width = Some((min, max.max(min)));
        self
    }

    /// Wraps the content in `resize`; its grip is drawn at the content's corner.
    pub fn resize(mut self, resize: Resize) -> Self {
        self.resize = Some(resize);
        self
    }

    /// Whether the body and footer sit in a [`FieldTab`] scope.
    pub fn field_tab(mut self, field_tab: bool) -> Self {
        self.field_tab = field_tab;
        self
    }

    pub fn style(tokens: &UiTokens, pixels_per_point: f32) -> DialogFrameStyle {
        let d = &tokens.component.dialog;
        DialogFrameStyle {
            surface: Surface::style(tokens, SurfaceKind::Dialog),
            pad: d.pad,
            pad_roomy: d.pad_roomy,
            title_gap: d.title_gap,
            body_gap: d.body_gap,
            footer_gap: d.footer_gap,
            close_box: d.close_box,
            drag_pad: d.drag_pad,
            title_role: TypeRole::Title,
            title_line: tokens.typography.get(TypeRole::Title).line_height,
            title: Content::Normal.color(tokens).to_egui(),
            rule: Stroke::new(Hairline::Hairline.width(pixels_per_point), tokens.color.line.divider.to_egui()),
        }
    }

    /// Shows the frame with `body` under the title rule.
    pub fn show(self, ui: &mut Ui, body: impl FnOnce(&mut Ui)) -> DialogFrameResponse {
        self.show_inner(ui, body, None::<fn(&mut Ui)>)
    }

    /// Shows the frame with `body` and, `dialog.footer_gap` under it, `footer`.
    pub fn show_footer(self, ui: &mut Ui, body: impl FnOnce(&mut Ui), footer: impl FnOnce(&mut Ui)) -> DialogFrameResponse {
        self.show_inner(ui, body, Some(footer))
    }

    fn show_inner(self, ui: &mut Ui, body: impl FnOnce(&mut Ui), footer: Option<impl FnOnce(&mut Ui)>) -> DialogFrameResponse {
        let tokens = Tokens::of(ui);
        let style = DialogFrame::style(&tokens, ui.pixels_per_point());
        let id = ui.make_persistent_id(self.id_salt);
        let pad = if self.roomy { style.pad_roomy } else { style.pad };
        let Self { title, close_label, width, resize, field_tab, .. } = self;

        let mut parts: Option<(Response, Option<Response>, f32, Vec2, f32)> = None;
        let framed = Surface::new(SurfaceKind::Dialog).show(ui, |ui| {
            Frame::NONE.inner_margin(Margin::from(pad)).show(ui, |ui| {
                let content = |ui: &mut Ui| {
                    if let Some((min, max)) = width {
                        ui.set_min_width(min);
                        ui.set_max_width(max);
                    }
                    let content_size = ui.available_size();
                    let top = ui.min_rect().top();
                    let (drag, close, title_height) = title_row(ui, &style, id, &title, close_label.as_deref());
                    ui.add_space(style.title_gap);
                    Divider::horizontal().show(ui);
                    ui.add_space(style.body_gap);
                    let mut body_height = 0.0;
                    let inside = |ui: &mut Ui| {
                        let before = ui.min_rect().bottom();
                        body(ui);
                        body_height = ui.min_rect().bottom() - before;
                        if let Some(footer) = footer {
                            ui.add_space(style.footer_gap);
                            footer(ui);
                        }
                    };
                    if field_tab {
                        FieldTab::new(id.with("fields")).show(ui, inside);
                    } else {
                        inside(ui);
                    }
                    let used = ui.min_rect().bottom() - top;
                    parts = Some((drag, close, title_height, content_size, used - body_height));
                };
                match resize {
                    Some(resize) => resize.show(ui, content),
                    None => content(ui),
                }
            });
        });
        let rect = framed.response.rect;
        let (drag, close, title_height, content_size, inner_chrome) =
            parts.unwrap_or_else(|| (ui.interact(rect, id.with("title"), Sense::DRAG), None, style.title_line, Vec2::ZERO, 0.0));
        DialogFrameResponse { rect, title_height, drag, close, content_size, chrome: inner_chrome + 2.0 * pad }
    }
}

/// The title, its draggable band and the close box. The band is made before the close box so the
/// box lies over it.
fn title_row(ui: &mut Ui, style: &DialogFrameStyle, id: egui::Id, title: &str, close_label: Option<&str>) -> (Response, Option<Response>, f32) {
    let tokens = Tokens::of(ui);
    let row_height = if close_label.is_some() { style.close_box.max(style.title_line) } else { style.title_line };
    let (_, row) = ui.allocate_space(vec2(ui.available_width(), row_height));
    let forward = direction::forward(ui);
    let (start, end) = (direction::start_x(ui, row), direction::end_x(ui, row));
    let near = if close_label.is_some() { end - forward * style.close_box } else { end };
    let [side, up, down] = style.drag_pad;
    let span = |a: f32, b: f32| a.min(b)..=a.max(b);
    let band = if close_label.is_some() {
        Rect::from_x_y_ranges(span(start - forward * side, near), (row.top() - up)..=(row.bottom() + down))
    } else {
        Rect::from_x_y_ranges(span(start - forward * side, end + forward * side), (row.top() - up)..=(row.bottom() + down))
    };
    let drag = ui.interact(band, id.with("title"), Sense::DRAG);

    let text = Rect::from_x_y_ranges(span(start, near), row.y_range());
    if text.width() >= 1.0 {
        let format = role_format(&tokens, style.title_role, style.title);
        let mut title_ui = ui.new_child(UiBuilder::new().max_rect(text).layout(direction::row(ui)));
        let label = title_ui.add(Label::new(egui::text::LayoutJob::single_section(title.to_owned(), format)).truncate().selectable(false));
        label.widget_info(|| WidgetInfo::labeled(Role::Heading, true, title));
        ui.ctx().accesskit_node_builder(label.id, |node| node.set_level(1));
    }
    let close = close_label.map(|label| {
        let square = Rect::from_x_y_ranges(span(near, end), (row.center().y - style.close_box / 2.0)..=(row.center().y + style.close_box / 2.0));
        IconButton::new(Icon::Close, label).size(ControlSize::Sm).id_salt((id, "close")).show_at(ui, square)
    });
    (drag, close, row_height)
}
