//! [`Handle`]: canvas handles for selections, paths and transforms.

use egui::accesskit::Role;
use egui::emath::GuiRounding as _;
use egui::{AsIdSalt, Color32, CursorIcon, IdSalt, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, WidgetInfo, vec2};

use crate::inspect::{self, RecordKind};
use crate::story::{Args, Level, Story};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};

/// What a handle does, which sets its shape and pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum HandleShape {
    /// Corners and anchors that resize or move: a square.
    Square,
    /// Control points and centres: a circle.
    Round,
    /// Rotation: a solid dot.
    Rotate,
}

impl HandleShape {
    /// Every shape, in declaration order.
    pub const ALL: &'static [HandleShape] = &[HandleShape::Square, HandleShape::Round, HandleShape::Rotate];

    /// The name in story args and dumps.
    pub const fn name(self) -> &'static str {
        match self {
            HandleShape::Square => "square",
            HandleShape::Round => "round",
            HandleShape::Rotate => "rotate",
        }
    }
}

/// A handle's look, from [`Handle::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct HandleStyle {
    /// Side or diameter in points: `handle.size`.
    pub size: f32,
    /// The inside: `overlay.handle_fill` (`overlay.handle_stroke` for a rotate dot).
    pub fill: Color32,
    /// The outline inside `size`: `handle.stroke` of `overlay.handle_stroke`; none on a rotate dot.
    pub stroke: Stroke,
    /// The outline outside `size`: `handle.halo` of `overlay.halo`, so the handle reads on any
    /// image.
    pub halo: Stroke,
    /// Side of the square that takes the pointer, centred on the handle: `handle.hit`.
    pub hit: f32,
    /// The pointer over the handle.
    pub cursor: CursorIcon,
}

/// A canvas handle at a point: `Handle::new(HandleShape::Square, "Top left corner").show(ui,
/// center)` draws it and returns its response, to clicks and drags by default.
///
/// Handles stay out of the layout and the Tab order: the canvas they sit on owns keyboard
/// editing. Each is in the AccessKit tree as a graphics object with its label and, when the canvas
/// gives one, its [`value`](Handle::value).
#[derive(Clone, Debug)]
pub struct Handle {
    shape: HandleShape,
    label: String,
    value: Option<String>,
    sense: Sense,
    id_salt: Option<IdSalt>,
    cursor: Option<CursorIcon>,
    look: Option<HandleStyle>,
    selected: Option<bool>,
}

impl Handle {
    /// A handle of `shape` whose accessible name is `label`, e.g. `"Top left corner"`.
    pub fn new(shape: HandleShape, label: impl Into<String>) -> Self {
        Self { shape, label: label.into(), value: None, sense: Sense::CLICK | Sense::DRAG, id_salt: None, cursor: None, look: None, selected: None }
    }

    /// The position a screen reader reads after the label, in the document's units, e.g.
    /// `"120, 48 px"`.
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// What the handle responds to; clicks and drags by default. A handle never takes keyboard
    /// focus, so [`Sense::FOCUSABLE`] is ignored.
    pub fn sense(mut self, sense: Sense) -> Self {
        self.sense = sense;
        self
    }

    /// A stable id for a handle whose position in the call order changes, e.g. one of a path's
    /// points while points are added. Handles otherwise take the `Ui`'s next automatic id.
    pub fn id_salt(mut self, id_salt: impl AsIdSalt) -> Self {
        self.id_salt = Some(IdSalt::new(id_salt));
        self
    }

    /// A pointer for what this handle does, e.g. [`CursorIcon::ResizeNwSe`] for a top-left corner.
    pub fn cursor(mut self, cursor: CursorIcon) -> Self {
        self.cursor = Some(cursor);
        self
    }

    /// Colours and sizes of the caller's own, in place of the `overlay.*` look: a handle that sits
    /// in a control rather than on a canvas, e.g. a curve editor's points.
    pub fn look(mut self, style: HandleStyle) -> Self {
        self.look = Some(style);
        self
    }

