//! [`PaintChip`] (F04.06): a shape's fill or stroke paint as a chip that opens a popover with No
//! Color, Solid and Gradient, as PhotoCraft's shape options bar and Properties panel; and
//! [`PaintList`], named paints in rows with their chips.
//!
//! The gradient editor is craft-draw's, and craft-draw depends on craft-color, so the chip takes
//! the editor's body from the host ([`PaintChip::gradient_editor`]); so do the Pattern and Picture
//! tabs ([`PaintChip::pattern_editor`], [`PaintChip::picture_editor`]: craft-draw's
//! `PatternFillEditor` and `PictureFillEditor`).

use std::cell::RefCell;

use craft_theme::{ControlSize, Srgba};
use egui::accesskit::HasPopup;
use egui::{
    Align, AsIdSalt, Color32, Id, IdSalt, Key, LayerId, Layout, Mesh, Modifiers, Order, Painter, PopupCloseBehavior, Rect, Stroke, Ui, pos2, vec2,
};

use super::{ColorPicker, ColorPickerLayout, HexField, ScreenColorPicker, ScreenPickerLabels};
use craft_ui::behavior::{RowInfo, Selection};
use craft_ui::controls::{Badge, BadgeStyle, BadgeTone, Button, IconButton, NumberField, Segmented, Swatch, SwatchStyle, TextField, checkerboard};
use craft_ui::edit::{EditEvent, EditPolicy, EditResponse};
use craft_ui::icons::Icon;
use craft_ui::list::{List, ListEvent, ListRow, RowEvent};
use craft_ui::localized::{Labels as _, LocaleNumbers, Unit, fmt};
use craft_ui::popup_list::PopupList;
use craft_ui::primitives::{Interactive, Surface, SurfaceKind};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, UiTokens, role_format};

craft_ui::words! {
    /// The paint popover's tabs, English by default.
    #[non_exhaustive]
    pub struct PaintLabels {
        no_color: "paint.noColor" = "No Color";
        solid: "paint.solid" = "Solid";
        gradient: "paint.gradient" = "Gradient";
    }
}

craft_ui::words! {
    /// The paint popover's Pattern and Picture tabs, English by default (Photoshop's Pattern,
    /// PowerPoint's Picture). Kept apart from [`PaintLabels`] until task RG1 registers them.
    #[non_exhaustive]
    pub struct PaintKindLabels {
        pattern: "paint.pattern" = "Pattern";
        picture: "paint.picture" = "Picture";
        /// The chip's value for a screen reader: `{name}` the pattern's name.
        pattern_named: "paint.patternNamed" = "Pattern: {name}";
        /// The chip's value for a screen reader: `{name}` the picture's name.
        picture_named: "paint.pictureNamed" = "Picture: {name}";
    }
}

craft_ui::words! {
    /// The row actions of [`PaintList`] (FigCraft's Selection colors; US spelling).
    #[non_exhaustive]
    pub struct PaintListLabels {
        /// A row's chip: opens the picker.
        edit: "paintList.edit" = "Edit color";
        hex: "paintList.hex" = "Color";
        invalid: "paintList.invalid" = "Invalid color";
        opacity: "paintList.opacity" = "Opacity";
        detach_style: "paintList.detachStyle" = "Detach style";
        detach_variable: "paintList.detachVariable" = "Detach variable";
        styles: "paintList.styles" = "Color styles";
        select_users: "paintList.selectUsers" = "Select layers with this color";
        add: "paintList.add" = "Add";
        remove: "paintList.remove" = "Remove";
    }
}

/// A gradient as the chip draws it: stops at 0–1 along a left-to-right ramp. The gradient itself
/// (angle, style, midpoints) is the host's; it gives this preview with the paint.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GradientPreview {
    pub stops: Vec<(f32, Srgba)>,
}

/// A pattern or picture as the chip draws it: the host's texture of the image (a pattern's tile,
/// tiled across the chip; a picture, stretched to it) and its name for a screen reader. The
/// pattern or picture itself is the host's.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ImagePreview {
    /// "Bubbles", "photo.png".
    pub name: String,
    /// The image's texture, `None` while the host has none (the chip shows the field fill).
    pub texture: Option<egui::TextureId>,
    /// The image's size in pixels (a pattern tile's size, so the chip tiles it at 1:1).
    pub size: [f32; 2],
}

/// A paint: none, a solid colour, a gradient, a pattern or a picture. More kinds may come: a
/// `match` on it outside craft-color needs a `_` arm.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Paint {
    None,
    Solid(Srgba),
    Gradient(GradientPreview),
    /// A pattern fill (Photoshop's shape Pattern tab, PowerPoint's Pattern fill).
    Pattern(ImagePreview),
    /// A picture fill (PowerPoint's Picture or texture fill).
    Picture(ImagePreview),
}

/// The kinds of paint, as the chip's tabs and an app's fill-type switch (`Segmented<FillKind>` or
/// a radio list) name them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FillKind {
    None,
    Solid,
    Gradient,
    Pattern,
    Picture,
}

