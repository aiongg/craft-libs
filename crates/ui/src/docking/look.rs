//! The look of a dock area (egui-typed colors, strokes, sizes and font) and [`DockSpec`], what
//! [`super::DockArea`] draws from.

use std::borrow::Cow;

use craft_theme::{Contrast, Identify, RowStyle, TypeRole};
use egui::{Color32, CornerRadius, FontId, Frame, InnerResponse, Margin, Shadow, Stroke, Ui};

use super::{DockConfig, DockStyle};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens, corner_radius};

/// Colors, strokes, sizes and font of a [`super::DockArea`]'s chrome. Behaviour and labels are
/// in [`DockConfig`].
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct DockLook {
    /// Height of a group's tab strip: `size.panel_header`.
    pub tab_height: f32,
    /// Space between the groups of a split.
    pub gap: f32,
    /// Gap beside attached side docks; `None`: `gap`. Other split dividers keep `gap`.
    pub dock_gap: Option<f32>,
    /// Vertical gap between stacked controls in a panel body, docked or floating; 0: controls
    /// space themselves.
    pub body_gap: f32,
    /// Least width or height of a pane in a split: `dock.min_pane`.
    pub min_pane: f32,
    /// Minimum width of the drag bar left of a group's header: `dock.drag_bar_width`. Tabs shrink,
    /// then overflow, before the bar narrows; a strip narrower than the header plus this width
    /// narrows it.
    pub drag_bar_width: f32,
    /// Width of a group's tab strip overflow button: `dock.overflow_width`.
    pub overflow_width: f32,
    /// Tallest list in a tab overflow or move menu: `dock.menu_max_height`.
    pub menu_max_height: f32,
    /// Room inside controls: around a title, a float hint and an icon's label.
    pub pad: f32,
    /// Fill of panel bodies.
    pub background: Color32,
    /// Tab strip, column bar and floating title bar.
    pub tab_strip: Color32,
    /// The selected tab, and the tab under the pointer.
    pub tab_active: Color32,
    /// Top corners of the selected tab: `shape.radius.control` in rounded-row themes.
    pub tab_radius: f32,
    /// Fill of the current icon cell in an iconic column.
    pub icon_selected: Color32,
    #[allow(missing_docs)]
    pub text: Color32,
    #[allow(missing_docs)]
    pub inactive_text: Color32,
    /// Outline of panel bodies and the dock's edges.
    pub border: Stroke,
    /// A resize handle under the pointer or being dragged.
    pub resize: Color32,
    /// Drop previews, insertion marker and drop outlines.
    pub drop: Color32,
    /// Keyboard focus strokes.
    pub focus: Color32,
    #[allow(missing_docs)]
    pub font: FontId,
    /// Horizontal room around a tab's label, both when measuring its natural width and when
    /// cutting it.
    pub tab_padding: f32,
    /// Painted under every floating window, so it stays opaque when the panel colors are not.
    pub floating_background: Color32,
    /// Outline of a floating window.
    pub floating_stroke: Stroke,
    /// Painted over a native floating window while [`super::DockArea::input_blocked`] is on.
    pub blocked_dim: Color32,
    /// Height of a floating window's title bar: its label's line and `gap.tight` above and below.
    pub title_height: f32,
    /// \[D\] Height of the bar above each dock column, as the title bar's. It drags the column and holds its »/« button.
    pub column_bar_height: f32,
    /// \[D\] Side of a panel icon in an iconic column; each icon row is `icon_pad` taller.
    pub icon_size: f32,
    /// \[D\] Width of an iconic column without a saved width, and the narrowest it gets.
    pub iconic_width: f32,
    /// Widest an iconic column gets by dragging its inner edge: `dock.iconic_max_width`.
    pub iconic_max_width: f32,
    /// An iconic column at least this wide shows panel names beside the icons:
    /// `dock.iconic_label_width`.
    pub iconic_label_width: f32,
    /// Height of an iconic column's flyout without a saved height: `dock.flyout_height`.
    pub flyout_height: f32,
    /// Added to `icon_size` for the height of an iconic column's row.
    pub icon_pad: f32,
    /// Label and icon on the current icon cell.
    pub icon_selected_text: Color32,
    /// Corner radius of the current icon cell and of the letter box.
    pub icon_radius: f32,
    /// Corner radius of the float hint box.
    pub hint_radius: f32,
    /// Width of drop outlines and of the line under a dragged column.
    pub drop_width: f32,
    /// Width of a keyboard focus stroke and of a handle's hover outline.
    pub focus_width: f32,
    /// Corner radius of the insertion marker.
    pub marker_radius: f32,
    /// Alpha factor of the filled drop preview.
    pub drop_fill: f32,
    /// Alpha factor of the highlighted zone under a dragged tab.
    pub ghost_fill: f32,
    /// Alpha factor of the outline of every zone under a dragged tab.
    pub ghost_stroke: f32,
    /// Alpha factor of the light fill inside a tab drop's outline.
    pub tab_fill: f32,
    /// Alpha factor of a handle's outline at hover and focus.
    pub grip_alpha: f32,
    /// Least height of a floating window fitted to its content: `float.min_fitted_height`.
    pub min_fitted_height: f32,
    /// Offset of a floated window from the dock area's corner or the window it cascades from:
    /// `size.control.sm`.
    pub cascade_offset: f32,
    /// Distance from the pointer to the centre of the float hint: `size.control.md`.
    pub hint_offset: f32,
    /// Narrowest a resized floating window gets: `float.min_width`.
    pub float_min_width: f32,
    /// Room for content that a resized floating window keeps below its title bar:
    /// `float.min_body_height`.
    pub float_min_body_height: f32,
    /// Corner radius of a floating window's fill and outline.
    pub floating_radius: CornerRadius,
    /// Shadow of a floating window.
    pub floating_shadow: Shadow,
    /// Fill of a floating window's title bar.
    pub title_background: Color32,
    #[allow(missing_docs)]
    pub title_text: Color32,
    /// The ▾ and × glyphs of a floating title bar at rest.
    pub title_muted: Color32,
    /// The diagonal resize grip of a floating window.
    pub grip: Color32,
}

