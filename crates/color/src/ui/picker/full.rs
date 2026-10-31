//! The Full layout of [`ColorPicker`]: PhotoCraft's Color Picker dialog.

use craft_theme::{ColorModel, ControlSize, HsbModel, OklchModel, RgbModel, Srgba, TypeRole};
use egui::accesskit::Role;
use egui::text::LayoutJob;
use egui::{
    Align, AsIdSalt, Color32, CornerRadius, CursorIcon, Id, IdSalt, Layout, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, Vec2, vec2,
};

use super::{ColorPicker, Inner, Parts, PickerOp, RowPart, channel_name, texts, unit_of};
use crate::ui::{AreaMode, ColorArea, ColorStrip};
use craft_ui::controls::{Button, Checkbox, NumberField, PreparedRadioSet, RadioGroup, RadioSet, Swatch, SwatchStyle, TextField, TooltipExt as _};
use craft_ui::dialog::{Clicked, FooterButtons, focus_inside};
use craft_ui::direction;
use craft_ui::edit::ops;
use craft_ui::icons::{self, Icon, IconSize};
use craft_ui::primitives::{Content, Gap, Interactive, Stack, Text};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf, UiTokens, role_format};

/// A warning the host shows beside the Full picker's compare swatch, as PhotoCraft's out-of-gamut
/// and web-colour warnings: an icon over a chip of the nearest colour that passes. A click on it
/// sets that colour; AccessKit and the tooltip name it by its tip.
#[derive(Clone, Debug)]
pub struct ColorWarning {
    tip: String,
    nearest: Srgba,
    icon: Icon,
}

impl ColorWarning {
    /// A warning that says `tip` (translated: "Out of gamut for printing") and offers `nearest`.
    pub fn new(tip: impl Into<String>, nearest: Srgba) -> Self {
        Self { tip: tip.into(), nearest, icon: Icon::Warning }
    }

    /// A warning that the colour is not a web-safe colour (the cube Photoshop's Color Picker shows),
    /// offering `nearest`, the nearest web colour.
    pub fn web_colors(tip: impl Into<String>, nearest: Srgba) -> Self {
        Self::new(tip, nearest).icon(Icon::WebColors)
    }

    /// The colour that passes, nearest the warned one: what a click on the warning sets.
    pub fn nearest(&self) -> Srgba {
        self.nearest
    }

    /// The icon over the chip; `warning` by default ([`ColorWarning::web_colors`] sets `web_colors`).
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = icon;
        self
    }
}

/// The grid column of a model: HSB and RGB first, then Lab and the host's models, then OKLCH.
fn column_of(model: &dyn ColorModel) -> usize {
    match model.id() {
        id if id == HsbModel.id() || id == RgbModel.id() => 0,
        id if id == OklchModel.id() => 2,
        _ => 1,
    }
}

/// A row whose items line up at their tops, in reading order.
fn top_row<R>(ui: &mut Ui, gap: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    let layout = if direction::is_rtl(ui) { Layout::right_to_left(Align::Min) } else { Layout::left_to_right(Align::Min) };
    ui.allocate_ui_with_layout(vec2(ui.available_width(), 0.0), layout, |ui| {
        ui.spacing_mut().item_spacing.x = gap;
        add(ui)
    })
    .inner
}

/// `add` in a box `width` wide and `height` high, whatever it takes.
fn sized<R>(ui: &mut Ui, size: Vec2, layout: Layout, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.allocate_ui_with_layout(size, layout, |ui| {
        ui.set_min_size(size);
        add(ui)
    })
    .inner
}

/// The size of `text` in `role`.
fn text_size(ui: &Ui, tokens: &UiTokens, role: TypeRole, text: &str) -> Vec2 {
    ui.painter().layout_job(LayoutJob::single_section(text.to_owned(), role_format(tokens, role, Color32::PLACEHOLDER))).size()
}

/// The width of `text` in `role`.
fn text_width(ui: &Ui, tokens: &UiTokens, role: TypeRole, text: &str) -> f32 {
    text_size(ui, tokens, role, text).x
}

/// The height of the compare swatch's caption above the "new" half.
fn compare_caption_height(ui: &Ui, tokens: &UiTokens) -> f32 {
    text_size(ui, tokens, TypeRole::Caption, "new").y
}

