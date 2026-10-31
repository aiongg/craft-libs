//! [`PaintChip`] (F04.06): a shape's fill or stroke paint as a chip that opens a popover with No
//! Color, Solid and Gradient, as PhotoCraft's shape options bar and Properties panel; and
//! [`PaintList`], named paints in rows with their chips.
//!
//! The gradient editor is craft-draw's, and craft-draw depends on craft-color, so the chip takes
//! the editor's body from the host ([`PaintChip::gradient_editor`]).

use craft_theme::Srgba;
use egui::accesskit::HasPopup;
use egui::{AsIdSalt, Color32, Id, IdSalt, Key, LayerId, Mesh, Modifiers, Order, Painter, PopupCloseBehavior, Rect, Stroke, Ui, pos2};

use super::{ColorPicker, ColorPickerLayout, ScreenColorPicker, ScreenPickerLabels};
use craft_ui::behavior::{RowInfo, Selection};
use craft_ui::controls::{Segmented, Swatch, SwatchStyle, checkerboard};
use craft_ui::edit::{EditEvent, EditResponse};
use craft_ui::list::{List, ListEvent, ListRow, RowEvent};
use craft_ui::localized::{Labels as _, LocaleNumbers};
use craft_ui::popup_list::PopupList;
use craft_ui::primitives::{Interactive, Surface, SurfaceKind};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, UiTokens};

craft_ui::words! {
    /// The paint popover's tabs, English by default.
    #[non_exhaustive]
    pub struct PaintLabels {
        no_color: "paint.noColor" = "No Color";
        solid: "paint.solid" = "Solid";
        gradient: "paint.gradient" = "Gradient";
    }
}

/// A gradient as the chip draws it: stops at 0–1 along a left-to-right ramp. The gradient itself
/// (angle, style, midpoints) is the host's; it gives this preview with the paint.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GradientPreview {
    pub stops: Vec<(f32, Srgba)>,
}

/// A paint: none, a solid colour, or a gradient.
#[derive(Clone, Debug, PartialEq)]
pub enum Paint {
    None,
    Solid(Srgba),
    Gradient(GradientPreview),
}

/// The host's gradient editor body, drawn in the popover's Gradient tab: it edits the host's
/// gradient and returns the new preview when it changed.
type GradientEditor<'a> = dyn FnMut(&mut Ui) -> Option<GradientPreview> + 'a;

/// A paint chip, 30 by 22 points in PhotoCraft (the `swatch` control's Md size here).
///
/// - Look: a solid paint fills the chip over the checkerboard (alpha shows); a gradient is a
///   left-to-right ramp through its stops; No Color is the field fill with a red diagonal
///   (`status.danger`) from bottom left to top right. Edge: `line.control_edge`, the focus ring
///   when focused.
/// - Click, Enter or Space opens a popover: a segmented No Color / Solid / Gradient switch, then
///   the Compact [`ColorPicker`](super::ColorPicker) (Solid, with the screen eyedropper) or the
///   host's gradient editor (Gradient). Switching to Solid starts from the last solid colour (or
///   the fallback), to Gradient from the host's default gradient
///   ([`default_gradient`](Self::default_gradient)).
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
    enabled: bool,
}

