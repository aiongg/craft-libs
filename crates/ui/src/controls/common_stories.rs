//! Stories of the common controls added in step 5: chips, tree picker, icon picker, linked
//! fields, preview selects, split button, breadcrumb, rating, date picker, alt text, autocomplete,
//! apply rows and reset dots.
//!
//! Every string a story shows (chip names, album names, presets, paths, suggestions) goes
//! through [`ControlStoryLabels`] or the shared [`StoryLabels`], so the gallery's pseudo-locale
//! test sees no stray English. No story reads the platform: the date story fixes today with
//! `DatePicker::today`, the tree picker's and breadcrumb's data are fixed lists.

use egui::text::{CCursor, CCursorRange};
use egui::{Color32, Id, Key, Modifiers, Painter, Rect, Sense, TextEdit, Ui, vec2};

use super::slider::{store, with_values};
use super::{
    AltText, AltTextField, ApplyRow, ApplyValue, Autocomplete, BreadcrumbBar, Button, ButtonVariant, Chip, ChipField, ControlSize, DatePicker,
    DatePrecision, IconPicker, IconPickerGroup, IconPickerItem, LineStyle, LineWeights, LinkMode, LinkedFields, LinkedLayout, LinkedSlot,
    NumberField, PickedDate, Rating, ResetDot, Select, SelectText, SplitButton, Suggestion, TextField, TreeEntry, TreePicker, completion_rest,
    paint_inline_completion, paint_line_style, paint_rating, path_crumbs,
};
use crate::icons::{self, Icon, IconSize};
use crate::localized::{Labels as _, LocaleNumbers, Words};
use crate::menu::{MenuEntry, MenuItem};
use crate::popover::{Popover, PopoverAnchor};
use crate::primitives::{Content, Gap, Row, Text};
use crate::story::{Level, NoArgs, SizeArg, Story, StoryArgs, StoryChoice, StoryLabels, variant_list, variant_row};
use crate::theme::{ToEgui as _, Tokens, TokensOf as _};
use crate::typed_story;

