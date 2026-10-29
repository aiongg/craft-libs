//! Token text dump: every token of a resolved [`Tokens`] as ordered `key = value` lines.
//!
//! The struct destructuring is exhaustive on purpose: a new token or component field doesn't
//! compile until it is dumped.

use std::fmt::Write as _;

use crate::{
    AngleDialTokens, BadgePart, BadgeStyle, BadgeTokens, BadgeTone, BarSizes, BezierTokens, BorderColors, ButtonColors, ButtonTokens, BySize,
    CheckOffColors, CheckOnColors, CheckSwitchColors, CheckTokens, ColorPickerTokens, ColorToken, ComponentTokens, ControlSizes, CursorTokens,
    CurveTokens, Dimmed, DockTokens, Easing, Edge, Elevation, ElevationTokens, EmptyStateTokens, FieldTokens, FloatTokens, FontSlot, Gaps,
    GradientEditorTokens, GradientLineTokens, GuideTokens, HandleTokens, HistogramTokens, IconButtonTokens, IconSizes, ItemColors, KbdTokens,
    LevelsTokens, ListSizes, ListTokens, ListboxTokens, MarqueeTokens, MenuHue, MenuTokens, Motion, MotionTokens, NoticeTokens, Pads,
    PanelFooterTokens, PanelHeaderTokens, PopupTokens, ProgressTokens, PropertyTokens, RadiusSet, RailSizes, ReadoutTokens, RefPointTokens, Rgba,
    RulerTokens, ScrollTokens, SectionTokens, SegmentedTokens, ShapeTokens, SizeTokens, SliderTokens, SpaceTokens, States, SwatchColor, SwatchTokens,
    TabSizes, TabTokens, ThumbColors, ThumbSizes, TitleBarTokens, ToastTokens, ToggleButtonTokens, Tokens, Tone, TooltipTokens, TransformTokens,
    TypeRole, TypeStyle, TypeTokens,
};

/// The text of every token, one `key = value` line each, in a fixed order.
pub fn tokens_text(tokens: &Tokens) -> String {
    let mut out = Lines::default();

    let Tokens { color, shape, space, size, typography, elevation, motion, component } = tokens;
    for &token in ColorToken::ALL {
        out.line(token.path(), hex(color.get(token)));
    }
    shape_lines(&mut out, shape);
    space_lines(&mut out, space);
    size_lines(&mut out, size);
    type_lines(&mut out, typography);
    elevation_lines(&mut out, elevation);
    motion_lines(&mut out, motion);
    component_lines(&mut out, component);
    out.0
}

/// Ordered `key = value` lines.
#[derive(Default)]
struct Lines(String);

impl Lines {
    fn line(&mut self, key: &str, value: impl std::fmt::Display) {
        let _ = writeln!(self.0, "{key} = {value}");
    }
}

/// `#rrggbbaa`, alpha always written.
fn hex(c: Rgba) -> String {
    let [r, g, b, a] = c.0;
    format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
}

/// A length with up to two decimals.
fn len(v: f32) -> String {
    decimals(v, 2)
}

/// `v` with up to `places` decimals, trailing zeros dropped.
fn decimals(v: f32, places: usize) -> String {
    let s = format!("{v:.places$}");
    let s = if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.') } else { &s };
    if s == "-0" { "0".to_owned() } else { s.to_owned() }
}

fn edge(e: Edge) -> String {
    match e {
        Edge::Flat => "flat".to_owned(),
        Edge::Bevel { light, dark, width } => format!("bevel {} {} {}", hex(light), hex(dark), len(width)),
    }
}

fn shape_lines(out: &mut Lines, shape: &ShapeTokens) {
    let ShapeTokens { radius, edge: e } = shape;
    let RadiusSet { control, popup, window, panel, button, pill } = radius;
    for (k, v) in [("control", control), ("popup", popup), ("window", window), ("panel", panel), ("button", button), ("pill", pill)] {
        out.line(&format!("shape.radius.{k}"), len(*v));
    }
    out.line("shape.edge", edge(*e));
}

fn space_lines(out: &mut Lines, space: &SpaceTokens) {
    let SpaceTokens { gap, pad } = space;
    let Gaps { inline, row, stack, section } = gap;
    for (k, v) in [("inline", inline), ("row", row), ("stack", stack), ("section", section)] {
        out.line(&format!("space.gap.{k}"), len(*v));
    }
    let Pads { control, panel, dialog } = pad;
    for (k, v) in [("control", control), ("panel", panel), ("dialog", dialog)] {
        out.line(&format!("space.pad.{k}"), len(*v));
    }
}

fn size_lines(out: &mut Lines, size: &SizeTokens) {
    let SizeTokens { control, icon, rail, bar, panel_header, list, tab: tab_sizes } = size;
    let ControlSizes { sm, md } = control;
    out.line("size.control.sm", len(*sm));
    out.line("size.control.md", len(*md));
    let IconSizes { sm, md, lg } = icon;
    out.line("size.icon.sm", len(*sm));
    out.line("size.icon.md", len(*md));
    out.line("size.icon.lg", len(*lg));
    let RailSizes { width, button, panel_width, panel_button } = rail;
    out.line("size.rail.width", len(*width));
    out.line("size.rail.button", len(*button));
    out.line("size.rail.panel_width", len(*panel_width));
    out.line("size.rail.panel_button", len(*panel_button));
    let BarSizes { title, tab, status, options } = bar;
    for (k, v) in [("title", title), ("tab", tab), ("status", status), ("options", options)] {
        out.line(&format!("size.bar.{k}"), len(*v));
    }
    out.line("size.panel_header", len(*panel_header));
    let ListSizes { thumb, thumb_row_min, indent, disclosure, toggle_column, min_label, trailing_pad, rounded, dividers } = list;
    let ThumbSizes { sm, md, lg } = thumb;
    out.line("size.list.thumb.sm", len(*sm));
    out.line("size.list.thumb.md", len(*md));
    out.line("size.list.thumb.lg", len(*lg));
    out.line("size.list.thumb_row_min", len(*thumb_row_min));
    out.line("size.list.indent", len(*indent));
    out.line("size.list.disclosure", len(*disclosure));
    out.line("size.list.toggle_column", len(*toggle_column));
    out.line("size.list.min_label", len(*min_label));
    out.line("size.list.trailing_pad", len(*trailing_pad));
    out.line("size.list.rounded", rounded);
    out.line("size.list.dividers", dividers);
    let TabSizes { panel_height, height_large, min_width, close, close_icon, rounded, dividers } = tab_sizes;
    out.line("size.tab.panel_height", len(*panel_height));
    out.line("size.tab.height_large", len(*height_large));
    out.line("size.tab.min_width", len(*min_width));
    out.line("size.tab.close", len(*close));
    out.line("size.tab.close_icon", len(*close_icon));
    out.line("size.tab.rounded", rounded);
    out.line("size.tab.dividers", dividers);
}

