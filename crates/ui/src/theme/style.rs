//! The egui [`Style`] of a set of tokens, so stock egui widgets look as close to the design
//! system as they can.

use craft_theme::{ButtonColors, Tokens, relative_luminance};
use egui::style::{Selection, TextCursorStyle, WidgetVisuals, Widgets};
use egui::{Stroke, Style, TextStyle, Visuals, vec2};

use super::convert::{FontFamilies, ToEgui, corner_radius, margin, stroke};

/// The egui style for `tokens`. Pure: the same tokens and families give the same style.
///
/// Fields without a matching token keep egui's default (of `Visuals::dark` or `Visuals::light`,
/// by whether the text is lighter than the panel):
/// - `Visuals`: `text_options`, `override_text_color`, `weak_text_alpha`, `text_edit_bg_color`
///   (egui then uses `extreme_bg_color`), `window_highlight_topmost`, `clip_rect_margin`,
///   `button_frame`, `collapsing_header_frame`, `indent_has_left_vline`, `striped`,
///   `slider_trailing_fill`, `handle_shape`, `interact_cursor`, `image_loading_spinners`,
///   `numeric_color_space`, `disabled_alpha`; in `text_cursor` everything but the stroke colour;
///   in `ime_composition` the widths and `legacy_visuals`; the widgets' `fg_stroke` widths.
/// - `Spacing`: `indent`, `interact_size.x`, `slider_width`, `slider_rail_height`, `combo_width`,
///   `text_edit_width`, `extra_text_line_spacing`, `icon_width_inner`, `default_area_size`,
///   `menu_width`, `menu_spacing`, `indent_ends_with_horizontal_line`,
///   `combo_height`, `scroll`.
/// - `Style`: everything outside `visuals`, `spacing`, `text_styles` and `animation_time`
///   (interaction but `tooltip_delay`, wrapping, number formatting, scroll animation, debug
///   options, direction). `TextStyle::Name` styles are not added.
pub fn style_for(tokens: &Tokens, fonts: &FontFamilies) -> Style {
    let mut style = Style { visuals: visuals(tokens), ..Style::default() };

    let t = &tokens.typography;
    style.text_styles = [
        (TextStyle::Small, t.caption),
        (TextStyle::Body, t.body),
        (TextStyle::Button, t.label),
        (TextStyle::Heading, t.heading),
        (TextStyle::Monospace, t.mono),
    ]
    .into_iter()
    .map(|(text_style, type_style)| (text_style, fonts.font_id(type_style)))
    .collect();

    let (space, size, button) = (&tokens.space, &tokens.size, &tokens.component.button);
    let s = &mut style.spacing;
    s.item_spacing = vec2(space.gap.inline, space.gap.stack);
    // The label's line height, centred in the button's height.
    s.button_padding = vec2(button.pad_x.md, ((button.height.md - t.label.line_height) / 2.0).max(0.0));
    s.window_margin = margin(space.pad.dialog);
    s.menu_margin = margin(space.pad.control);
    s.interact_size.y = size.control.md;
    s.icon_width = size.icon.sm;
    s.icon_spacing = space.gap.inline;
    s.tooltip_width = tokens.component.tooltip.max_width;

    style.animation_time = tokens.motion.hover.millis as f32 / 1000.0;
    style.interaction.tooltip_delay = crate::controls::Tooltip::DELAY;
    style
}

/// Whether the text is lighter than the panel: tokens don't name their mode.
fn is_dark(tokens: &Tokens) -> bool {
    relative_luminance(tokens.color.text.normal) > relative_luminance(tokens.color.surface.panel)
}

fn visuals(tokens: &Tokens) -> Visuals {
    let c = &tokens.color;
    let dark = is_dark(tokens);
    let mut v = if dark { Visuals::dark() } else { Visuals::light() };
    let radius = &tokens.shape.radius;
    let button = &tokens.component.button;

    v.dark_mode = dark;
    v.weak_text_color = Some(c.text.muted.to_egui());
    v.widgets = widgets(tokens, &button.secondary);
    v.selection = Selection { bg_fill: c.state.selected.to_egui(), stroke: stroke(1.0, c.text.on_selected) };
    v.ime_composition.active_underline_stroke.color = c.text.normal.to_egui();
    v.ime_composition.inactive_underline_stroke.color = c.text.muted.to_egui();
    v.hyperlink_color = c.accent.text.to_egui();
    v.faint_bg_color = c.surface.raised.to_egui();
    v.extreme_bg_color = c.surface.sunken.to_egui();
    v.code_bg_color = c.surface.sunken.to_egui();
    v.warn_fg_color = c.status.warn.fg.to_egui();
    v.error_fg_color = c.status.bad.fg.to_egui();

    v.window_corner_radius = corner_radius(radius.window);
    v.window_shadow = tokens.elevation.floating.to_egui();
    v.window_fill = c.surface.panel.to_egui();
    v.window_stroke = stroke(1.0, c.line.border);
    v.menu_corner_radius = corner_radius(radius.popup);
    v.panel_fill = c.surface.panel.to_egui();
    v.popup_shadow = super::tokens::popup_shadow(tokens);
    v.resize_corner_size = tokens.size.icon.sm;
    v.text_cursor = TextCursorStyle { stroke: Stroke { color: c.text.normal.to_egui(), ..v.text_cursor.stroke }, ..v.text_cursor };
    v
}

/// egui's widget states take the secondary button's colours: `inactive` its rest state,
/// `hovered` hover, `active` pressed, `open` (a menu button whose menu is open) pressed with the
/// rest text. `noninteractive` is the panel itself: labels, separators, frames.
fn widgets(tokens: &Tokens, colors: &ButtonColors) -> Widgets {
    let c = &tokens.color;
    let button = &tokens.component.button;
    let control = corner_radius(button.radius);
    let defaults = if is_dark(tokens) { Widgets::dark() } else { Widgets::light() };
    let state = |base: WidgetVisuals, fill, border, text| WidgetVisuals {
        bg_fill: fill,
        weak_bg_fill: fill,
        bg_stroke: Stroke::new(button.border_width, border),
        fg_stroke: Stroke { color: text, ..base.fg_stroke },
        corner_radius: control,
        expansion: 0.0,
    };
    let (fill, border, text) = (&colors.fill, &colors.border, &colors.text);
    Widgets {
        noninteractive: WidgetVisuals {
            bg_fill: c.surface.panel.to_egui(),
            weak_bg_fill: c.surface.panel.to_egui(),
            bg_stroke: stroke(1.0, c.line.divider),
            fg_stroke: Stroke { color: c.text.normal.to_egui(), ..defaults.noninteractive.fg_stroke },
            corner_radius: corner_radius(tokens.shape.radius.control),
            expansion: 0.0,
        },
        inactive: state(defaults.inactive, fill.rest.to_egui(), border.rest.to_egui(), text.rest.to_egui()),
        hovered: state(defaults.hovered, fill.hover.to_egui(), border.hover.to_egui(), text.hover.to_egui()),
        active: state(defaults.active, fill.pressed.to_egui(), border.pressed.to_egui(), text.pressed.to_egui()),
        open: state(defaults.open, fill.pressed.to_egui(), border.pressed.to_egui(), text.rest.to_egui()),
    }
}