impl FillKind {
    /// The kind of `paint`.
    pub fn of(paint: &Paint) -> FillKind {
        match paint {
            Paint::None => FillKind::None,
            Paint::Solid(_) => FillKind::Solid,
            Paint::Gradient(_) => FillKind::Gradient,
            Paint::Pattern(_) => FillKind::Pattern,
            Paint::Picture(_) => FillKind::Picture,
        }
    }
}

/// The host's gradient editor body, drawn in the popover's Gradient tab: it edits the host's
/// gradient and returns the new preview when it changed.
type GradientEditor<'a> = dyn FnMut(&mut Ui) -> Option<GradientPreview> + 'a;

/// The host's pattern or picture editor body, drawn in the popover's Pattern or Picture tab: it
/// edits the host's fill and returns the new preview when it changed.
type ImageEditor<'a> = dyn FnMut(&mut Ui) -> Option<ImagePreview> + 'a;

/// A paint chip, 30 by 22 points in PhotoCraft (the `swatch` control's Md size here).
///
/// - Look: a solid paint fills the chip over the checkerboard (alpha shows); a gradient is a
///   left-to-right ramp through its stops; No Color is the field fill with a red diagonal
///   (`status.danger`) from bottom left to top right. Edge: `line.control_edge`, the focus ring
///   when focused.
/// - Click, Enter or Space opens a popover: a segmented No Color / Solid / Gradient / Pattern /
///   Picture switch ([`kinds`](Self::kinds): Gradient, Pattern and Picture only when the host gave
///   their editor), then the Compact [`ColorPicker`](super::ColorPicker) (Solid, with the screen
///   eyedropper) or the host's editor. Switching to Solid starts from the last solid colour (or
///   the fallback), to Gradient, Pattern or Picture from the host's default
///   ([`default_gradient`](Self::default_gradient)). Without the new editors the chip and popover
///   are as before (PhotoCraft's look).
/// - A pattern draws its tile across the chip; a picture is stretched to it.
/// - Edits: a switch is a [`Set`](craft_ui::edit::EditEvent::Set); picker gestures report as the
///   picker does. Alpha is kept through a screen sample.
/// - AccessKit: a `ColorWell` named by `label`, its value the hex or "No Color"/"Gradient".
pub struct PaintChip<'a> {
    id_salt: IdSalt,
    label: String,
    paint: &'a Paint,
    fallback: Srgba,
    numbers: &'a LocaleNumbers,
    none_allowed: bool,
    gradient: Option<(GradientPreview, Box<GradientEditor<'a>>)>,
    pattern: Option<(ImagePreview, Box<ImageEditor<'a>>)>,
    picture: Option<(ImagePreview, Box<ImageEditor<'a>>)>,
    enabled: bool,
}

impl<'a> PaintChip<'a> {
    /// A chip for `paint`; `fallback` is the solid colour a switch to Solid starts from when the
    /// paint has none.
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, paint: &'a Paint, fallback: Srgba, numbers: &'a LocaleNumbers) -> PaintChip<'a> {
        PaintChip {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            paint,
            fallback,
            numbers,
            none_allowed: true,
            gradient: None,
            pattern: None,
            picture: None,
            enabled: true,
        }
    }

    /// Whether No Color is offered; on by default (a stroke can be none, a text colour cannot).
    pub fn none_allowed(mut self, allowed: bool) -> PaintChip<'a> {
        self.none_allowed = allowed;
        self
    }

    /// The Gradient tab: `default_gradient` is what a switch to Gradient sets, `editor` draws the
    /// host's editor. Without it there is no Gradient tab.
    pub fn gradient_editor(
        mut self,
        default_gradient: GradientPreview,
        editor: impl FnMut(&mut Ui) -> Option<GradientPreview> + 'a,
    ) -> PaintChip<'a> {
        self.gradient = Some((default_gradient, Box::new(editor)));
        self
    }

    /// The Pattern tab: `default_pattern` is what a switch to Pattern sets, `editor` draws the
    /// host's editor (craft-draw's `PatternFillEditor`). Without it there is no Pattern tab.
    pub fn pattern_editor(mut self, default_pattern: ImagePreview, editor: impl FnMut(&mut Ui) -> Option<ImagePreview> + 'a) -> PaintChip<'a> {
        self.pattern = Some((default_pattern, Box::new(editor)));
        self
    }

    /// The Picture tab: `default_picture` is what a switch to Picture sets, `editor` draws the
    /// host's editor (craft-draw's `PictureFillEditor`). Without it there is no Picture tab.
    pub fn picture_editor(mut self, default_picture: ImagePreview, editor: impl FnMut(&mut Ui) -> Option<ImagePreview> + 'a) -> PaintChip<'a> {
        self.picture = Some((default_picture, Box::new(editor)));
        self
    }

    pub fn enabled(mut self, enabled: bool) -> PaintChip<'a> {
        self.enabled = enabled;
        self
    }

    /// The tabs the popover shows, in order: No Color (when allowed), Solid, then Gradient,
    /// Pattern and Picture when the host gave their editors.
    pub fn kinds(&self) -> Vec<FillKind> {
        let offered = [
            (FillKind::None, self.none_allowed),
            (FillKind::Solid, true),
            (FillKind::Gradient, self.gradient.is_some()),
            (FillKind::Pattern, self.pattern.is_some()),
            (FillKind::Picture, self.picture.is_some()),
        ];
        offered.into_iter().filter_map(|(kind, on)| on.then_some(kind)).collect()
    }

    /// The gradient a switch to Gradient sets, when the host gave an editor.
    pub fn default_gradient(&self) -> Option<&GradientPreview> {
        self.gradient.as_ref().map(|(g, _)| g)
    }

    // pc: crates/ui-egui/src/tools/shape_paint_ui.rs:64 `swatch` (the chip at :72, the ramp at
    // :80, No Color at :103, the popover tabs at :133); built on `ColorPopover`'s popover and
    // `ColorPicker` Compact, `Segmented` for the tabs.
    // pc: crates/ui-egui/src/tools/shape_paint_ui.rs:21-24 (the swatches never offered a pattern);
    // crates/ui-egui/src/panels/preset_panels.rs:277-279 (the pattern grid meant for paint popups).
    // inv-deckcraft: crates/ui-egui/src/panels/format_shape.rs:369-375 (the fill types).
    // new: the Pattern and Picture tabs (Photoshop's Fill popover, PowerPoint's Picture fill); a
    // chip without their editors is PhotoCraft's.
    pub fn show(self, ui: &mut Ui) -> EditResponse<Paint> {
        let Self { id_salt, label, paint, fallback, numbers, none_allowed, gradient, pattern, picture, enabled } = self;
        let ctx = ui.ctx().clone();
        let labels = PaintLabels::of(ui);
        let tokens = Tokens::of(ui);
        let enabled = enabled && ui.is_enabled();
        let chip = tokens.component.paint_chip;
        let style = Swatch::style(&tokens, craft_theme::ControlSize::Md);

        let solid = match paint {
            Paint::Solid(color) => *color,
            _ => fallback,
        };
        let well = match paint {
            Paint::Solid(color) => Swatch::new((&id_salt, "chip"), label.as_str(), color.to_rgba()),
            _ => Swatch::new((&id_salt, "chip"), label.as_str(), solid.to_rgba()).mixed(true),
        };
        let response = ui.add(well.dimensions(chip.size[0], chip.size[1]).enabled(enabled));
        if !matches!(paint, Paint::Solid(_)) {
            let mut painter = ui.painter().clone();
            painter.set_clip_rect(painter.clip_rect().intersect(response.rect.expand(1.0)));
            paint_paint(&painter, response.rect, paint, &style, &tokens, enabled, ui.pixels_per_point());
        }
        let last_solid = response.id.with("last-solid");
        if let Paint::Solid(color) = paint {
            ctx.data_mut(|d| d.insert_temp(last_solid, *color));
        }
        let remembered = ctx.data(|d| d.get_temp::<Srgba>(last_solid)).unwrap_or(fallback);
        let value = match paint {
            Paint::None => labels.no_color.clone(),
            Paint::Solid(color) => Swatch::hex(color.to_rgba()),
            Paint::Gradient(_) => labels.gradient.clone(),
            Paint::Pattern(_) | Paint::Picture(_) => image_value(&PaintKindLabels::of(ui), paint),
        };
        ctx.accesskit_node_builder(response.id, |node| node.set_value(value));

        let state_id = response.id.with(STATE);
        let stored = ctx.data(|d| d.get_temp::<Open>(state_id)).filter(|_| enabled).unwrap_or_default();
        let was_open = stored.open;
        let mut open = was_open;
        // A frame may run in two passes; the click shows in both.
        let frame = ctx.cumulative_frame_nr();
        if response.clicked() && enabled {
            if stored.clicked != Some(frame) {
                open = !open;
            }
            if open {
                // The key that opened the popup is not the picker's.
                ui.input_mut(|i| {
                    i.consume_key(Modifiers::NONE, Key::Enter);
                    i.consume_key(Modifiers::NONE, Key::Space);
                });
            }
        }
        let clicked = if response.clicked() { Some(frame) } else { stored.clicked };

        let mut event: Option<EditEvent<Paint>> = None;
        let current = Tab::of(paint);
        let popup = egui::Popup::menu(&response);
        let popup_id = popup.get_id();
        let (has_gradient, has_pattern, has_picture) = (gradient.is_some(), pattern.is_some(), picture.is_some());
        let (mut gradient, mut pattern, mut picture) = (gradient, pattern, picture);
        let kind_labels = PaintKindLabels::of(ui);
        PopupList::with_popup(popup, ui, &response)
            .open(&mut open)
            .focus_owners(&[response.id])
            .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
            .height(f32::INFINITY)
            .surface(Surface::style(&tokens, SurfaceKind::Popup))
            .show_ui(|ui| {
                let mut tab = current;
                let mut tabs = Segmented::new((&id_salt, "tabs"), label.as_str(), &mut tab);
                if none_allowed {
                    tabs = tabs.item(Tab::None, labels.no_color.as_str());
                }
                tabs = tabs.item(Tab::Solid, labels.solid.as_str());
                if has_gradient {
                    tabs = tabs.item(Tab::Gradient, labels.gradient.as_str());
                }
                if has_pattern {
                    tabs = tabs.item(Tab::Pattern, kind_labels.pattern.as_str());
                }
                if has_picture {
                    tabs = tabs.item(Tab::Picture, kind_labels.picture.as_str());
                }
                let tabs = tabs.enabled(enabled).show(ui);
                if tabs.changed() && tab != current {
                    let next = match tab {
                        Tab::None => Paint::None,
                        Tab::Solid => Paint::Solid(remembered),
                        Tab::Gradient => Paint::Gradient(gradient.as_ref().map(|(g, _)| g.clone()).unwrap_or_default()),
                        Tab::Pattern => Paint::Pattern(pattern.as_ref().map(|(p, _)| p.clone()).unwrap_or_default()),
                        Tab::Picture => Paint::Picture(picture.as_ref().map(|(p, _)| p.clone()).unwrap_or_default()),
                    };
                    event = Some(EditEvent::set(next, Modifiers::NONE));
                }
                match paint {
                    Paint::None => {}
                    Paint::Solid(color) => {
                        let picker_id = Interactive::id_in(ui, (&id_salt, "picker"));
                        let screen_label = ScreenPickerLabels::of(ui).button;
                        let mut picker =
                            ColorPicker::new((&id_salt, "picker"), label.as_str(), *color, numbers).layout(ColorPickerLayout::Compact).alpha(true);
                        if ScreenColorPicker::available(&ctx) {
                            picker = picker.eyedropper(screen_label);
                        }
                        let shown = picker.enabled(enabled).show(ui);
                        let sampler = shown.edit.response.id;
                        if let Some(button) = &shown.eyedropper
                            && let Some(sampled) = ScreenColorPicker::hold_drag(ui, button, sampler)
                        {
                            ColorPicker::sampled(&ctx, sampler, sampled);
                        }
                        let waiting = picker_id.with("screen-sample");
                        if shown.eyedropper_requested {
                            ScreenColorPicker::open(&ctx, sampler);
                            ctx.data_mut(|d| d.insert_temp(waiting, true));
                        }
                        if ctx.data(|d| d.get_temp::<bool>(waiting)).unwrap_or(false) {
                            if let Some(sampled) = ScreenColorPicker::take(&ctx, sampler) {
                                ColorPicker::sampled(&ctx, sampler, sampled);
                                ctx.data_mut(|d| d.remove::<bool>(waiting));
                            } else if !ScreenColorPicker::busy(&ctx) {
                                ctx.data_mut(|d| d.remove::<bool>(waiting));
                            }
                        }
                        if event.is_none() {
                            event = shown.edit.event.map(|e| e.map(|picked| Paint::Solid(picked.color)));
                        }
                    }
                    Paint::Gradient(_) => {
                        if let Some((_, editor)) = gradient.as_mut()
                            && let Some(edited) = editor(ui)
                            && event.is_none()
                        {
                            event = Some(EditEvent::set(Paint::Gradient(edited), Modifiers::NONE));
                        }
                    }
                    Paint::Pattern(_) | Paint::Picture(_) => {
                        let (editor, kind) = match paint {
                            Paint::Pattern(_) => (pattern.as_mut(), Paint::Pattern as fn(ImagePreview) -> Paint),
                            _ => (picture.as_mut(), Paint::Picture as fn(ImagePreview) -> Paint),
                        };
                        if let Some((_, editor)) = editor
                            && let Some(edited) = editor(ui)
                            && event.is_none()
                        {
                            event = Some(EditEvent::set(kind(edited), Modifiers::NONE));
                        }
                    }
                }
            });
        ctx.data_mut(|d| d.insert_temp(state_id, Open { open, clicked }));
        if was_open && !open && focus_free_or_inside(&ctx, popup_id) {
            ctx.memory_mut(|m| m.request_focus(response.id));
        }
        ctx.accesskit_node_builder(response.id, |node| {
            node.set_has_popup(HasPopup::Dialog);
            node.set_expanded(open);
        });
        if open {
            ctx.accesskit_node_builder(popup_id, |node| node.set_role(egui::accesskit::Role::Dialog));
        }
        EditResponse::new(response, event)
    }
}