fn slot(s: FontSlot) -> &'static str {
    match s {
        FontSlot::Ui => "ui",
        FontSlot::UiSemibold => "ui_semibold",
        FontSlot::Mono => "mono",
    }
}

fn role(r: TypeRole) -> &'static str {
    match r {
        TypeRole::Caption => "caption",
        TypeRole::Label => "label",
        TypeRole::Body => "body",
        TypeRole::BodyStrong => "body_strong",
        TypeRole::Title => "title",
        TypeRole::Heading => "heading",
        TypeRole::Mono => "mono",
    }
}

fn type_lines(out: &mut Lines, t: &TypeTokens) {
    let TypeTokens { caption, label, body, body_strong, title, heading, mono } = t;
    let roles = [
        ("caption", caption),
        ("label", label),
        ("body", body),
        ("body_strong", body_strong),
        ("title", title),
        ("heading", heading),
        ("mono", mono),
    ];
    for (k, style) in roles {
        let TypeStyle { size, line_height, font } = style;
        out.line(&format!("typography.{k}.size"), len(*size));
        out.line(&format!("typography.{k}.line_height"), len(*line_height));
        out.line(&format!("typography.{k}.font"), slot(*font));
    }
}

fn elevation_lines(out: &mut Lines, e: &ElevationTokens) {
    let ElevationTokens { popup, dialog, floating, knob } = e;
    for (k, level) in [("popup", popup), ("dialog", dialog), ("floating", floating), ("knob", knob)] {
        let Elevation { offset: [x, y], blur, spread, alpha } = level;
        out.line(&format!("elevation.{k}.offset"), format!("{} {}", len(*x), len(*y)));
        out.line(&format!("elevation.{k}.blur"), len(*blur));
        out.line(&format!("elevation.{k}.spread"), len(*spread));
        out.line(&format!("elevation.{k}.alpha"), decimals(*alpha, 3));
    }
}

fn easing(e: Easing) -> &'static str {
    match e {
        Easing::Standard => "standard",
        Easing::Enter => "enter",
        Easing::Exit => "exit",
    }
}