impl<'a> PaintChip<'a> {
    /// A chip for `paint`; `fallback` is the solid colour a switch to Solid starts from when the
    /// paint has none.
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, paint: &'a Paint, fallback: Srgba, numbers: &'a LocaleNumbers) -> PaintChip<'a> {
        PaintChip { id_salt: IdSalt::new(id_salt), label: label.into(), paint, fallback, numbers, none_allowed: true, gradient: None, enabled: true }
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

    pub fn enabled(mut self, enabled: bool) -> PaintChip<'a> {
        self.enabled = enabled;
        self
    }

    /// The gradient a switch to Gradient sets, when the host gave an editor.
    pub fn default_gradient(&self) -> Option<&GradientPreview> {
        self.gradient.as_ref().map(|(g, _)| g)
    }

    // pc: crates/ui-egui/src/tools/shape_paint_ui.rs:64 `swatch` (the chip at :72, the ramp at
    // :80, No Color at :103, the popover tabs at :133); built on `ColorPopover`'s popover and
    // `ColorPicker` Compact, `Segmented` for the tabs.
    pub fn show(self, ui: &mut Ui) -> EditResponse<Paint> {
        let Self { id_salt, label, paint, fallback, numbers, none_allowed, gradient, enabled } = self;
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
            Paint::Solid(color) => Swatch::new(id_salt, label.as_str(), color.to_rgba()),
            _ => Swatch::new(id_salt, label.as_str(), solid.to_rgba()).mixed(true),
        };
        let response = ui.add(well.dimensions(chip.width, chip.height).enabled(enabled));
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
        let has_gradient = gradient.is_some();
        let mut gradient = gradient;
        PopupList::with_popup(popup, ui, &response)
            .open(&mut open)
            .focus_owners(&[response.id])
            .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
            .height(f32::INFINITY)
            .surface(Surface::style(&tokens, SurfaceKind::Popup))
            .show_ui(|ui| {
                let mut tab = current;
                let mut tabs = Segmented::new("tabs", label.as_str(), &mut tab);
                if none_allowed {
                    tabs = tabs.item(Tab::None, labels.no_color.as_str());
                }
                tabs = tabs.item(Tab::Solid, labels.solid.as_str());
                if has_gradient {
                    tabs = tabs.item(Tab::Gradient, labels.gradient.as_str());
                }
                let tabs = tabs.enabled(enabled).show(ui);
                if tabs.changed() && tab != current {
                    let next = match tab {
                        Tab::None => Paint::None,
                        Tab::Solid => Paint::Solid(remembered),
                        Tab::Gradient => Paint::Gradient(gradient.as_ref().map(|(g, _)| g.clone()).unwrap_or_default()),
                    };
                    event = Some(EditEvent::set(next, Modifiers::NONE));
                }
                match paint {
                    Paint::None => {}
                    Paint::Solid(color) => {
                        let picker_id = Interactive::id_in(ui, "picker");
                        let screen_label = ScreenPickerLabels::of(ui).button;
                        let mut picker = ColorPicker::new("picker", label.as_str(), *color, numbers).layout(ColorPickerLayout::Compact).alpha(true);
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
}

impl Tab {
    fn of(paint: &Paint) -> Tab {
        match paint {
            Paint::None => Tab::None,
            Paint::Solid(_) => Tab::Solid,
            Paint::Gradient(_) => Tab::Gradient,
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
            let inner = rect.shrink(2.0);
            painter.line_segment([inner.left_bottom(), inner.right_top()], Stroke::new(1.5, tokens.color.status.bad.fg.to_egui()));
            outline(painter);
        }
    }
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

/// Named paints in rows: the chip, then the name; a click on a row selects it, a double-click
/// renames it in place. The Paints list of a document (PhotoCraft's shape paints; FigCraft's
/// and VectorCraft's document colours).
pub struct PaintList<'a> {
    id_salt: IdSalt,
    label: String,
    paints: &'a [(String, Paint)],
    selected: Option<usize>,
}

/// What [`PaintList::show`] returns.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct PaintListResponse {
    pub selected: Option<usize>,
    /// A row was renamed: its index and new name.
    pub renamed: Option<(usize, String)>,
}

impl<'a> PaintList<'a> {
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, paints: &'a [(String, Paint)]) -> PaintList<'a> {
        PaintList { id_salt: IdSalt::new(id_salt), label: label.into(), paints, selected: None }
    }

    pub fn selected(mut self, selected: Option<usize>) -> PaintList<'a> {
        self.selected = selected;
        self
    }

    // pc: crates/ui-egui/src/tools/shape_paint_ui.rs:64 (the chip per row); rows on
    // `craft_ui::list` with an in-place `TextField` rename.
    pub fn show(self, ui: &mut Ui) -> PaintListResponse {
        let Self { id_salt, label, paints, selected } = self;
        let tokens = Tokens::of(ui);
        let chip = tokens.component.paint_chip;
        let style = Swatch::style(&tokens, craft_theme::ControlSize::Md);
        let ppp = ui.pixels_per_point();
        let enabled = ui.is_enabled();
        let rows: Vec<RowInfo<usize>> = (0..paints.len()).map(RowInfo::leaf).collect();
        let mut selection = selected.filter(|&i| i < paints.len()).map_or_else(Selection::new, Selection::only);
        let output = List::new(id_salt, label, rows.as_slice()).selection(&mut selection).renamable(true).show(ui, |_, &index| {
            let (name, paint) = &paints[index];
            let (style, tokens) = (&style, &tokens);
            ListRow::new(index, name.as_str()).thumb_sized(egui::vec2(chip.width, chip.height), move |painter, rect| {
                paint_paint(painter, rect, paint, style, tokens, enabled, ppp);
            })
        });
        let mut response = PaintListResponse { selected, renamed: None };
        for event in output.events {
            match event {
                ListEvent::Selection(change) => response.selected = change.row.or(response.selected),
                ListEvent::Row(RowEvent::Renamed { row, text }) => response.renamed = Some((row, text)),
                _ => {}
            }
        }
        response
    }
}
