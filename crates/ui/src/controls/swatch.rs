//! [`Swatch`]: a colour well.

use craft_theme::{ControlSize, Rgba, SwatchTokens};
use egui::accesskit::{Action, Role};
use egui::emath::GuiRounding as _;
use egui::epaint::RectShape;
use egui::{AsIdSalt, Color32, CornerRadius, CursorIcon, Id, IdSalt, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, Widget, pos2, vec2};

use super::{SIZE_NAMES, size_arg};
use crate::primitives::{Content, FocusRing, Gap, Interactive, Row, Stack, Text};
use crate::story::{Args, Level, Story};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens, corner_radius};

/// Checkerboard cells per side. An odd count puts the same cell in all four corners, which the
/// rounded outline then covers whole.
const CHECKER_CELLS: usize = 5;

/// A swatch's size and the colours it draws itself, from [`Swatch::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SwatchStyle {
    /// Side of the swatch: `swatch.size`.
    pub side: f32,
    /// From the swatch's outline to the well: the selection ring's room, `swatch.selected_gap`
    /// and `swatch.selected_width`.
    pub inset: f32,
    /// The well's corners: `swatch.radius`. The rings around it are concentric.
    pub radius: CornerRadius,
    /// The ring on the well's outline, one point wide: `swatch.edge`.
    pub edge: Color32,
    /// The checkerboard behind a translucent colour: `swatch.checker.a` holds the corners,
    /// `swatch.checker.b` the other cells.
    pub checker: [Color32; 2],
    /// The ring outside the well while selected: `swatch.selected`, `swatch.selected_width` wide,
    /// `swatch.selected_gap` from the well.
    pub selected: Stroke,
    /// The well of a mixed value: `swatch.mixed.fill`.
    pub mixed_fill: Color32,
    /// The dash on a mixed well: `swatch.mixed.mark`.
    pub mixed_mark: Color32,
    tokens: SwatchTokens,
}

impl SwatchStyle {
    /// The ring just inside the edge of a well showing `color`, or the mixed well for `None`:
    /// black or white at partial alpha, whichever stands out from the colour.
    pub fn inner_edge(&self, color: Option<Rgba>) -> Color32 {
        self.tokens.inner_edge(color.unwrap_or(self.tokens.mixed_fill)).to_egui()
    }
}

