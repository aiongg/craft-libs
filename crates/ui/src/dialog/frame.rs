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

const TITLE_ROLE: TypeRole = TypeRole::Heading;

/// The parts of a shown frame: band, close box, title height, content size, chrome and first field.
type Parts = (Response, Option<Response>, f32, Vec2, f32, Option<egui::Id>);

/// What a dialog frame draws and measures, from [`DialogFrame::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct DialogFrameStyle {
    /// The surface: [`SurfaceKind::Dialog`], the popup family's look.
    pub surface: SurfaceStyle,
    /// Frame inner margin.
    pub pad: f32,
    /// Frame inner margin of a roomy dialog.
    pub pad_roomy: f32,
    /// Between the title and the rule.
    pub title_gap: f32,
    /// Between the rule and the body.
    pub body_gap: f32,
    /// Between the body and the footer.
    pub footer_gap: f32,
    /// Side of the close box.
    pub close_box: f32,
    /// How far the title band reaches past the title, as `[side, up, down]`.
    pub drag_pad: [f32; 3],
    /// The title's type role.
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
    /// The first enabled field of the [`FieldTab`] scope, as of this pass.
    pub first_field: Option<egui::Id>,
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
    fit_content: bool,
    resize: Option<Resize>,
    field_tab: bool,
}

impl DialogFrame {
    /// A frame titled `title`. `id_salt` keeps the band's and close box's state when the title is
    /// translated.
    pub fn new(id_salt: impl egui::AsIdSalt, title: impl Into<String>) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            title: title.into(),
            close_label: None,
            roomy: false,
            width: None,
            fit_content: false,
            resize: None,
            field_tab: false,
        }
    }

    /// Shows a close box with this accessible name.
    pub fn close_label(mut self, label: impl Into<String>) -> Self {
        self.close_label = Some(label.into());
        self
    }

    /// The larger inner margin (the colour picker dialog).
    pub fn roomy(mut self, roomy: bool) -> Self {
        self.roomy = roomy;
        self
    }

    /// The content's width follows the body: in egui's sizing pass the title row takes the title's
    /// own width and the rule none, so the area is as wide as the body (at most `max`).
    pub fn fit_content(mut self, max: f32) -> Self {
        self.width = Some((0.0, max));
        self.fit_content = true;
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

    /// Resolves the frame's surface, padding and metrics from `tokens`; `pixels_per_point` snaps them to physical pixels.
    pub fn style(tokens: &UiTokens, pixels_per_point: f32) -> DialogFrameStyle {
        DialogFrameStyle {
            surface: Surface::style(tokens, SurfaceKind::Dialog),
            pad: tokens.space.pad.panel,
            pad_roomy: tokens.space.pad.dialog,
            title_gap: tokens.space.gap.inline,
            body_gap: tokens.space.gap.stack,
            footer_gap: tokens.space.gap.stack,
            close_box: tokens.size.control.sm,
            drag_pad: [tokens.space.gap.inline; 3],
            title_role: TITLE_ROLE,
            title_line: tokens.typography.get(TITLE_ROLE).line_height,
            title: Content::Normal.color(tokens).to_egui(),
            rule: Stroke::new(Hairline::Hairline.width(pixels_per_point), tokens.color.line.divider.to_egui()),
        }
    }

    /// Shows the frame with `body` under the title rule.
    pub fn show(self, ui: &mut Ui, body: impl FnOnce(&mut Ui)) -> DialogFrameResponse {
        self.show_inner(ui, body, None::<fn(&mut Ui)>)
    }

    /// Shows the frame with `body` and, `space.gap.stack` under it, `footer`.
    pub fn show_footer(self, ui: &mut Ui, body: impl FnOnce(&mut Ui), footer: impl FnOnce(&mut Ui)) -> DialogFrameResponse {
        self.show_inner(ui, body, Some(footer))
    }

    fn show_inner(self, ui: &mut Ui, body: impl FnOnce(&mut Ui), footer: Option<impl FnOnce(&mut Ui)>) -> DialogFrameResponse {
        let tokens = Tokens::of(ui);
        let style = DialogFrame::style(&tokens, ui.pixels_per_point());
        let id = ui.make_persistent_id(self.id_salt);
        let pad = if self.roomy { style.pad_roomy } else { style.pad };
        let Self { title, close_label, width, fit_content, resize, field_tab, .. } = self;

        let mut parts: Option<Parts> = None;
        let framed = Surface::new(SurfaceKind::Dialog).show(ui, |ui| {
            Frame::NONE.inner_margin(Margin::from(pad)).show(ui, |ui| {
                let content = |ui: &mut Ui| {
                    if let Some((min, max)) = width {
                        ui.set_min_width(min);
                        // A content-sized area keeps the width its sizing pass gave it: raising the
                        // maximum again would hand the title row and rule the whole window.
                        if !fit_content || ui.is_sizing_pass() {
                            ui.set_max_width(max);
                        }
                    }
                    let content_size = ui.available_size();
                    let top = ui.min_rect().top();
                    let (drag, close, title_height) = title_row(ui, &style, id, &title, close_label.as_deref(), fit_content);
                    ui.add_space(style.title_gap);
                    let rule = Divider::horizontal();
                    if fit_content { rule.follow_content() } else { rule }.show(ui);
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
                    let mut first_field = None;
                    if field_tab {
                        FieldTab::new(id.with("fields")).show(ui, inside);
                        first_field = FieldTab::first(ui, id.with("fields"));
                    } else {
                        inside(ui);
                    }
                    let used = ui.min_rect().bottom() - top;
                    parts = Some((drag, close, title_height, content_size, used - body_height, first_field));
                };
                match resize {
                    Some(resize) => resize.show(ui, content),
                    None => content(ui),
                }
            });
        });
        let rect = framed.response.rect;
        let (drag, close, title_height, content_size, inner_chrome, first_field) =
            parts.unwrap_or_else(|| (ui.interact(rect, id.with("title"), Sense::DRAG), None, style.title_line, Vec2::ZERO, 0.0, None));
        DialogFrameResponse { rect, title_height, drag, close, content_size, first_field, chrome: inner_chrome + 2.0 * pad }
    }
}