crate::words! {
    /// The sample text of the step 5 controls' stories, English by default.
    #[non_exhaustive]
    pub struct ControlStoryLabels {
        kw_sunset: "controlStory.kwSunset" = "Sunset";
        kw_harbor: "controlStory.kwHarbor" = "Harbor";
        kw_portrait: "controlStory.kwPortrait" = "Portrait";
        kw_studio: "controlStory.kwStudio" = "Studio";
        kw_mountain: "controlStory.kwMountain" = "Mountain";
        kw_lake: "controlStory.kwLake" = "Lake";
        kw_street: "controlStory.kwStreet" = "Street";
        kw_night: "controlStory.kwNight" = "Night";
        kw_winter: "controlStory.kwWinter" = "Winter";
        kw_macro: "controlStory.kwMacro" = "Macro";
        kw_travel: "controlStory.kwTravel" = "Travel";
        kw_family: "controlStory.kwFamily" = "Family";
        keywords: "controlStory.keywords" = "Keywords";
        filters: "controlStory.filters" = "Filters";
        filter_rating: "controlStory.filterRating" = "Rating: 4 or more";
        filter_camera: "controlStory.filterCamera" = "Camera: X100V";
        filter_year: "controlStory.filterYear" = "Year: 2024";
        filter_flagged: "controlStory.filterFlagged" = "Flagged";
        album: "controlStory.album" = "Album";
        album_placeholder: "controlStory.albumPlaceholder" = "Choose an album";
        album_travel: "controlStory.albumTravel" = "Travel";
        album_italy: "controlStory.albumItaly" = "Italy";
        album_japan: "controlStory.albumJapan" = "Japan";
        album_family: "controlStory.albumFamily" = "Family";
        album_birthdays: "controlStory.albumBirthdays" = "Birthdays";
        album_smart: "controlStory.albumSmart" = "Smart Albums";
        album_recent: "controlStory.albumRecent" = "Recently Added";
        album_blocked: "controlStory.albumBlocked" = "A smart album fills itself.";
        icons: "controlStory.icons" = "Icons";
        icon_button: "controlStory.iconButton" = "Choose an icon";
        icon_chosen: "controlStory.iconChosen" = "Chosen: {name}";
        icon_none: "controlStory.iconNone" = "Nothing chosen";
        group_tools: "controlStory.groupTools" = "Tools";
        group_shapes: "controlStory.groupShapes" = "Shapes";
        group_edit: "controlStory.groupEdit" = "Edit";
        group_align: "controlStory.groupAlign" = "Align";
        icon_move: "controlStory.iconMove" = "Move";
        icon_brush: "controlStory.iconBrush" = "Brush";
        icon_eraser: "controlStory.iconEraser" = "Eraser";
        icon_crop: "controlStory.iconCrop" = "Crop";
        icon_square: "controlStory.iconSquare" = "Square";
        icon_circle: "controlStory.iconCircle" = "Circle";
        icon_undo: "controlStory.iconUndo" = "Undo";
        icon_redo: "controlStory.iconRedo" = "Redo";
        icon_copy: "controlStory.iconCopy" = "Copy";
        icon_align_left: "controlStory.iconAlignLeft" = "Align Left";
        icon_align_center: "controlStory.iconAlignCenter" = "Align Center";
        icon_align_right: "controlStory.iconAlignRight" = "Align Right";
        clone_x: "controlStory.cloneX" = "X";
        clone_y: "controlStory.cloneY" = "Y";
        clone_w: "controlStory.cloneW" = "W";
        clone_h: "controlStory.cloneH" = "H";
        margin_top: "controlStory.marginTop" = "Top";
        margin_right: "controlStory.marginRight" = "Right";
        margin_bottom: "controlStory.marginBottom" = "Bottom";
        margin_left: "controlStory.marginLeft" = "Left";
        underline: "controlStory.underline" = "Underline";
        stroke: "controlStory.stroke" = "Stroke type";
        trace: "controlStory.trace" = "Trace";
        trace_presets: "controlStory.tracePresets" = "Tracing Presets";
        preset_default: "controlStory.presetDefault" = "Default Trace";
        preset_bw: "controlStory.presetBw" = "Black and White";
        preset_photo: "controlStory.presetPhoto" = "High Fidelity Photo";
        preset_sketch: "controlStory.presetSketch" = "Sketched Art";
        preset_silhouette: "controlStory.presetSilhouette" = "Silhouettes";
        trace_chosen: "controlStory.traceChosen" = "Ran: {name}";
        path_photos: "controlStory.pathPhotos" = "Photos";
        path_italy: "controlStory.pathItaly" = "Italy";
        path_rome: "controlStory.pathRome" = "Rome";
        path_forum: "controlStory.pathForum" = "Forum";
        path_morning: "controlStory.pathMorning" = "Morning";
        path_edits: "controlStory.pathEdits" = "Edits";
        path_opened: "controlStory.pathOpened" = "Opened: {name}";
        rating_medium: "controlStory.ratingMedium" = "Medium stars";
        rating_small: "controlStory.ratingSmall" = "Small stars";
        rating_painted: "controlStory.ratingPainted" = "Painted stars";
        date_full: "controlStory.dateFull" = "Year, month or day";
        date_month: "controlStory.dateMonth" = "Year or month";
        date_year: "controlStory.dateYear" = "Year only";
        formula: "controlStory.formula" = "Formula";
        column: "controlStory.column" = "Column";
        fn_sum_desc: "controlStory.fnSumDesc" = "Adds the numbers in a range.";
        fn_average_desc: "controlStory.fnAverageDesc" = "Averages the numbers in a range.";
        fn_count_desc: "controlStory.fnCountDesc" = "Counts the cells that hold numbers.";
        fn_max_desc: "controlStory.fnMaxDesc" = "Returns the largest number.";
        fn_min_desc: "controlStory.fnMinDesc" = "Returns the smallest number.";
        col_revenue: "controlStory.colRevenue" = "Revenue";
        col_region: "controlStory.colRegion" = "Region";
        col_returns: "controlStory.colReturns" = "Returns";
        apply_name: "controlStory.applyName" = "Layer";
        field_radius: "controlStory.fieldRadius" = "Radius";
        field_rotation: "controlStory.fieldRotation" = "Rotation";
        restore: "controlStory.restore" = "Restore";
        field_corner: "controlStory.fieldCorner" = "Corner size";
    }
}