/// A colour well: `if ui.add(Swatch::new("fill", "Fill colour", colour)).clicked() { open_picker }`.
///
/// It shows `color` over a checkerboard where it has alpha, inside two one-point rings: the outer
/// ring (`line.control_edge`) separates the well from the panel, and the inner one, black or
/// white at partial alpha by the colour's lightness, from the colour, so the well's bounds show
/// whatever the colour. A click is the host's cue to open its picker; the swatch edits nothing.
/// AccessKit sees a button named by `label` whose value is the colour as `#rrggbb` (`#rrggbbaa`
/// when translucent).
///
/// [`mixed`](Swatch::mixed) shows the look of several objects with different colours: a neutral
/// well with a dash and no colour value. [`selected`](Swatch::selected) puts the selection ring
/// around the well, a gap away, as a palette of swatches marks its choice. The swatch keeps that
/// ring's room in its size whether selected or not, and the keyboard focus ring goes outside it.
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
        Interactive::id_in(ui, IdSalt::new(id_salt))
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
            inset: s.selected_gap + s.selected_width,
            radius: corner_radius(s.radius),
            edge: s.edge.to_egui(),
            checker: [s.checker_a.to_egui(), s.checker_b.to_egui()],
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

    /// The corners of the swatch's outline, concentric with the well's: the selection ring's,
    /// which the focus ring follows outside it.
    pub fn outline_radius(style: &SwatchStyle) -> CornerRadius {
        grow(style.radius, style.inset)
    }

    /// The shapes of a swatch whose outline is `rect`: the well inset by the selection ring's
    /// room, showing the colour (over the checkerboard where translucent) or, for `None`, the
    /// mixed look; the inner and outer rings; the selection ring when `selected`. A disabled well
    /// fades its rings.
    pub fn shapes(style: &SwatchStyle, rect: Rect, color: Option<Rgba>, selected: bool, enabled: bool, pixels_per_point: f32) -> Vec<Shape> {
        let rect = rect.round_to_pixels(pixels_per_point);
        let well = rect.shrink(style.inset);
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
                    out.push(Shape::Rect(RectShape::filled(well, radius, style.checker[0])));
                    let cell = well.width() / CHECKER_CELLS as f32;
                    for row in 0..CHECKER_CELLS {
                        for col in (0..CHECKER_CELLS).filter(|col| (row + col) % 2 == 1) {
                            let min = well.min + vec2(col as f32 * cell, row as f32 * cell);
                            out.push(Shape::Rect(RectShape::filled(
                                Rect::from_min_size(min, vec2(cell, cell)),
                                CornerRadius::ZERO,
                                style.checker[1],
                            )));
                        }
                    }
                }
                out.push(Shape::Rect(RectShape::filled(well, radius, color.to_egui())));
            }
        }
        out.extend(Swatch::rings(style, rect, style.inner_edge(color), selected, enabled, pixels_per_point));
        out
    }

    /// The two one-point rings of a swatch whose outline is `rect`, the inner one in `inner`, and
    /// the selection ring when `selected`; a disabled well fades its rings.
    pub(crate) fn rings(style: &SwatchStyle, rect: Rect, inner: Color32, selected: bool, enabled: bool, pixels_per_point: f32) -> Vec<Shape> {
        let rect = rect.round_to_pixels(pixels_per_point);
        let well = rect.shrink(style.inset);
        let radius = style.radius;
        let fade = |c: Color32| if enabled { c } else { c.gamma_multiply(0.5) };
        let inner_radius = CornerRadius {
            nw: radius.nw.saturating_sub(1),
            ne: radius.ne.saturating_sub(1),
            sw: radius.sw.saturating_sub(1),
            se: radius.se.saturating_sub(1),
        };
        let mut out = vec![Shape::Rect(RectShape::stroke(well.shrink(1.0), inner_radius, Stroke::new(1.0, fade(inner)), StrokeKind::Inside))];
        out.push(Shape::Rect(RectShape::stroke(well, radius, Stroke::new(1.0, fade(style.edge)), StrokeKind::Inside)));
        if selected {
            out.push(Shape::Rect(RectShape::stroke(rect, Swatch::outline_radius(style), style.selected, StrokeKind::Inside)));
        }
        out
    }
}

/// `radius` grown by `by` points, for a concentric outline `by` points further out.
fn grow(radius: CornerRadius, by: f32) -> CornerRadius {
    let by = by.round() as u8;
    CornerRadius {
        nw: radius.nw.saturating_add(by),
        ne: radius.ne.saturating_add(by),
        sw: radius.sw.saturating_add(by),
        se: radius.se.saturating_add(by),
    }
}

impl Widget for Swatch {
    fn ui(self, ui: &mut Ui) -> Response {
        let style = Swatch::style(&Tokens::of(ui), self.size);
        let role = if self.radio.is_some() { Role::RadioButton } else { Role::Button };
        let mut element = Interactive::new(self.id_salt, role, self.label.as_str())
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
        FocusRing::paint_radius(ui, response.rect, Swatch::outline_radius(&style), &response);
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
        Story {
            id: "controls/swatch/states",
            level: Level::Controls,
            component: "Swatch",
            name: "States",
            args: || Args::new().with_choice("size", SIZE_NAMES, 1),
            show: show_states,
        },
        Story {
            id: "controls/swatch/palette",
            level: Level::Controls,
            component: "Swatch",
            name: "Palette",
            args: || Args::new().with_choice("size", SIZE_NAMES, 1),
            show: show_palette,
        },
    ]
}

/// Opaque, translucent, a colour equal to the panel, mixed, selected and disabled, each named.
fn show_states(ui: &mut Ui, args: &mut Args) {
    let size = size_arg(args.choice("size"));
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
fn show_palette(ui: &mut Ui, args: &mut Args) {
    let size = size_arg(args.choice("size"));
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