/// Between the swatch's top and the first warning's icon.
const WARNINGS_INSET: f32 = 2.0;

impl ColorPicker<'_> {
    /// The Full layout: the square area and the strip, with the alpha row and the eyedropper
    /// button under them; beside them the compare swatch and the warnings over the channel grid.
    /// The radios write `mode` before the area draws. Whether the eyedropper button was pressed,
    /// and the side action clicked.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn full(
        &self,
        ui: &mut Ui,
        id: Id,
        mode: &mut AreaMode,
        models: &[&dyn ColorModel],
        inner: Inner,
        translate: &dyn Fn(&'static str) -> String,
        parts: &mut Parts<'_>,
        opened: bool,
    ) -> (bool, Option<Clicked>) {
        let tokens = Tokens::of(ui);
        let style = ColorArea::style(&tokens);
        let gap = Gap::Inline.points(&tokens);
        let offered: Vec<AreaMode> =
            AreaMode::PHOTOSHOP.into_iter().chain(AreaMode::OKLCH).filter(|m| models.iter().any(|x| x.id() == m.model().id())).collect();
        if let Some(first) = offered.first()
            && !offered.contains(mode)
        {
            *mode = *first;
        }
        let mut set = RadioSet::new("modes", translate(texts::FIELD_SHOWS), mode).enabled(inner.enabled);
        for m in &offered {
            set = set.item(*m);
        }
        let mut radios = set.begin(ui);
        let mode = *mode;

        let mut requested = false;
        let mut side_clicked = None;
        let mut keys = None;
        top_row(ui, gap, |ui| {
            let side = style.area + gap + ColorStrip::outer_width(&style, style.strip_full);
            ui.allocate_ui_with_layout(vec2(side, 0.0), Layout::top_down_justified(Align::Min), |ui| {
                ui.set_width(side);
                let visible: Vec<&dyn ColorModel> = models.iter().copied().chain([mode.model()]).collect();
                self.area_and_strip(ui, mode, &visible, inner, translate, parts, Vec2::splat(style.area), style.strip_full);
                if self.web_colors.is_some() {
                    only_web_colors(ui, inner.enabled, translate, parts);
                }
                if self.alpha {
                    self.alpha_row(ui, inner, translate, parts);
                }
                if let Some(label) = &self.eyedropper {
                    ui.with_layout(direction::column(ui), |ui| {
                        let button =
                            Button::new(label.as_str()).icon(Icon::Eyedropper).size(ControlSize::Sm).enabled(inner.enabled && !self.eyedropper_busy);
                        requested = ui.add(button).tooltip(translate(texts::SAMPLE_SCREEN)).clicked();
                    });
                }
            });
            let grid_width = self.grid_width(ui, models, inner, translate);
            // The side actions end at the content's trailing edge, whatever width the host gives;
            // an unbounded host gets the grid's width.
            let trailing = if ui.available_width().is_finite() { ui.available_width().max(grid_width) } else { grid_width };
            ui.with_layout(direction::column(ui), |ui| {
                ui.spacing_mut().item_spacing.y = tokens.component.color_picker.grid_gap;
                if self.compare.is_some() || !self.warnings.is_empty() || !self.side_actions.is_empty() {
                    top_row(ui, gap, |ui| {
                        if let Some(current) = self.compare {
                            compare(ui, &tokens, current, inner.enabled, translate, parts);
                        }
                        // Beside the swatch, level with its top, not its caption's.
                        Stack::new(Gap::Stack).show(ui, |ui| {
                            if self.compare.is_some() {
                                ui.add_space(compare_caption_height(ui, &tokens) + WARNINGS_INSET);
                            }
                            for (i, warning) in self.warnings.iter().enumerate() {
                                show_warning(ui, i, warning, inner.enabled, parts);
                            }
                        });
                        if !self.side_actions.is_empty() {
                            let buttons = FooterButtons::new((id, "side-actions"), self.side_actions.clone());
                            let width = buttons.column_width(ui);
                            ui.add_space((trailing - ui.min_rect().width() - width - gap).max(0.0));
                            ui.allocate_ui_with_layout(vec2(width, 0.0), Layout::top_down(Align::Min), |ui| {
                                ui.set_width(width);
                                ui.add_enabled_ui(inner.enabled, |ui| {
                                    let shown = buttons.place_column(ui, id.with("side-actions"));
                                    side_clicked = shown.clicked;
                                    keys = Some(shown);
                                });
                            });
                        }
                    });
                }
                self.grid(ui, id, &mut radios, &offered, models, inner, translate, parts);
            });
        });
        let _ = radios.end(ui);
        // Esc and a field's Enter take the focus away within the pass, so the keys go by where the
        // focus was when the pass began, as a modeless dialog's do.
        let inside_id = id.with("side-keys-inside");
        let inside_before = ui.data(|d| d.get_temp::<bool>(inside_id)).unwrap_or(false);
        let inside_now = focus_inside(ui.ctx(), ui.layer_id());
        ui.data_mut(|d| d.insert_temp(inside_id, inside_now));
        if let Some(shown) = keys.filter(|_| inner.enabled) {
            let buttons = FooterButtons::new((id, "side-actions"), self.side_actions.clone());
            // As a dialog's: the top modal answers whatever has the focus, a layer under a modal
            // answers nothing, and any other layer answers with the focus inside.
            let layer = ui.layer_id();
            let active = ui.memory(|m| m.allows_interaction(layer) && (m.top_modal_layer() == Some(layer) || inside_before));
            side_clicked = side_clicked.or(buttons.key_click(ui.ctx(), &shown, active, opened));
        }
        (requested, side_clicked)
    }

    /// The channel grid: every model's channels, in columns ([`column_of`]), each row a radio
    /// with the short label (a plain label on a host model's row) and the number field; "#" and
    /// the hex field end the first column.
    #[allow(clippy::too_many_arguments)]
    fn grid(
        &self,
        ui: &mut Ui,
        id: Id,
        radios: &mut PreparedRadioSet<AreaMode>,
        offered: &[AreaMode],
        models: &[&dyn ColorModel],
        inner: Inner,
        translate: &dyn Fn(&'static str) -> String,
        parts: &mut Parts<'_>,
    ) {
        let tokens = Tokens::of(ui);
        let radio = RadioGroup::style(&tokens, ControlSize::Md);
        let field = TextField::style(&tokens, ControlSize::Md);
        let gap = Gap::Inline.points(&tokens);
        let (row_gap, group_gap) = (tokens.component.color_picker.grid_row_gap, tokens.component.color_picker.grid_group_gap);
        let height = field.height.max(radio.height);
        let columns = self.grid_columns(ui, models, translate);
        let hex_width = self.hex_width(ui, &tokens);
        top_row(ui, Gap::Section.points(&tokens), |ui| {
            for GridColumn { index: c, column, labels, cell } in &columns {
                let cell = *cell;
                ui.with_layout(direction::column(ui), |ui| {
                    ui.spacing_mut().item_spacing.y = row_gap;
                    for (g, (model, labels)) in column.iter().zip(labels).enumerate() {
                        if g > 0 {
                            ui.add_space((group_gap - row_gap).max(0.0));
                        }
                        for (i, (ch, label)) in model.channels().iter().zip(labels).enumerate() {
                            let value = parts.value().values_in(*model).1[i];
                            let name = channel_name(*model, i, models, translate);
                            let index = offered.iter().position(|m| m.model().id() == model.id() && m.strip() == i);
                            let edit = channel_row(
                                ui,
                                cell,
                                inner.width,
                                height,
                                gap,
                                |ui| match index {
                                    Some(index) => {
                                        let _ = radios.radio_named(ui, index, label, &name);
                                    }
                                    None => {
                                        ui.add_space(radio.diameter + radio.gap);
                                        ui.add(Text::new(label.as_str()).role(radio.label));
                                    }
                                },
                                |ui| {
                                    NumberField::new(("full", model.id(), i), &name, value, self.numbers)
                                        .range(f64::from(ch.min)..=f64::from(ch.max))
                                        .step(f64::from(ch.step))
                                        .decimals(0..=usize::from(ch.decimals))
                                        .unit(unit_of(ch.unit, translate))
                                        .policy(inner.policy)
                                        .part(true)
                                        .enabled(inner.enabled)
                                        .show(ui)
                                },
                            );
                            let constrain = parts.constrain;
                            let ops = ops::take(ui.ctx(), edit.response.id);
                            let ops = ops.into_iter().map(|op| PickerOp::Channel { model: model.id(), i, part: RowPart::Field, op });
                            parts.typing(&edit, ops, |x, now| now.with_channel(*model, i, x, constrain));
                        }
                    }
                    if *c == 0 {
                        ui.add_space((gap - row_gap).max(0.0));
                        channel_row(
                            ui,
                            cell,
                            hex_width,
                            height,
                            gap,
                            |ui| {
                                ui.add_space(radio.diameter + radio.gap);
                                ui.add(Text::new("#").role(TypeRole::Mono).content(Content::Muted));
                            },
                            |ui| self.hex_field(ui, id, hex_width, translate, inner.enabled, parts),
                        );
                    }
                });
            }
        });
    }
}

