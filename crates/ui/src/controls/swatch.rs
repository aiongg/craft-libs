//! [`Swatch`]: a colour well.

use craft_theme::{ControlSize, Rgba, SwatchTokens};
use egui::accesskit::{Action, Role};
use egui::emath::GuiRounding as _;
use egui::epaint::RectShape;
use egui::{AsIdSalt, Color32, CornerRadius, CursorIcon, Id, IdSalt, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, Vec2, Widget, pos2, vec2};

use super::SizeArgs;
use crate::primitives::{Content, FocusRing, Gap, Interactive, Row, Stack, Text};
use crate::story::{Level, Story};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens, corner_radius};
use crate::typed_story;

/// A swatch's size and the colours it draws itself, from [`Swatch::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SwatchStyle {
    /// Side of the swatch: `swatch.size`.
    pub side: f32,
    /// The well's corners: `swatch.radius`.
    pub radius: CornerRadius,
    /// The ring on the well's outline, one point wide: `swatch.edge`.
    pub edge: Color32,
    /// The checkerboard behind a translucent colour: `swatch.checker.a` holds the corners,
    /// `swatch.checker.b` the other cells.
    pub checker: [Color32; 2],
    /// Side of a checkerboard cell: `swatch.checker_cell`.
    pub checker_cell: f32,
    /// The ring inside the well's outline while selected: `swatch.selected`,
    /// `swatch.selected_width` wide.
    pub selected: Stroke,
    /// The well of a mixed value: `swatch.mixed.fill`.
    pub mixed_fill: Color32,
    /// The dash on a mixed well: `swatch.mixed.mark`.
    pub mixed_mark: Color32,
    tokens: SwatchTokens,
}

impl SwatchStyle {
    /// A ring of black or white at partial alpha that stands out from `color`, or from the mixed
    /// well for `None`: for compositions that draw a well apart from its colours.
    pub fn inner_edge(&self, color: Option<Rgba>) -> Color32 {
        self.tokens.inner_edge(color.unwrap_or(self.tokens.mixed_fill)).to_egui()
    }
}

/// A colour well: `if ui.add(Swatch::new("fill", "Fill colour", colour)).clicked() { open_picker }`.
///
/// It shows `color` over a checkerboard where it has alpha, filling its box inside a one-point
/// `line.control_edge` ring that separates the well from the panel. A click is the host's cue to
/// open its picker; the swatch edits nothing.
/// AccessKit sees a button named by `label` whose value is the colour as `#rrggbb` (`#rrggbbaa`
/// when translucent).
///
/// [`mixed`](Swatch::mixed) shows the look of several objects with different colours: a neutral
/// well with a dash and no colour value. [`selected`](Swatch::selected) draws the selection ring
/// inside the well's outline, as a palette of swatches marks its choice; the keyboard focus ring
/// goes outside the well.
///
/// `color` is an sRGB colour with straight alpha ([`Rgba`]); an egui [`Color32`] converts into
/// one, as closely as its premultiplied channels allow. Colour management is the app's.
#[derive(Clone, Debug)]
pub struct Swatch {
    label: String,
    color: Rgba,
    size: ControlSize,
    mixed: bool,
    selected: bool,
    /// A radio button of a group: whether it is the checked one, and its place in the set.
    radio: Option<bool>,
    position: Option<usize>,
    /// Whether keyboard focus can reach it; a palette keeps one swatch a stop.
    tab_stop: bool,
    enabled: bool,
    id_salt: IdSalt,
}