const STATE: &str = "craft_color::paint_chip";

/// The chip's popup between frames.
#[derive(Clone, Copy, Default)]
struct Open {
    open: bool,
    /// The frame of the last click on the chip.
    clicked: Option<u64>,
}

/// The popup's tabs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tab {
    None,
    Solid,
    Gradient,
    Pattern,
    Picture,
}

impl Tab {
    fn of(paint: &Paint) -> Tab {
        match paint {
            Paint::None => Tab::None,
            Paint::Solid(_) => Tab::Solid,
            Paint::Gradient(_) => Tab::Gradient,
            Paint::Pattern(_) => Tab::Pattern,
            Paint::Picture(_) => Tab::Picture,
        }
    }
}

/// Whether no control has the focus, or the focused one is in the popup `popup`: a close that
/// leaves the focus there gives it back to the chip.
fn focus_free_or_inside(ctx: &egui::Context, popup: Id) -> bool {
    let Some(focused) = ctx.memory(|m| m.focused()) else { return true };
    ctx.read_response(focused).is_none_or(|r| r.layer_id == LayerId::new(Order::Foreground, popup))
}

/// Draws `paint` in `rect` as a chip: a solid paint over the checkerboard where it has alpha, a
/// gradient as a left-to-right ramp through its stops over the checkerboard, No Color as the
/// field fill with a diagonal in `status.bad.fg` from bottom left to top right; then the edge.
fn paint_paint(painter: &Painter, rect: Rect, paint: &Paint, style: &SwatchStyle, tokens: &UiTokens, enabled: bool, ppp: f32) {
    let outline = |painter: &Painter| painter.extend(Swatch::outline(style, rect, false, enabled, ppp));
    match paint {
        Paint::Solid(color) => painter.extend(Swatch::shapes(style, rect, Some(color.to_rgba()), false, enabled, ppp)),
        Paint::Gradient(preview) => {
            painter.extend(checkerboard(rect, style.radius, style.checker, style.checker_cell));
            if !preview.stops.is_empty() {
                painter.add(ramp(rect, preview));
            }
            outline(painter);
        }
        Paint::None => {
            painter.rect_filled(rect, style.radius, tokens.component.field.fill.get(enabled).to_egui());
            let (inset, line) = (tokens.component.paint_chip.none_inset, tokens.component.paint_chip.none_line);
            let inner = rect.shrink(inset);
            painter.line_segment([inner.left_bottom(), inner.right_top()], Stroke::new(line, tokens.color.status.bad.fg.to_egui()));
            outline(painter);
        }
        Paint::Pattern(preview) | Paint::Picture(preview) => paint_image(painter, rect, paint, preview, style, tokens, enabled, ppp),
    }
}