fn motion_lines(out: &mut Lines, m: &MotionTokens) {
    let MotionTokens { hover, open, r#move } = m;
    for (k, motion) in [("hover", hover), ("open", open), ("move", r#move)] {
        let Motion { millis, easing: e } = motion;
        out.line(&format!("motion.{k}.millis"), millis);
        out.line(&format!("motion.{k}.easing"), easing(*e));
    }
}

fn by_size(out: &mut Lines, key: &str, v: &BySize<f32>) {
    let BySize { sm, md } = v;
    out.line(&format!("{key}.sm"), len(*sm));
    out.line(&format!("{key}.md"), len(*md));
}

fn states(out: &mut Lines, key: &str, v: &States<Rgba>) {
    let States { rest, hover, pressed, disabled } = v;
    for (k, c) in [("rest", rest), ("hover", hover), ("pressed", pressed), ("disabled", disabled)] {
        out.line(&format!("{key}.{k}"), hex(*c));
    }
}

fn component_lines(out: &mut Lines, c: &ComponentTokens) {
    let ComponentTokens {
        angle_dial,
        badge,
        bezier,
        button,
        check,
        color_picker,
        curve,
        cursor,
        dock,
        empty_state,
        field,
        float,
        gradient_editor,
        gradient_line,
        guide,
        handle,
        histogram,
        icon_button,
        kbd,
        levels,
        list,
        listbox,
        marquee,
        menu,
        notice,
        panel_footer,
        panel_header,
        popup,
        progress,
        property,
        readout,
        ref_point,
        ruler,
        scroll,
        section,
        segmented,
        slider,
        swatch,
        tab,
        title_bar,
        toast,
        toggle_button,
        tooltip,
        transform,
    } = c;
    angle_dial_lines(out, angle_dial);
    badge_lines(out, badge);
    bezier_lines(out, bezier);
    button_lines(out, button);
    check_lines(out, check);
    color_picker_lines(out, color_picker);
    curve_lines(out, curve);
    cursor_lines(out, cursor);
    dock_lines(out, dock);
    empty_state_lines(out, empty_state);
    field_lines(out, field);
    float_lines(out, float);
    gradient_editor_lines(out, gradient_editor);
    gradient_line_lines(out, gradient_line);
    guide_lines(out, guide);
    handle_lines(out, handle);
    histogram_lines(out, histogram);
    icon_button_lines(out, icon_button);
    kbd_lines(out, kbd);
    levels_lines(out, levels);
    list_lines(out, list);
    listbox_lines(out, listbox);
    marquee_lines(out, marquee);
    menu_lines(out, menu);
    notice_lines(out, notice);
    panel_footer_lines(out, panel_footer);
    panel_header_lines(out, panel_header);
    popup_lines(out, popup);
    progress_lines(out, progress);
    property_lines(out, property);
    readout_lines(out, readout);
    ref_point_lines(out, ref_point);
    ruler_lines(out, ruler);
    scroll_lines(out, scroll);
    section_lines(out, section);
    segmented_lines(out, segmented);
    slider_lines(out, slider);
    swatch_lines(out, swatch);
    tab_lines(out, tab);
    title_bar_lines(out, title_bar);
    toast_lines(out, toast);
    toggle_button_lines(out, toggle_button);
    tooltip_lines(out, tooltip);
    transform_lines(out, transform);
}

fn item_lines(out: &mut Lines, key: &str, colors: &ItemColors) {
    let ItemColors { fill, text } = colors;
    states(out, &format!("{key}.fill"), fill);
    states(out, &format!("{key}.text"), text);
}

fn check_lines(out: &mut Lines, c: &CheckTokens) {
    let CheckTokens { height, size, gap, box_size, box_gap, outline, label, radius, mark, dot, track, thumb_inset, off, on, switch, thumb } = c;
    by_size(out, "check.height", height);
    by_size(out, "check.size", size);
    out.line("check.gap", len(*gap));
    by_size(out, "check.box_size", box_size);
    out.line("check.box_gap", len(*box_gap));
    out.line("check.outline", len(*outline));
    out.line("check.label", role(*label));
    out.line("check.radius", len(*radius));
    by_size(out, "check.mark", mark);
    by_size(out, "check.dot", dot);
    by_size(out, "check.track", track);
    out.line("check.thumb_inset", len(*thumb_inset));
    let CheckOffColors { fill, border } = off;
    states(out, "check.off.fill", fill);
    states(out, "check.off.border", border);
    let CheckOnColors { fill, border, mark } = on;
    states(out, "check.on.fill", fill);
    states(out, "check.on.border", border);
    states(out, "check.on.mark", mark);
    let CheckSwitchColors { fill, border } = switch;
    states(out, "check.switch.fill", fill);
    states(out, "check.switch.border", border);
    thumb_lines(out, "check.thumb", thumb);
}

fn thumb_lines(out: &mut Lines, key: &str, colors: &ThumbColors) {
    let ThumbColors { fill, border } = colors;
    states(out, &format!("{key}.fill"), fill);
    states(out, &format!("{key}.border"), border);
}

fn icon_button_lines(out: &mut Lines, i: &IconButtonTokens) {
    let IconButtonTokens { size, icon, badge, selected } = i;
    by_size(out, "icon_button.size", size);
    by_size(out, "icon_button.icon", icon);
    by_size(out, "icon_button.badge", badge);
    let ToggleButtonTokens { on_fill, on_border, on_text, on_disabled_fill, on_disabled_border, on_disabled_text } = selected;
    out.line("icon_button.selected.fill", hex(*on_fill));
    out.line("icon_button.selected.border", hex(*on_border));
    out.line("icon_button.selected.text", hex(*on_text));
    out.line("icon_button.selected_disabled.fill", hex(*on_disabled_fill));
    out.line("icon_button.selected_disabled.border", hex(*on_disabled_border));
    out.line("icon_button.selected_disabled.text", hex(*on_disabled_text));
}

fn segmented_lines(out: &mut Lines, s: &SegmentedTokens) {
    let SegmentedTokens { height, pad_x, icon, gap, label, segment, selected, selected_border } = s;
    by_size(out, "segmented.height", height);
    by_size(out, "segmented.pad_x", pad_x);
    by_size(out, "segmented.icon", icon);
    out.line("segmented.gap", len(*gap));
    out.line("segmented.label", role(*label));
    item_lines(out, "segmented.segment", segment);
    item_lines(out, "segmented.selected", selected);
    states(out, "segmented.selected.border", selected_border);
}

fn field_lines(out: &mut Lines, f: &FieldTokens) {
    let FieldTokens {
        height,
        width,
        number_width,
        pad_x,
        icon,
        gap,
        label_gap,
        help_gap,
        radius,
        role: text_role,
        fill,
        border,
        text,
        placeholder,
        unit,
        scrub,
        selection,
        selected_text,
        caret,
        invalid,
    } = f;
    by_size(out, "field.height", height);
    by_size(out, "field.width", width);
    by_size(out, "field.number_width", number_width);
    out.line("field.pad_x", len(*pad_x));
    by_size(out, "field.icon", icon);
    out.line("field.gap", len(*gap));
    out.line("field.label_gap", len(*label_gap));
    out.line("field.help_gap", len(*help_gap));
    out.line("field.radius", len(*radius));
    out.line("field.role", role(*text_role));
    for (key, d) in [("field.fill", fill), ("field.text", text), ("field.placeholder", placeholder), ("field.unit", unit), ("field.scrub", scrub)] {
        let Dimmed { rest, disabled } = d;
        out.line(&format!("{key}.rest"), hex(*rest));
        out.line(&format!("{key}.disabled"), hex(*disabled));
    }
    let BorderColors { rest, hover, focused, disabled } = border;
    for (state, c) in [("rest", rest), ("hover", hover), ("focused", focused), ("disabled", disabled)] {
        out.line(&format!("field.border.{state}"), hex(*c));
    }
    out.line("field.selection.fill", hex(*selection));
    out.line("field.selection.text", hex(*selected_text));
    out.line("field.caret", hex(*caret));
    out.line("field.invalid", hex(*invalid));
}

fn slider_lines(out: &mut Lines, s: &SliderTokens) {
    let SliderTokens {
        height,
        width,
        thumb,
        thumb_width,
        range_thumb_width,
        range_thumb_height,
        range_thumb_radius,
        track_height,
        gradient_height,
        track_radius,
        radius,
        track,
        fill,
        thumb_colors,
        ring,
        ring_width,
        ring_offset,
        mark,
        snap,
    } = s;
    by_size(out, "slider.height", height);
    by_size(out, "slider.width", width);
    by_size(out, "slider.thumb", thumb);
    by_size(out, "slider.thumb_width", thumb_width);
    by_size(out, "slider.range_thumb_width", range_thumb_width);
    by_size(out, "slider.range_thumb_height", range_thumb_height);
    out.line("slider.range_thumb_radius", len(*range_thumb_radius));
    out.line("slider.track_height", len(*track_height));
    out.line("slider.gradient_height", len(*gradient_height));
    out.line("slider.track_radius", len(*track_radius));
    out.line("slider.radius", len(*radius));
    out.line("slider.track", hex(*track));
    states(out, "slider.fill", fill);
    thumb_lines(out, "slider.thumb", thumb_colors);
    states(out, "slider.ring", ring);
    out.line("slider.ring_width", len(*ring_width));
    out.line("slider.ring_offset", len(*ring_offset));
    by_size(out, "slider.mark", mark);
    out.line("slider.snap", len(*snap));
}

fn toggle_button_lines(out: &mut Lines, t: &ToggleButtonTokens) {
    let ToggleButtonTokens { on_fill, on_border, on_text, on_disabled_fill, on_disabled_border, on_disabled_text } = t;
    out.line("toggle_button.on.fill", hex(*on_fill));
    out.line("toggle_button.on.border", hex(*on_border));
    out.line("toggle_button.on.text", hex(*on_text));
    out.line("toggle_button.on_disabled.fill", hex(*on_disabled_fill));
    out.line("toggle_button.on_disabled.border", hex(*on_disabled_border));
    out.line("toggle_button.on_disabled.text", hex(*on_disabled_text));
}

fn color_picker_lines(out: &mut Lines, p: &ColorPickerTokens) {
    let ColorPickerTokens {
        area,
        area_height,
        strip_full,
        strip,
        compare_width,
        compare_height,
        marker_inner_radius,
        marker_inner_width,
        marker_outer_radius,
        marker_outer_width,
        pointer_width,
        pointer_height,
        warning_icon,
        warning_chip,
        pair_well,
        pair_step,
        pair_gap,
        pair_ring,
        marker,
        marker_halo,
    } = p;
    out.line("color_picker.area", len(*area));
    out.line("color_picker.area_height", len(*area_height));
    out.line("color_picker.strip_full", len(*strip_full));
    out.line("color_picker.strip", len(*strip));
    out.line("color_picker.compare_width", len(*compare_width));
    out.line("color_picker.compare_height", len(*compare_height));
    out.line("color_picker.marker_inner_radius", len(*marker_inner_radius));
    out.line("color_picker.marker_inner_width", len(*marker_inner_width));
    out.line("color_picker.marker_outer_radius", len(*marker_outer_radius));
    out.line("color_picker.marker_outer_width", len(*marker_outer_width));
    out.line("color_picker.pointer_width", len(*pointer_width));
    out.line("color_picker.pointer_height", len(*pointer_height));
    out.line("color_picker.warning_icon", len(*warning_icon));
    out.line("color_picker.warning_chip", len(*warning_chip));
    out.line("color_picker.pair_well", len(*pair_well));
    out.line("color_picker.pair_step", len(*pair_step));
    out.line("color_picker.pair_gap", len(*pair_gap));
    out.line("color_picker.pair_ring", len(*pair_ring));
    out.line("color_picker.marker", hex(*marker));
    out.line("color_picker.marker_halo", hex(*marker_halo));
}

fn property_lines(out: &mut Lines, p: &PropertyTokens) {
    let PropertyTokens { glyph_width, word_width, min_control, column_gap, link_width } = p;
    out.line("property.glyph_width", len(*glyph_width));
    out.line("property.word_width", len(*word_width));
    out.line("property.min_control", len(*min_control));
    out.line("property.column_gap", len(*column_gap));
    out.line("property.link_width", len(*link_width));
}

fn section_lines(out: &mut Lines, t: &SectionTokens) {
    let SectionTokens { row_height, chevron, chevron_box, title_offset } = t;
    out.line("section.row_height", len(*row_height));
    out.line("section.chevron", len(*chevron));
    out.line("section.chevron_box", len(*chevron_box));
    out.line("section.title_offset", len(*title_offset));
}

fn list_lines(out: &mut Lines, t: &ListTokens) {
    let ListTokens {
        row_height,
        thumb,
        thumb_pad,
        thumb_row_min,
        pad_x,
        gap,
        indent,
        disclosure,
        toggle_column,
        min_label,
        trailing_pad,
        radius,
        dividers,
        drop_width,
        drop_halo_width,
        label,
        detail_role,
        text,
        dimmed,
        selected_text,
        selected_dimmed,
        detail,
        selected_detail,
        icon,
        icon_hover,
        hover,
        selected,
        selected_hover,
        divider,
        thumb_border,
        thumb_border_selected,
        drop,
        drop_halo,
        ghost_fill,
        ghost_border,
    } = t;
    out.line("list.row_height", len(*row_height));
    let ThumbSizes { sm, md, lg } = thumb;
    out.line("list.thumb.sm", len(*sm));
    out.line("list.thumb.md", len(*md));
    out.line("list.thumb.lg", len(*lg));
    out.line("list.thumb_pad", len(*thumb_pad));
    out.line("list.thumb_row_min", len(*thumb_row_min));
    out.line("list.pad_x", len(*pad_x));
    out.line("list.gap", len(*gap));
    out.line("list.indent", len(*indent));
    out.line("list.disclosure", len(*disclosure));
    out.line("list.toggle_column", len(*toggle_column));
    out.line("list.min_label", len(*min_label));
    out.line("list.trailing_pad", len(*trailing_pad));
    out.line("list.radius", len(*radius));
    out.line("list.dividers", dividers);
    out.line("list.drop_width", len(*drop_width));
    out.line("list.drop_halo_width", len(*drop_halo_width));
    out.line("list.label", role(*label));
    out.line("list.detail_role", role(*detail_role));
    for (k, c) in [
        ("text", text),
        ("text.dimmed", dimmed),
        ("text.selected", selected_text),
        ("text.selected.dimmed", selected_dimmed),
        ("detail", detail),
        ("detail.selected", selected_detail),
        ("icon", icon),
        ("icon.hover", icon_hover),
        ("hover", hover),
        ("selected", selected),
        ("selected.hover", selected_hover),
        ("divider", divider),
        ("thumb.border", thumb_border),
        ("thumb.border.selected", thumb_border_selected),
        ("drop", drop),
        ("drop.halo", drop_halo),
        ("ghost.fill", ghost_fill),
        ("ghost.border", ghost_border),
    ] {
        out.line(&format!("list.{k}"), hex(*c));
    }
}

fn tab_lines(out: &mut Lines, t: &TabTokens) {
    let TabTokens {
        height,
        panel_height,
        height_large,
        pad_x,
        gap,
        min_width,
        close_box,
        close_icon,
        overflow,
        radius,
        dividers,
        drop_width,
        drop_halo_width,
        progress_height,
        indicator_width,
        tear_off,
        label,
        label_large,
        detail_role,
        strip,
        active,
        text,
        inactive_text,
        detail,
        divider,
        close_fill,
        close_text,
        close_hover,
        drop,
        drop_halo,
        indicator,
    } = t;
    out.line("tab.height", len(*height));
    out.line("tab.panel_height", len(*panel_height));
    out.line("tab.height_large", len(*height_large));
    out.line("tab.pad_x", len(*pad_x));
    out.line("tab.gap", len(*gap));
    out.line("tab.min_width", len(*min_width));
    out.line("tab.close_box", len(*close_box));
    out.line("tab.close_icon", len(*close_icon));
    out.line("tab.overflow", len(*overflow));
    out.line("tab.radius", len(*radius));
    out.line("tab.dividers", dividers);
    out.line("tab.drop_width", len(*drop_width));
    out.line("tab.drop_halo_width", len(*drop_halo_width));
    out.line("tab.progress_height", len(*progress_height));
    out.line("tab.indicator_width", len(*indicator_width));
    out.line("tab.tear_off", len(*tear_off));
    out.line("tab.label", role(*label));
    out.line("tab.label_large", role(*label_large));
    out.line("tab.detail_role", role(*detail_role));
    for (k, c) in [
        ("strip", strip),
        ("active", active),
        ("text", text),
        ("text.inactive", inactive_text),
        ("detail", detail),
        ("divider", divider),
        ("close.fill", close_fill),
        ("close", close_text),
        ("close.hover", close_hover),
        ("drop", drop),
        ("drop.halo", drop_halo),
        ("indicator", indicator),
    ] {
        out.line(&format!("tab.{k}"), hex(*c));
    }
}

fn dock_lines(out: &mut Lines, t: &DockTokens) {
    let DockTokens {
        gap,
        border_width,
        title_height,
        column_bar_height,
        icon_size,
        icon_pad,
        iconic_width,
        drop_width,
        focus_width,
        marker_radius,
        drop_fill,
        ghost_fill,
        ghost_stroke,
        tab_fill,
        grip_alpha,
        icon_radius,
        hint_radius,
        background,
        border,
        resize,
        drop,
        icon_selected,
        icon_selected_text,
    } = t;
    for (k, v) in [
        ("gap", gap),
        ("border_width", border_width),
        ("title_height", title_height),
        ("column_bar_height", column_bar_height),
        ("icon_size", icon_size),
        ("icon_pad", icon_pad),
        ("iconic_width", iconic_width),
        ("drop_width", drop_width),
        ("focus_width", focus_width),
        ("marker_radius", marker_radius),
        ("icon.radius", icon_radius),
        ("hint.radius", hint_radius),
    ] {
        out.line(&format!("dock.{k}"), len(*v));
    }
    for (k, v) in
        [("drop_fill", drop_fill), ("ghost_fill", ghost_fill), ("ghost_stroke", ghost_stroke), ("tab_fill", tab_fill), ("grip_alpha", grip_alpha)]
    {
        out.line(&format!("dock.{k}"), decimals(*v, 3));
    }
    for (k, c) in [
        ("background", background),
        ("border", border),
        ("resize", resize),
        ("drop", drop),
        ("icon.selected", icon_selected),
        ("icon.selected.text", icon_selected_text),
    ] {
        out.line(&format!("dock.{k}"), hex(*c));
    }
}

fn float_lines(out: &mut Lines, t: &FloatTokens) {
    let FloatTokens { border_width, radius, shadow, fill, border, blocked, title, title_text, title_muted, grip, focus } = t;
    out.line("float.border_width", len(*border_width));
    out.line("float.radius", len(*radius));
    let Elevation { offset: [x, y], blur, spread, alpha } = shadow;
    out.line("float.shadow.offset", format!("{} {}", len(*x), len(*y)));
    out.line("float.shadow.blur", len(*blur));
    out.line("float.shadow.spread", len(*spread));
    out.line("float.shadow.alpha", decimals(*alpha, 3));
    for (k, c) in [
        ("fill", fill),
        ("border", border),
        ("blocked", blocked),
        ("title", title),
        ("title.text", title_text),
        ("title.muted", title_muted),
        ("grip", grip),
        ("focus", focus),
    ] {
        out.line(&format!("float.{k}"), hex(*c));
    }
}

fn title_bar_lines(out: &mut Lines, t: &TitleBarTokens) {
    let TitleBarTokens {
        height,
        height_adwaita,
        pad,
        title_gap,
        title_min_gap,
        title_role,
        edge,
        corner,
        bar,
        title,
        glyph,
        glyph_inactive,
        hover,
        pressed,
        separator,
        windows_close,
        windows_close_glyph,
        breeze_close_hover,
        breeze_close_hover_glyph,
        breeze_close_pressed,
        breeze_close_pressed_glyph,
    } = t;
    out.line("title_bar.height", len(*height));
    out.line("title_bar.height_adwaita", len(*height_adwaita));
    out.line("title_bar.pad", len(*pad));
    out.line("title_bar.title_gap", len(*title_gap));
    out.line("title_bar.title_min_gap", len(*title_min_gap));
    out.line("title_bar.title_role", role(*title_role));
    out.line("title_bar.edge", len(*edge));
    out.line("title_bar.corner", len(*corner));
    for (k, c) in [
        ("bar", bar),
        ("title", title),
        ("glyph", glyph),
        ("glyph.inactive", glyph_inactive),
        ("hover", hover),
        ("pressed", pressed),
        ("separator", separator),
        ("windows_close", windows_close),
        ("windows_close_glyph", windows_close_glyph),
        ("breeze_close_hover", breeze_close_hover),
        ("breeze_close_hover_glyph", breeze_close_hover_glyph),
        ("breeze_close_pressed", breeze_close_pressed),
        ("breeze_close_pressed_glyph", breeze_close_pressed_glyph),
    ] {
        out.line(&format!("title_bar.{k}"), hex(*c));
    }
}

fn panel_footer_lines(out: &mut Lines, t: &PanelFooterTokens) {
    let PanelFooterTokens { height, gap, button } = t;
    out.line("panel_footer.height", len(*height));
    out.line("panel_footer.gap", len(*gap));
    out.line("panel_footer.button", len(*button));
}

fn panel_header_lines(out: &mut Lines, t: &PanelHeaderTokens) {
    let PanelHeaderTokens { height, pad, gap, button } = t;
    out.line("panel_header.height", len(*height));
    out.line("panel_header.pad", len(*pad));
    out.line("panel_header.gap", len(*gap));
    out.line("panel_header.button", len(*button));
}

fn angle_dial_lines(out: &mut Lines, t: &AngleDialTokens) {
    let AngleDialTokens { size, margin, needle_width, needle_inset, hub, dead_zone, face, edge, needle, hub_fill } = t;
    out.line("angle_dial.size", len(*size));
    out.line("angle_dial.margin", len(*margin));
    out.line("angle_dial.needle_width", len(*needle_width));
    out.line("angle_dial.needle_inset", len(*needle_inset));
    out.line("angle_dial.hub", len(*hub));
    out.line("angle_dial.dead_zone", len(*dead_zone));
    out.line("angle_dial.face", hex(*face));
    out.line("angle_dial.edge", hex(*edge));
    out.line("angle_dial.needle", hex(*needle));
    out.line("angle_dial.hub_fill", hex(*hub_fill));
}

fn curve_lines(out: &mut Lines, t: &CurveTokens) {
    let CurveTokens {
        min_side,
        max_side,
        pad,
        bar,
        bar_gap,
        hit,
        drag_off,
        width,
        other_width,
        other_alpha,
        fill,
        border,
        grid,
        baseline,
        point_fill,
        point_stroke,
        point_halo,
    } = t;
    out.line("curve.min_side", len(*min_side));
    out.line("curve.max_side", len(*max_side));
    out.line("curve.pad", len(*pad));
    out.line("curve.bar", len(*bar));
    out.line("curve.bar_gap", len(*bar_gap));
    out.line("curve.hit", len(*hit));
    out.line("curve.drag_off", len(*drag_off));
    out.line("curve.width", len(*width));
    out.line("curve.other_width", len(*other_width));
    out.line("curve.other_alpha", decimals(*other_alpha, 2));
    out.line("curve.fill", hex(*fill));
    out.line("curve.border", hex(*border));
    out.line("curve.grid", hex(*grid));
    out.line("curve.baseline", hex(*baseline));
    out.line("curve.point_fill", hex(*point_fill));
    out.line("curve.point_stroke", hex(*point_stroke));
    out.line("curve.point_halo", hex(*point_halo));
}

fn levels_lines(out: &mut Lines, t: &LevelsTokens) {
    let LevelsTokens { min_width, max_width, row, thumb_width, thumb_height, hit, histogram, bar, thumb_edge, disabled_edge, ring } = t;
    out.line("levels.min_width", len(*min_width));
    out.line("levels.max_width", len(*max_width));
    out.line("levels.row", len(*row));
    out.line("levels.thumb_width", len(*thumb_width));
    out.line("levels.thumb_height", len(*thumb_height));
    out.line("levels.hit", len(*hit));
    out.line("levels.histogram", len(*histogram));
    out.line("levels.bar", len(*bar));
    out.line("levels.thumb_edge", hex(*thumb_edge));
    out.line("levels.disabled_edge", hex(*disabled_edge));
    states(out, "levels.ring", ring);
}

fn gradient_editor_lines(out: &mut Lines, t: &GradientEditorTokens) {
    let GradientEditorTokens { ramp_height, inset, marker, pointer, diamond, hit, diamond_hit, tear_off, drop_line, edge, marker_edge, selected } = t;
    out.line("gradient_editor.ramp_height", len(*ramp_height));
    out.line("gradient_editor.inset", len(*inset));
    out.line("gradient_editor.marker", len(*marker));
    out.line("gradient_editor.pointer", len(*pointer));
    out.line("gradient_editor.diamond", len(*diamond));
    out.line("gradient_editor.hit", len(*hit));
    out.line("gradient_editor.diamond_hit", len(*diamond_hit));
    out.line("gradient_editor.tear_off", len(*tear_off));
    out.line("gradient_editor.drop_line", len(*drop_line));
    out.line("gradient_editor.edge", hex(*edge));
    out.line("gradient_editor.marker_edge", hex(*marker_edge));
    out.line("gradient_editor.selected", hex(*selected));
}

fn histogram_lines(out: &mut Lines, t: &HistogramTokens) {
    let HistogramTokens { height, backdrop, line, selection_alpha, marker_width, marker_height } = t;
    out.line("histogram.height", len(*height));
    out.line("histogram.backdrop", hex(*backdrop));
    out.line("histogram.selection_alpha", len(*selection_alpha));
    out.line("histogram.line", len(*line));
    out.line("histogram.marker_width", len(*marker_width));
    out.line("histogram.marker_height", len(*marker_height));
}

fn guide_lines(out: &mut Lines, g: &GuideTokens) {
    let GuideTokens { width, hit, dash, gap, dot_gap, dot_radius, measure_cap, measure_pad_x, measure_pad_y } = g;
    out.line("guide.width", len(*width));
    out.line("guide.hit", len(*hit));
    out.line("guide.dash", len(*dash));
    out.line("guide.gap", len(*gap));
    out.line("guide.dot_gap", len(*dot_gap));
    out.line("guide.dot_radius", len(*dot_radius));
    out.line("guide.measure_cap", len(*measure_cap));
    out.line("guide.measure_pad_x", len(*measure_pad_x));
    out.line("guide.measure_pad_y", len(*measure_pad_y));
}

fn transform_lines(out: &mut Lines, t: &TransformTokens) {
    let TransformTokens {
        line,
        hit,
        rotate_reach,
        band_inner,
        band_outer,
        click_slop,
        pivot_radius,
        pivot_arm,
        turn_radius,
        turn_line,
        turn_halo,
        warp_minor,
        warp_anchor,
        warp_control,
        warp_reveal,
    } = t;
    out.line("transform.line", len(*line));
    out.line("transform.hit", len(*hit));
    out.line("transform.rotate_reach", len(*rotate_reach));
    out.line("transform.band_inner", len(*band_inner));
    out.line("transform.band_outer", len(*band_outer));
    out.line("transform.click_slop", len(*click_slop));
    out.line("transform.pivot_radius", len(*pivot_radius));
    out.line("transform.pivot_arm", len(*pivot_arm));
    out.line("transform.turn_radius", len(*turn_radius));
    out.line("transform.turn_line", len(*turn_line));
    out.line("transform.turn_halo", len(*turn_halo));
    out.line("transform.warp_minor", len(*warp_minor));
    out.line("transform.warp_anchor", len(*warp_anchor));
    out.line("transform.warp_control", len(*warp_control));
    out.line("transform.warp_reveal", len(*warp_reveal));
}

fn gradient_line_lines(out: &mut Lines, t: &GradientLineTokens) {
    let GradientLineTokens { line, halo, end_ring, end_width, end_halo, stop, stop_ring, diamond, hit, click_slop, tear_off, dash, gap } = t;
    out.line("gradient_line.line", len(*line));
    out.line("gradient_line.halo", len(*halo));
    out.line("gradient_line.end_ring", len(*end_ring));
    out.line("gradient_line.end_width", len(*end_width));
    out.line("gradient_line.end_halo", len(*end_halo));
    out.line("gradient_line.stop", len(*stop));
    out.line("gradient_line.stop_ring", len(*stop_ring));
    out.line("gradient_line.diamond", len(*diamond));
    out.line("gradient_line.hit", len(*hit));
    out.line("gradient_line.click_slop", len(*click_slop));
    out.line("gradient_line.tear_off", len(*tear_off));
    out.line("gradient_line.dash", len(*dash));
    out.line("gradient_line.gap", len(*gap));
}

fn bezier_lines(out: &mut Lines, t: &BezierTokens) {
    let BezierTokens {
        line,
        selected_segment,
        draft_line,
        anchor,
        control,
        control_line,
        hit,
        click_slop,
        flatten,
        rubber_dash,
        rubber_gap,
        close_anchor,
        close_ring,
        close_offset,
    } = t;
    out.line("bezier.line", len(*line));
    out.line("bezier.selected_segment", len(*selected_segment));
    out.line("bezier.draft_line", len(*draft_line));
    out.line("bezier.anchor", len(*anchor));
    out.line("bezier.control", len(*control));
    out.line("bezier.control_line", len(*control_line));
    out.line("bezier.hit", len(*hit));
    out.line("bezier.click_slop", len(*click_slop));
    out.line("bezier.flatten", len(*flatten));
    out.line("bezier.rubber_dash", len(*rubber_dash));
    out.line("bezier.rubber_gap", len(*rubber_gap));
    out.line("bezier.close_anchor", len(*close_anchor));
    out.line("bezier.close_ring", len(*close_ring));
    out.line("bezier.close_offset", len(*close_offset));
}

fn ref_point_lines(out: &mut Lines, t: &RefPointTokens) {
    let RefPointTokens { compact, large, dot } = t;
    out.line("ref_point.compact", len(*compact));
    out.line("ref_point.large", len(*large));
    out.line("ref_point.dot", len(*dot));
}

fn marquee_lines(out: &mut Lines, m: &MarqueeTokens) {
    let MarqueeTokens { dash, speed, tick_ms, band_dash, band_gap, drawing_halo, click_slop } = m;
    out.line("marquee.dash", len(*dash));
    out.line("marquee.speed", len(*speed));
    out.line("marquee.tick_ms", tick_ms.to_string());
    out.line("marquee.band_dash", len(*band_dash));
    out.line("marquee.band_gap", len(*band_gap));
    out.line("marquee.drawing_halo", len(*drawing_halo));
    out.line("marquee.click_slop", len(*click_slop));
}

fn readout_lines(out: &mut Lines, r: &ReadoutTokens) {
    let ReadoutTokens { offset_x, offset_y, pad_x, pad_y, col_gap, row_gap } = r;
    out.line("readout.offset_x", len(*offset_x));
    out.line("readout.offset_y", len(*offset_y));
    out.line("readout.pad_x", len(*pad_x));
    out.line("readout.pad_y", len(*pad_y));
    out.line("readout.col_gap", len(*col_gap));
    out.line("readout.row_gap", len(*row_gap));
}

fn ruler_lines(out: &mut Lines, r: &RulerTokens) {
    let RulerTokens { size, label, label_pitch, label_pad, step_min, tick_mid, tick_minor } = r;
    out.line("ruler.size", len(*size));
    out.line("ruler.label", len(*label));
    out.line("ruler.label_pitch", len(*label_pitch));
    out.line("ruler.label_pad", len(*label_pad));
    out.line("ruler.step_min", len(*step_min));
    out.line("ruler.tick_mid", len(*tick_mid));
    out.line("ruler.tick_minor", len(*tick_minor));
}

fn scroll_lines(out: &mut Lines, s: &ScrollTokens) {
    let ScrollTokens { solid, floating_width, handle_opacity, track_opacity, canvas_width, thumb_inset, thumb_min } = s;
    out.line("scroll.solid", solid);
    out.line("scroll.floating_width", len(*floating_width));
    out.line("scroll.handle_opacity", len(*handle_opacity));
    out.line("scroll.track_opacity", len(*track_opacity));
    out.line("scroll.canvas_width", len(*canvas_width));
    out.line("scroll.thumb_inset", len(*thumb_inset));
    out.line("scroll.thumb_min", len(*thumb_min));
}

fn cursor_lines(out: &mut Lines, c: &CursorTokens) {
    let CursorTokens {
        crosshair_reach,
        crosshair_gap,
        badge_half,
        badge_offset,
        badge_beside_x,
        badge_beside_y,
        dart_length,
        dart_half_width,
        dart_outline,
        skew_gap,
        skew_half_length,
        skew_head,
        skew_back,
        skew_head_outline,
    } = c;
    out.line("cursor.crosshair_reach", len(*crosshair_reach));
    out.line("cursor.crosshair_gap", len(*crosshair_gap));
    out.line("cursor.badge_half", len(*badge_half));
    out.line("cursor.badge_offset", len(*badge_offset));
    out.line("cursor.badge_beside_x", len(*badge_beside_x));
    out.line("cursor.badge_beside_y", len(*badge_beside_y));
    out.line("cursor.dart_length", len(*dart_length));
    out.line("cursor.dart_half_width", len(*dart_half_width));
    out.line("cursor.dart_outline", len(*dart_outline));
    out.line("cursor.skew_gap", len(*skew_gap));
    out.line("cursor.skew_half_length", len(*skew_half_length));
    out.line("cursor.skew_head", len(*skew_head));
    out.line("cursor.skew_back", len(*skew_back));
    out.line("cursor.skew_head_outline", len(*skew_head_outline));
}

fn handle_lines(out: &mut Lines, h: &HandleTokens) {
    let HandleTokens { size, stroke, halo, hit } = h;
    out.line("handle.size", len(*size));
    out.line("handle.stroke", len(*stroke));
    out.line("handle.halo", len(*halo));
    out.line("handle.hit", len(*hit));
}

fn button_lines(out: &mut Lines, b: &ButtonTokens) {
    let ButtonTokens {
        height,
        pad_x,
        icon,
        gap,
        label,
        radius,
        border_width,
        hover_opacity,
        pressed_opacity,
        primary,
        secondary,
        ghost,
        danger,
        default_frame,
    } = b;
    by_size(out, "button.height", height);
    by_size(out, "button.pad_x", pad_x);
    by_size(out, "button.icon", icon);
    out.line("button.gap", len(*gap));
    out.line("button.label", role(*label));
    out.line("button.radius", len(*radius));
    out.line("button.border_width", len(*border_width));
    out.line("button.hover_opacity", len(*hover_opacity));
    out.line("button.pressed_opacity", len(*pressed_opacity));
    out.line("button.default_frame", hex(*default_frame));
    for (k, colors) in [("primary", primary), ("secondary", secondary), ("ghost", ghost), ("danger", danger)] {
        let ButtonColors { fill, border, text } = colors;
        states(out, &format!("button.{k}.fill"), fill);
        states(out, &format!("button.{k}.border"), border);
        states(out, &format!("button.{k}.text"), text);
    }
}

fn badge_lines(out: &mut Lines, b: &BadgeTokens) {
    out.line("badge.height", len(b.height));
    out.line("badge.pad_y", len(b.pad_y));
    out.line("badge.pad_x", len(b.pad_x));
    out.line("badge.radius", len(b.radius));
    out.line("badge.label", role(b.label));
    for tone in BadgeTone::ALL {
        for style in BadgeStyle::ALL {
            let colors = b.colors(tone, style);
            for part in BadgePart::ALL {
                out.line(&format!("badge.{}.{}.{}", tone.name(), style.name(), part.name()), hex(colors.part(part)));
            }
        }
    }
}

fn listbox_lines(out: &mut Lines, l: &ListboxTokens) {
    let ListboxTokens {
        height,
        pad_x,
        max_height,
        gap,
        icon,
        label,
        group_label,
        separator_pad,
        radius,
        text,
        active_fill,
        active_text,
        mark,
        group,
        separator,
    } = l;
    by_size(out, "listbox.height", height);
    out.line("listbox.pad_x", len(*pad_x));
    out.line("listbox.max_height", len(*max_height));
    out.line("listbox.gap", len(*gap));
    by_size(out, "listbox.icon", icon);
    out.line("listbox.label", role(*label));
    out.line("listbox.group_label", role(*group_label));
    out.line("listbox.separator_pad", len(*separator_pad));
    out.line("listbox.radius", len(*radius));
    out.line("listbox.text.rest", hex(text.rest));
    out.line("listbox.text.disabled", hex(text.disabled));
    out.line("listbox.active.fill", hex(*active_fill));
    out.line("listbox.active.text", hex(*active_text));
    out.line("listbox.mark", hex(*mark));
    out.line("listbox.group", hex(*group));
    out.line("listbox.separator", hex(*separator));
}

fn empty_state_lines(out: &mut Lines, e: &EmptyStateTokens) {
    let EmptyStateTokens { max_width, text } = e;
    out.line("empty_state.max_width", len(*max_width));
    out.line("empty_state.text", hex(*text));
}

fn notice_lines(out: &mut Lines, n: &NoticeTokens) {
    let NoticeTokens { icon_gap, fg } = n;
    out.line("notice.icon_gap", len(*icon_gap));
    for (tone, color) in Tone::ALL.into_iter().zip(fg) {
        out.line(&format!("notice.{}.fg", tone.name()), hex(*color));
    }
}

fn toast_lines(out: &mut Lines, t: &ToastTokens) {
    let ToastTokens { width, pad, inset, gap, radius, fill, shadow_offset, shadow_blur, line, more, edge, title } = t;
    out.line("toast.width", len(*width));
    out.line("toast.pad", len(*pad));
    out.line("toast.inset.x", len(inset[0]));
    out.line("toast.inset.y", len(inset[1]));
    out.line("toast.gap", len(*gap));
    out.line("toast.radius", len(*radius));
    out.line("toast.fill", hex(*fill));
    out.line("toast.shadow.x", len(shadow_offset[0]));
    out.line("toast.shadow.y", len(shadow_offset[1]));
    out.line("toast.shadow.blur", len(*shadow_blur));
    out.line("toast.line", hex(*line));
    out.line("toast.more", hex(*more));
    for (tone, (edge, title)) in Tone::ALL.into_iter().zip(edge.iter().zip(title)) {
        out.line(&format!("toast.{}.edge", tone.name()), hex(*edge));
        out.line(&format!("toast.{}.title", tone.name()), hex(*title));
    }
}

fn popup_lines(out: &mut Lines, p: &PopupTokens) {
    let PopupTokens { fill, border, shadow } = p;
    out.line("popup.fill", hex(*fill));
    out.line("popup.border", hex(*border));
    out.line("popup.shadow", hex(*shadow));
}

fn menu_lines(out: &mut Lines, m: &MenuTokens) {
    let MenuTokens { min_width, shortcut_gap, scroll_strip, active_fill, active_text, open_fill, shortcut, tints } = m;
    out.line("menu.min_width", len(*min_width));
    out.line("menu.shortcut_gap", len(*shortcut_gap));
    out.line("menu.scroll_strip", len(*scroll_strip));
    out.line("menu.active.fill", hex(*active_fill));
    out.line("menu.active.text", hex(*active_text));
    out.line("menu.open.fill", hex(*open_fill));
    out.line("menu.shortcut", hex(*shortcut));
    for (hue, tint) in MenuHue::ALL.into_iter().zip(tints) {
        out.line(&format!("menu.tint.{}.fill", hue.name()), hex(tint.fill));
        out.line(&format!("menu.tint.{}.text", hue.name()), hex(tint.text));
        out.line(&format!("menu.tint.{}.shortcut", hue.name()), hex(tint.shortcut));
    }
}

fn kbd_lines(out: &mut Lines, k: &KbdTokens) {
    let KbdTokens { height, pad_y, pad_x, label, radius, fill, text } = k;
    out.line("kbd.height", len(*height));
    out.line("kbd.pad_y", len(*pad_y));
    out.line("kbd.pad_x", len(*pad_x));
    out.line("kbd.label", role(*label));
    out.line("kbd.radius", len(*radius));
    out.line("kbd.fill", hex(*fill));
    out.line("kbd.text", hex(*text));
}

fn progress_lines(out: &mut Lines, p: &ProgressTokens) {
    let ProgressTokens { track_height, radius, track, fill } = p;
    out.line("progress.track_height", len(*track_height));
    out.line("progress.radius", len(*radius));
    out.line("progress.track", hex(*track));
    out.line("progress.fill", hex(*fill));
}

fn swatch_lines(out: &mut Lines, s: &SwatchTokens) {
    by_size(out, "swatch.size", &s.size);
    out.line("swatch.radius", len(s.radius));
    out.line("swatch.selected_width", len(s.selected_width));
    out.line("swatch.checker_cell", len(s.checker_cell));
    for color in SwatchColor::ALL {
        out.line(&format!("swatch.{}", color.name()), hex(s.color(color)));
    }
}

fn tooltip_lines(out: &mut Lines, t: &TooltipTokens) {
    let TooltipTokens { max_width, radius, fill, text, border } = t;
    out.line("tooltip.max_width", len(*max_width));
    out.line("tooltip.radius", len(*radius));
    out.line("tooltip.fill", hex(*fill));
    out.line("tooltip.text", hex(*text));
    out.line("tooltip.border", hex(*border));
}
