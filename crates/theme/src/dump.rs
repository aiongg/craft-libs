//! Token text dump: every token of a resolved [`Tokens`] as ordered `key = value` lines.
//!
//! The struct destructuring is exhaustive on purpose: a new token or component field doesn't
//! compile until it is dumped.

use std::fmt::Write as _;

use crate::{
    AboutTokens, ActionsTokens, AddBesideTokens, AdjustTokens, AlignTokens, AltTextTokens, AngleDialTokens, AppShellTokens, ArtboardTokens,
    AutocompleteTokens, BadgePart, BadgeStyle, BadgeTokens, BadgeTone, BarSizes, BeforeAfterTokens, BezierTokens, BlendIfTokens, BlurPinsTokens,
    BorderColors, BreadcrumbTokens, BrushTokens, ButtonColors, ButtonTokens, BySize, CanvasTokens, ChannelsTokens, CheckOffColors, CheckOnColors,
    CheckSpellingTokens, CheckSwitchColors, CheckTokens, ChipTokens, ColorPickerTokens, ColorRampTokens, ColorToken, ColorWheelTokens,
    CommandPaletteTokens, ComponentTokens, ConnectorTokens, ConstrainTokens, ControlSizes, CornerTokens, CropTokens, CursorTokens, CurveTokens,
    DateTokens, DialogTokens, Dimmed, DockTokens, DragTokens, DropVeilTokens, Easing, Edge, Elevation, ElevationTokens, EmptyStateTokens,
    EventLogTokens, ExportTokens, EyedropperTokens, FieldTokens, FileListTokens, FilterGalleryTokens, FindingsTokens, FloatTokens, FontPickerTokens,
    FontSlot, FormSpacing, FormTokens, FullWindowTokens, Gaps, GlyphsTokens, GradientEditorTokens, GradientLineTokens, GradientPaletteTokens,
    GridTokens, GripTokens, GroupBoxTokens, GuideTokens, HandleTokens, HarmonyTokens, HistogramTokens, HistoryTokens, HoverTokens, IconButtonTokens,
    IconPickerTokens, IconSizes, InspectorTokens, ItemColors, KbdTokens, LabelColor, LayerStyleTokens, LayersTokens, LevelsTokens, LineGuideTokens,
    LinkTokens, LiquifyTokens, ListManagerTokens, ListSizes, ListTokens, ListboxTokens, MarqueeTokens, MeasureTokens, MenuHue, MenuTokens,
    MetadataTokens, Motion, MotionTokens, NavigatorTokens, NewDocumentTokens, NoticeTokens, OptionsBarTokens, OutlineDraftTokens, Pads,
    PaintChipTokens, PanelFooterTokens, PanelHeaderTokens, PanelTokens, PasswordTokens, PasteboardTokens, PopoverTokens, PopupTokens,
    PreferencesTokens, PresetSizes, PresetTokens, ProgressTokens, PropertyTokens, RadiusSet, RailSizes, RailTokens, RatingTokens, ReadoutTokens,
    RefPointTokens, ResetDotTokens, Rgba, RulerTokens, SamplerTokens, ScopesTokens, ScreenPickerTokens, ScrollTokens, SectionTokens, SegmentedTokens,
    SelectMaskTokens, SelectTokens, SelectionBarTokens, SelectionFrameTokens, ShapeDragTokens, ShapeTokens, SizeTokens, SliceTokens, SliderRowTokens,
    SliderTokens, SpaceTokens, SpinnerTokens, SplitButtonTokens, SplitTokens, StartTokens, States, StatusBarTokens, StrokeEditorTokens, SwatchColor,
    SwatchPaletteTokens, SwatchTokens, SymmetryTokens, TabSizes, TabTokens, TableTokens, TextEditorTokens, TextStylesTokens, ThemeChipTokens,
    ThumbColors, ThumbSizes, ThumbnailTokens, TileTokens, TitleBarTokens, ToastTokens, ToggleButtonTokens, Tokens, Tone, ToolPillTokens,
    ToolbarTokens, TooltipTokens, TourTokens, TransformTokens, TreePickerTokens, TypeRole, TypeStyle, TypeTokens, VanishingPointTokens,
    WideAngleTokens,
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
    let SizeTokens { control, icon, rail, bar, panel_header, list, tab: tab_sizes, preset } = size;
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
    let BarSizes { title, tab, status, options, options_narrow } = bar;
    for (k, v) in [("title", title), ("tab", tab), ("status", status), ("options", options), ("options_narrow", options_narrow)] {
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
    let PresetSizes {
        header_height,
        grid_gap,
        grid_inset,
        cell_radius,
        caption_gap,
        slider_width,
        picker_wide_width,
        picker_wide_height,
        picker_square,
        picker_cell,
        name_width,
        popup_max_height,
        footer_room,
    } = preset;
    for (k, v) in [
        ("header_height", header_height),
        ("grid_gap", grid_gap),
        ("grid_inset", grid_inset),
        ("cell_radius", cell_radius),
        ("caption_gap", caption_gap),
        ("slider_width", slider_width),
        ("picker_wide_width", picker_wide_width),
        ("picker_wide_height", picker_wide_height),
        ("picker_square", picker_square),
        ("picker_cell", picker_cell),
        ("name_width", name_width),
        ("popup_max_height", popup_max_height),
        ("footer_room", footer_room),
    ] {
        out.line(&format!("size.preset.{k}"), len(*v));
    }
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
        TypeRole::BodyLarge => "body_large",
        TypeRole::Mono => "mono",
    }
}