/// The title, its draggable band and the close box. The band is made before the close box so the
/// box lies over it.
fn title_row(
    ui: &mut Ui,
    style: &DialogFrameStyle,
    id: egui::Id,
    title: &str,
    close_label: Option<&str>,
    fit_content: bool,
) -> (Response, Option<Response>, f32) {
    let tokens = Tokens::of(ui);
    // pc: crates/ui-egui/src/dialogs.rs:630 the title is a bare label, so the row is the font's own
    // height, not the role's line height.
    let mut format = role_format(&tokens, style.title_role, style.title);
    format.line_height = None;
    let row_height = ui.fonts_mut(|f| f.layout_job(egui::text::LayoutJob::single_section(title.to_owned(), format.clone()))).size().y;
    let row_width = if fit_content && ui.is_sizing_pass() {
        let text = ui.fonts_mut(|f| f.layout_job(egui::text::LayoutJob::single_section(title.to_owned(), format.clone()))).size().x;
        text + if close_label.is_some() { style.close_box } else { 0.0 }
    } else {
        ui.available_width()
    };
    let (_, row) = ui.allocate_space(vec2(row_width, row_height));
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
        let mut title_ui = ui.new_child(UiBuilder::new().max_rect(text).layout(direction::row(ui)));
        let label = title_ui.add(Label::new(egui::text::LayoutJob::single_section(title.to_owned(), format)).truncate().selectable(false));
        label.widget_info(|| WidgetInfo::labeled(Role::Heading, true, title));
        ui.ctx().accesskit_node_builder(label.id, |node| node.set_level(1));
    }
    // pc: crates/ui-egui/src/dialogs.rs:255-267 the close box overlays the row, centred on it, and
    // does not set the row's height.
    let close = close_label.map(|label| {
        let square = Rect::from_x_y_ranges(span(near, end), (row.center().y - style.close_box / 2.0)..=(row.center().y + style.close_box / 2.0));
        IconButton::new(Icon::Close, label).size(ControlSize::Sm).id_salt((id, "close")).show_at(ui, square)
    });
    (drag, close, if close_label.is_some() { row_height.max(style.close_box) } else { row_height })
}
