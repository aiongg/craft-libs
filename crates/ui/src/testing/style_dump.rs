//! Text dump of an egui [`Style`]: all of `Visuals`, all of `Spacing`, the text styles and the
//! animation time.
//!
//! The destructuring is exhaustive, so an egui update that adds a `Visuals` or `Spacing` field
//! doesn't compile until the field is dumped.

use egui::style::{
    ImeComposition, KineticScrollStyle, ScrollFadeStyle, ScrollStyle, Selection, Spacing, TextCursorStyle, Visuals, WidgetVisuals, Widgets,
};
use egui::{Style, TextStyle};

use super::dump::Dump;

/// Writes the keyed dump of `style` to `d`. Text styles are keyed by their lowercase name;
/// `TextStyle::Name` styles by `name.<name>`, so a named style never shares a key with a built-in
/// one.
pub fn style_dump_into(d: &mut Dump, style: &Style) {
    d.group("visuals", |d| visuals(d, &style.visuals));
    d.group("spacing", |d| spacing(d, &style.spacing));
    d.group("text_styles", |d| {
        for (text_style, font) in &style.text_styles {
            let key = match text_style {
                TextStyle::Name(name) => format!("name.{name}"),
                built_in => built_in.to_string().to_lowercase(),
            };
            d.line(&key, font);
        }
    });
    d.line("animation_time", style.animation_time);
}

#[allow(deprecated)] // `clip_rect_margin`, named only to skip it
fn visuals(d: &mut Dump, v: &Visuals) {
    let Visuals {
        dark_mode,
        text_options,
        override_text_color,
        weak_text_alpha,
        weak_text_color,
        widgets,
        selection,
        ime_composition,
        hyperlink_color,
        faint_bg_color,
        extreme_bg_color,
        text_edit_bg_color,
        code_bg_color,
        warn_fg_color,
        error_fg_color,
        window_corner_radius,
        window_shadow,
        window_fill,
        window_stroke,
        window_highlight_topmost,
        menu_corner_radius,
        panel_fill,
        popup_shadow,
        resize_corner_size,
        text_cursor,
        clip_rect_margin: _,
        button_frame,
        collapsing_header_frame,
        indent_has_left_vline,
        striped,
        slider_trailing_fill,
        handle_shape,
        interact_cursor,
        image_loading_spinners,
        numeric_color_space,
        disabled_alpha,
    } = v;
    d.line("dark_mode", dark_mode)
        .line("text_options", format!("{text_options:?}"))
        .line("override_text_color", override_text_color)
        .line("weak_text_alpha", weak_text_alpha)
        .line("weak_text_color", weak_text_color);
    let Widgets { noninteractive, inactive, hovered, active, open } = widgets;
    for (name, w) in [("noninteractive", noninteractive), ("inactive", inactive), ("hovered", hovered), ("active", active), ("open", open)] {
        d.group(&format!("widgets.{name}"), |d| widget(d, w));
    }
    let Selection { bg_fill, stroke } = selection;
    d.line("selection.bg_fill", bg_fill).line("selection.stroke", stroke);
    let ImeComposition { active_underline_stroke, inactive_underline_stroke, legacy_visuals } = ime_composition;
    d.line("ime_composition.active_underline_stroke", active_underline_stroke)
        .line("ime_composition.inactive_underline_stroke", inactive_underline_stroke)
        .line("ime_composition.legacy_visuals", legacy_visuals);
    d.line("hyperlink_color", hyperlink_color)
        .line("faint_bg_color", faint_bg_color)
        .line("extreme_bg_color", extreme_bg_color)
        .line("text_edit_bg_color", text_edit_bg_color)
        .line("code_bg_color", code_bg_color)
        .line("warn_fg_color", warn_fg_color)
        .line("error_fg_color", error_fg_color)
        .line("window_corner_radius", window_corner_radius)
        .line("window_shadow", window_shadow)
        .line("window_fill", window_fill)
        .line("window_stroke", window_stroke)
        .line("window_highlight_topmost", window_highlight_topmost)
        .line("menu_corner_radius", menu_corner_radius)
        .line("panel_fill", panel_fill)
        .line("popup_shadow", popup_shadow)
        .line("resize_corner_size", resize_corner_size);
    let TextCursorStyle { stroke, preview, blink, on_duration, off_duration } = text_cursor;
    d.line("text_cursor.stroke", stroke)
        .line("text_cursor.preview", preview)
        .line("text_cursor.blink", blink)
        .line("text_cursor.on_duration", on_duration)
        .line("text_cursor.off_duration", off_duration);
    d.line("button_frame", button_frame)
        .line("collapsing_header_frame", collapsing_header_frame)
        .line("indent_has_left_vline", indent_has_left_vline)
        .line("striped", striped)
        .line("slider_trailing_fill", slider_trailing_fill)
        .line("handle_shape", format!("{handle_shape:?}"))
        .line("interact_cursor", format!("{interact_cursor:?}"))
        .line("image_loading_spinners", image_loading_spinners)
        .line("numeric_color_space", format!("{numeric_color_space:?}"))
        .line("disabled_alpha", disabled_alpha);
}