pub(super) fn stories() -> Vec<Story> {
    use super::{SizeArgs, SizeDisabledArgs};
    vec![
        typed_story!(
            "controls/chip-field/keywords",
            Level::Controls,
            "Chip field",
            "Keywords",
            KeywordArgs { size: SizeArg::Md, limit_lines: true, max_lines: 2, disabled: false },
            show_keywords
        ),
        typed_story!("controls/chip-field/filters", Level::Controls, "Chip field", "Filters", SizeDisabledArgs::default(), show_filters),
        typed_story!("controls/tree-picker/albums", Level::Controls, "Tree picker", "Albums", SizeDisabledArgs::default(), show_albums),
        typed_story!("controls/icon-picker/icons", Level::Controls, "Icon picker", "Icons", NoArgs {}, show_icons),
        typed_story!("controls/linked-fields/size", Level::Controls, "Linked fields", "Size", SizeDisabledArgs::default(), show_size),
        typed_story!(
            "controls/linked-fields/clone-source",
            Level::Controls,
            "Linked fields",
            "Clone source",
            SizeDisabledArgs::default(),
            show_clone_source
        ),
        typed_story!("controls/linked-fields/margins", Level::Controls, "Linked fields", "Margins", SizeDisabledArgs::default(), show_margins),
        typed_story!("controls/preview-select/underline", Level::Controls, "Preview select", "Underline", SizeArgs::default(), show_underline),
        typed_story!(
            "controls/split-button/trace",
            Level::Controls,
            "Split button",
            "Trace",
            TraceArgs { variant: VariantArg::Secondary, size: SizeArg::Md, disabled: false },
            show_trace
        ),
        typed_story!("controls/breadcrumb/path", Level::Controls, "Breadcrumb", "Path", PathArgs { width: 420.0 }, show_path),
        typed_story!("controls/rating/stars", Level::Controls, "Rating", "Stars", DisabledArgs { disabled: false }, show_stars),
        typed_story!("controls/date-picker/picker", Level::Controls, "Date picker", "Picker", SizeDisabledArgs::default(), show_dates),
        typed_story!("controls/alt-text/field", Level::Controls, "Alt text", "Field", AltArgs { mixed: false }, show_alt_text),
        typed_story!("controls/autocomplete/functions", Level::Controls, "Autocomplete", "Functions", NoArgs {}, show_autocomplete),
        typed_story!("controls/apply-row/apply-rows", Level::Controls, "Apply row", "Apply rows", SizeArgs::default(), show_apply_rows),
        typed_story!("controls/reset-dot/fields", Level::Controls, "Reset dot", "Fields", SizeArgs::default(), show_reset_dots),
    ]
}

