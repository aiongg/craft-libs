//! Size sets: the numbers behind a Size choice. A set holds the base values (what the semantic
//! `size`, `space`, `shape` and `typography` tokens start from) and the component values that
//! differ by set. The built-in sets are the JSON files in `sizes/`, read by
//! [`format::parse_sizes`](crate::format::parse_sizes).

use std::sync::LazyLock;

use super::format;
use super::scale::Radius;
use super::theme::Size;
use super::tokens::{Edge, FontSlot, RadiusSet, SizeTokens, SpaceTokens, TypeTokens};

/// The Size-dependent semantic values: sizes, spacing, default shape and type roles.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Preset {
    /// Control, icon, bar and list sizes.
    pub size: SizeTokens,
    /// Gaps and padding.
    pub space: SpaceTokens,
    /// Default corner radii.
    pub radius: RadiusSet,
    /// Default edge.
    pub edge: Edge,
    /// Type roles.
    pub typography: TypeTokens,
}

/// The component values that differ by size set: what each family's `defaults()` reads in
/// place of a constant.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ComponentSizes {
    /// `options_bar.button`: side of the options bar's icon buttons.
    pub options_bar_button: f32,
    /// `progress.width`: a progress bar's length when the caller gives none.
    pub progress_width: f32,
    /// `button.height.sm`.
    pub button_height_sm: f32,
    /// `button.height.md`.
    pub button_height_md: f32,
    /// `button.pad_x`: padding at the start and end.
    pub button_pad_x: f32,
    /// `dock.body_gap`: between stacked controls in a panel body.
    pub dock_body_gap: f32,
    /// `status_bar.field_width`: the zoom field cell.
    pub status_bar_field_width: f32,
    /// `command_palette.row_height`.
    pub command_palette_row_height: f32,
    /// `inspector.header_height`.
    pub inspector_header_height: f32,
    /// `inspector.icon`: the kind icon in the header.
    pub inspector_icon: f32,
    /// `toolbar.button`: horizontal toolbar button.
    pub toolbar_button: f32,
    /// `toolbar.button_large`: vertical strip button.
    pub toolbar_button_large: f32,
    /// `toolbar.gap`: between the buttons of a group.
    pub toolbar_gap: f32,
    /// `toolbar.group_gap`: between the groups of a strip.
    pub toolbar_group_gap: f32,
    /// `toolbar.pad`: strip inset, across and along.
    pub toolbar_pad: [f32; 2],
    /// `toolbar.width`: vertical strip width.
    pub toolbar_width: f32,
    /// `toolbar.overflow`: overflow button width.
    pub toolbar_overflow: f32,
    /// `toast.pad`: between a card's edge and its content.
    pub toast_pad: f32,
    /// `toast.inset`: from the host's reading-end and bottom edges to the stack.
    pub toast_inset: [f32; 2],
}

/// A size set: what a [`Size`] choice stands for.
#[derive(Clone, Debug, PartialEq)]
pub struct SizeSet {
    /// The set's id, such as `compact`.
    pub id: String,
    /// Its display name.
    pub name: String,
    /// The base values, at `base.*` in the file.
    pub base: Preset,
    /// The component values, at `components.*` in the file.
    pub components: ComponentSizes,
}

static COMPACT: LazyLock<SizeSet> = LazyLock::new(|| builtin_set(include_str!("../sizes/compact.json")));
static COMFORTABLE: LazyLock<SizeSet> = LazyLock::new(|| builtin_set(include_str!("../sizes/comfortable.json")));

#[allow(clippy::panic)] // The built-in files are checked by a test.
fn builtin_set(text: &str) -> SizeSet {
    match format::parse_sizes(text.as_bytes()) {
        Ok(file) => file.set,
        Err(e) => panic!("built-in size set: {e}"),
    }
}

/// The built-in set a [`Size`] names.
pub fn builtin(size: Size) -> &'static SizeSet {
    match size {
        Size::Compact => &COMPACT,
        Size::Comfortable => &COMFORTABLE,
    }
}