/// Draws a pattern tiled across `rect` at the tile's pixel size, or a picture stretched to it,
/// over the checkerboard; the field fill while the preview has no texture; then the edge.
#[expect(clippy::too_many_arguments, reason = "the chip painter's arguments plus the preview")]
fn paint_image(
    painter: &Painter,
    rect: Rect,
    paint: &Paint,
    preview: &ImagePreview,
    style: &SwatchStyle,
    tokens: &UiTokens,
    enabled: bool,
    ppp: f32,
) {
    // inv-deckcraft: crates/ui-egui/src/panels/fillui.rs:478-515 (pattern cells painted in their
    // colours). new: one textured quad per tile, so the texture needs no repeat wrapping.
    painter.extend(checkerboard(rect, style.radius, style.checker, style.checker_cell));
    match preview.texture {
        Some(texture) => {
            let mut mesh = Mesh::with_texture(texture);
            let tile = vec2(preview.size[0], preview.size[1]) / ppp;
            let tiled = matches!(paint, Paint::Pattern(_)) && tile.x > 0.0 && tile.y > 0.0 && tile.x.is_finite() && tile.y.is_finite();
            let (cols, rows) = if tiled { ((rect.width() / tile.x).ceil(), (rect.height() / tile.y).ceil()) } else { (1.0, 1.0) };
            if cols * rows > MAX_TILES {
                mesh.add_rect_with_uv(rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
            } else if tiled {
                for row in 0..rows as usize {
                    for col in 0..cols as usize {
                        let min = rect.min + vec2(col as f32 * tile.x, row as f32 * tile.y);
                        let cell = Rect::from_min_size(min, tile).intersect(rect);
                        let uv_max = pos2(cell.width() / tile.x, cell.height() / tile.y);
                        mesh.add_rect_with_uv(cell, Rect::from_min_max(pos2(0.0, 0.0), uv_max), Color32::WHITE);
                    }
                }
            } else {
                mesh.add_rect_with_uv(rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
            }
            painter.add(mesh);
        }
        None => {
            painter.rect_filled(rect, style.radius, tokens.component.field.fill.get(enabled).to_egui());
        }
    }
    painter.extend(Swatch::outline(style, rect, false, enabled, ppp));
}

/// The most tiles a pattern chip draws; a tile so small that more would fit is drawn as one
/// stretched image.
const MAX_TILES: f32 = 4096.0;

/// The chip's value for a screen reader: "Pattern: Bubbles", "Picture: photo.png"; the kind alone
/// for an image without a name.
fn image_value(labels: &PaintKindLabels, paint: &Paint) -> String {
    // new: `pattern_named` or `picture_named` with the preview's name.
    let (named, kind, preview) = match paint {
        Paint::Pattern(preview) => (&labels.pattern_named, &labels.pattern, preview),
        Paint::Picture(preview) => (&labels.picture_named, &labels.picture, preview),
        _ => return String::new(),
    };
    if preview.name.is_empty() { kind.clone() } else { fmt(named, &[("name", preview.name.as_str())]) }
}

/// The gradient's stops as a mesh across `rect`, held flat before the first and after the last.
fn ramp(rect: Rect, preview: &GradientPreview) -> Mesh {
    let mut stops: Vec<(f32, Color32)> = preview.stops.iter().map(|(at, color)| (at.clamp(0.0, 1.0), Color32::from(*color))).collect();
    stops.sort_by(|a, b| a.0.total_cmp(&b.0));
    stops.insert(0, (0.0, stops[0].1));
    stops.push((1.0, stops[stops.len() - 1].1));
    let mut mesh = Mesh::default();
    for (i, (at, color)) in stops.iter().enumerate() {
        let x = rect.left() + rect.width() * at;
        mesh.colored_vertex(pos2(x, rect.top()), *color);
        mesh.colored_vertex(pos2(x, rect.bottom()), *color);
        if i > 0 {
            let k = (i as u32) * 2;
            mesh.add_triangle(k - 2, k - 1, k);
            mesh.add_triangle(k - 1, k, k + 1);
        }
    }
    mesh
}

/// How a paint is bound (FigCraft's colour style or variable).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PaintLinkKind {
    Style,
    Variable,
}

/// A row's binding: the style's or variable's name, shown as a badge after the paint's name, with
/// a Detach button when [`PaintListOptions::bind`] is on.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PaintLink {
    pub name: String,
    pub kind: PaintLinkKind,
}