#[derive(StoryArgs, Debug, PartialEq)]
struct KeywordArgs {
    size: SizeArg,
    #[arg(name = "limit lines")]
    limit_lines: bool,
    #[arg(name = "max lines", range = 1..=6)]
    max_lines: usize,
    disabled: bool,
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum VariantArg {
    Primary,
    Secondary,
    Ghost,
    Danger,
}

impl VariantArg {
    fn variant(self) -> ButtonVariant {
        match self {
            VariantArg::Primary => ButtonVariant::Primary,
            VariantArg::Secondary => ButtonVariant::Secondary,
            VariantArg::Ghost => ButtonVariant::Ghost,
            VariantArg::Danger => ButtonVariant::Danger,
        }
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
struct TraceArgs {
    variant: VariantArg,
    size: SizeArg,
    disabled: bool,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct PathArgs {
    #[arg(range = 120.0..=640.0)]
    width: f32,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct DisabledArgs {
    disabled: bool,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct AltArgs {
    mixed: bool,
}

/// One value kept across passes under `key`.
fn state<T: Clone + Send + Sync + 'static>(ui: &mut Ui, key: &str, initial: T, show: impl FnOnce(&mut Ui, &mut T)) {
    with_values(ui, key, [initial], |ui, [value]| show(ui, value));
}

/// The keywords, in the order the story lists them.
fn keyword_words(words: &ControlStoryLabels) -> [&str; 12] {
    [
        &words.kw_sunset,
        &words.kw_harbor,
        &words.kw_portrait,
        &words.kw_studio,
        &words.kw_mountain,
        &words.kw_lake,
        &words.kw_street,
        &words.kw_night,
        &words.kw_winter,
        &words.kw_macro,
        &words.kw_travel,
        &words.kw_family,
    ]
}

/// The keywords only some of the selected photos have.
const PARTIAL_KEYWORDS: [usize; 3] = [3, 7, 11];

/// A keyword field of a dozen keywords; a × removes one, "+N more" lists those past the lines.
fn show_keywords(ui: &mut Ui, args: &mut KeywordArgs) {
    let words = ControlStoryLabels::of(ui);
    let all = keyword_words(&words);
    state(ui, "story-keywords", (0..all.len()).collect::<Vec<usize>>(), |ui, shown| {
        let chips: Vec<Chip> = shown.iter().map(|&i| Chip::new(all[i]).partial(PARTIAL_KEYWORDS.contains(&i))).collect();
        let mut field = ChipField::new("story-keywords", &words.keywords, &chips).size(args.size.control()).enabled(!args.disabled);
        if args.limit_lines {
            field = field.max_lines(args.max_lines);
        }
        if let Some(at) = field.show(ui).removed {
            shown.remove(at);
        }
        if shown.is_empty() && ui.add(Button::new(&words.restore)).clicked() {
            *shown = (0..all.len()).collect();
        }
    });
}

/// The filter strip: each active filter a chip, Clear All at the end of the last line.
fn show_filters(ui: &mut Ui, args: &mut super::SizeDisabledArgs) {
    let words = ControlStoryLabels::of(ui);
    let all = [&words.filter_rating, &words.filter_camera, &words.filter_year, &words.filter_flagged];
    state(ui, "story-filters", (0..all.len()).collect::<Vec<usize>>(), |ui, shown| {
        let chips: Vec<Chip> = shown.iter().map(|&i| Chip::new(all[i].as_str())).collect();
        let out = ChipField::new("story-filters", &words.filters, &chips).clear_all(true).size(args.size.control()).enabled(!args.disabled).show(ui);
        if let Some(at) = out.removed {
            shown.remove(at);
        }
        if out.cleared {
            shown.clear();
        }
        if shown.is_empty() && ui.add(Button::new(&words.restore)).clicked() {
            *shown = (0..all.len()).collect();
        }
    });
}

/// Folders and albums; the smart album can't be picked and says why.
fn show_albums(ui: &mut Ui, args: &mut super::SizeDisabledArgs) {
    let words = ControlStoryLabels::of(ui);
    let entry = |key: u32, name: &str, parent: Option<u32>, folder: bool, blocked: Option<&str>| TreeEntry {
        key,
        name: name.to_owned(),
        parent,
        folder,
        blocked: blocked.map(str::to_owned),
        icon: None,
    };
    let entries = [
        entry(1, &words.album_travel, None, true, None),
        entry(2, &words.album_italy, Some(1), false, None),
        entry(3, &words.album_japan, Some(1), false, None),
        entry(4, &words.album_family, None, true, None),
        entry(5, &words.album_birthdays, Some(4), false, None),
        entry(6, &words.album_smart, None, true, None),
        entry(7, &words.album_recent, Some(6), false, Some(&words.album_blocked)),
    ];
    state(ui, "story-albums", Some(2_u32), |ui, value| {
        let _ = TreePicker::new("story-albums", &words.album, value, &entries)
            .placeholder(&words.album_placeholder)
            .size(args.size.control())
            .enabled(!args.disabled)
            .show(ui);
    });
}

/// Paints an icon in a picker cell.
fn paint_icon(painter: &Painter, rect: Rect, icon: &Icon, color: Color32) {
    let ctx = painter.ctx();
    let ppp = ctx.pixels_per_point();
    let px = icons::pixels(rect.width().min(rect.height()), ppp);
    let texture = icons::texture_in(ctx, *icon, icons::active(ctx), px, IconSize::Md.stroke() * ppp);
    let side = px as f32 / ppp;
    let min = egui::emath::GuiRounding::round_to_pixels(rect.center() - vec2(side, side) / 2.0, ppp);
    painter.image(texture, Rect::from_min_size(min, vec2(side, side)), Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), color);
}

/// The icons in four groups, each named by its words.
fn icon_groups(words: &ControlStoryLabels) -> Vec<IconPickerGroup<Icon>> {
    let group = |name: &str, items: &[(Icon, &str)]| IconPickerGroup {
        name: name.to_owned(),
        items: items.iter().map(|(key, label)| IconPickerItem { key: *key, label: (*label).to_owned(), keywords: Vec::new() }).collect(),
    };
    vec![
        group(
            &words.group_tools,
            &[(Icon::Move, &words.icon_move), (Icon::Brush, &words.icon_brush), (Icon::Eraser, &words.icon_eraser), (Icon::Crop, &words.icon_crop)],
        ),
        group(&words.group_shapes, &[(Icon::Square, &words.icon_square), (Icon::Circle, &words.icon_circle)]),
        group(&words.group_edit, &[(Icon::Undo, &words.icon_undo), (Icon::Redo, &words.icon_redo), (Icon::Copy, &words.icon_copy)]),
        group(
            &words.group_align,
            &[(Icon::AlignLeft, &words.icon_align_left), (Icon::AlignCenter, &words.icon_align_center), (Icon::AlignRight, &words.icon_align_right)],
        ),
    ]
}

/// The picker in a popover under a button; picking closes it and names the pick.
fn show_icons(ui: &mut Ui, _args: &mut NoArgs) {
    let words = ControlStoryLabels::of(ui);
    let groups = icon_groups(&words);
    state(ui, "story-icons", (false, None::<String>), |ui, (open, chosen)| {
        Row::new(Gap::Section).show(ui, |ui| {
            let button = ui.add(Button::new(&words.icon_button));
            if button.clicked() {
                *open = !*open;
            }
            let shown = Popover::new("story-icons", PopoverAnchor::Control(button.clone())).toggle(button.id).label(words.icons.as_str()).show(
                ui,
                open,
                |ui, pop| {
                    let picked = IconPicker::new("story-icons", &words.icons, &groups, paint_icon).show(ui).picked;
                    if picked.is_some() {
                        pop.close();
                    }
                    picked
                },
            );
            if let Some(icon) = shown.and_then(|s| s.inner) {
                *chosen = groups.iter().flat_map(|g| &g.items).find(|item| item.key == icon).map(|item| item.label.clone());
            }
            let text = match chosen {
                Some(name) => craft_i18n::fmt(&words.icon_chosen, &[("name", name)]),
                None => words.icon_none.clone(),
            };
            ui.add(Text::new(text).content(Content::Muted));
        });
    });
}

/// Width and height with the ratio link.
fn show_size(ui: &mut Ui, args: &mut super::SizeDisabledArgs) {
    let words = StoryLabels::of(ui);
    let numbers = LocaleNumbers::of(ui);
    state(ui, "story-size", ([1920.0, 1080.0], true), |ui, (values, linked)| {
        let edit = LinkedFields::new("story-size", [&words.width, &words.height], *values, linked, LinkMode::Ratio, &numbers)
            .range(1.0..=16_384.0)
            .decimals(0..=0)
            .size(args.size.control())
            .enabled(!args.disabled)
            .show(ui);
        store(&edit, values);
    });
}

/// PhotoCraft's Clone Source grid: X and W with the link on one row, Y and H under them. The
/// host places its own X and Y fields in the same grid as the linked pair.
fn show_clone_source(ui: &mut Ui, args: &mut super::SizeDisabledArgs) {
    let words = ControlStoryLabels::of(ui);
    let numbers = LocaleNumbers::of(ui);
    let (size, enabled) = (args.size.control(), !args.disabled);
    state(ui, "story-clone-source", ([100.0, 100.0], true, [0.0, 0.0]), |ui, (scale, linked, offset)| {
        egui::Grid::new("story-clone-source-grid").show(ui, |ui| {
            let offset_field = |ui: &mut Ui, axis: usize, caption: &str, offset: &mut [f64; 2]| {
                ui.add(Text::new(caption));
                let edit = NumberField::new(("story-clone-offset", axis), caption, offset[axis], &numbers).size(size).enabled(enabled).show(ui);
                store(&edit, &mut offset[axis]);
            };
            let edit =
                LinkedFields::new("story-clone-size", [words.clone_w.as_str(), words.clone_h.as_str()], *scale, linked, LinkMode::Ratio, &numbers)
                    .range(-1000.0..=1000.0)
                    .decimals(0..=1)
                    .size(size)
                    .enabled(enabled)
                    .show_cells(ui, |ui, slot, add| match slot {
                        LinkedSlot::Field(0) => {
                            offset_field(ui, 0, &words.clone_x, offset);
                            ui.add(Text::new(words.clone_w.as_str()));
                            add(ui);
                        }
                        LinkedSlot::Link => {
                            add(ui);
                            ui.end_row();
                            offset_field(ui, 1, &words.clone_y, offset);
                            ui.add(Text::new(words.clone_h.as_str()));
                        }
                        LinkedSlot::Field(_) => add(ui),
                    });
            store(&edit, scale);
        });
    });
}

/// Four margins in a column with the equal link.
fn show_margins(ui: &mut Ui, args: &mut super::SizeDisabledArgs) {
    let words = ControlStoryLabels::of(ui);
    let numbers = LocaleNumbers::of(ui);
    state(ui, "story-margins", ([12.0; 4], true), |ui, (values, linked)| {
        let captions = [words.margin_top.as_str(), words.margin_right.as_str(), words.margin_bottom.as_str(), words.margin_left.as_str()];
        let edit = LinkedFields::new("story-margins", captions, *values, linked, LinkMode::Equal, &numbers)
            .range(0.0..=200.0)
            .decimals(0..=1)
            .layout(LinkedLayout::Columns)
            .size(args.size.control())
            .enabled(!args.disabled)
            .show(ui);
        store(&edit, values);
    });
}

/// A select of previews only (underline styles) and one that shows the preview in the closed box
/// and the text in the list (stroke types).
fn show_underline(ui: &mut Ui, args: &mut super::SizeArgs) {
    let story = ControlStoryLabels::of(ui);
    let words = Words::of(ui);
    let ink = Tokens::of(ui).color.text.normal.to_egui();
    let size = args.size.control();
    let name = |style: LineStyle| words.get(style.word());
    state(ui, "story-line-styles", [LineStyle::Solid, LineStyle::Solid], |ui, [underline, stroke]| {
        variant_list(ui, "story-line-styles", |form| {
            variant_row(form, &story.underline, |ui| {
                Select::new("story-underline", &story.underline, underline)
                    .rows(LineStyle::UNDERLINES.len(), |i| LineStyle::UNDERLINES[i], |i| name(LineStyle::UNDERLINES[i]))
                    .row_content(vec2(120.0, 22.0), move |painter, rect, style| paint_line_style(painter, rect, *style, ink, &LineWeights::UNDERLINE))
                    .text_shown(SelectText::Never)
                    .size(size)
                    .show(ui)
                    .response
            });
            variant_row(form, &story.stroke, |ui| {
                Select::new("story-stroke", &story.stroke, stroke)
                    .rows(LineStyle::STROKES.len(), |i| LineStyle::STROKES[i], |i| name(LineStyle::STROKES[i]))
                    .row_content(vec2(120.0, 22.0), move |painter, rect, style| paint_line_style(painter, rect, *style, ink, &LineWeights::STROKE))
                    .text_shown(SelectText::InList)
                    .size(size)
                    .show(ui)
                    .response
            });
        });
    });
}

/// A main action with five presets under the arrow.
fn show_trace(ui: &mut Ui, args: &mut TraceArgs) {
    let words = ControlStoryLabels::of(ui);
    let presets = [&words.preset_default, &words.preset_bw, &words.preset_photo, &words.preset_sketch, &words.preset_silhouette];
    let entries: Vec<MenuEntry<usize>> = presets.iter().enumerate().map(|(i, name)| MenuItem::new(name.as_str(), i).into()).collect();
    state(ui, "story-trace", None::<usize>, |ui, ran| {
        Row::new(Gap::Section).show(ui, |ui| {
            let out = SplitButton::new("story-trace", &words.trace, &entries)
                .arrow_label(&words.trace_presets)
                .icon(Icon::Image)
                .variant(args.variant.variant())
                .size(args.size.control())
                .enabled(!args.disabled)
                .show(ui);
            if out.main.clicked() {
                *ran = Some(0);
            }
            if let Some(at) = out.menu.chosen {
                *ran = Some(at);
            }
            if let Some(at) = *ran {
                ui.add(Text::new(craft_i18n::fmt(&words.trace_chosen, &[("name", presets[at])])).content(Content::Muted));
            }
        });
    });
}

/// A deep path; the bar keeps the last segments that fit and collapses the rest under "…".
fn show_path(ui: &mut Ui, args: &mut PathArgs) {
    let words = ControlStoryLabels::of(ui);
    let segments = [&words.path_photos, &words.path_italy, &words.path_rome, &words.path_forum, &words.path_morning, &words.path_edits];
    let path = segments.iter().fold(String::new(), |path, segment| path + "/" + segment);
    let crumbs = path_crumbs(&path);
    state(ui, "story-path", None::<String>, |ui, opened| {
        ui.allocate_ui(vec2(args.width.min(ui.available_width()), 0.0), |ui| {
            if let Some(key) = BreadcrumbBar::new("story-path", &crumbs).show(ui).opened {
                *opened = crumbs.iter().find(|c| c.key == key).map(|c| c.label.clone());
            }
        });
        if let Some(name) = opened {
            ui.add(Text::new(craft_i18n::fmt(&words.path_opened, &[("name", name)])).content(Content::Muted));
        }
    });
}

/// The rating at both control sizes, and the painted stars of both sizes.
fn show_stars(ui: &mut Ui, args: &mut DisabledArgs) {
    let words = ControlStoryLabels::of(ui);
    let tokens = Tokens::of(ui);
    let star = tokens.component.rating.star.to_egui();
    state(ui, "story-stars", [3_u8, 0], |ui, [medium, small]| {
        variant_list(ui, "story-stars", |form| {
            variant_row(form, &words.rating_medium, |ui| {
                store(&Rating::new("story-stars-md", &words.rating_medium, medium).size(ControlSize::Md).enabled(!args.disabled).show(ui), medium)
            });
            variant_row(form, &words.rating_small, |ui| {
                store(&Rating::new("story-stars-sm", &words.rating_small, small).size(ControlSize::Sm).enabled(!args.disabled).show(ui), small)
            });
            variant_row(form, &words.rating_painted, |ui| {
                ui.horizontal(|ui| {
                    for (size, rating) in [(ControlSize::Md, *medium), (ControlSize::Sm, *small)] {
                        let (side, step) = (tokens.component.rating.badge_star.get(size), tokens.component.rating.badge_step.get(size));
                        let (rect, _) = ui.allocate_exact_size(vec2(step * 5.0, side), Sense::hover());
                        paint_rating(ui.painter(), &tokens, rect.left_center(), rating, size, star);
                    }
                })
                .response
            });
        });
    });
}

/// A date picker for each set of precisions; today is fixed.
fn show_dates(ui: &mut Ui, args: &mut super::SizeDisabledArgs) {
    let words = ControlStoryLabels::of(ui);
    let today = PickedDate::day(2026, 11, 1);
    let size = args.size.control();
    const FULL: [DatePrecision; 3] = DatePrecision::ALL;
    const MONTH: [DatePrecision; 2] = [DatePrecision::Year, DatePrecision::Month];
    const YEAR: [DatePrecision; 1] = [DatePrecision::Year];
    state(ui, "story-dates", [Some(PickedDate::day(2026, 10, 14)), Some(PickedDate::month(2026, 9)), None], |ui, [full, month, year]| {
        variant_list(ui, "story-dates", |form| {
            let row =
                |form: &mut crate::composites::Form<'_>, label: &str, salt: &str, value: &mut Option<PickedDate>, precisions: &[DatePrecision]| {
                    variant_row(form, label, |ui| {
                        let edit = DatePicker::new(salt.to_owned(), label, value)
                            .precisions(precisions)
                            .today(today)
                            .size(size)
                            .enabled(!args.disabled)
                            .show(ui);
                        store(&edit, value)
                    });
                };
            row(form, &words.date_full, "story-date-full", full, &FULL);
            row(form, &words.date_month, "story-date-month", month, &MONTH);
            row(form, &words.date_year, "story-date-year", year, &YEAR);
        });
    });
}

/// The alt text editor; `mixed` shows the placeholder for several descriptions.
fn show_alt_text(ui: &mut Ui, args: &mut AltArgs) {
    state(ui, "story-alt", AltText::default(), |ui, value| {
        // The box is 340 points wide on its own; a tile is narrower, so there it takes the tile.
        let layout = if crate::story::in_overview(ui.ctx()) { egui::Layout::top_down_justified(egui::Align::Min) } else { *ui.layout() };
        ui.with_layout(layout, |ui| {
            let _ = AltTextField::new("story-alt", value).mixed(args.mixed).show(ui);
        });
    });
}

/// The word being typed at the end of `text`.
fn last_word(text: &str) -> &str {
    let start = text.rfind(|c: char| !c.is_alphanumeric()).map_or(0, |at| at + text[at..].chars().next().map_or(1, char::len_utf8));
    &text[start..]
}

/// Puts the caret at the end of the editor `id`.
fn caret_to_end(ui: &Ui, id: Id, len: usize) {
    let mut state = TextEdit::load_state(ui.ctx(), id).unwrap_or_default();
    state.cursor.set_char_range(Some(CCursorRange::one(CCursor::new(len))));
    state.store(ui.ctx(), id);
}

/// A formula box with function suggestions and descriptions, and a column box that completes the
/// name inline.
fn show_autocomplete(ui: &mut Ui, _args: &mut NoArgs) {
    let words = ControlStoryLabels::of(ui);
    let functions = [
        ("SUM", &words.fn_sum_desc),
        ("AVERAGE", &words.fn_average_desc),
        ("COUNT", &words.fn_count_desc),
        ("MAX", &words.fn_max_desc),
        ("MIN", &words.fn_min_desc),
    ];
    let columns = [&words.col_revenue, &words.col_region, &words.col_returns];
    state(ui, "story-autocomplete", (String::from("=SU"), String::new()), |ui, (formula, column)| {
        let word = last_word(formula).to_owned();
        let suggestions: Vec<Suggestion> = if word.is_empty() {
            Vec::new()
        } else {
            functions
                .iter()
                .filter(|(name, _)| name.to_lowercase().starts_with(&word.to_lowercase()))
                .map(|(name, description)| Suggestion::new(*name).mark("ƒ").description(description.as_str()))
                .collect()
        };
        ui.label(words.formula.as_str());
        let id = ui.make_persistent_id("story-formula-text");
        let list = Autocomplete::new("story-formula", id, &suggestions);
        let keys = list.take_keys(ui);
        let field = TextEdit::singleline(formula).id(id).show(ui);
        ui.ctx().accesskit_node_builder(id, |node| node.set_label(words.formula.as_str()));
        let out = list.show(ui, field.response.rect, keys);
        if let Some(at) = out.accepted {
            let keep = formula.len() - word.len();
            formula.truncate(keep);
            formula.push_str(&suggestions[at].text);
            caret_to_end(ui, id, formula.chars().count());
            ui.ctx().memory_mut(|m| m.request_focus(id));
        }

        ui.add_space(ui.spacing().item_spacing.y);
        ui.label(words.column.as_str());
        let id = ui.make_persistent_id("story-column-text");
        let rest = (!column.is_empty())
            .then(|| columns.iter().find_map(|name| completion_rest(name, column).map(|rest| ((*name).as_str(), rest.to_owned()))))
            .flatten();
        let focused = ui.memory(|m| m.has_focus(id));
        if let Some((name, _)) = rest.as_ref().filter(|_| focused)
            && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Tab))
        {
            *column = (*name).to_owned();
            caret_to_end(ui, id, column.chars().count());
        }
        let field = TextEdit::singleline(column).id(id).show(ui);
        ui.ctx().accesskit_node_builder(id, |node| node.set_label(words.column.as_str()));
        if let Some((_, rest)) = rest.filter(|_| field.response.has_focus()) {
            paint_inline_completion(ui, field.galley_pos, &field.galley, &rest);
        }
    });
}

/// The name field's width: with the check and the "Mixed" note the row fits a narrow tile.
const NAME_WIDTH: f32 = 100.0;

/// Three bulk-edit rows: a name that differs across the selection, an opacity and a rotation that
/// differs.
fn show_apply_rows(ui: &mut Ui, args: &mut super::SizeArgs) {
    let words = ControlStoryLabels::of(ui);
    let shared = StoryLabels::of(ui);
    let numbers = LocaleNumbers::of(ui);
    let size = args.size.control();
    state(
        ui,
        "story-apply",
        (
            ApplyValue { value: words.apply_name.clone(), mixed: true, apply: false },
            ApplyValue { value: 100.0, mixed: false, apply: true },
            ApplyValue { value: 15.0, mixed: true, apply: false },
        ),
        |ui, (name, opacity, rotation)| {
            ui.horizontal(|ui| {
                let _ = ApplyRow::new("story-apply-name", &shared.name, name).show(ui, |ui, value| {
                    let edit = TextField::new("story-apply-name-text", &shared.name, value.as_str()).size(size).width(NAME_WIDTH).show(ui);
                    if let Some(v) = edit.event.as_ref().and_then(|e| e.value()) {
                        *value = v.clone();
                    }
                });
            });
            ui.horizontal(|ui| {
                let _ = ApplyRow::new("story-apply-opacity", &shared.opacity, opacity).show(ui, |ui, value| {
                    let edit =
                        NumberField::new("story-apply-opacity-field", &shared.opacity, *value, &numbers).range(0.0..=100.0).size(size).show(ui);
                    store(&edit, value);
                });
            });
            ui.horizontal(|ui| {
                let _ = ApplyRow::new("story-apply-rotation", &words.field_rotation, rotation).show(ui, |ui, value| {
                    let edit = NumberField::new("story-apply-rotation-field", &words.field_rotation, *value, &numbers)
                        .range(-180.0..=180.0)
                        .size(size)
                        .show(ui);
                    store(&edit, value);
                });
            });
        },
    );
}

/// Three fields with a reset dot each; the dot of a field that differs from its default is filled.
fn show_reset_dots(ui: &mut Ui, args: &mut super::SizeArgs) {
    let words = ControlStoryLabels::of(ui);
    let numbers = LocaleNumbers::of(ui);
    let size = args.size.control();
    const DEFAULTS: [f64; 3] = [8.0, 0.0, 4.0];
    let captions = [&words.field_radius, &words.field_rotation, &words.field_corner];
    state(ui, "story-reset-dots", [12.0, 0.0, 4.0], |ui, values| {
        variant_list(ui, "story-reset-dots", |form| {
            for (i, caption) in captions.iter().enumerate() {
                variant_row(form, caption, |ui| {
                    ui.horizontal(|ui| {
                        let edit = NumberField::new(("story-reset-field", i), (*caption).as_str(), values[i], &numbers)
                            .range(-360.0..=360.0)
                            .size(size)
                            .show(ui);
                        store(&edit, &mut values[i]);
                        let default = numbers.format(DEFAULTS[i], 0..=1);
                        if ResetDot::new(("story-reset-dot", i), values[i] != DEFAULTS[i], default).show(ui).clicked() {
                            values[i] = DEFAULTS[i];
                        }
                    })
                    .response
                });
            }
        });
    });
}