impl Swatch {
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, color: impl Into<Rgba>) -> Self {
        Self {
            label: label.into(),
            color: color.into(),
            size: ControlSize::Md,
            mixed: false,
            selected: false,
            radio: None,
            position: None,
            tab_stop: true,
            enabled: true,
            id_salt: IdSalt::new(id_salt),
        }
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// The mixed-value look instead of the colour.
    pub fn mixed(mut self, mixed: bool) -> Self {
        self.mixed = mixed;
        self
    }

    /// The selection ring, and AccessKit selected.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// A radio button of a group of swatches ([`SwatchPalette`](crate::composites::SwatchPalette)):
    /// AccessKit `RadioButton`, checked when `checked`, which also puts the selection ring round
    /// the well. Keys and focus between the swatches are the group's.
    pub fn radio(mut self, checked: bool) -> Self {
        self.radio = Some(checked);
        self.selected = checked;
        self
    }

    /// The swatch's place in its group, from 0, for a screen reader.
    pub(crate) fn position_in_set(mut self, index: usize) -> Self {
        self.position = Some(index);
        self
    }

    /// Whether Tab reaches the swatch; one that isn't a Tab stop still takes clicks, and offers a
    /// screen reader the Focus action, which its group handles (a palette moves its stop there).
    pub(crate) fn tab_stop(mut self, tab_stop: bool) -> Self {
        self.tab_stop = tab_stop;
        self
    }

    /// The id a swatch made with `id_salt` has in `ui`.
    pub(crate) fn id_in(ui: &Ui, id_salt: impl AsIdSalt) -> Id {
        Interactive::id_in(ui, id_salt)
    }

    /// Whether it takes clicks; a disabled swatch keeps its colour and fades its rings.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn style(tokens: &UiTokens, size: ControlSize) -> SwatchStyle {
        let s = &tokens.component.swatch;
        SwatchStyle {
            side: s.size.get(size),
            radius: corner_radius(s.radius),
            edge: s.edge.to_egui(),
            checker: [s.checker_a.to_egui(), s.checker_b.to_egui()],
            checker_cell: s.checker_cell,
            selected: Stroke::new(s.selected_width, s.selected.to_egui()),
            mixed_fill: s.mixed_fill.to_egui(),
            mixed_mark: s.mixed_mark.to_egui(),
            tokens: *s,
        }
    }

    /// The colour as AccessKit reports it: `#rrggbb`, or `#rrggbbaa` when translucent. The
    /// channels are kept whatever the alpha.
    pub fn hex(color: Rgba) -> String {
        color.to_hex()
    }

    /// The shapes of a swatch whose outline is `rect`: the well showing the colour (over the
    /// checkerboard where translucent) or, for `None`, the mixed look; then the edge and, when
    /// `selected`, the selection ring. A disabled well fades its edge and ring.
    pub fn shapes(style: &SwatchStyle, rect: Rect, color: Option<Rgba>, selected: bool, enabled: bool, pixels_per_point: f32) -> Vec<Shape> {
        let well = rect.round_to_pixels(pixels_per_point);
        let radius = style.radius;
        let mut out = Vec::new();
        match color {
            None => {
                out.push(Shape::Rect(RectShape::filled(well, radius, style.mixed_fill)));
                let half = well.width() * 0.25;
                let y = well.center().y;
                out.push(Shape::line_segment([pos2(well.center().x - half, y), pos2(well.center().x + half, y)], Stroke::new(2.0, style.mixed_mark)));
            }
            Some(color) => {
                if color.a() < 255 {
                    out.extend(checkerboard(well, radius, style.checker, style.checker_cell));
                }
                out.push(Shape::Rect(RectShape::filled(well, radius, color.to_egui())));
            }
        }
        out.extend(Swatch::outline(style, well, selected, enabled, pixels_per_point));
        out
    }

    /// The edge of a swatch whose outline is `rect` and, when `selected`, the selection ring; a
    /// disabled well fades them.
    pub(crate) fn outline(style: &SwatchStyle, rect: Rect, selected: bool, enabled: bool, pixels_per_point: f32) -> Vec<Shape> {
        let well = rect.round_to_pixels(pixels_per_point);
        let fade = |c: Color32| if enabled { c } else { c.gamma_multiply(0.5) };
        let mut out = vec![Shape::Rect(RectShape::stroke(well, style.radius, Stroke::new(1.0, fade(style.edge)), StrokeKind::Inside))];
        if selected {
            out.push(Shape::Rect(RectShape::stroke(
                well,
                style.radius,
                Stroke::new(style.selected.width, fade(style.selected.color)),
                StrokeKind::Inside,
            )));
        }
        out
    }
}

/// A checkerboard of `cell_size` cells over `rect`, the light cell at its top left, with the
/// corners of `radius` where a cell meets a corner of `rect`.
pub fn checkerboard(rect: Rect, radius: CornerRadius, checker: [Color32; 2], cell_size: f32) -> Vec<Shape> {
    let mut out = vec![Shape::Rect(RectShape::filled(rect, radius, checker[0]))];
    let (cols, rows) = ((rect.width() / cell_size).ceil() as usize, (rect.height() / cell_size).ceil() as usize);
    for row in 0..rows {
        for col in (0..cols).filter(|col| (row + col) % 2 == 1) {
            let min = rect.min + vec2(col as f32 * cell_size, row as f32 * cell_size);
            let cell = Rect::from_min_size(min, Vec2::splat(cell_size)).intersect(rect);
            let round = |touches: bool, r: u8| if touches { r } else { 0 };
            let corners = CornerRadius {
                nw: round(cell.min == rect.min, radius.nw),
                ne: round(cell.right_top() == rect.right_top(), radius.ne),
                sw: round(cell.left_bottom() == rect.left_bottom(), radius.sw),
                se: round(cell.max == rect.max, radius.se),
            };
            out.push(Shape::Rect(RectShape::filled(cell, corners, checker[1])));
        }
    }
    out
}