    /// Whether the handle is the selected one of its set, for a screen reader.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = Some(selected);
        self
    }

    pub fn style(tokens: &UiTokens, shape: HandleShape) -> HandleStyle {
        let (h, o) = (&tokens.component.handle, &tokens.color.overlay);
        let (fill, stroke) = match shape {
            HandleShape::Square | HandleShape::Round => (o.handle_fill, Stroke::new(h.stroke, o.handle_stroke.to_egui())),
            HandleShape::Rotate => (o.handle_stroke, Stroke::NONE),
        };
        let cursor = match shape {
            HandleShape::Square | HandleShape::Round => CursorIcon::Move,
            HandleShape::Rotate => CursorIcon::Alias,
        };
        HandleStyle { size: h.size, fill: fill.to_egui(), stroke, halo: Stroke::new(h.halo, o.halo.to_egui()), hit: h.hit, cursor }
    }

    /// The handle's shapes centred on `center`, snapped to the pixel grid: halo, fill, outline.
    pub fn shapes(tokens: &UiTokens, shape: HandleShape, center: Pos2, pixels_per_point: f32) -> Vec<Shape> {
        Handle::shapes_with(&Handle::style(tokens, shape), shape, center, pixels_per_point)
    }

    /// [`Handle::shapes`] in `style`'s colours and sizes. A halo or outline of no width is left
    /// out.
    pub fn shapes_with(style: &HandleStyle, shape: HandleShape, center: Pos2, pixels_per_point: f32) -> Vec<Shape> {
        let body = Rect::from_center_size(center, vec2(style.size, style.size)).round_to_pixels(pixels_per_point);
        let halo = body.expand(style.halo.width);
        match shape {
            HandleShape::Square => {
                let mut out = Vec::with_capacity(3);
                if style.halo.width > 0.0 {
                    out.push(Shape::rect_filled(halo, 0.0, style.halo.color));
                }
                out.push(Shape::rect_filled(body, 0.0, style.fill));
                if style.stroke.width > 0.0 {
                    out.push(Shape::rect_stroke(body, 0.0, style.stroke, StrokeKind::Inside));
                }
                out
            }
            HandleShape::Round | HandleShape::Rotate => {
                let (c, r) = (body.center(), body.width() / 2.0);
                let mut out = Vec::with_capacity(3);
                if style.halo.width > 0.0 {
                    out.push(Shape::circle_filled(c, r + style.halo.width, style.halo.color));
                }
                out.push(Shape::circle_filled(c, r, style.fill));
                if style.stroke.width > 0.0 {
                    out.push(Shape::circle_stroke(c, r - style.stroke.width / 2.0, style.stroke));
                }
                out
            }
        }
    }

    /// Draws the handle at `center` and senses the pointer on the `handle.hit` square around it.
    pub fn show(self, ui: &mut Ui, center: Pos2) -> Response {
        let tokens = Tokens::of(ui);
        let style = self.look.unwrap_or_else(|| Handle::style(&tokens, self.shape));
        let id = match self.id_salt {
            Some(salt) => ui.make_persistent_id(salt),
            None => {
                let id = ui.next_auto_id();
                ui.skip_ahead_auto_ids(1);
                id
            }
        };
        let rect = Rect::from_center_size(center, vec2(style.hit, style.hit));
        let response = ui.interact(rect, id, self.sense - Sense::FOCUSABLE);
        inspect::record(ui, id, RecordKind::Handle, rect, Rect::from_center_size(center, vec2(style.size, style.size)));
        response.widget_info(|| {
            let mut info = WidgetInfo::labeled(Role::GraphicsObject, ui.is_enabled(), &self.label);
            info.current_text_value.clone_from(&self.value);
            info
        });
        if let Some(selected) = self.selected {
            ui.ctx().accesskit_node_builder(id, |node| node.set_selected(selected));
        }
        if ui.is_rect_visible(rect) {
            ui.painter().extend(Handle::shapes_with(&style, self.shape, center, ui.pixels_per_point()));
        }
        if response.hovered() || response.dragged() {
            ui.ctx().set_cursor_icon(self.cursor.unwrap_or(style.cursor));
        }
        response
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Handle"];

pub(crate) fn stories() -> Vec<Story> {
    vec![Story { id: "primitives/handle/shapes", level: Level::Primitives, component: "Handle", name: "Shapes", args: Args::new, show }]
}

/// The three shapes on a light and a dark strip, draggable.
fn show(ui: &mut Ui, _: &mut Args) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width().min(320.0), 96.0), Sense::hover());
    let painter = ui.painter_at(rect);
    let (top, bottom) = rect.split_top_bottom_at_fraction(0.5);
    painter.rect_filled(top, 0.0, Color32::from_gray(235));
    painter.rect_filled(bottom, 0.0, Color32::from_gray(30));
    let offsets_id = ui.scope_id().with("handle-story-offsets");
    let mut offsets: [egui::Vec2; 6] = ui.data(|d| d.get_temp(offsets_id).unwrap_or_default());
    for (i, offset) in offsets.iter_mut().enumerate() {
        let shape = HandleShape::ALL[i % 3];
        let strip = if i < 3 { top } else { bottom };
        let base = strip.left_center() + vec2(strip.width() * (i % 3 + 1) as f32 / 4.0, 0.0);
        let label = format!("{} handle {}", shape.name(), i / 3 + 1);
        let position = base + *offset;
        let response =
            Handle::new(shape, label).value(format!("{:.0}, {:.0}", position.x, position.y)).id_salt(("handle-story", i)).show(ui, position);
        *offset += response.drag_delta();
    }
    ui.data_mut(|d| d.insert_temp(offsets_id, offsets));
}