/// What a row shows beyond the chip and the name, by index in the list's paints.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct PaintRow {
    /// The style or variable the paint is bound to.
    pub link: Option<PaintLink>,
}

impl PaintRow {
    /// A row bound to `link`.
    pub fn linked(link: PaintLink) -> Self {
        PaintRow { link: Some(link) }
    }
}

/// The row actions a [`PaintList`] offers; all off by default (the list is then PhotoCraft's).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct PaintListOptions {
    /// A click on a row's chip opens the colour picker on it.
    pub edit: bool,
    /// A hex field per row.
    pub hex: bool,
    /// An opacity field per row (percent).
    pub opacity: bool,
    /// Bind and Detach buttons (styles and variables).
    pub bind: bool,
    /// "Select layers with this color".
    pub select_users: bool,
    /// An Add button under the rows.
    pub add: bool,
    /// A Remove button per row.
    pub remove: bool,
}

impl PaintListOptions {
    /// FigCraft's Selection colors: edit, hex, opacity, bind and select users.
    pub const SELECTION_COLORS: PaintListOptions =
        PaintListOptions { edit: true, hex: true, opacity: true, bind: true, select_users: true, add: false, remove: false };

    /// Whether any row action is on.
    pub fn any(&self) -> bool {
        self.edit || self.hex || self.opacity || self.bind || self.select_users || self.add || self.remove
    }
}