impl Widget for Swatch {
    fn ui(self, ui: &mut Ui) -> Response {
        let style = Swatch::style(&Tokens::of(ui), self.size);
        let role = if self.radio.is_some() { Role::RadioButton } else { Role::Button };
        let mut element = Interactive::salted(self.id_salt, role, self.label.as_str())
            .state_layer(false)
            .focus_ring(false)
            .selected(self.selected)
            .enabled(self.enabled);
        if !self.tab_stop {
            element = element.sense(Sense::CLICK);
        }
        if let Some(checked) = self.radio {
            element = element.checked(checked);
        }
        if let Some(position) = self.position {
            element = element.position_in_set(position);
        }
        if !self.mixed {
            element = element.value(Swatch::hex(self.color));
        }
        let response = element.show(ui, vec2(style.side, style.side));
        if !self.tab_stop && self.enabled {
            ui.ctx().accesskit_node_builder(response.id, |node| node.add_action(Action::Focus));
        }
        if ui.is_rect_visible(response.rect) {
            let color = (!self.mixed).then_some(self.color);
            ui.painter().extend(Swatch::shapes(&style, response.rect, color, self.selected, self.enabled, ui.pixels_per_point()));
        }
        FocusRing::paint_radius(ui, response.rect, style.radius, &response);
        if self.enabled { response.on_hover_cursor(CursorIcon::PointingHand) } else { response }
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Swatch"];

const PALETTE: [[u8; 3]; 12] = [
    [0xe5, 0x48, 0x4d],
    [0xf7, 0x6b, 0x15],
    [0xf5, 0xa5, 0x24],
    [0xf5, 0xd9, 0x0a],
    [0x7a, 0xc5, 0x5c],
    [0x30, 0xa4, 0x6c],
    [0x12, 0xa5, 0x94],
    [0x00, 0x9e, 0xe1],
    [0x3e, 0x63, 0xdd],
    [0x8e, 0x4e, 0xc6],
    [0xd6, 0x40, 0x9f],
    [0x88, 0x8d, 0x91],
];

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!("controls/swatch/states", Level::Controls, "Swatch", "States", SizeArgs::default(), show_states),
        typed_story!("controls/swatch/palette", Level::Controls, "Swatch", "Palette", SizeArgs::default(), show_palette),
    ]
}

/// Opaque, translucent, a colour equal to the panel, mixed, selected and disabled, each named.
fn show_states(ui: &mut Ui, args: &mut SizeArgs) {
    let size = args.size.control();
    let panel = Tokens::of(ui).color.surface.panel;
    let samples: [(&str, Swatch); 7] = [
        ("Opaque", Swatch::new("opaque", "Opaque", Rgba::rgb(0x3e, 0x63, 0xdd))),
        ("Translucent", Swatch::new("translucent", "Translucent", Rgba([0xe5, 0x48, 0x4d, 110]))),
        ("Clear", Swatch::new("clear", "Clear", Rgba::TRANSPARENT)),
        ("Panel colour", Swatch::new("panel", "Panel colour", panel)),
        ("Mixed", Swatch::new("mixed", "Mixed", Rgba::BLACK).mixed(true)),
        ("Selected", Swatch::new("selected", "Selected", Rgba::rgb(0x30, 0xa4, 0x6c)).selected(true)),
        ("Disabled", Swatch::new("disabled", "Disabled", Rgba::rgb(0xf5, 0xa5, 0x24)).enabled(false)),
    ];
    let state_id = ui.scope_id().with("swatch-clicks");
    let mut clicks = ui.data(|d| d.get_temp::<u32>(state_id).unwrap_or_default());
    Stack::new(Gap::Stack).show(ui, |ui| {
        Row::new(Gap::Inline).show(ui, |ui| {
            for (_, swatch) in samples {
                if ui.add(swatch.size(size)).clicked() {
                    clicks += 1;
                }
            }
        });
        ui.add(Text::new(format!("Clicked {clicks} times")).content(Content::Muted));
    });
    ui.data_mut(|d| d.insert_temp(state_id, clicks));
}

/// A row of 12 swatches, one selected, each a Tab stop.
fn show_palette(ui: &mut Ui, args: &mut SizeArgs) {
    let size = args.size.control();
    let state_id = ui.scope_id().with("swatch-palette");
    let mut chosen = ui.data(|d| d.get_temp::<usize>(state_id).unwrap_or(7));
    Row::new(Gap::Inline).show(ui, |ui| {
        for (i, [r, g, b]) in PALETTE.into_iter().enumerate() {
            let label = format!("Colour {}", i + 1);
            if ui.add(Swatch::new(("palette", i), label, Rgba::rgb(r, g, b)).size(size).selected(i == chosen)).clicked() {
                chosen = i;
            }
        }
    });
    ui.data_mut(|d| d.insert_temp(state_id, chosen));
}