impl ColorPicker<'_> {
    /// The grid's columns with their labels and label cells ([`GridColumn`]); column 0 (HSB and
    /// RGB, then "#" and the hex field) is always there.
    fn grid_columns<'m>(&self, ui: &Ui, models: &[&'m dyn ColorModel], translate: &dyn Fn(&'static str) -> String) -> Vec<GridColumn<'m>> {
        let tokens = Tokens::of(ui);
        let radio = RadioGroup::style(&tokens, ControlSize::Md);
        let mut columns: [Vec<&'m dyn ColorModel>; 3] = Default::default();
        for model in models {
            columns[column_of(*model)].push(*model);
        }
        columns
            .into_iter()
            .enumerate()
            .filter(|(c, column)| *c == 0 || !column.is_empty())
            .map(|(index, column)| {
                let labels: Vec<Vec<String>> = column
                    .iter()
                    .map(|m| m.channels().iter().map(|ch| translate(texts::GRID_LABEL).replace("{label}", &translate(ch.label))).collect())
                    .collect();
                let mut cell = labels.iter().flatten().map(|l| text_width(ui, &tokens, radio.label, l)).fold(0.0, f32::max);
                if index == 0 {
                    cell = cell.max(text_width(ui, &tokens, TypeRole::Mono, "#"));
                }
                GridColumn { index, column, labels, cell: radio.diameter + radio.gap + cell }
            })
            .collect()
    }

    /// The hex field's width in the grid: "#" and six digits, or eight, as much as the field takes.
    fn hex_width(&self, ui: &Ui, tokens: &UiTokens) -> f32 {
        let field = TextField::style(tokens, ControlSize::Md);
        (2.0 * field.pad_x + text_width(ui, tokens, TypeRole::Mono, if self.alpha { "#00000000" } else { "#000000" })).ceil()
    }

    /// The width the grid takes, which the compare swatch's row above lines its trailing edge up with.
    fn grid_width(&self, ui: &Ui, models: &[&dyn ColorModel], inner: Inner, translate: &dyn Fn(&'static str) -> String) -> f32 {
        let tokens = Tokens::of(ui);
        let gap = Gap::Inline.points(&tokens);
        let hex = self.hex_width(ui, &tokens);
        let columns = self.grid_columns(ui, models, translate);
        let widths = columns.iter().map(|c| c.cell + gap + if c.index == 0 { inner.width.max(hex) } else { inner.width });
        widths.sum::<f32>() + Gap::Section.points(&tokens) * columns.len().saturating_sub(1) as f32
    }
}

/// One column of the channel grid: its models, their labels, and the width of its label cell.
struct GridColumn<'m> {
    index: usize,
    column: Vec<&'m dyn ColorModel>,
    labels: Vec<Vec<String>>,
    cell: f32,
}

/// The "Only Web Colors" check box; switching it on sets the snapped colour.
fn only_web_colors(ui: &mut Ui, enabled: bool, translate: &dyn Fn(&'static str) -> String, parts: &mut Parts<'_>) {
    let mut on = parts.web.get();
    let changed = Checkbox::bool(translate(texts::ONLY_WEB_COLORS), &mut on).id_salt("only-web-colors").enabled(enabled).show(ui).response.changed();
    if changed {
        parts.web.set(on);
        let color = parts.value().color;
        if on && !(parts.constrain)(color).same_rgb8(color) {
            parts.edits.input(super::Input::Click(color));
        }
    }
}

/// One grid row: `label` in a cell `cell` wide at the reading start, then `field` in a box
/// `width` wide.
fn channel_row<R>(ui: &mut Ui, cell: f32, width: f32, height: f32, gap: f32, label: impl FnOnce(&mut Ui), field: impl FnOnce(&mut Ui) -> R) -> R {
    sized(ui, vec2(cell + gap + width, height), direction::row(ui), |ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        sized(ui, vec2(cell, height), direction::row(ui), label);
        ui.add_space(gap);
        sized(ui, vec2(width, height), Layout::top_down_justified(Align::Min), field)
    })
}

/// The compare swatch: the colour being edited ("new") over `current`, with their captions; a
/// click on `current` sets it.
fn compare(ui: &mut Ui, tokens: &UiTokens, current: Srgba, enabled: bool, translate: &dyn Fn(&'static str) -> String, parts: &mut Parts<'_>) {
    let t = &tokens.component.color_picker;
    let new = parts.value().color;
    let swatch = Swatch::style(tokens, ControlSize::Md);
    ui.allocate_ui_with_layout(vec2(t.compare_width, 0.0), Layout::top_down(Align::Center), |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        ui.add(Text::new(translate(texts::NEW)).role(TypeRole::Caption).content(Content::Muted));
        let (rect, _) = ui.allocate_exact_size(vec2(t.compare_width, t.compare_height), Sense::hover());
        let (new_rect, current_rect) = (rect.split_top_bottom_at_fraction(0.5).0, rect.split_top_bottom_at_fraction(0.5).1);
        let _ = Interactive::new("compare-new", Role::Image, translate(texts::NEW_COLOUR))
            .sense(Sense::hover())
            .value(Swatch::hex(new.to_rgba()))
            .interact_at(ui, new_rect)
            .finish(ui);
        let restore = translate(texts::RESTORE);
        let response = Interactive::new("compare-current", Role::Button, restore.as_str())
            .value(Swatch::hex(current.to_rgba()))
            .state_layer(false)
            .enabled(enabled)
            .interact_at(ui, current_rect)
            .finish(ui);
        if ui.is_rect_visible(rect) {
            let mut shapes = Vec::new();
            fill(&mut shapes, &swatch, new_rect, new);
            fill(&mut shapes, &swatch, current_rect, current);
            let edge = if enabled { swatch.edge } else { swatch.edge.gamma_multiply(0.5) };
            shapes.push(Shape::Rect(egui::epaint::RectShape::stroke(rect, CornerRadius::ZERO, Stroke::new(1.0, edge), StrokeKind::Outside)));
            ui.painter().extend(shapes);
        }
        let response = response.tooltip(restore);
        if response.clicked() {
            parts.edits.input(super::Input::Click(current));
        }
        if enabled {
            let _ = response.on_hover_cursor(CursorIcon::PointingHand);
        }
        ui.add(Text::new(translate(texts::CURRENT)).role(TypeRole::Caption).content(Content::Muted));
    });
}

/// `rect` filled with `color`, over the swatch checkerboard where it is translucent.
fn fill(shapes: &mut Vec<Shape>, swatch: &SwatchStyle, rect: Rect, color: Srgba) {
    let color = color.to_rgba();
    if color.a() < 255 {
        shapes.push(Shape::rect_filled(rect, CornerRadius::ZERO, swatch.checker[0]));
        let cell = rect.height() / 4.0;
        let cols = (rect.width() / cell).ceil() as usize;
        for row in 0..4 {
            for col in (0..cols).filter(|col| (row + col) % 2 == 1) {
                let min = rect.min + vec2(col as f32 * cell, row as f32 * cell);
                let cell = Rect::from_min_size(min, Vec2::splat(cell)).intersect(rect);
                shapes.push(Shape::rect_filled(cell, CornerRadius::ZERO, swatch.checker[1]));
            }
        }
    }
    shapes.push(Shape::rect_filled(rect, CornerRadius::ZERO, color.to_egui()));
}

/// A [`ColorWarning`] as the Full picker draws it, for a host that draws it beside its own
/// swatch (PhotoCraft's Color panel): the warning's icon over a chip of the nearest passing
/// colour, one button. The host applies [`ColorWarning::nearest`] when the response is clicked.
///
/// ```
/// use craft_theme::Srgba;
/// use craft_color::ui::{ColorWarning, ColorWarningChip};
/// # egui::__run_test_ui(|ui| { let mut color = Srgba::rgb(1.0, 0.0, 1.0);
/// let warning = ColorWarning::new("Out of gamut for printing", Srgba::rgb(0.9, 0.1, 0.9));
/// if ColorWarningChip::new("warning", warning.clone()).show(ui).clicked() {
///     color = warning.nearest();
/// }
/// # });
/// ```
///
/// AccessKit: a `Button` named by the warning's tip, with the nearest colour's hex as its value.
/// The tip is also the tooltip.
#[derive(Clone, Debug)]
#[must_use = "show the chip with `show`"]
pub struct ColorWarningChip {
    id_salt: IdSalt,
    warning: ColorWarning,
    enabled: bool,
}

impl ColorWarningChip {
    /// A chip for `warning`; `id_salt` keeps its state apart from another chip in the same `Ui`.
    pub fn new(id_salt: impl AsIdSalt, warning: ColorWarning) -> Self {
        Self { id_salt: IdSalt::new(id_salt), warning, enabled: true }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Shows the chip; the response covers the icon and the chip.
    pub fn show(self, ui: &mut Ui) -> Response {
        let tokens = Tokens::of(ui);
        let t = &tokens.component.color_picker;
        let warning = &self.warning;
        let enabled = self.enabled && ui.is_enabled();
        let gap = Gap::Inline.points(&tokens);
        let size = vec2(t.warning_icon.max(t.warning_chip), t.warning_icon + gap + t.warning_chip);
        let response = Interactive::salted(self.id_salt, Role::Button, warning.tip.as_str())
            .value(Swatch::hex(warning.nearest.to_rgba()))
            .state_layer(false)
            .enabled(enabled)
            .show(ui, size);
        if ui.is_rect_visible(response.rect) {
            let rect = response.rect;
            let icon = Rect::from_center_size(egui::pos2(rect.center().x, rect.top() + t.warning_icon / 2.0), Vec2::splat(t.warning_icon));
            let color = Content::Normal.color(&tokens).to_egui();
            let color = if enabled { color } else { color.gamma_multiply(0.5) };
            icons::paint_in(ui, icon, warning.icon, icons::active(ui.ctx()), t.warning_icon, IconSize::Sm.stroke(), color);
            let chip = Rect::from_center_size(egui::pos2(rect.center().x, rect.bottom() - t.warning_chip / 2.0), Vec2::splat(t.warning_chip));
            let swatch = Swatch::style(&tokens, ControlSize::Md);
            let mut shapes = Vec::new();
            fill(&mut shapes, &swatch, chip, warning.nearest);
            let edge = if enabled { swatch.edge } else { swatch.edge.gamma_multiply(0.5) };
            shapes.push(Shape::Rect(egui::epaint::RectShape::stroke(chip, CornerRadius::ZERO, Stroke::new(1.0, edge), StrokeKind::Outside)));
            ui.painter().extend(shapes);
        }
        let response = response.tooltip(warning.tip.as_str());
        if enabled {
            return response.on_hover_cursor(CursorIcon::PointingHand);
        }
        response
    }
}

/// A warning: its chip; a click sets the colour.
fn show_warning(ui: &mut Ui, index: usize, warning: &ColorWarning, enabled: bool, parts: &mut Parts<'_>) {
    let response = ColorWarningChip::new(("warning", index), warning.clone()).enabled(enabled).show(ui);
    if response.clicked() {
        parts.edits.input(super::Input::Click(warning.nearest));
    }
}

#[cfg(test)]
mod warning_tests {
    use super::*;

    #[test]
    fn the_web_colour_warning_shows_the_cube_and_the_gamut_warning_the_triangle() {
        let nearest = Srgba::rgb(0.2, 0.4, 0.6);
        assert_eq!(ColorWarning::web_colors("Not a web colour", nearest).icon, Icon::WebColors);
        assert_eq!(ColorWarning::new("Out of gamut for printing", nearest).icon, Icon::Warning);
    }
}