fn widget(d: &mut Dump, w: &WidgetVisuals) {
    let WidgetVisuals { bg_fill, weak_bg_fill, bg_stroke, corner_radius, fg_stroke, expansion } = w;
    d.line("bg_fill", bg_fill)
        .line("weak_bg_fill", weak_bg_fill)
        .line("bg_stroke", bg_stroke)
        .line("corner_radius", corner_radius)
        .line("fg_stroke", fg_stroke)
        .line("expansion", expansion);
}

fn spacing(d: &mut Dump, s: &Spacing) {
    let Spacing {
        item_spacing,
        window_margin,
        button_padding,
        menu_margin,
        indent,
        interact_size,
        slider_width,
        slider_rail_height,
        combo_width,
        text_edit_width,
        extra_text_line_spacing,
        icon_width,
        icon_width_inner,
        icon_spacing,
        default_area_size,
        tooltip_width,
        menu_width,
        menu_spacing,
        indent_ends_with_horizontal_line,
        combo_height,
        scroll,
    } = s;
    d.line("item_spacing", item_spacing)
        .line("window_margin", window_margin)
        .line("button_padding", button_padding)
        .line("menu_margin", menu_margin)
        .line("indent", indent)
        .line("interact_size", interact_size)
        .line("slider_width", slider_width)
        .line("slider_rail_height", slider_rail_height)
        .line("combo_width", combo_width)
        .line("text_edit_width", text_edit_width)
        .line("extra_text_line_spacing", extra_text_line_spacing)
        .line("icon_width", icon_width)
        .line("icon_width_inner", icon_width_inner)
        .line("icon_spacing", icon_spacing)
        .line("default_area_size", default_area_size)
        .line("tooltip_width", tooltip_width)
        .line("menu_width", menu_width)
        .line("menu_spacing", menu_spacing)
        .line("indent_ends_with_horizontal_line", indent_ends_with_horizontal_line)
        .line("combo_height", combo_height);
    d.group("scroll", |d| scroll_style(d, scroll));
}

fn scroll_style(d: &mut Dump, s: &ScrollStyle) {
    let ScrollStyle {
        floating,
        content_margin,
        bar_width,
        handle_min_length,
        bar_inner_margin,
        bar_outer_margin,
        floating_width,
        floating_allocated_width,
        foreground_color,
        dormant_background_opacity,
        active_background_opacity,
        interact_background_opacity,
        dormant_handle_opacity,
        active_handle_opacity,
        interact_handle_opacity,
        fade,
        kinetic,
    } = s;
    d.line("floating", floating)
        .line("content_margin", content_margin)
        .line("bar_width", bar_width)
        .line("handle_min_length", handle_min_length)
        .line("bar_inner_margin", bar_inner_margin)
        .line("bar_outer_margin", bar_outer_margin)
        .line("floating_width", floating_width)
        .line("floating_allocated_width", floating_allocated_width)
        .line("foreground_color", foreground_color)
        .line("dormant_background_opacity", dormant_background_opacity)
        .line("active_background_opacity", active_background_opacity)
        .line("interact_background_opacity", interact_background_opacity)
        .line("dormant_handle_opacity", dormant_handle_opacity)
        .line("active_handle_opacity", active_handle_opacity)
        .line("interact_handle_opacity", interact_handle_opacity);
    let ScrollFadeStyle { strength, size } = fade;
    d.line("fade.strength", strength).line("fade.size", size);
    let KineticScrollStyle { decay_time, stop_distance, rubber_band } = kinetic;
    d.line("kinetic.decay_time", decay_time).line("kinetic.stop_distance", stop_distance).line("kinetic.rubber_band", rubber_band);
}