/// A row action of [`PaintList`], by row index.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum PaintListEvent {
    /// The chip's picker edited the paint (as [`PaintChip`] reports).
    Edit {
        row: usize,
        event: EditEvent<Paint>,
    },
    /// A hex typed and committed.
    Hex {
        row: usize,
        color: Srgba,
    },
    /// The opacity field: `Set` when typed, a gesture when scrubbed; 0–1.
    Opacity {
        row: usize,
        event: EditEvent<f32>,
    },
    /// Bind the paint to a style or variable: the app opens its picker.
    Bind {
        row: usize,
    },
    /// Detach the row's style or variable.
    Detach {
        row: usize,
    },
    /// Select the layers that use this paint.
    SelectUsers {
        row: usize,
    },
    Add,
    Remove {
        row: usize,
    },
    /// Renamed in place (also in [`PaintListResponse::renamed`]).
    Rename {
        row: usize,
        name: String,
    },
}

/// Named paints in rows: the chip, then the name; a click on a row selects it, a double-click
/// renames it in place. The Paints list of a document (PhotoCraft's shape paints; FigCraft's
/// and VectorCraft's document colours).
///
/// With [`options`](Self::options) a row also offers FigCraft's Selection colors actions, after
/// the name: a chip click edits the paint, a hex field, an opacity field, the binding's badge with
/// Bind and Detach ([`rows`](Self::rows)), "Select layers with this color", and Remove; Add under
/// the rows. Each reports a `PaintListEvent`; every control is a Tab stop in the row's order.
pub struct PaintList<'a> {
    id_salt: IdSalt,
    label: String,
    paints: &'a [(String, Paint)],
    selected: Option<usize>,
    options: PaintListOptions,
    rows: &'a [PaintRow],
    fallback: Option<Srgba>,
}

/// What [`PaintList::show`] returns.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct PaintListResponse {
    pub selected: Option<usize>,
    /// A row was renamed: its index and new name.
    pub renamed: Option<(usize, String)>,
    /// The row actions this pass, in order (with [`PaintList::options`]); a rename is here too.
    pub events: Vec<PaintListEvent>,
}

impl<'a> PaintList<'a> {
    /// Black: a switch to Solid in a list with no solid paint and no [`fallback`](Self::fallback).
    pub const DEFAULT_SOLID: Srgba = Srgba::rgb(0.0, 0.0, 0.0);

    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, paints: &'a [(String, Paint)]) -> PaintList<'a> {
        PaintList {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            paints,
            selected: None,
            options: PaintListOptions::default(),
            rows: &[],
            fallback: None,
        }
    }