impl DockLook {
    /// Draws `add` as the body of a panel outside a dock area: the dock's body fill and edge, the
    /// dock's `body_gap` between stacked controls, and `margin` around the content (a dock area
    /// draws bodies edge to edge and leaves the margin to the panel's content).
    pub fn panel_body<R>(&self, ui: &mut Ui, margin: Margin, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        Frame::new().fill(self.background).stroke(self.border).inner_margin(margin).show(ui, |ui| {
            self.space_body(ui);
            add(ui)
        })
    }

    /// Sets the spacing between the controls of a panel body.
    pub(super) fn space_body(&self, ui: &mut Ui) {
        ui.spacing_mut().item_spacing.y = self.body_gap;
    }

    /// The look of the `ui`'s theme.
    pub fn of(ui: &Ui) -> Self {
        Self::from_tokens(&Tokens::of(ui))
    }

    /// The look the semantic tokens describe, with the dock and floating-window styles.
    pub fn from_tokens(tokens: &UiTokens) -> Self {
        let (c, space, size, stroke) = (&tokens.color, &tokens.space, &tokens.size, &tokens.stroke);
        let dock = crate::style::dock::DockMetrics::new(tokens);
        let float = crate::style::float::FloatStyle::new(tokens);
        let bar_height = tokens.typography.get(TypeRole::Label).line_height + 2.0 * space.gap.tight;
        let high = tokens.appearance.contrast == Contrast::High;
        let (icon_selected, icon_selected_text) = Identify::on(&[c.surface.panel], high).fill(c.state.selected, c.text.on_selected);
        Self {
            tab_height: size.panel_header,
            gap: space.gap.inline,
            dock_gap: None,
            body_gap: 0.0,
            min_pane: dock.min_pane,
            drag_bar_width: dock.drag_bar_width,
            overflow_width: dock.overflow_width,
            menu_max_height: dock.menu_max_height,
            pad: space.pad.control,
            background: c.surface.panel.to_egui(),
            tab_strip: c.surface.background.to_egui(),
            tab_active: c.surface.panel.to_egui(),
            tab_radius: if tokens.shape.rows == RowStyle::Rounded { tokens.shape.radius.control } else { 0.0 },
            icon_selected: icon_selected.to_egui(),
            text: c.text.normal.to_egui(),
            inactive_text: c.text.muted.to_egui(),
            border: Stroke::new(stroke.thin, c.line.border.to_egui()),
            resize: c.accent.solid.to_egui(),
            drop: c.accent.solid.to_egui(),
            focus: c.line.focus.to_egui(),
            font: tokens.font(TypeRole::Label),
            tab_padding: 4.0 * space.pad.control,
            floating_background: c.surface.panel.to_egui(),
            floating_stroke: Stroke::new(stroke.thin, c.line.border.to_egui()),
            blocked_dim: c.surface.scrim.to_egui(),
            title_height: bar_height,
            column_bar_height: bar_height,
            icon_size: size.icon.lg,
            iconic_width: dock.iconic_width,
            iconic_max_width: dock.iconic_max_width,
            iconic_label_width: dock.iconic_label_width,
            flyout_height: dock.flyout_height,
            icon_pad: space.gap.stack,
            icon_selected_text: icon_selected_text.to_egui(),
            icon_radius: tokens.shape.radius.control,
            hint_radius: tokens.shape.radius.popup,
            drop_width: stroke.thick,
            focus_width: stroke.thin,
            marker_radius: tokens.shape.radius.sm,
            drop_fill: dock.drop_fill,
            ghost_fill: dock.ghost_fill,
            ghost_stroke: dock.ghost_stroke,
            tab_fill: dock.tab_fill,
            grip_alpha: dock.grip_alpha,
            min_fitted_height: float.min_fitted_height,
            cascade_offset: size.control.sm,
            hint_offset: size.control.md,
            float_min_width: float.min_width,
            float_min_body_height: float.min_body_height,
            floating_radius: corner_radius(tokens.shape.radius.panel),
            floating_shadow: tokens.elevation.floating.to_egui(),
            title_background: c.surface.background.to_egui(),
            title_text: c.text.normal.to_egui(),
            title_muted: c.text.muted.to_egui(),
            grip: c.accent.solid.to_egui(),
        }
    }