/// The JSON form of one value in a set.
pub(crate) enum Slot<'a> {
    /// A number.
    Num(&'a mut f32),
    /// A number or `"full"`.
    Length(&'a mut f32),
    /// Two numbers.
    Pair(&'a mut [f32; 2]),
    /// A switch.
    Bool(&'a mut bool),
    /// A font slot name.
    Font(&'a mut FontSlot),
    /// `"flat"`.
    Edge(&'a mut Edge),
}

/// One value of a set: its path in the file, the range it must lie in, and where it is stored.
pub(crate) struct Leaf<'a> {
    pub path: &'static str,
    pub lo: f32,
    pub hi: f32,
    pub slot: Slot<'a>,
}

/// The radius value of `"full"`.
pub(crate) const FULL: f32 = Radius::Full.px();

macro_rules! leaves {
    ($out:ident; $( $path:literal $kind:ident $lo:literal..=$hi:literal => $place:expr; )*) => {
        $( $out.push(Leaf { path: $path, lo: $lo, hi: $hi, slot: Slot::$kind(&mut $place) }); )*
    };
}

/// Every value of a set with its range, in file order. The one table the parser, validator and
/// writer share.
pub(crate) fn leaves<'a>(b: &'a mut Preset, c: &'a mut ComponentSizes) -> Vec<Leaf<'a>> {
    let mut out = Vec::new();
    let s = &mut b.size;
    leaves! { out;
        "base.size.control.sm" Num 16.0..=48.0 => s.control.sm;
        "base.size.control.md" Num 16.0..=48.0 => s.control.md;
        "base.size.icon.sm" Num 10.0..=32.0 => s.icon.sm;
        "base.size.icon.md" Num 10.0..=32.0 => s.icon.md;
        "base.size.icon.lg" Num 10.0..=32.0 => s.icon.lg;
        "base.size.rail.width" Num 24.0..=96.0 => s.rail.width;
        "base.size.rail.button" Num 16.0..=64.0 => s.rail.button;
        "base.size.rail.panel_width" Num 24.0..=96.0 => s.rail.panel_width;
        "base.size.rail.panel_button" Num 16.0..=64.0 => s.rail.panel_button;
        "base.size.bar.title" Num 16.0..=80.0 => s.bar.title;
        "base.size.bar.tab" Num 16.0..=64.0 => s.bar.tab;
        "base.size.bar.status" Num 16.0..=64.0 => s.bar.status;
        "base.size.bar.options" Num 16.0..=80.0 => s.bar.options;
        "base.size.bar.options_narrow" Num 16.0..=80.0 => s.bar.options_narrow;
        "base.size.panel_header" Num 16.0..=64.0 => s.panel_header;
        "base.size.list.thumb.sm" Num 12.0..=96.0 => s.list.thumb.sm;
        "base.size.list.thumb.md" Num 12.0..=96.0 => s.list.thumb.md;
        "base.size.list.thumb.lg" Num 12.0..=96.0 => s.list.thumb.lg;
        "base.size.list.thumb_row_min" Num 16.0..=96.0 => s.list.thumb_row_min;
        "base.size.list.indent" Num 4.0..=32.0 => s.list.indent;
        "base.size.list.disclosure" Num 8.0..=32.0 => s.list.disclosure;
        "base.size.list.toggle_column" Num 16.0..=64.0 => s.list.toggle_column;
        "base.size.list.min_label" Num 32.0..=160.0 => s.list.min_label;
        "base.size.list.trailing_pad" Num 0.0..=32.0 => s.list.trailing_pad;
        "base.size.list.rounded" Bool 0.0..=0.0 => s.list.rounded;
        "base.size.list.dividers" Bool 0.0..=0.0 => s.list.dividers;
        "base.size.tab.panel_height" Num 16.0..=64.0 => s.tab.panel_height;
        "base.size.tab.height_large" Num 16.0..=64.0 => s.tab.height_large;
        "base.size.tab.min_width" Num 24.0..=120.0 => s.tab.min_width;
        "base.size.tab.close" Num 8.0..=32.0 => s.tab.close;
        "base.size.tab.close_icon" Num 6.0..=24.0 => s.tab.close_icon;
        "base.size.tab.rounded" Bool 0.0..=0.0 => s.tab.rounded;
        "base.size.tab.dividers" Bool 0.0..=0.0 => s.tab.dividers;
        "base.size.preset.header_height" Num 16.0..=64.0 => s.preset.header_height;
        "base.size.preset.grid_gap" Num 0.0..=32.0 => s.preset.grid_gap;
        "base.size.preset.grid_inset" Num 0.0..=32.0 => s.preset.grid_inset;
        "base.size.preset.cell_radius" Num 0.0..=32.0 => s.preset.cell_radius;
        "base.size.preset.caption_gap" Num 0.0..=16.0 => s.preset.caption_gap;
        "base.size.preset.slider_width" Num 48.0..=240.0 => s.preset.slider_width;
    }
    let sp = &mut b.space;
    leaves! { out;
        "base.space.gap.inline" Num 0.0..=32.0 => sp.gap.inline;
        "base.space.gap.row" Num 0.0..=32.0 => sp.gap.row;
        "base.space.gap.stack" Num 0.0..=32.0 => sp.gap.stack;
        "base.space.gap.section" Num 0.0..=32.0 => sp.gap.section;
        "base.space.pad.control" Num 0.0..=32.0 => sp.pad.control;
        "base.space.pad.panel" Num 0.0..=32.0 => sp.pad.panel;
        "base.space.pad.dialog" Num 0.0..=32.0 => sp.pad.dialog;
    }
    let r = &mut b.radius;
    leaves! { out;
        "base.radius.control" Length 0.0..=64.0 => r.control;
        "base.radius.popup" Length 0.0..=64.0 => r.popup;
        "base.radius.window" Length 0.0..=64.0 => r.window;
        "base.radius.panel" Length 0.0..=64.0 => r.panel;
        "base.radius.button" Length 0.0..=64.0 => r.button;
        "base.radius.pill" Length 0.0..=64.0 => r.pill;
    }
    leaves! { out;
        "base.edge" Edge 0.0..=0.0 => b.edge;
    }
    let t = &mut b.typography;
    leaves! { out;
        "base.typography.caption.size" Num 8.0..=32.0 => t.caption.size;
        "base.typography.caption.line_height" Num 8.0..=48.0 => t.caption.line_height;
        "base.typography.caption.font" Font 0.0..=0.0 => t.caption.font;
        "base.typography.label.size" Num 8.0..=32.0 => t.label.size;
        "base.typography.label.line_height" Num 8.0..=48.0 => t.label.line_height;
        "base.typography.label.font" Font 0.0..=0.0 => t.label.font;
        "base.typography.body.size" Num 8.0..=32.0 => t.body.size;
        "base.typography.body.line_height" Num 8.0..=48.0 => t.body.line_height;
        "base.typography.body.font" Font 0.0..=0.0 => t.body.font;
        "base.typography.body_strong.size" Num 8.0..=32.0 => t.body_strong.size;
        "base.typography.body_strong.line_height" Num 8.0..=48.0 => t.body_strong.line_height;
        "base.typography.body_strong.font" Font 0.0..=0.0 => t.body_strong.font;
        "base.typography.title.size" Num 8.0..=32.0 => t.title.size;
        "base.typography.title.line_height" Num 8.0..=48.0 => t.title.line_height;
        "base.typography.title.font" Font 0.0..=0.0 => t.title.font;
        "base.typography.heading.size" Num 8.0..=32.0 => t.heading.size;
        "base.typography.heading.line_height" Num 8.0..=48.0 => t.heading.line_height;
        "base.typography.heading.font" Font 0.0..=0.0 => t.heading.font;
        "base.typography.mono.size" Num 8.0..=32.0 => t.mono.size;
        "base.typography.mono.line_height" Num 8.0..=48.0 => t.mono.line_height;
        "base.typography.mono.font" Font 0.0..=0.0 => t.mono.font;
    }
    leaves! { out;
        "components.options_bar.button" Num 16.0..=64.0 => c.options_bar_button;
        "components.progress.width" Num 40.0..=400.0 => c.progress_width;
        "components.button.height.sm" Num 16.0..=48.0 => c.button_height_sm;
        "components.button.height.md" Num 16.0..=48.0 => c.button_height_md;
        "components.button.pad_x" Num 0.0..=32.0 => c.button_pad_x;
        "components.dock.body_gap" Num 0.0..=32.0 => c.dock_body_gap;
        "components.status_bar.field_width" Num 32.0..=160.0 => c.status_bar_field_width;
        "components.command_palette.row_height" Num 16.0..=64.0 => c.command_palette_row_height;
        "components.inspector.header_height" Num 16.0..=64.0 => c.inspector_header_height;
        "components.inspector.icon" Num 10.0..=32.0 => c.inspector_icon;
        "components.toolbar.button" Num 16.0..=64.0 => c.toolbar_button;
        "components.toolbar.button_large" Num 16.0..=80.0 => c.toolbar_button_large;
        "components.toolbar.gap" Num 0.0..=32.0 => c.toolbar_gap;
        "components.toolbar.group_gap" Num 0.0..=32.0 => c.toolbar_group_gap;
        "components.toolbar.pad" Pair 0.0..=32.0 => c.toolbar_pad;
        "components.toolbar.width" Num 24.0..=120.0 => c.toolbar_width;
        "components.toolbar.overflow" Num 16.0..=64.0 => c.toolbar_overflow;
        "components.toast.pad" Num 0.0..=32.0 => c.toast_pad;
        "components.toast.inset" Pair 0.0..=64.0 => c.toast_inset;
    }
    out
}