    pub fn selected(mut self, selected: Option<usize>) -> PaintList<'a> {
        self.selected = selected;
        self
    }

    /// The row actions offered; none by default.
    pub fn options(mut self, options: PaintListOptions) -> PaintList<'a> {
        self.options = options;
        self
    }

    /// The solid colour a row's switch to Solid starts from when the row was never solid: the
    /// host's default. Without it, the list's last solid paint, else [`DEFAULT_SOLID`](Self::DEFAULT_SOLID).
    /// A row that was solid before starts from its last solid colour whatever this is.
    pub fn fallback(mut self, color: Srgba) -> PaintList<'a> {
        self.fallback = Some(color);
        self
    }

    /// Each paint's row data (its binding), by index; rows past the end have none.
    pub fn rows(mut self, rows: &'a [PaintRow]) -> PaintList<'a> {
        self.rows = rows;
        self
    }

    // pc: crates/ui-egui/src/tools/shape_paint_ui.rs:64 (the chip per row); rows on
    // `craft_ui::list` with an in-place `TextField` rename.
    pub fn show(self, ui: &mut Ui) -> PaintListResponse {
        if self.options.any() {
            return self.show_with_actions(ui);
        }
        let Self { id_salt, label, paints, selected, .. } = self;
        let tokens = Tokens::of(ui);
        let chip = tokens.component.paint_chip;
        let style = Swatch::style(&tokens, craft_theme::ControlSize::Md);
        let ppp = ui.pixels_per_point();
        let enabled = ui.is_enabled();
        let rows: Vec<RowInfo<usize>> = (0..paints.len()).map(RowInfo::leaf).collect();
        let mut selection = selected.filter(|&i| i < paints.len()).map_or_else(Selection::new, Selection::only);
        let output = List::new((&id_salt, "list"), label, rows.as_slice()).selection(&mut selection).renamable(true).show(ui, |_, &index| {
            let (name, paint) = &paints[index];
            let (style, tokens) = (&style, &tokens);
            ListRow::new(index, name.as_str()).thumb_sized(egui::vec2(chip.size[0], chip.size[1]), move |painter, rect| {
                paint_paint(painter, rect, paint, style, tokens, enabled, ppp);
            })
        });
        let mut response = PaintListResponse { selected, renamed: None, events: Vec::new() };
        for event in output.events {
            match event {
                ListEvent::Selection(change) => response.selected = change.row.or(response.selected),
                ListEvent::Row(RowEvent::Renamed { row, text }) => {
                    response.events.push(PaintListEvent::Rename { row, name: text.clone() });
                    response.renamed = Some((row, text));
                }
                _ => {}
            }
        }
        response
    }

    /// [`show`](Self::show) with row actions: the rows as before plus the trailing items of
    /// [`PaintListOptions`], and the Add button under them.
    fn show_with_actions(self, ui: &mut Ui) -> PaintListResponse {
        // inv-figcraft: crates/ui-egui/src/panels/selection_colors.rs:1-130 (per row a swatch that
        // opens the picker, hex, opacity, style or variable bind and detach, "Select layers with this
        // color").
        // new: `ListRow` trailing items (`HexField`, `NumberField`, `IconButton`s). The chip is a
        // trailing item too (a `PaintChip`, which owns the picker), so a row with `edit` has no
        // painted thumbnail; hex and opacity are for solid paints, and other rows keep their
        // columns empty.
        let Self { id_salt, label, paints, selected, options, rows: row_data, fallback } = self;
        let fallback = fallback
            .or_else(|| paints.iter().rev().find_map(|(_, paint)| if let Paint::Solid(color) = paint { Some(*color) } else { None }))
            .unwrap_or(PaintList::DEFAULT_SOLID);
        let tokens = Tokens::of(ui);
        let labels = PaintListLabels::of(ui);
        let ctx = ui.ctx().clone();
        let numbers = LocaleNumbers::of_ctx(&ctx);
        let chip = tokens.component.paint_chip;
        let style = Swatch::style(&tokens, craft_theme::ControlSize::Md);
        let ppp = ui.pixels_per_point();
        let enabled = ui.is_enabled();
        let side = IconButton::style(&tokens, ControlSize::Md).side;
        let hex_width = TextField::style(&tokens, ControlSize::Md).width;
        let percent = Unit::new("{n}%");
        let percent_width = NumberField::fit_width(ui, ControlSize::Md, &numbers, 0.0..=100.0, 0, &percent);
        let badge = Badge::look(&tokens, BadgeTone::Neutral, BadgeStyle::Soft);
        let badge_font = role_format(&tokens, badge.label, badge.text);
        let acted = RefCell::new(Vec::new());
        let rows: Vec<RowInfo<usize>> = (0..paints.len()).map(RowInfo::leaf).collect();
        let mut selection = selected.filter(|&i| i < paints.len()).map_or_else(Selection::new, Selection::only);
        // The Add button takes the bottom strip first: the list fills the height left over.
        let mut add_clicked = false;
        let mut listed = None;
        ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
            if options.add {
                add_clicked = ui.add(Button::new(labels.add.as_str()).icon(Icon::Plus).enabled(enabled)).clicked();
            }
            ui.with_layout(Layout::top_down(Align::Min), |ui| {
                listed = Some(List::new((&id_salt, "list"), label, rows.as_slice()).selection(&mut selection).renamable(true).show(ui, |_, &row| {
                    let (name, paint) = &paints[row];
                    let (style, tokens, acted, labels, numbers) = (&style, &tokens, &acted, &labels, &*numbers);
                    let solid = match paint {
                        Paint::Solid(color) => Some(*color),
                        _ => None,
                    };
                    let mut item = ListRow::new(row, name.as_str());
                    if let Some(color) = solid {
                        item = item.accessible_name(format!("{name}, {}", Swatch::hex(color.to_rgba())));
                    }
                    if !options.edit {
                        item = item.thumb_sized(vec2(chip.size[0], chip.size[1]), move |painter, rect| {
                            paint_paint(painter, rect, paint, style, tokens, enabled, ppp);
                        });
                    }
                    // The first item added is the outermost, so they go in from the trailing end.
                    if options.remove {
                        item = item.trailing_item(side, move |ui| {
                            if ui.add(IconButton::new(Icon::Minus, labels.remove.as_str()).id_salt((&id_salt, "remove", row))).clicked() {
                                acted.borrow_mut().push(PaintListEvent::Remove { row });
                            }
                        });
                    }
                    if options.select_users {
                        item = item.trailing_item(side, move |ui| {
                            if ui.add(IconButton::new(Icon::Layers, labels.select_users.as_str()).id_salt((&id_salt, "select-users", row))).clicked()
                            {
                                acted.borrow_mut().push(PaintListEvent::SelectUsers { row });
                            }
                        });
                    }
                    let link = row_data.get(row).and_then(|data| data.link.as_ref());
                    if options.bind {
                        item = item.trailing_item(side, move |ui| match link {
                            Some(link) => {
                                let text = match link.kind {
                                    PaintLinkKind::Style => labels.detach_style.as_str(),
                                    PaintLinkKind::Variable => labels.detach_variable.as_str(),
                                };
                                if ui.add(IconButton::new(Icon::Unlink, text).id_salt((&id_salt, "detach", row))).clicked() {
                                    acted.borrow_mut().push(PaintListEvent::Detach { row });
                                }
                            }
                            None => {
                                if ui.add(IconButton::new(Icon::Palette, labels.styles.as_str()).id_salt((&id_salt, "bind", row))).clicked() {
                                    acted.borrow_mut().push(PaintListEvent::Bind { row });
                                }
                            }
                        });
                    }
                    if options.opacity {
                        item = item.trailing_item(percent_width, move |ui| {
                            let Some(color) = solid else { return };
                            let shown = NumberField::new((&id_salt, "opacity", row), labels.opacity.as_str(), f64::from(color.a) * 100.0, numbers)
                                .range(0.0..=100.0)
                                .decimals(0..=0)
                                .unit(Unit::new("{n}%"))
                                .width(percent_width)
                                .enabled(enabled)
                                .show(ui);
                            if let Some(event) = shown.event {
                                acted.borrow_mut().push(PaintListEvent::Opacity { row, event: event.map(|percent| (percent / 100.0) as f32) });
                            }
                        });
                    }
                    if options.hex {
                        item = item.trailing_item(hex_width, move |ui| {
                            let Some(color) = solid else { return };
                            let shown = HexField::new((&id_salt, "hex", row), labels.hex.as_str(), color.to_rgba())
                                .reason(labels.invalid.as_str())
                                .width(hex_width)
                                .policy(EditPolicy::default().on_release_only(true))
                                .enabled(enabled)
                                .show(ui);
                            if let Some(typed) = shown.committed() {
                                // The field has no alpha digits: the colour keeps its alpha.
                                let color = Srgba { a: color.a, ..Srgba::from(*typed) };
                                acted.borrow_mut().push(PaintListEvent::Hex { row, color });
                            }
                        });
                    }
                    if options.edit {
                        item = item.trailing_item(chip.size[0], move |ui| {
                            let fallback = solid.unwrap_or(fallback);
                            let shown =
                                PaintChip::new((&id_salt, "chip", row), labels.edit.as_str(), paint, fallback, numbers).enabled(enabled).show(ui);
                            if let Some(event) = shown.event {
                                acted.borrow_mut().push(PaintListEvent::Edit { row, event });
                            }
                        });
                    }
                    if let Some(link) = link {
                        let text = link.name.clone();
                        let galley = ctx.fonts_mut(|f| f.layout_no_wrap(text.clone(), badge_font.font_id.clone(), badge.text));
                        let width = (galley.size().x + 2.0 * badge.pad_x).max(badge.height).ceil();
                        item = item.optional_item(width, move |ui| {
                            ui.add(Badge::new(text));
                        });
                    }
                    item
                }));
            });
        });
        let mut response = PaintListResponse { selected, renamed: None, events: acted.into_inner() };
        for event in listed.map(|output| output.events).unwrap_or_default() {
            match event {
                ListEvent::Selection(change) => response.selected = change.row.or(response.selected),
                ListEvent::Row(RowEvent::Renamed { row, text }) => {
                    response.events.push(PaintListEvent::Rename { row, name: text.clone() });
                    response.renamed = Some((row, text));
                }
                _ => {}
            }
        }
        if add_clicked {
            response.events.push(PaintListEvent::Add);
        }
        response
    }
}