    /// Sizes finite and within 0..=4096, and the tab height and font size positive.
    pub(super) fn valid(&self) -> bool {
        [
            self.tab_height,
            self.gap,
            self.body_gap,
            self.pad,
            self.border.width,
            self.font.size,
            self.tab_padding,
            self.floating_stroke.width,
            self.title_height,
            self.column_bar_height,
            self.icon_size,
            self.iconic_width,
            self.icon_pad,
            self.min_pane,
            self.drag_bar_width,
            self.overflow_width,
            self.menu_max_height,
            self.iconic_max_width,
            self.iconic_label_width,
            self.flyout_height,
            self.min_fitted_height,
            self.cascade_offset,
            self.hint_offset,
            self.float_min_width,
            self.float_min_body_height,
            self.icon_radius,
            self.hint_radius,
            self.drop_width,
            self.focus_width,
            self.marker_radius,
            self.drop_fill,
            self.ghost_fill,
            self.ghost_stroke,
            self.tab_fill,
            self.grip_alpha,
        ]
        .iter()
        .all(|v| v.is_finite() && (0.0..=4096.0).contains(v))
            && self.tab_height > 0.0
            && self.font.size > 0.0
            && self.dock_gap.is_none_or(|gap| gap.is_finite() && (0.0..=4096.0).contains(&gap))
    }
}

/// What a [`super::DockArea`] draws from: its behaviour and labels, and its look. Implemented by
/// [`DockConfig`] (the look of the `ui`'s theme), [`DockWith`] (a config and a look the
/// application built) and the flat [`DockStyle`].
pub trait DockSpec {
    /// The config and the look for this frame.
    fn parts(&self, ui: &Ui) -> (Cow<'_, DockConfig>, Cow<'_, DockLook>);
}

impl<T: DockSpec + ?Sized> DockSpec for &T {
    fn parts(&self, ui: &Ui) -> (Cow<'_, DockConfig>, Cow<'_, DockLook>) {
        (**self).parts(ui)
    }
}

impl DockSpec for DockConfig {
    fn parts(&self, ui: &Ui) -> (Cow<'_, DockConfig>, Cow<'_, DockLook>) {
        (Cow::Borrowed(self), Cow::Owned(DockLook::of(ui)))
    }
}

impl DockSpec for DockStyle {
    fn parts(&self, _ui: &Ui) -> (Cow<'_, DockConfig>, Cow<'_, DockLook>) {
        let (config, look) = self.split();
        (Cow::Owned(config), Cow::Owned(look))
    }
}

/// A config and a look the application built.
#[derive(Clone, Debug)]
pub struct DockWith {
    /// The behaviour and labels.
    pub config: DockConfig,
    /// The colors, sizes and font.
    pub look: DockLook,
}

impl DockSpec for DockWith {
    fn parts(&self, _ui: &Ui) -> (Cow<'_, DockConfig>, Cow<'_, DockLook>) {
        (Cow::Borrowed(&self.config), Cow::Borrowed(&self.look))
    }
}

/// Whether `config` and `look` together describe a drawable dock area.
pub(super) fn valid(config: &DockConfig, look: &DockLook) -> bool {
    config.valid(look) && look.valid() && look.iconic_width <= look.iconic_max_width
}