fn type_lines(out: &mut Lines, t: &TypeTokens) {
    let TypeTokens { caption, label, body, body_strong, title, heading, body_large, mono } = t;
    let roles = [
        ("caption", caption),
        ("label", label),
        ("body", body),
        ("body_strong", body_strong),
        ("title", title),
        ("heading", heading),
        ("body_large", body_large),
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
    let ElevationTokens { popup, floating, knob } = e;
    for (k, level) in [("popup", popup), ("floating", floating), ("knob", knob)] {
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
    let MotionTokens { hover, open, r#move, caret, outline, reduced } = m;
    for (k, motion) in [("hover", hover), ("open", open), ("move", r#move), ("caret", caret), ("outline", outline)] {
        let Motion { millis, easing: e } = motion;
        out.line(&format!("motion.{k}.millis"), millis);
        out.line(&format!("motion.{k}.easing"), easing(*e));
    }
    out.line("motion.reduced", reduced);
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
        table,
        chip,
        icon_picker,
        split_button,
        breadcrumb,
        rating,
        date,
        reset_dot,
        drag,
        drop_veil,
        event_log,
        autocomplete,
        metadata,
        about,
        password,
        list_manager,
        align,
        tree_picker,
        alt_text,
        export,
        file_list,
        findings,
        new_document,
        theme_chip,
        thumbnail,
        tile,
        link,
        navigator,
        scopes,
        actions,
        app_shell,
        badge,
        bezier,
        button,
        canvas,
        channels,
        check,
        color_picker,
        select,
        hover,
        outline_draft,
        selection_frame,
        grip,
        constrain,
        line_guide,
        before_after,
        brush,
        full_window,
        corner,
        stroke_editor,
        measure,
        symmetry,
        adjust,
        layer_style,
        blend_if,
        add_beside,
        shape_drag,
        slice,
        artboard,
        connector,
        vanishing_point,
        wide_angle,
        blur_pins,
        liquify,
        select_mask,
        filter_gallery,
        color_ramp,
        color_wheel,
        command_palette,
        curve,
        crop,
        cursor,
        dialog,
        dock,
        empty_state,
        eyedropper,
        field,
        float,
        form,
        check_spelling,
        font_picker,
        text_styles,
        glyphs,
        gradient_editor,
        gradient_line,
        grid,
        group_box,
        gradient_palette,
        guide,
        handle,
        harmony,
        histogram,
        history,
        icon_button,
        inspector,
        kbd,
        layers,
        levels,
        list,
        listbox,
        marquee,
        menu,
        notice,
        options_bar,
        paint_chip,
        panel,
        panel_footer,
        panel_header,
        pasteboard,
        popover,
        popup,
        preferences,
        preset,
        progress,
        property,
        rail,
        readout,
        ref_point,
        ruler,
        sampler,
        screen_picker,
        scroll,
        section,
        segmented,
        selection_bar,
        slider,
        slider_row,
        spinner,
        split,
        start,
        status_bar,
        swatch,
        swatch_palette,
        tab,
        text_editor,
        title_bar,
        toast,
        toggle_button,
        tool_pill,
        toolbar,
        tooltip,
        tour,
        transform,
    } = c;
    angle_dial_lines(out, angle_dial);
    table_lines(out, table);
    chip_lines(out, chip);
    icon_picker_lines(out, icon_picker);
    split_button_lines(out, split_button);
    breadcrumb_lines(out, breadcrumb);
    rating_lines(out, rating);
    date_lines(out, date);
    reset_dot_lines(out, reset_dot);
    drag_lines(out, drag);
    drop_veil_lines(out, drop_veil);
    event_log_lines(out, event_log);
    autocomplete_lines(out, autocomplete);
    metadata_lines(out, metadata);
    about_lines(out, about);
    password_lines(out, password);
    list_manager_lines(out, list_manager);
    align_lines(out, align);
    tree_picker_lines(out, tree_picker);
    alt_text_lines(out, alt_text);
    export_lines(out, export);
    file_list_lines(out, file_list);
    findings_lines(out, findings);
    new_document_lines(out, new_document);
    theme_chip_lines(out, theme_chip);
    app_shell_lines(out, app_shell);
    badge_lines(out, badge);
    bezier_lines(out, bezier);
    button_lines(out, button);
    canvas_lines(out, canvas);
    check_lines(out, check);
    color_picker_lines(out, color_picker);
    select_lines(out, select);
    hover_lines(out, hover);
    outline_draft_lines(out, outline_draft);
    selection_frame_lines(out, selection_frame);
    grip_lines(out, grip);
    constrain_lines(out, constrain);
    line_guide_lines(out, line_guide);
    before_after_lines(out, before_after);
    brush_lines(out, brush);
    full_window_lines(out, full_window);
    corner_lines(out, corner);
    stroke_editor_lines(out, stroke_editor);
    measure_lines(out, measure);
    symmetry_lines(out, symmetry);
    adjust_lines(out, adjust);
    layer_style_lines(out, layer_style);
    blend_if_lines(out, blend_if);
    add_beside_lines(out, add_beside);
    shape_drag_lines(out, shape_drag);
    slice_lines(out, slice);
    artboard_lines(out, artboard);
    connector_lines(out, connector);
    vanishing_point_lines(out, vanishing_point);
    wide_angle_lines(out, wide_angle);
    blur_pins_lines(out, blur_pins);
    liquify_lines(out, liquify);
    select_mask_lines(out, select_mask);
    filter_gallery_lines(out, filter_gallery);
    color_ramp_lines(out, color_ramp);
    color_wheel_lines(out, color_wheel);
    command_palette_lines(out, command_palette);
    curve_lines(out, curve);
    crop_lines(out, crop);
    cursor_lines(out, cursor);
    dialog_lines(out, dialog);
    dock_lines(out, dock);
    empty_state_lines(out, empty_state);
    eyedropper_lines(out, eyedropper);
    field_lines(out, field);
    float_lines(out, float);
    form_lines(out, form);
    check_spelling_lines(out, check_spelling);
    font_picker_lines(out, font_picker);
    text_styles_lines(out, text_styles);
    glyphs_lines(out, glyphs);
    gradient_editor_lines(out, gradient_editor);
    gradient_line_lines(out, gradient_line);
    grid_lines(out, grid);
    group_box_lines(out, group_box);
    gradient_palette_lines(out, gradient_palette);
    guide_lines(out, guide);
    handle_lines(out, handle);
    harmony_lines(out, harmony);
    histogram_lines(out, histogram);
    icon_button_lines(out, icon_button);
    inspector_lines(out, inspector);
    kbd_lines(out, kbd);
    levels_lines(out, levels);
    list_lines(out, list);
    listbox_lines(out, listbox);
    marquee_lines(out, marquee);
    menu_lines(out, menu);
    notice_lines(out, notice);
    options_bar_lines(out, options_bar);
    paint_chip_lines(out, paint_chip);
    panel_lines(out, panel);
    panel_footer_lines(out, panel_footer);
    panel_header_lines(out, panel_header);
    pasteboard_lines(out, pasteboard);
    popover_lines(out, popover);
    popup_lines(out, popup);
    preferences_lines(out, preferences);
    preset_lines(out, preset);
    progress_lines(out, progress);
    property_lines(out, property);
    rail_lines(out, rail);
    readout_lines(out, readout);
    ref_point_lines(out, ref_point);
    ruler_lines(out, ruler);
    sampler_lines(out, sampler);
    screen_picker_lines(out, screen_picker);
    scroll_lines(out, scroll);
    section_lines(out, section);
    segmented_lines(out, segmented);
    selection_bar_lines(out, selection_bar);
    slider_lines(out, slider);
    slider_row_lines(out, slider_row);
    spinner_lines(out, spinner);
    split_lines(out, split);
    start_lines(out, start);
    status_bar_lines(out, status_bar);
    swatch_lines(out, swatch);
    swatch_palette_lines(out, swatch_palette);
    thumbnail_lines(out, thumbnail);
    tile_lines(out, tile);
    link_lines(out, link);
    navigator_lines(out, navigator);
    scopes_lines(out, scopes);
    actions_lines(out, actions);
    channels_lines(out, channels);
    layers_lines(out, layers);
    history_lines(out, history);
    tab_lines(out, tab);
    text_editor_lines(out, text_editor);
    title_bar_lines(out, title_bar);
    toast_lines(out, toast);
    toggle_button_lines(out, toggle_button);
    tool_pill_lines(out, tool_pill);
    toolbar_lines(out, toolbar);
    tooltip_lines(out, tooltip);
    tour_lines(out, tour);
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

fn color_wheel_lines(out: &mut Lines, t: &ColorWheelTokens) {
    let ColorWheelTokens {
        side,
        side_min,
        ring_min,
        ring_max,
        ring_ratio,
        outer_inset,
        triangle_inset,
        grab_outer,
        grab_inner,
        disc_inset,
        disc_marker,
        disc_marker_selected,
        disc_grab,
        disc_floor,
        spoke,
    } = t;
    out.line("color_wheel.side", len(*side));
    out.line("color_wheel.side_min", len(*side_min));
    out.line("color_wheel.ring_min", len(*ring_min));
    out.line("color_wheel.ring_max", len(*ring_max));
    out.line("color_wheel.ring_ratio", len(*ring_ratio));
    out.line("color_wheel.outer_inset", len(*outer_inset));
    out.line("color_wheel.triangle_inset", len(*triangle_inset));
    out.line("color_wheel.grab_outer", len(*grab_outer));
    out.line("color_wheel.grab_inner", len(*grab_inner));
    out.line("color_wheel.disc_inset", len(*disc_inset));
    out.line("color_wheel.disc_marker", len(*disc_marker));
    out.line("color_wheel.disc_marker_selected", len(*disc_marker_selected));
    out.line("color_wheel.disc_grab", len(*disc_grab));
    out.line("color_wheel.disc_floor", len(*disc_floor));
    out.line("color_wheel.spoke", hex(*spoke));
}

fn color_ramp_lines(out: &mut Lines, t: &ColorRampTokens) {
    let ColorRampTokens { height, height_min, boxes, boxes_gap, cells_x, cells_y } = t;
    out.line("color_ramp.height", len(*height));
    out.line("color_ramp.height_min", len(*height_min));
    out.line("color_ramp.boxes", len(*boxes));
    out.line("color_ramp.boxes_gap", len(*boxes_gap));
    out.line("color_ramp.cells_x", *cells_x);
    out.line("color_ramp.cells_y", *cells_y);
}

fn gradient_palette_lines(out: &mut Lines, t: &GradientPaletteTokens) {
    let GradientPaletteTokens { cell_width, cell_height, list_swatch_width, list_swatch_height, max_height, cell_room } = t;
    out.line("gradient_palette.cell_width", len(*cell_width));
    out.line("gradient_palette.cell_height", len(*cell_height));
    out.line("gradient_palette.list_swatch_width", len(*list_swatch_width));
    out.line("gradient_palette.list_swatch_height", len(*list_swatch_height));
    out.line("gradient_palette.max_height", len(*max_height));
    out.line("gradient_palette.cell_room", len(*cell_room));
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
        pair_icon,
        pair_icon_gap,
        readout_hex_width,
        readout_channel_width,
        grid_gap,
        grid_row_gap,
        grid_group_gap,
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
    out.line("color_picker.pair_icon", len(*pair_icon));
    out.line("color_picker.pair_icon_gap", len(*pair_icon_gap));
    out.line("color_picker.readout_hex_width", len(*readout_hex_width));
    out.line("color_picker.readout_channel_width", len(*readout_channel_width));
    out.line("color_picker.grid_gap", len(*grid_gap));
    out.line("color_picker.grid_row_gap", len(*grid_row_gap));
    out.line("color_picker.grid_group_gap", len(*grid_group_gap));
    out.line("color_picker.marker", hex(*marker));
    out.line("color_picker.marker_halo", hex(*marker_halo));
}

fn slider_row_lines(out: &mut Lines, t: &SliderRowTokens) {
    let SliderRowTokens { field_width, gap } = t;
    out.line("slider_row.field_width", len(*field_width));
    out.line("slider_row.gap", len(*gap));
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
        row_height_dense,
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
        bracket_outset,
        bracket_arm,
        ghost_offset,
        label,
        label_strong,
        detail_role,
        text,
        dimmed,
        muted,
        selected_text,
        selected_dimmed,
        detail,
        selected_detail,
        icon,
        icon_hover,
        hover,
        selected,
        selected_hover,
        stripe,
        divider,
        thumb_border,
        thumb_border_selected,
        thumb_radius,
        layer_hover,
        layer_selected,
        layer_outline,
        layer_radius,
        layer_selected_text,
        layer_selected_dimmed,
        layer_selected_detail,
        drop,
        drop_halo,
        ghost_fill,
        ghost_border,
        labels,
    } = t;
    out.line("list.row_height", len(*row_height));
    out.line("list.row_height_dense", len(*row_height_dense));
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
    out.line("list.thumb.radius", len(*thumb_radius));
    out.line("list.layer.radius", len(*layer_radius));
    out.line("list.drop_width", len(*drop_width));
    out.line("list.drop_halo_width", len(*drop_halo_width));
    out.line("list.bracket_outset", len(*bracket_outset));
    out.line("list.bracket_arm", len(*bracket_arm));
    out.line("list.ghost_offset", format!("{} {}", len(ghost_offset[0]), len(ghost_offset[1])));
    out.line("list.label", role(*label));
    out.line("list.label_strong", role(*label_strong));
    out.line("list.detail_role", role(*detail_role));
    for (k, c) in [
        ("text", text),
        ("text.dimmed", dimmed),
        ("text.muted", muted),
        ("text.selected", selected_text),
        ("text.selected.dimmed", selected_dimmed),
        ("detail", detail),
        ("detail.selected", selected_detail),
        ("icon", icon),
        ("icon.hover", icon_hover),
        ("hover", hover),
        ("selected", selected),
        ("selected.hover", selected_hover),
        ("stripe", stripe),
        ("divider", divider),
        ("thumb.border", thumb_border),
        ("thumb.border.selected", thumb_border_selected),
        ("layer.hover", layer_hover),
        ("layer.selected", layer_selected),
        ("layer.outline", layer_outline),
        ("layer.text.selected", layer_selected_text),
        ("layer.text.selected.dimmed", layer_selected_dimmed),
        ("layer.detail.selected", layer_selected_detail),
        ("drop", drop),
        ("drop.halo", drop_halo),
        ("ghost.fill", ghost_fill),
        ("ghost.border", ghost_border),
    ] {
        out.line(&format!("list.{k}"), hex(*c));
    }
    for (color, tint) in LabelColor::ALL.into_iter().zip(labels) {
        out.line(&format!("list.label.{}.fill", color.name()), hex(tint.fill));
        out.line(&format!("list.label.{}.icon", color.name()), hex(tint.icon));
        out.line(&format!("list.label.{}.swatch", color.name()), hex(tint.swatch));
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
        text_gap,
        text_height,
        underline_width,
        underline_gap,
        label,
        label_large,
        detail_role,
        text_label_role,
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
        underline,
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
    out.line("tab.text_gap", len(*text_gap));
    out.line("tab.text_height", len(*text_height));
    out.line("tab.underline_width", len(*underline_width));
    out.line("tab.underline_gap", len(*underline_gap));
    out.line("tab.label", role(*label));
    out.line("tab.label_large", role(*label_large));
    out.line("tab.detail_role", role(*detail_role));
    out.line("tab.text_label_role", role(*text_label_role));
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
        ("underline", underline),
    ] {
        out.line(&format!("tab.{k}"), hex(*c));
    }
}

fn dock_lines(out: &mut Lines, t: &DockTokens) {
    let DockTokens {
        gap,
        body_gap,
        border_width,
        title_height,
        column_bar_height,
        icon_size,
        icon_pad,
        iconic_width,
        min_pane,
        drag_bar_width,
        iconic_max_width,
        iconic_label_width,
        flyout_height,
        overflow_width,
        menu_max_height,
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
        ("body_gap", body_gap),
        ("border_width", border_width),
        ("title_height", title_height),
        ("column_bar_height", column_bar_height),
        ("icon_size", icon_size),
        ("icon_pad", icon_pad),
        ("iconic_width", iconic_width),
        ("min_pane", min_pane),
        ("drag_bar_width", drag_bar_width),
        ("iconic_max_width", iconic_max_width),
        ("iconic_label_width", iconic_label_width),
        ("flyout_height", flyout_height),
        ("overflow_width", overflow_width),
        ("menu_max_height", menu_max_height),
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

fn dialog_lines(out: &mut Lines, t: &DialogTokens) {
    let DialogTokens {
        pad,
        pad_roomy,
        title_role,
        title_gap,
        body_gap,
        footer_gap,
        button_gap,
        button_min,
        side_gap,
        side_min,
        close_box,
        drag_pad,
        width_compact,
        width_min,
        width_standard,
        width_wide,
        width_alert,
        viewport_size,
        preview_pane,
        viewport_min,
        window_margin,
        resize_margin,
        keep_visible,
        scroll_min,
        anchor_cell,
        anchor_inset,
        anchor_mark_radius,
    } = t;
    out.line("dialog.pad", len(*pad));
    out.line("dialog.pad_roomy", len(*pad_roomy));
    out.line("dialog.title_role", role(*title_role));
    out.line("dialog.title_gap", len(*title_gap));
    out.line("dialog.body_gap", len(*body_gap));
    out.line("dialog.footer_gap", len(*footer_gap));
    out.line("dialog.button_gap", len(*button_gap));
    out.line("dialog.button_min", len(*button_min));
    out.line("dialog.side_gap", len(*side_gap));
    out.line("dialog.side_min", len(*side_min));
    out.line("dialog.close_box", len(*close_box));
    out.line("dialog.drag_pad", format!("{} {} {}", len(drag_pad[0]), len(drag_pad[1]), len(drag_pad[2])));
    out.line("dialog.width_compact", len(*width_compact));
    out.line("dialog.width_min", len(*width_min));
    out.line("dialog.width_standard", len(*width_standard));
    out.line("dialog.width_wide", len(*width_wide));
    out.line("dialog.width_alert", len(*width_alert));
    out.line("dialog.viewport_size", format!("{} {}", len(viewport_size[0]), len(viewport_size[1])));
    out.line("dialog.preview_pane", format!("{} {}", len(preview_pane[0]), len(preview_pane[1])));
    out.line("dialog.viewport_min", format!("{} {}", len(viewport_min[0]), len(viewport_min[1])));
    out.line("dialog.window_margin", len(*window_margin));
    out.line("dialog.resize_margin", len(*resize_margin));
    out.line("dialog.keep_visible", len(*keep_visible));
    out.line("dialog.scroll_min", len(*scroll_min));
    out.line("dialog.anchor_cell", len(*anchor_cell));
    out.line("dialog.anchor_inset", len(*anchor_inset));
    out.line("dialog.anchor_mark_radius", len(*anchor_mark_radius));
}

fn form_lines(out: &mut Lines, t: &FormTokens) {
    let FormTokens { normal, roomy } = t;
    for (prefix, FormSpacing { column_gap, row_gap, label_min }) in [("form", normal), ("form.roomy", roomy)] {
        out.line(&format!("{prefix}.column_gap"), len(*column_gap));
        out.line(&format!("{prefix}.row_gap"), len(*row_gap));
        out.line(&format!("{prefix}.label_min"), len(*label_min));
    }
}

fn float_lines(out: &mut Lines, t: &FloatTokens) {
    let FloatTokens {
        border_width,
        radius,
        width,
        height,
        min_fitted_height,
        cascade_offset,
        hint_offset,
        min_width,
        min_body_height,
        shadow,
        fill,
        border,
        blocked,
        title,
        title_text,
        title_muted,
        grip,
        focus,
    } = t;
    out.line("float.border_width", len(*border_width));
    out.line("float.radius", len(*radius));
    for (k, v) in [
        ("width", width),
        ("height", height),
        ("min_fitted_height", min_fitted_height),
        ("cascade_offset", cascade_offset),
        ("hint_offset", hint_offset),
        ("min_width", min_width),
        ("min_body_height", min_body_height),
    ] {
        out.line(&format!("float.{k}"), len(*v));
    }
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

fn options_bar_lines(out: &mut Lines, t: &OptionsBarTokens) {
    let OptionsBarTokens { height, height_narrow, row_gap, pad, gap, divider, field_sm, field_md, field_lg, scroll_bar, button, chevron, ink_room } =
        t;
    out.line("options_bar.height", len(*height));
    out.line("options_bar.height_narrow", len(*height_narrow));
    out.line("options_bar.row_gap", len(*row_gap));
    out.line("options_bar.pad", len(*pad));
    out.line("options_bar.gap", len(*gap));
    out.line("options_bar.divider", len(*divider));
    out.line("options_bar.field_sm", len(*field_sm));
    out.line("options_bar.field_md", len(*field_md));
    out.line("options_bar.field_lg", len(*field_lg));
    out.line("options_bar.scroll_bar", len(*scroll_bar));
    out.line("options_bar.button", len(*button));
    out.line("options_bar.chevron", format!("{} {}", len(chevron[0]), len(chevron[1])));
    out.line("options_bar.ink_room", len(*ink_room));
}

fn panel_lines(out: &mut Lines, t: &PanelTokens) {
    let PanelTokens { pad, fill, header_line, footer_line } = t;
    out.line("panel.pad", len(*pad));
    out.line("panel.fill", hex(*fill));
    out.line("panel.header.line", hex(*header_line));
    out.line("panel.footer.line", hex(*footer_line));
}

fn rail_lines(out: &mut Lines, t: &RailTokens) {
    let RailTokens {
        width,
        button,
        panel_width,
        panel_button,
        icon,
        panel_icon,
        gap,
        row_gap,
        panel_gap,
        group_gap,
        pad_y,
        toggle_height,
        toggle_icon,
        flyout_gap,
        indicator_size,
        indicator_inset,
        marker_radius,
        selected_border_width,
        radius,
        fill,
        selected,
        selected_border,
        selected_text,
        hover,
        icon_color,
        panel_icon_color,
        icon_hover,
        indicator_color,
        marker,
        edge,
    } = t;
    for (k, v) in [
        ("width", width),
        ("button", button),
        ("panel_width", panel_width),
        ("panel_button", panel_button),
        ("icon_size", icon),
        ("panel_icon_size", panel_icon),
        ("gap", gap),
        ("row_gap", row_gap),
        ("panel_gap", panel_gap),
        ("group_gap", group_gap),
        ("pad_y", pad_y),
        ("toggle_height", toggle_height),
        ("toggle_icon", toggle_icon),
        ("flyout_gap", flyout_gap),
        ("indicator_size", indicator_size),
        ("indicator_inset", indicator_inset),
        ("marker_radius", marker_radius),
        ("selected_border_width", selected_border_width),
        ("radius", radius),
    ] {
        out.line(&format!("rail.{k}"), len(*v));
    }
    for (k, c) in [
        ("fill", fill),
        ("selected", selected),
        ("selected.border", selected_border),
        ("selected.text", selected_text),
        ("hover", hover),
        ("icon", icon_color),
        ("icon.panel", panel_icon_color),
        ("icon.hover", icon_hover),
        ("indicator", indicator_color),
        ("marker", marker),
        ("edge", edge),
    ] {
        out.line(&format!("rail.{k}"), hex(*c));
    }
}

fn group_box_lines(out: &mut Lines, t: &GroupBoxTokens) {
    let GroupBoxTokens { edge, radius, padding, gap } = t;
    out.line("group_box.edge", hex(*edge));
    out.line("group_box.radius", len(*radius));
    out.line("group_box.padding", len(*padding));
    out.line("group_box.gap", len(*gap));
}

fn preferences_lines(out: &mut Lines, t: &PreferencesTokens) {
    let PreferencesTokens {
        list_width,
        page_gap,
        choice_width,
        text_width,
        page_inset,
        page_row_height,
        hex_width,
        checks_inset,
        body_fallback_height,
    } = t;
    out.line("preferences.list_width", len(*list_width));
    out.line("preferences.page_gap", len(*page_gap));
    out.line("preferences.choice_width", len(*choice_width));
    out.line("preferences.text_width", len(*text_width));
    out.line("preferences.page_inset", len(*page_inset));
    out.line("preferences.page_row_height", len(*page_row_height));
    out.line("preferences.hex_width", len(*hex_width));
    out.line("preferences.checks_inset", len(*checks_inset));
    out.line("preferences.body_fallback_height", len(*body_fallback_height));
}

fn preset_lines(out: &mut Lines, t: &PresetTokens) {
    let PresetTokens {
        header_height,
        grid_gap,
        grid_inset,
        cell_radius,
        caption_gap,
        slider_width,
        picker_square,
        picker_cell,
        picker_wide_width,
        picker_wide_height,
        name_width,
        popup_max_height,
        footer_room,
        thumb_list_min,
        thumb_list_max,
        ghost_thumb,
        toolbar_gap,
        ring_hover_width,
        ring_halo_width,
        border,
        ring_hover,
        ring_halo,
        caption,
        caption_selected,
        count,
        header_text,
        header_icon,
    } = t;
    for (k, v) in [
        ("header_height", header_height),
        ("grid_gap", grid_gap),
        ("grid_inset", grid_inset),
        ("cell_radius", cell_radius),
        ("caption_gap", caption_gap),
        ("slider_width", slider_width),
        ("picker_square", picker_square),
        ("picker_cell", picker_cell),
        ("picker_wide_width", picker_wide_width),
        ("picker_wide_height", picker_wide_height),
        ("name_width", name_width),
        ("popup_max_height", popup_max_height),
        ("footer_room", footer_room),
        ("thumb_list_min", thumb_list_min),
        ("thumb_list_max", thumb_list_max),
        ("ghost_thumb", ghost_thumb),
        ("toolbar_gap", toolbar_gap),
        ("ring.hover_width", ring_hover_width),
        ("ring.halo_width", ring_halo_width),
    ] {
        out.line(&format!("preset.{k}"), len(*v));
    }
    for (k, c) in [
        ("border", border),
        ("ring.hover", ring_hover),
        ("ring.halo", ring_halo),
        ("caption", caption),
        ("caption.selected", caption_selected),
        ("count", count),
        ("header.text", header_text),
        ("header.icon", header_icon),
    ] {
        out.line(&format!("preset.{k}"), hex(*c));
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

fn crop_lines(out: &mut Lines, t: &CropTokens) {
    let CropTokens {
        hit,
        line,
        guide_line,
        bracket,
        bracket_arm,
        bar_share,
        shield_opacity,
        shield_editing,
        grid_step,
        grid_max,
        straighten_min,
        straighten_line,
        straighten_halo,
        perspective_corner,
        perspective_corner_edge,
        perspective_outline,
        perspective_grid_width,
        perspective_grid_step,
        perspective_click_dot,
        perspective_click_move,
        perspective_reach,
        menu_width,
        menu_max_width,
    } = t;
    out.line("crop.hit", len(*hit));
    out.line("crop.line", len(*line));
    out.line("crop.guide_line", len(*guide_line));
    out.line("crop.bracket", len(*bracket));
    out.line("crop.bracket_arm", len(*bracket_arm));
    out.line("crop.bar_share", len(*bar_share));
    out.line("crop.shield_opacity", len(*shield_opacity));
    out.line("crop.shield_editing", len(*shield_editing));
    out.line("crop.grid_step", len(*grid_step));
    out.line("crop.grid_max", len(*grid_max));
    out.line("crop.straighten_min", len(*straighten_min));
    out.line("crop.straighten_line", len(*straighten_line));
    out.line("crop.straighten_halo", len(*straighten_halo));
    out.line("crop.perspective_corner", len(*perspective_corner));
    out.line("crop.perspective_corner_edge", len(*perspective_corner_edge));
    out.line("crop.perspective_outline", len(*perspective_outline));
    out.line("crop.perspective_grid_width", len(*perspective_grid_width));
    out.line("crop.perspective_grid_step", len(*perspective_grid_step));
    out.line("crop.perspective_click_dot", len(*perspective_click_dot));
    out.line("crop.perspective_click_move", len(*perspective_click_move));
    out.line("crop.perspective_reach", len(*perspective_reach));
    out.line("crop.menu_width", len(*menu_width));
    out.line("crop.menu_max_width", len(*menu_max_width));
}

fn check_spelling_lines(out: &mut Lines, t: &CheckSpellingTokens) {
    let CheckSpellingTokens { field_width, suggestions_height } = t;
    out.line("check_spelling.field_width", len(*field_width));
    out.line("check_spelling.suggestions_height", len(*suggestions_height));
}

fn font_picker_lines(out: &mut Lines, t: &FontPickerTokens) {
    let FontPickerTokens { list_height, list_width, class_column, class_glyph, missing_list_height } = t;
    out.line("font_picker.list_height", len(*list_height));
    out.line("font_picker.list_width", len(*list_width));
    out.line("font_picker.class_column", len(*class_column));
    out.line("font_picker.class_glyph", len(*class_glyph));
    out.line("font_picker.missing_list_height", len(*missing_list_height));
}

fn text_styles_lines(out: &mut Lines, t: &TextStylesTokens) {
    let TextStylesTokens { list_height } = t;
    out.line("text_styles.list_height", len(*list_height));
}

fn glyphs_lines(out: &mut Lines, t: &GlyphsTokens) {
    let GlyphsTokens { grid_height, cell, cell_min, cell_max, glyph_inset, recent_cell, recent_pitch, recent_pad, recent_glyph_inset } = t;
    out.line("glyphs.grid_height", len(*grid_height));
    out.line("glyphs.cell", len(*cell));
    out.line("glyphs.cell_min", len(*cell_min));
    out.line("glyphs.cell_max", len(*cell_max));
    out.line("glyphs.glyph_inset", len(*glyph_inset));
    out.line("glyphs.recent_cell", len(*recent_cell));
    out.line("glyphs.recent_pitch", len(*recent_pitch));
    out.line("glyphs.recent_pad", len(*recent_pad));
    out.line("glyphs.recent_glyph_inset", len(*recent_glyph_inset));
}

fn text_editor_lines(out: &mut Lines, t: &TextEditorTokens) {
    let TextEditorTokens { caret_halo, caret_line, selection_alpha, preedit_underline, active_underline } = t;
    out.line("text_editor.caret_halo", len(*caret_halo));
    out.line("text_editor.caret_line", len(*caret_line));
    out.line("text_editor.selection_alpha", len(*selection_alpha));
    out.line("text_editor.preedit_underline", len(*preedit_underline));
    out.line("text_editor.active_underline", len(*active_underline));
}

fn grid_lines(out: &mut Lines, t: &GridTokens) {
    let GridTokens { width, minor_alpha, min_step, crossing, crossing_min } = t;
    out.line("grid.width", len(*width));
    out.line("grid.minor_alpha", len(*minor_alpha));
    out.line("grid.min_step", len(*min_step));
    out.line("grid.crossing", len(*crossing));
    out.line("grid.crossing_min", len(*crossing_min));
}

fn guide_lines(out: &mut Lines, g: &GuideTokens) {
    let GuideTokens {
        width,
        hit,
        dash,
        gap,
        dot_gap,
        dot_radius,
        measure_cap,
        measure_pad_x,
        measure_pad_y,
        number_width,
        size_width,
        margin_width,
        position_width,
        color_width,
        layout_list_width,
        layout_color_width,
    } = g;
    out.line("guide.width", len(*width));
    out.line("guide.hit", len(*hit));
    out.line("guide.dash", len(*dash));
    out.line("guide.gap", len(*gap));
    out.line("guide.dot_gap", len(*dot_gap));
    out.line("guide.dot_radius", len(*dot_radius));
    out.line("guide.measure_cap", len(*measure_cap));
    out.line("guide.measure_pad_x", len(*measure_pad_x));
    out.line("guide.measure_pad_y", len(*measure_pad_y));
    out.line("guide.number_width", len(*number_width));
    out.line("guide.size_width", len(*size_width));
    out.line("guide.margin_width", len(*margin_width));
    out.line("guide.position_width", len(*position_width));
    out.line("guide.color_width", len(*color_width));
    out.line("guide.layout_list_width", len(*layout_list_width));
    out.line("guide.layout_color_width", len(*layout_color_width));
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
        pin,
        pin_ring,
        pin_core,
        pin_turn,
        pin_turn_line,
        mesh_line,
        plane_corner,
        plane_ring,
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
    for (name, v) in [
        ("pin", pin),
        ("pin_ring", pin_ring),
        ("pin_core", pin_core),
        ("pin_turn", pin_turn),
        ("pin_turn_line", pin_turn_line),
        ("mesh_line", mesh_line),
        ("plane_corner", plane_corner),
        ("plane_ring", plane_ring),
    ] {
        out.line(&format!("transform.{name}"), len(*v));
    }
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

fn canvas_lines(out: &mut Lines, t: &CanvasTokens) {
    let CanvasTokens { compass, compass_margin } = t;
    out.line("canvas.compass", len(*compass));
    out.line("canvas.compass_margin", len(*compass_margin));
}

fn split_lines(out: &mut Lines, t: &SplitTokens) {
    let SplitTokens { gap, active_edge } = t;
    out.line("split.gap", len(*gap));
    out.line("split.active_edge", len(*active_edge));
}

fn pasteboard_lines(out: &mut Lines, t: &PasteboardTokens) {
    let PasteboardTokens { name_gap, outline_width } = t;
    out.line("pasteboard.name_gap", len(*name_gap));
    out.line("pasteboard.outline_width", len(*outline_width));
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
        precise_reach,
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
        stroke,
        badge,
        icon,
    } = c;
    out.line("cursor.crosshair_reach", len(*crosshair_reach));
    out.line("cursor.crosshair_gap", len(*crosshair_gap));
    out.line("cursor.precise_reach", len(*precise_reach));
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
    out.line("cursor.stroke", len(*stroke));
    out.line("cursor.badge", len(*badge));
    out.line("cursor.icon", len(*icon));
}

fn handle_lines(out: &mut Lines, h: &HandleTokens) {
    let HandleTokens { size, stroke, halo, hit } = h;
    out.line("handle.size", len(*size));
    out.line("handle.stroke", len(*stroke));
    out.line("handle.halo", len(*halo));
    out.line("handle.hit", len(*hit));
}

fn button_lines(out: &mut Lines, b: &ButtonTokens) {
    let ButtonTokens { height, pad_x, icon, gap, label, radius, border_width, hover_opacity, pressed_opacity, primary, secondary, ghost, danger } = b;
    by_size(out, "button.height", height);
    by_size(out, "button.pad_x", pad_x);
    by_size(out, "button.icon", icon);
    out.line("button.gap", len(*gap));
    out.line("button.label", role(*label));
    out.line("button.radius", len(*radius));
    out.line("button.border_width", len(*border_width));
    out.line("button.hover_opacity", len(*hover_opacity));
    out.line("button.pressed_opacity", len(*pressed_opacity));
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
        active_border,
        chosen_fill,
        chosen_text,
        chosen_border,
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
    out.line("listbox.active.border", hex(*active_border));
    out.line("listbox.chosen.fill", hex(*chosen_fill));
    out.line("listbox.chosen.text", hex(*chosen_text));
    out.line("listbox.chosen.border", hex(*chosen_border));
    out.line("listbox.mark", hex(*mark));
    out.line("listbox.group", hex(*group));
    out.line("listbox.separator", hex(*separator));
}

fn empty_state_lines(out: &mut Lines, e: &EmptyStateTokens) {
    let EmptyStateTokens { max_width, text, action_min_width, brand_title, brand_byline, brand_gap, brand_byline_lift } = e;
    out.line("empty_state.max_width", len(*max_width));
    out.line("empty_state.text", hex(*text));
    out.line("empty_state.action_min_width", len(*action_min_width));
    for (k, style) in [("brand_title", brand_title), ("brand_byline", brand_byline)] {
        let TypeStyle { size, line_height, font } = style;
        out.line(&format!("empty_state.{k}.size"), len(*size));
        out.line(&format!("empty_state.{k}.line_height"), len(*line_height));
        out.line(&format!("empty_state.{k}.font"), slot(*font));
    }
    out.line("empty_state.brand_gap", len(*brand_gap));
    out.line("empty_state.brand_byline_lift", len(*brand_byline_lift));
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

fn popover_lines(out: &mut Lines, p: &PopoverTokens) {
    let PopoverTokens { padding, grip_size, grip_pitch, grip_tick, screen_margin } = p;
    out.line("popover.padding", len(*padding));
    out.line("popover.grip.size", len(*grip_size));
    out.line("popover.grip.pitch", len(*grip_pitch));
    out.line("popover.grip.tick", hex(*grip_tick));
    out.line("popover.screen_margin", len(*screen_margin));
}

fn popup_lines(out: &mut Lines, p: &PopupTokens) {
    let PopupTokens { fill, border, shadow } = p;
    out.line("popup.fill", hex(*fill));
    out.line("popup.border", hex(*border));
    out.line("popup.shadow", hex(*shadow));
}

fn menu_lines(out: &mut Lines, m: &MenuTokens) {
    let MenuTokens { min_width, shortcut_gap, scroll_strip, radius, arrow_base, arrow_depth, active_fill, active_text, open_fill, shortcut, tints } =
        m;
    out.line("menu.min_width", len(*min_width));
    out.line("menu.shortcut_gap", len(*shortcut_gap));
    out.line("menu.scroll_strip", len(*scroll_strip));
    out.line("menu.radius", len(*radius));
    out.line("menu.arrow_base", len(*arrow_base));
    out.line("menu.arrow_depth", len(*arrow_depth));
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

fn inspector_lines(out: &mut Lines, t: &InspectorTokens) {
    let InspectorTokens { header_height, icon, pad, kind_min_name, title, kind } = t;
    out.line("inspector.header_height", len(*header_height));
    out.line("inspector.icon", len(*icon));
    out.line("inspector.pad", len(*pad));
    out.line("inspector.kind_min_name", len(*kind_min_name));
    out.line("inspector.title", hex(*title));
    out.line("inspector.kind", hex(*kind));
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
    let ProgressTokens { width, track_height, radius, track, edge, fill } = p;
    out.line("progress.width", len(*width));
    out.line("progress.track_height", len(*track_height));
    out.line("progress.radius", len(*radius));
    out.line("progress.track", hex(*track));
    out.line("progress.edge", hex(*edge));
    out.line("progress.fill", hex(*fill));
}

fn thumbnail_lines(out: &mut Lines, t: &ThumbnailTokens) {
    let ThumbnailTokens { checker_a, checker_b, checker_cell, placeholder, failed_icon } = t;
    out.line("thumbnail.checker.a", hex(*checker_a));
    out.line("thumbnail.checker.b", hex(*checker_b));
    out.line("thumbnail.checker_cell", len(*checker_cell));
    out.line("thumbnail.placeholder", hex(*placeholder));
    out.line("thumbnail.failed_icon", hex(*failed_icon));
}

fn tile_lines(out: &mut Lines, t: &TileTokens) {
    let TileTokens {
        gap,
        inset,
        group_header,
        caption_gap,
        bar,
        flag,
        dot,
        checkbox,
        checkbox_backdrop,
        badge_height,
        status_band,
        range_bar,
        ring_width,
        ring_hover_width,
        ring_halo_width,
        wheel_share,
        compare_caption,
        compare_margin,
        compare_gap,
        compare_name_room,
        bin_chip,
        bin_chip_radius,
        ring_selected,
        ring_halo,
        ring_hover,
        caption,
        caption_selected,
        detail,
        group_text,
        group_count,
        group_pinned,
        scrim,
        badge_text,
        veil,
        range,
        scrub,
        marquee_fill,
        marquee_line,
    } = t;
    out.line("tile.gap", len(*gap));
    out.line("tile.inset", len(*inset));
    out.line("tile.group_header", len(*group_header));
    out.line("tile.caption_gap", len(*caption_gap));
    out.line("tile.bar", len(*bar));
    out.line("tile.flag", len(*flag));
    out.line("tile.dot", len(*dot));
    out.line("tile.checkbox", len(*checkbox));
    out.line("tile.checkbox_backdrop", len(*checkbox_backdrop));
    out.line("tile.badge_height", len(*badge_height));
    out.line("tile.status_band", len(*status_band));
    out.line("tile.range_bar", len(*range_bar));
    out.line("tile.ring_width", len(*ring_width));
    out.line("tile.ring_hover_width", len(*ring_hover_width));
    out.line("tile.ring_halo_width", len(*ring_halo_width));
    out.line("tile.wheel_share", len(*wheel_share));
    out.line("tile.compare_caption", len(*compare_caption));
    out.line("tile.compare_margin", len(*compare_margin));
    out.line("tile.compare_gap", len(*compare_gap));
    out.line("tile.compare_name_room", len(*compare_name_room));
    out.line("tile.bin_chip", len(*bin_chip));
    out.line("tile.bin_chip_radius", len(*bin_chip_radius));
    out.line("tile.ring.selected", hex(*ring_selected));
    out.line("tile.ring.halo", hex(*ring_halo));
    out.line("tile.ring.hover", hex(*ring_hover));
    out.line("tile.caption", hex(*caption));
    out.line("tile.caption_selected", hex(*caption_selected));
    out.line("tile.detail", hex(*detail));
    out.line("tile.group.text", hex(*group_text));
    out.line("tile.group.count", hex(*group_count));
    out.line("tile.group.pinned", hex(*group_pinned));
    out.line("tile.scrim", hex(*scrim));
    out.line("tile.badge.text", hex(*badge_text));
    out.line("tile.veil", hex(*veil));
    out.line("tile.range", hex(*range));
    out.line("tile.scrub", hex(*scrub));
    out.line("tile.marquee.fill", hex(*marquee_fill));
    out.line("tile.marquee.line", hex(*marquee_line));
}

fn navigator_lines(out: &mut Lines, t: &NavigatorTokens) {
    let NavigatorTokens { proxy, proxy_width, halo, proxy_radius, inset, min_preview, checker, row_gap, proxy_colors } = t;
    out.line("navigator.proxy", hex(*proxy));
    out.line("navigator.proxy_width", len(*proxy_width));
    out.line("navigator.halo", len(*halo));
    out.line("navigator.proxy_radius", len(*proxy_radius));
    out.line("navigator.inset", len(*inset));
    out.line("navigator.min_preview", len(*min_preview));
    out.line("navigator.checker", len(*checker));
    out.line("navigator.row_gap", len(*row_gap));
    for (name, c) in ["red", "orange", "yellow", "green", "blue", "violet", "gray"].into_iter().zip(proxy_colors) {
        out.line(&format!("navigator.proxy_color.{name}"), hex(*c));
    }
}

fn scopes_lines(out: &mut Lines, t: &ScopesTokens) {
    let ScopesTokens {
        warning_size,
        statistics_gap,
        all_channels_height,
        vector_ring_alpha,
        vector_dot_min,
        vector_dot_max,
        hover_badge_height,
        zone_alpha,
        vector_height_min,
        vector_height_max,
        vector_margin,
        warning_inset,
    } = t;
    out.line("scopes.warning_size", len(*warning_size));
    out.line("scopes.statistics_gap", len(*statistics_gap));
    out.line("scopes.all_channels_height", len(*all_channels_height));
    out.line("scopes.vector_ring_alpha", len(*vector_ring_alpha));
    out.line("scopes.vector_dot_min", len(*vector_dot_min));
    out.line("scopes.vector_dot_max", len(*vector_dot_max));
    out.line("scopes.hover_badge_height", len(*hover_badge_height));
    out.line("scopes.zone_alpha", len(*zone_alpha));
    out.line("scopes.vector_height_min", len(*vector_height_min));
    out.line("scopes.vector_height_max", len(*vector_height_max));
    out.line("scopes.vector_margin", len(*vector_margin));
    out.line("scopes.warning_inset", len(*warning_inset));
}

fn actions_lines(out: &mut Lines, t: &ActionsTokens) {
    let ActionsTokens { set_row_height, row_height, step_row_height, recording_dot_radius, colors, label_on_color } = t;
    out.line("actions.set_row_height", len(*set_row_height));
    out.line("actions.row_height", len(*row_height));
    out.line("actions.step_row_height", len(*step_row_height));
    out.line("actions.recording_dot_radius", len(*recording_dot_radius));
    for (name, c) in ["red", "orange", "yellow", "green", "blue", "violet"].into_iter().zip(colors) {
        out.line(&format!("actions.color.{name}"), hex(*c));
    }
    out.line("actions.color.label", hex(*label_on_color));
}

fn link_lines(out: &mut Lines, t: &LinkTokens) {
    let LinkTokens { badge, preview, candidates_height, table_min, table_max, table_share, dialog_max_width } = t;
    out.line("link.badge", len(*badge));
    out.line("link.preview.width", len(preview[0]));
    out.line("link.preview.height", len(preview[1]));
    out.line("link.candidates_height", len(*candidates_height));
    out.line("link.table_min", len(*table_min));
    out.line("link.table_max", len(*table_max));
    out.line("link.table_share", len(*table_share));
    out.line("link.dialog_max_width", len(*dialog_max_width));
}

fn channels_lines(out: &mut Lines, t: &ChannelsTokens) {
    let ChannelsTokens { thumb, spot_swatch } = t;
    out.line("channels.thumb", len(*thumb));
    out.line("channels.spot_swatch", len(*spot_swatch));
}

fn history_lines(out: &mut Lines, t: &HistoryTokens) {
    let HistoryTokens { snapshot_thumb, undo_list_width } = t;
    out.line("history.snapshot_thumb", len(*snapshot_thumb));
    out.line("history.undo_list_width", len(*undo_list_width));
}

fn layers_lines(out: &mut Lines, t: &LayersTokens) {
    let LayersTokens {
        indicator_icon,
        indicator_triangle,
        indicator_fx,
        indicator_blend,
        indicator_lock,
        indicator_link,
        indicator_warn,
        mask_badge,
        mask_badge_fill,
        mask_badge_glyph,
        mask_disabled,
        blend_min_width,
        percent_field_width,
        filter_gap,
        filter_type_width,
        filter_type_gap,
        options_sample_scale,
        blend_label_slack,
        blend_floor_width,
        blend_row_gap,
        kind_type_ink,
        indicator_fx_gap,
        mask_cross_width,
        mask_cross_inset,
        indicator_ring_width,
    } = t;
    out.line("layers.indicator.icon", len(*indicator_icon));
    out.line("layers.indicator.triangle", len(*indicator_triangle));
    out.line("layers.indicator.fx", hex(*indicator_fx));
    out.line("layers.indicator.blend", hex(*indicator_blend));
    out.line("layers.indicator.lock", hex(*indicator_lock));
    out.line("layers.indicator.link", hex(*indicator_link));
    out.line("layers.indicator.warn", hex(*indicator_warn));
    out.line("layers.mask.badge", len(*mask_badge));
    out.line("layers.mask.badge_fill", hex(*mask_badge_fill));
    out.line("layers.mask.badge_glyph", hex(*mask_badge_glyph));
    out.line("layers.mask.disabled", hex(*mask_disabled));
    out.line("layers.blend.min_width", len(*blend_min_width));
    out.line("layers.percent_field_width", len(*percent_field_width));
    out.line("layers.filter.gap", len(*filter_gap));
    out.line("layers.filter.type_width", len(*filter_type_width));
    out.line("layers.filter.type_gap", len(*filter_type_gap));
    out.line("layers.options.sample_scale", len(*options_sample_scale));
    out.line("layers.blend.label_slack", len(*blend_label_slack));
    out.line("layers.blend.floor_width", len(*blend_floor_width));
    out.line("layers.blend.row_gap", len(*blend_row_gap));
    out.line("layers.kind.type_ink", hex(*kind_type_ink));
    out.line("layers.indicator.fx_gap", len(*indicator_fx_gap));
    out.line("layers.mask.cross_width", len(*mask_cross_width));
    out.line("layers.mask.cross_inset", len(*mask_cross_inset));
    out.line("layers.indicator.ring_width", len(*indicator_ring_width));
}

fn swatch_palette_lines(out: &mut Lines, s: &SwatchPaletteTokens) {
    let SwatchPaletteTokens { table_cell } = s;
    out.line("swatch_palette.table_cell", len(*table_cell));
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

fn toolbar_lines(out: &mut Lines, t: &ToolbarTokens) {
    let ToolbarTokens { button, button_large, gap, group_gap, pad, width, overflow, spacing } = t;
    out.line("toolbar.button", len(*button));
    out.line("toolbar.button_large", len(*button_large));
    out.line("toolbar.gap", len(*gap));
    out.line("toolbar.group_gap", len(*group_gap));
    out.line("toolbar.pad", format!("{} {}", len(pad[0]), len(pad[1])));
    out.line("toolbar.width", len(*width));
    out.line("toolbar.overflow", len(*overflow));
    out.line("toolbar.spacing", len(*spacing));
}

fn tooltip_lines(out: &mut Lines, t: &TooltipTokens) {
    let TooltipTokens { max_width, pointer_gap, radius, fill, text, border } = t;
    out.line("tooltip.max_width", len(*max_width));
    out.line("tooltip.pointer_gap", len(*pointer_gap));
    out.line("tooltip.radius", len(*radius));
    out.line("tooltip.fill", hex(*fill));
    out.line("tooltip.text", hex(*text));
    out.line("tooltip.border", hex(*border));
}

fn command_palette_lines(out: &mut Lines, t: &CommandPaletteTokens) {
    let CommandPaletteTokens { width, top, pad, row_height, max_rows, recent } = t;
    out.line("command_palette.width", len(*width));
    out.line("command_palette.top", len(*top));
    out.line("command_palette.pad", len(*pad));
    out.line("command_palette.row_height", len(*row_height));
    out.line("command_palette.max_rows", max_rows.to_string());
    out.line("command_palette.recent", recent.to_string());
}

fn status_bar_lines(out: &mut Lines, t: &StatusBarTokens) {
    let StatusBarTokens { height, pad, gap, group_gap, divider, field_width } = t;
    out.line("status_bar.height", len(*height));
    out.line("status_bar.pad", len(*pad));
    out.line("status_bar.gap", len(*gap));
    out.line("status_bar.group_gap", len(*group_gap));
    out.line("status_bar.divider", len(*divider));
    out.line("status_bar.field_width", len(*field_width));
}

fn spinner_lines(out: &mut Lines, t: &SpinnerTokens) {
    let SpinnerTokens { inset, stroke } = t;
    out.line("spinner.inset", len(*inset));
    out.line("spinner.stroke", len(*stroke));
}

fn app_shell_lines(out: &mut Lines, t: &AppShellTokens) {
    let AppShellTokens { edge_reveal, backdrop, body_only_backdrop } = t;
    out.line("app_shell.edge_reveal", len(*edge_reveal));
    out.line("app_shell.backdrop", hex(*backdrop));
    out.line("app_shell.body_only_backdrop", hex(*body_only_backdrop));
}

fn eyedropper_lines(out: &mut Lines, t: &EyedropperTokens) {
    let EyedropperTokens { ring_radius, ring_band, ring_grey } = t;
    out.line("eyedropper.ring_radius", len(*ring_radius));
    out.line("eyedropper.ring_band", len(*ring_band));
    out.line("eyedropper.ring_grey", hex(*ring_grey));
}

fn sampler_lines(out: &mut Lines, t: &SamplerTokens) {
    let SamplerTokens { marker_radius, tick_start, tick_end } = t;
    out.line("sampler.marker_radius", len(*marker_radius));
    out.line("sampler.tick_start", len(*tick_start));
    out.line("sampler.tick_end", len(*tick_end));
}

fn screen_picker_lines(out: &mut Lines, t: &ScreenPickerTokens) {
    let ScreenPickerTokens { loupe_cells, zoom, zoom_min, zoom_max, zoom_step } = t;
    out.line("screen_picker.loupe_cells", *loupe_cells);
    out.line("screen_picker.zoom", *zoom);
    out.line("screen_picker.zoom_min", *zoom_min);
    out.line("screen_picker.zoom_max", *zoom_max);
    out.line("screen_picker.zoom_step", *zoom_step);
}

fn paint_chip_lines(out: &mut Lines, t: &PaintChipTokens) {
    let PaintChipTokens { size } = t;
    out.line("paint_chip.size", format!("{} {}", len(size[0]), len(size[1])));
}

fn harmony_lines(out: &mut Lines, t: &HarmonyTokens) {
    let HarmonyTokens { base, strip, row, cell } = t;
    out.line("harmony.base", len(*base));
    out.line("harmony.strip", len(*strip));
    out.line("harmony.row", len(*row));
    out.line("harmony.cell", len(*cell));
}

fn start_lines(out: &mut Lines, t: &StartTokens) {
    let StartTokens { card_min, card_max } = t;
    out.line("start.card_min", len(*card_min));
    out.line("start.card_max", len(*card_max));
}

fn tool_pill_lines(out: &mut Lines, t: &ToolPillTokens) {
    let ToolPillTokens { inset } = t;
    out.line("tool_pill.inset", len(*inset));
}

fn tour_lines(out: &mut Lines, t: &TourTokens) {
    let TourTokens { card_width, target_gap, highlight_offset, highlight_pulse } = t;
    out.line("tour.card_width", len(*card_width));
    out.line("tour.target_gap", len(*target_gap));
    out.line("tour.highlight_offset", len(*highlight_offset));
    out.line("tour.highlight_pulse", len(*highlight_pulse));
}

fn selection_bar_lines(out: &mut Lines, t: &SelectionBarTokens) {
    let SelectionBarTokens { height, gap_below, gap_above, inset, first_width, grip_width, grip_height, grip_dot, grip_pitch } = t;
    out.line("selection_bar.height", len(*height));
    out.line("selection_bar.gap_below", len(*gap_below));
    out.line("selection_bar.gap_above", len(*gap_above));
    out.line("selection_bar.inset", len(*inset));
    out.line("selection_bar.first_width", len(*first_width));
    out.line("selection_bar.grip_width", len(*grip_width));
    out.line("selection_bar.grip_height", len(*grip_height));
    out.line("selection_bar.grip_dot", len(*grip_dot));
    out.line("selection_bar.grip_pitch", len(*grip_pitch));
}

fn new_document_lines(out: &mut Lines, t: &NewDocumentTokens) {
    let NewDocumentTokens {
        width,
        card,
        card_columns,
        card_rows,
        page_icon,
        page_box,
        card_icon_center,
        card_title_center,
        card_line_center,
        scroll_room,
        saved_height,
        saved_row,
    } = t;
    out.line("new_document.width", len(*width));
    out.line("new_document.card", format!("{} {}", len(card[0]), len(card[1])));
    out.line("new_document.card_columns", card_columns.to_string());
    out.line("new_document.card_rows", card_rows.to_string());
    out.line("new_document.page_icon", len(*page_icon));
    out.line("new_document.page_box", len(*page_box));
    out.line("new_document.card_icon_center", len(*card_icon_center));
    out.line("new_document.card_title_center", len(*card_title_center));
    out.line("new_document.card_line_center", len(*card_line_center));
    out.line("new_document.scroll_room", len(*scroll_room));
    out.line("new_document.saved_height", len(*saved_height));
    out.line("new_document.saved_row", len(*saved_row));
}

fn table_lines(out: &mut Lines, t: &TableTokens) {
    let TableTokens { header_height, sort_glyph, sort_gap, grip_width, drop_line_width, min_column, drop_line_slack } = t;
    out.line("table.header_height", len(*header_height));
    out.line("table.sort_glyph", len(*sort_glyph));
    out.line("table.sort_gap", len(*sort_gap));
    out.line("table.grip_width", len(*grip_width));
    out.line("table.drop_line_width", len(*drop_line_width));
    out.line("table.min_column", len(*min_column));
    out.line("table.drop_line_slack", len(*drop_line_slack));
}

fn chip_lines(out: &mut Lines, t: &ChipTokens) {
    let ChipTokens { height, pad_x, remove_width, gap, cross_arm, cross_stroke } = t;
    by_size(out, "chip.height", height);
    out.line("chip.pad_x", len(*pad_x));
    out.line("chip.remove_width", len(*remove_width));
    out.line("chip.gap", len(*gap));
    out.line("chip.cross_arm", len(*cross_arm));
    out.line("chip.cross_stroke", len(*cross_stroke));
}

fn icon_picker_lines(out: &mut Lines, t: &IconPickerTokens) {
    let IconPickerTokens { cell, gap, inset, max_height } = t;
    out.line("icon_picker.cell", len(*cell));
    out.line("icon_picker.gap", len(*gap));
    out.line("icon_picker.inset", len(*inset));
    out.line("icon_picker.max_height", len(*max_height));
}

fn split_button_lines(out: &mut Lines, t: &SplitButtonTokens) {
    let SplitButtonTokens { arrow_width, divider_inset } = t;
    out.line("split_button.arrow_width", len(*arrow_width));
    out.line("split_button.divider_inset", len(*divider_inset));
}

fn breadcrumb_lines(out: &mut Lines, t: &BreadcrumbTokens) {
    let BreadcrumbTokens { height, separator, gap, min_segment } = t;
    out.line("breadcrumb.height", len(*height));
    out.line("breadcrumb.separator", len(*separator));
    out.line("breadcrumb.gap", len(*gap));
    out.line("breadcrumb.min_segment", len(*min_segment));
}

fn rating_lines(out: &mut Lines, t: &RatingTokens) {
    let RatingTokens { star, star_size, cell, badge_star, badge_step } = t;
    out.line("rating.star", hex(*star));
    by_size(out, "rating.star", star_size);
    by_size(out, "rating.cell", cell);
    by_size(out, "rating.badge_star", badge_star);
    by_size(out, "rating.badge_step", badge_step);
}

fn date_lines(out: &mut Lines, t: &DateTokens) {
    let DateTokens { cell, gap, popup_width, page_button, cell_pad } = t;
    out.line("date.cell", len(*cell));
    out.line("date.gap", len(*gap));
    out.line("date.popup_width", len(*popup_width));
    out.line("date.page_button", len(*page_button));
    out.line("date.cell_pad", len(*cell_pad));
}

fn reset_dot_lines(out: &mut Lines, t: &ResetDotTokens) {
    let ResetDotTokens { size, dot, radius } = t;
    out.line("reset_dot.size", len(*size));
    out.line("reset_dot.dot", len(*dot));
    out.line("reset_dot.radius", len(*radius));
}

fn drag_lines(out: &mut Lines, t: &DragTokens) {
    let DragTokens { badge_offset_x, badge_offset_y, badge_pad_x, badge_pad_y } = t;
    out.line("drag.badge_offset_x", len(*badge_offset_x));
    out.line("drag.badge_offset_y", len(*badge_offset_y));
    out.line("drag.badge_pad_x", len(*badge_pad_x));
    out.line("drag.badge_pad_y", len(*badge_pad_y));
}

fn drop_veil_lines(out: &mut Lines, t: &DropVeilTokens) {
    let DropVeilTokens { inset, frame_width, frame_radius, card_radius, card_gap, card_pad_x, card_pad_y, wrap_margin, min_wrap } = t;
    out.line("drop_veil.inset", len(*inset));
    out.line("drop_veil.frame_width", len(*frame_width));
    out.line("drop_veil.frame_radius", len(*frame_radius));
    out.line("drop_veil.card_radius", len(*card_radius));
    out.line("drop_veil.card_gap", len(*card_gap));
    out.line("drop_veil.card_pad_x", len(*card_pad_x));
    out.line("drop_veil.card_pad_y", len(*card_pad_y));
    out.line("drop_veil.wrap_margin", len(*wrap_margin));
    out.line("drop_veil.min_wrap", len(*min_wrap));
}

fn event_log_lines(out: &mut Lines, t: &EventLogTokens) {
    let EventLogTokens { details_height, details_pad_y } = t;
    out.line("event_log.details_height", len(*details_height));
    out.line("event_log.details_pad_y", len(*details_pad_y));
}

fn autocomplete_lines(out: &mut Lines, t: &AutocompleteTokens) {
    let AutocompleteTokens { min_width, gap } = t;
    out.line("autocomplete.min_width", len(*min_width));
    out.line("autocomplete.gap", len(*gap));
}

fn metadata_lines(out: &mut Lines, t: &MetadataTokens) {
    let MetadataTokens { column_gap, row_gap, group_gap, max_height } = t;
    out.line("metadata.column_gap", len(*column_gap));
    out.line("metadata.row_gap", len(*row_gap));
    out.line("metadata.group_gap", len(*group_gap));
    out.line("metadata.max_height", len(*max_height));
}

fn export_lines(out: &mut Lines, t: &ExportTokens) {
    let ExportTokens {
        web_width,
        web_field_width,
        settings_width,
        pane_gap,
        pdf_width,
        pdf_list_width,
        pdf_content_width,
        pdf_content_height,
        pdf_gap,
    } = t;
    out.line("export.web_width", len(*web_width));
    out.line("export.web_field_width", len(*web_field_width));
    out.line("export.settings_width", len(*settings_width));
    out.line("export.pane_gap", len(*pane_gap));
    out.line("export.pdf_width", len(*pdf_width));
    out.line("export.pdf_list_width", len(*pdf_list_width));
    out.line("export.pdf_content_width", len(*pdf_content_width));
    out.line("export.pdf_content_height", len(*pdf_content_height));
    out.line("export.pdf_gap", len(*pdf_gap));
}

fn theme_chip_lines(out: &mut Lines, t: &ThemeChipTokens) {
    let ThemeChipTokens { size, inset, strip, dot_radius, dot_offset, outline_radius, fill_radius, badge_radius, badge_inset, gap } = t;
    out.line("theme_chip.size", format!("{} {}", len(size[0]), len(size[1])));
    out.line("theme_chip.inset", len(*inset));
    out.line("theme_chip.strip", len(*strip));
    out.line("theme_chip.dot_radius", len(*dot_radius));
    out.line("theme_chip.dot_offset", len(*dot_offset));
    out.line("theme_chip.outline_radius", len(*outline_radius));
    out.line("theme_chip.fill_radius", len(*fill_radius));
    out.line("theme_chip.badge_radius", len(*badge_radius));
    out.line("theme_chip.badge_inset", format!("{} {}", len(badge_inset[0]), len(badge_inset[1])));
    out.line("theme_chip.gap", len(*gap));
}

fn findings_lines(out: &mut Lines, t: &FindingsTokens) {
    let FindingsTokens { status_dot } = t;
    out.line("findings.status_dot", len(*status_dot));
}

fn file_list_lines(out: &mut Lines, t: &FileListTokens) {
    let FileListTokens { size, button_min } = t;
    out.line("file_list.size", format!("{} {}", len(size[0]), len(size[1])));
    out.line("file_list.button_min", len(*button_min));
}

fn about_lines(out: &mut Lines, t: &AboutTokens) {
    let AboutTokens { width, list_height, bag_gap, community_width, flex_min } = t;
    out.line("about.width", len(*width));
    out.line("about.list_height", len(*list_height));
    out.line("about.bag_gap", format!("{} {}", len(bag_gap[0]), len(bag_gap[1])));
    out.line("about.community_width", len(*community_width));
    out.line("about.flex_min", len(*flex_min));
}

fn password_lines(out: &mut Lines, t: &PasswordTokens) {
    let PasswordTokens { width } = t;
    out.line("password.width", len(*width));
}

fn list_manager_lines(out: &mut Lines, t: &ListManagerTokens) {
    let ListManagerTokens { list, button_width, column_gap } = t;
    out.line("list_manager.list", format!("{} {}", len(list[0]), len(list[1])));
    out.line("list_manager.button_width", len(*button_width));
    out.line("list_manager.column_gap", len(*column_gap));
}

fn align_lines(out: &mut Lines, t: &AlignTokens) {
    let AlignTokens { button, icon, menu_gap } = t;
    out.line("align.button", len(*button));
    out.line("align.icon", len(*icon));
    out.line("align.menu_gap", len(*menu_gap));
}

fn tree_picker_lines(out: &mut Lines, t: &TreePickerTokens) {
    let TreePickerTokens { list_height, min_popup_width } = t;
    out.line("tree_picker.list_height", len(*list_height));
    out.line("tree_picker.min_popup_width", len(*min_popup_width));
}

fn alt_text_lines(out: &mut Lines, t: &AltTextTokens) {
    let AltTextTokens { box_width, row_gap } = t;
    out.line("alt_text.box_width", len(*box_width));
    out.line("alt_text.row_gap", len(*row_gap));
}

fn select_lines(out: &mut Lines, t: &SelectTokens) {
    let SelectTokens { close_radius, same_point, fastening, ring_radius, ring_offset, ring_halo, ring_line } = t;
    out.line("select.close_radius", len(*close_radius));
    out.line("select.same_point", len(*same_point));
    out.line("select.fastening", len(*fastening));
    out.line("select.ring_radius", len(*ring_radius));
    out.line("select.ring_offset", len(*ring_offset));
    out.line("select.ring_halo", len(*ring_halo));
    out.line("select.ring_line", len(*ring_line));
}

fn hover_lines(out: &mut Lines, t: &HoverTokens) {
    let HoverTokens { outline, fill_alpha, edge_alpha, debounce_ms, busy_ms } = t;
    out.line("hover.outline", len(*outline));
    out.line("hover.fill_alpha", len(*fill_alpha));
    out.line("hover.edge_alpha", len(*edge_alpha));
    out.line("hover.debounce_ms", len(*debounce_ms));
    out.line("hover.busy_ms", len(*busy_ms));
}

fn outline_draft_lines(out: &mut Lines, t: &OutlineDraftTokens) {
    let OutlineDraftTokens { close, same_point } = t;
    out.line("outline_draft.close", len(*close));
    out.line("outline_draft.same_point", len(*same_point));
}

fn selection_frame_lines(out: &mut Lines, t: &SelectionFrameTokens) {
    let SelectionFrameTokens { line, key_line, centre, outline, dash, gap, grow_step } = t;
    out.line("selection_frame.line", len(*line));
    out.line("selection_frame.key_line", len(*key_line));
    out.line("selection_frame.centre", len(*centre));
    out.line("selection_frame.outline", len(*outline));
    out.line("selection_frame.dash", len(*dash));
    out.line("selection_frame.gap", len(*gap));
    out.line("selection_frame.grow_step", len(*grow_step));
}

fn grip_lines(out: &mut Lines, t: &GripTokens) {
    let GripTokens { size, midpoint_height, stroke, hit_pad, warm_pad } = t;
    out.line("grip.size", len(*size));
    out.line("grip.midpoint_height", len(*midpoint_height));
    out.line("grip.stroke", len(*stroke));
    out.line("grip.hit_pad", len(*hit_pad));
    out.line("grip.warm_pad", len(*warm_pad));
}

fn constrain_lines(out: &mut Lines, t: &ConstrainTokens) {
    let ConstrainTokens { copy_reach, stroke_lock } = t;
    out.line("constrain.copy_reach", len(*copy_reach));
    out.line("constrain.stroke_lock", len(*stroke_lock));
}

fn line_guide_lines(out: &mut Lines, t: &LineGuideTokens) {
    let LineGuideTokens { width, halo } = t;
    out.line("line_guide.width", len(*width));
    out.line("line_guide.halo", len(*halo));
}

fn before_after_lines(out: &mut Lines, t: &BeforeAfterTokens) {
    let BeforeAfterTokens { line, halo, knob, gap_side, gap_stacked, caption_gap } = t;
    out.line("before_after.line", len(*line));
    out.line("before_after.halo", len(*halo));
    out.line("before_after.knob", len(*knob));
    out.line("before_after.gap_side", len(*gap_side));
    out.line("before_after.gap_stacked", len(*gap_stacked));
    out.line("before_after.caption_gap", len(*caption_gap));
}

fn brush_lines(out: &mut Lines, t: &BrushTokens) {
    let BrushTokens {
        resize_travel,
        tip_min_radius,
        tip_min_crosshair,
        picker_width,
        picker_height,
        picker_min_width,
        picker_min_height,
        picker_head_max,
        picker_list_min,
        picker_gap,
        picker_grip_room,
        card_pad,
        card_inset,
        card_tip_width,
        card_tip_base,
        card_name_height,
        card_name_inset,
        card_stroke_inset,
        card_stroke_min,
        card_stroke_max,
        card_stroke_height,
        card_max,
        card_tip_only,
        card_tip_name,
        card_radius,
        chip,
        chip_tip,
        chip_tip_at,
        chip_chevron,
        panel_width,
        section_list_width,
        section_row,
        section_box,
        section_lock,
        section_name_inset,
        section_switch_gap,
        strip_height,
        control_select,
        ellipse_size,
        ellipse_disc,
        ellipse_radius,
        ellipse_dot,
        ellipse_grab,
        ellipse_line,
        ellipse_arrow_reach,
        ellipse_head,
        tip_grid_cell,
        tip_grid_thumb,
        tip_grid_max,
        preview_tint,
        card_fill,
    } = t;
    out.line("brush.resize_travel", len(*resize_travel));
    out.line("brush.tip_min_radius", len(*tip_min_radius));
    out.line("brush.tip_min_crosshair", len(*tip_min_crosshair));
    out.line("brush.picker_width", len(*picker_width));
    out.line("brush.picker_height", len(*picker_height));
    out.line("brush.picker_min_width", len(*picker_min_width));
    out.line("brush.picker_min_height", len(*picker_min_height));
    out.line("brush.picker_head_max", len(*picker_head_max));
    out.line("brush.picker_list_min", len(*picker_list_min));
    out.line("brush.picker_gap", len(*picker_gap));
    out.line("brush.picker_grip_room", len(*picker_grip_room));
    out.line("brush.card_pad", len(*card_pad));
    out.line("brush.card_inset", len(*card_inset));
    out.line("brush.card_tip_width", len(*card_tip_width));
    out.line("brush.card_tip_base", len(*card_tip_base));
    out.line("brush.card_name_height", len(*card_name_height));
    out.line("brush.card_name_inset", len(*card_name_inset));
    out.line("brush.card_stroke_inset", len(*card_stroke_inset));
    out.line("brush.card_stroke_min", len(*card_stroke_min));
    out.line("brush.card_stroke_max", len(*card_stroke_max));
    out.line("brush.card_stroke_height", len(*card_stroke_height));
    out.line("brush.card_max", format!("{} {}", len(card_max[0]), len(card_max[1])));
    out.line("brush.card_tip_only", format!("{} {}", len(card_tip_only[0]), len(card_tip_only[1])));
    out.line("brush.card_tip_name", format!("{} {}", len(card_tip_name[0]), len(card_tip_name[1])));
    out.line("brush.card_radius", len(*card_radius));
    out.line("brush.chip", format!("{} {}", len(chip[0]), len(chip[1])));
    out.line("brush.chip_tip", len(*chip_tip));
    out.line("brush.chip_tip_at", format!("{} {}", len(chip_tip_at[0]), len(chip_tip_at[1])));
    out.line("brush.chip_chevron", len(*chip_chevron));
    out.line("brush.panel_width", len(*panel_width));
    out.line("brush.section_list_width", len(*section_list_width));
    out.line("brush.section_row", len(*section_row));
    out.line("brush.section_box", len(*section_box));
    out.line("brush.section_lock", format!("{} {}", len(section_lock[0]), len(section_lock[1])));
    out.line("brush.section_name_inset", len(*section_name_inset));
    out.line("brush.section_switch_gap", len(*section_switch_gap));
    out.line("brush.strip_height", len(*strip_height));
    out.line("brush.control_select", len(*control_select));
    out.line("brush.ellipse_size", len(*ellipse_size));
    out.line("brush.ellipse_disc", len(*ellipse_disc));
    out.line("brush.ellipse_radius", len(*ellipse_radius));
    out.line("brush.ellipse_dot", len(*ellipse_dot));
    out.line("brush.ellipse_grab", len(*ellipse_grab));
    out.line("brush.ellipse_line", len(*ellipse_line));
    out.line("brush.ellipse_arrow_reach", len(*ellipse_arrow_reach));
    out.line("brush.ellipse_head", format!("{} {}", len(ellipse_head[0]), len(ellipse_head[1])));
    out.line("brush.tip_grid_cell", format!("{} {}", len(tip_grid_cell[0]), len(tip_grid_cell[1])));
    out.line("brush.tip_grid_thumb", len(*tip_grid_thumb));
    out.line("brush.tip_grid_max", len(*tip_grid_max));
    out.line("brush.preview_tint", hex(*preview_tint));
    out.line("brush.card_fill", hex(*card_fill));
}

fn full_window_lines(out: &mut Lines, t: &FullWindowTokens) {
    let FullWindowTokens {
        side_width,
        side_width_wide,
        side_width_narrow,
        side_pad_x,
        side_pad_y,
        footer_gap,
        canvas_margin,
        zoom_bar_height,
        zoom_bar_pad_x,
        zoom_bar_pad_y,
        progress_width,
        side_fill,
        side_divider,
        canvas_fill,
        canvas_outline,
    } = t;
    out.line("full_window.side.width", len(*side_width));
    out.line("full_window.side.width_wide", len(*side_width_wide));
    out.line("full_window.side.width_narrow", len(*side_width_narrow));
    out.line("full_window.side.pad_x", len(*side_pad_x));
    out.line("full_window.side.pad_y", len(*side_pad_y));
    out.line("full_window.footer.gap", len(*footer_gap));
    out.line("full_window.canvas.margin", len(*canvas_margin));
    out.line("full_window.zoom_bar.height", len(*zoom_bar_height));
    out.line("full_window.zoom_bar.pad_x", len(*zoom_bar_pad_x));
    out.line("full_window.zoom_bar.pad_y", len(*zoom_bar_pad_y));
    out.line("full_window.progress.width", len(*progress_width));
    out.line("full_window.side.fill", hex(*side_fill));
    out.line("full_window.side.divider", hex(*side_divider));
    out.line("full_window.canvas.fill", hex(*canvas_fill));
    out.line("full_window.canvas.outline", hex(*canvas_outline));
}

fn corner_lines(out: &mut Lines, t: &CornerTokens) {
    let CornerTokens { widget, dot, line, hit, inset, min_side } = t;
    out.line("corner.widget", len(*widget));
    out.line("corner.dot", len(*dot));
    out.line("corner.line", len(*line));
    out.line("corner.hit", len(*hit));
    out.line("corner.inset", len(*inset));
    out.line("corner.min_side", len(*min_side));
}

fn stroke_editor_lines(out: &mut Lines, t: &StrokeEditorTokens) {
    let StrokeEditorTokens {
        width,
        screen_margin,
        screen_margin_y,
        min_width,
        min_height,
        opener_width,
        opener_height,
        preset_height,
        choice_width,
        field_width,
        sample_height,
        sample_stroke,
        sample_inset,
        sample_margin,
        off_stroke,
        off_slash_stroke,
    } = t;
    out.line("stroke_editor.width", len(*width));
    out.line("stroke_editor.screen_margin", len(*screen_margin));
    out.line("stroke_editor.screen_margin_y", len(*screen_margin_y));
    out.line("stroke_editor.min_width", len(*min_width));
    out.line("stroke_editor.min_height", len(*min_height));
    out.line("stroke_editor.opener_width", len(*opener_width));
    out.line("stroke_editor.opener_height", len(*opener_height));
    out.line("stroke_editor.preset_height", len(*preset_height));
    out.line("stroke_editor.choice_width", len(*choice_width));
    out.line("stroke_editor.field_width", len(*field_width));
    out.line("stroke_editor.sample_height", len(*sample_height));
    out.line("stroke_editor.sample_stroke", len(*sample_stroke));
    out.line("stroke_editor.sample_inset", len(*sample_inset));
    out.line("stroke_editor.sample_margin", len(*sample_margin));
    out.line("stroke_editor.off_stroke", len(*off_stroke));
    out.line("stroke_editor.off_slash_stroke", len(*off_slash_stroke));
}

fn measure_lines(out: &mut Lines, t: &MeasureTokens) {
    let MeasureTokens { hit, handle_arm, readout_gap } = t;
    out.line("measure.hit", len(*hit));
    out.line("measure.handle_arm", len(*handle_arm));
    out.line("measure.readout_gap", len(*readout_gap));
}

fn symmetry_lines(out: &mut Lines, t: &SymmetryTokens) {
    let SymmetryTokens { line, hit, handle_share } = t;
    out.line("symmetry.line", len(*line));
    out.line("symmetry.hit", len(*hit));
    out.line("symmetry.handle_share", len(*handle_share));
}

fn adjust_lines(out: &mut Lines, t: &AdjustTokens) {
    let AdjustTokens {
        spectrum_bar,
        range_band,
        spectrum_master_extra,
        range_thumb_width,
        range_thumb_height,
        picker_dot,
        picker_dot_inset,
        threshold_histogram,
        threshold_width,
    } = t;
    out.line("adjust.spectrum_bar", len(*spectrum_bar));
    out.line("adjust.range_band", len(*range_band));
    out.line("adjust.spectrum_master_extra", len(*spectrum_master_extra));
    out.line("adjust.range_thumb_width", len(*range_thumb_width));
    out.line("adjust.range_thumb_height", len(*range_thumb_height));
    out.line("adjust.picker_dot", len(*picker_dot));
    out.line("adjust.picker_dot_inset", len(*picker_dot_inset));
    out.line("adjust.threshold_histogram", len(*threshold_histogram));
    out.line("adjust.threshold_width", len(*threshold_width));
}

fn layer_style_lines(out: &mut Lines, t: &LayerStyleTokens) {
    let LayerStyleTokens {
        list_width,
        page_width,
        preset_thumb,
        preset_gap,
        preset_radius,
        presets_height,
        contour_thumb,
        contour_arrow,
        contour_cell,
        contour_graph,
    } = t;
    out.line("layer_style.list_width", len(*list_width));
    out.line("layer_style.page_width", len(*page_width));
    out.line("layer_style.preset_thumb", len(*preset_thumb));
    out.line("layer_style.preset_gap", len(*preset_gap));
    out.line("layer_style.preset_radius", len(*preset_radius));
    out.line("layer_style.presets_height", len(*presets_height));
    out.line("layer_style.contour_thumb", len(*contour_thumb));
    out.line("layer_style.contour_arrow", len(*contour_arrow));
    out.line("layer_style.contour_cell", len(*contour_cell));
    out.line("layer_style.contour_graph", len(*contour_graph));
}

fn blend_if_lines(out: &mut Lines, t: &BlendIfTokens) {
    let BlendIfTokens { width_min, width_max, height, ramp, inset, handle_width, handle_height } = t;
    out.line("blend_if.width_min", len(*width_min));
    out.line("blend_if.width_max", len(*width_max));
    out.line("blend_if.height", len(*height));
    out.line("blend_if.ramp", len(*ramp));
    out.line("blend_if.inset", len(*inset));
    out.line("blend_if.handle_width", len(*handle_width));
    out.line("blend_if.handle_height", len(*handle_height));
}

fn add_beside_lines(out: &mut Lines, t: &AddBesideTokens) {
    let AddBesideTokens { offset, radius, arm, line, ring } = t;
    out.line("add_beside.offset", len(*offset));
    out.line("add_beside.radius", len(*radius));
    out.line("add_beside.arm", len(*arm));
    out.line("add_beside.line", len(*line));
    out.line("add_beside.ring", len(*ring));
}

fn shape_drag_lines(out: &mut Lines, t: &ShapeDragTokens) {
    let ShapeDragTokens { click_slop, preview_line, preview_halo } = t;
    out.line("shape_drag.click_slop", len(*click_slop));
    out.line("shape_drag.preview_line", len(*preview_line));
    out.line("shape_drag.preview_halo", len(*preview_halo));
}

fn slice_lines(out: &mut Lines, t: &SliceTokens) {
    let SliceTokens {
        outline,
        selected_outline,
        auto_dash,
        auto_gap,
        auto_alpha,
        drag_dash,
        drag_gap,
        badge_offset,
        badge_pad,
        badge_radius,
        badge_min,
        click_slop,
    } = t;
    out.line("slice.outline", len(*outline));
    out.line("slice.selected_outline", len(*selected_outline));
    out.line("slice.auto_dash", len(*auto_dash));
    out.line("slice.auto_gap", len(*auto_gap));
    out.line("slice.auto_alpha", len(*auto_alpha));
    out.line("slice.drag_dash", len(*drag_dash));
    out.line("slice.drag_gap", len(*drag_gap));
    out.line("slice.badge_offset", len(*badge_offset));
    out.line("slice.badge_pad", format!("{} {}", len(badge_pad[0]), len(badge_pad[1])));
    out.line("slice.badge_radius", len(*badge_radius));
    out.line("slice.badge_min", format!("{} {}", len(badge_min[0]), len(badge_min[1])));
    out.line("slice.click_slop", len(*click_slop));
}

fn artboard_lines(out: &mut Lines, t: &ArtboardTokens) {
    let ArtboardTokens { click_slop, preview_line } = t;
    out.line("artboard.click_slop", len(*click_slop));
    out.line("artboard.preview_line", len(*preview_line));
}

fn connector_lines(out: &mut Lines, t: &ConnectorTokens) {
    let ConnectorTokens {
        line,
        attach,
        attach_active,
        attach_fill_alpha,
        ghost_alpha,
        ghost_dash,
        ghost_gap,
        ghost_line_alpha,
        click_slop,
        end_handle,
        path_handle,
        attach_reach,
        handle_reach,
        min_run,
        target_tolerance,
        path_ring,
        drop_outline,
    } = t;
    out.line("connector.line", len(*line));
    out.line("connector.attach", len(*attach));
    out.line("connector.attach_active", len(*attach_active));
    out.line("connector.attach_fill_alpha", len(*attach_fill_alpha));
    out.line("connector.ghost_alpha", len(*ghost_alpha));
    out.line("connector.ghost_dash", len(*ghost_dash));
    out.line("connector.ghost_gap", len(*ghost_gap));
    out.line("connector.ghost_line_alpha", len(*ghost_line_alpha));
    out.line("connector.click_slop", len(*click_slop));
    out.line("connector.end_handle", len(*end_handle));
    out.line("connector.path_handle", len(*path_handle));
    out.line("connector.attach_reach", len(*attach_reach));
    out.line("connector.handle_reach", len(*handle_reach));
    out.line("connector.min_run", len(*min_run));
    out.line("connector.target_tolerance", len(*target_tolerance));
    out.line("connector.path_ring", len(*path_ring));
    out.line("connector.drop_outline", len(*drop_outline));
}

fn vanishing_point_lines(out: &mut Lines, t: &VanishingPointTokens) {
    let VanishingPointTokens {
        line,
        line_selected,
        grid,
        grid_alpha,
        node,
        pending,
        halo_width,
        source_arm,
        press_reach,
        click_slop,
        ring_line,
        paste_alpha,
        halo,
    } = t;
    out.line("vanishing_point.line", len(*line));
    out.line("vanishing_point.line_selected", len(*line_selected));
    out.line("vanishing_point.grid", len(*grid));
    out.line("vanishing_point.grid_alpha", len(*grid_alpha));
    out.line("vanishing_point.node", len(*node));
    out.line("vanishing_point.pending", len(*pending));
    out.line("vanishing_point.halo_width", len(*halo_width));
    out.line("vanishing_point.source_arm", len(*source_arm));
    out.line("vanishing_point.press_reach", len(*press_reach));
    out.line("vanishing_point.click_slop", len(*click_slop));
    out.line("vanishing_point.ring_line", len(*ring_line));
    out.line("vanishing_point.paste_alpha", len(*paste_alpha));
    out.line("vanishing_point.halo", hex(*halo));
}

fn wide_angle_lines(out: &mut Lines, t: &WideAngleTokens) {
    let WideAngleTokens { line, end, hit, min_drag, halo_width, halo } = t;
    out.line("wide_angle.line", len(*line));
    out.line("wide_angle.end", len(*end));
    out.line("wide_angle.hit", len(*hit));
    out.line("wide_angle.min_drag", len(*min_drag));
    out.line("wide_angle.halo_width", len(*halo_width));
    out.line("wide_angle.halo", hex(*halo));
}

fn blur_pins_lines(out: &mut Lines, t: &BlurPinsTokens) {
    let BlurPinsTokens {
        dot,
        grab,
        ring,
        ring_band,
        handle,
        line_grab,
        line_width,
        line_halo,
        arc,
        arc_halo,
        dash,
        dash_gap,
        handle_outline,
        feather_extra,
        turn_extra,
        round_corner,
    } = t;
    out.line("blur_pins.dot", len(*dot));
    out.line("blur_pins.grab", len(*grab));
    out.line("blur_pins.ring", len(*ring));
    out.line("blur_pins.ring_band", len(*ring_band));
    out.line("blur_pins.handle", len(*handle));
    out.line("blur_pins.line_grab", len(*line_grab));
    out.line("blur_pins.line_width", len(*line_width));
    out.line("blur_pins.line_halo", len(*line_halo));
    out.line("blur_pins.arc", len(*arc));
    out.line("blur_pins.arc_halo", len(*arc_halo));
    out.line("blur_pins.dash", len(*dash));
    out.line("blur_pins.dash_gap", len(*dash_gap));
    out.line("blur_pins.handle_outline", len(*handle_outline));
    out.line("blur_pins.feather_extra", len(*feather_extra));
    out.line("blur_pins.turn_extra", len(*turn_extra));
    out.line("blur_pins.round_corner", len(*round_corner));
}

fn liquify_lines(out: &mut Lines, t: &LiquifyTokens) {
    let LiquifyTokens { lasso, mask_opacity, freeze, thaw } = t;
    out.line("liquify.lasso", len(*lasso));
    out.line("liquify.mask_opacity", len(*mask_opacity));
    out.line("liquify.freeze", hex(*freeze));
    out.line("liquify.thaw", hex(*thaw));
}

fn select_mask_lines(out: &mut Lines, t: &SelectMaskTokens) {
    let SelectMaskTokens { close_reach, click_reach, trail } = t;
    out.line("select_mask.close_reach", len(*close_reach));
    out.line("select_mask.click_reach", len(*click_reach));
    out.line("select_mask.trail", hex(*trail));
}

fn filter_gallery_lines(out: &mut Lines, t: &FilterGalleryTokens) {
    let FilterGalleryTokens { thumb_width, thumb_height, stack_height } = t;
    out.line("filter_gallery.thumb_width", len(*thumb_width));
    out.line("filter_gallery.thumb_height", len(*thumb_height));
    out.line("filter_gallery.stack_height", len(*stack_height));
}
