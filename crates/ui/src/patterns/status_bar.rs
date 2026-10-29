//! [`StatusBar`]: the bar at the bottom of the window.

use egui::accesskit::Role;
use egui::text::LayoutJob;
use egui::{AsIdSalt, Color32, IdSalt, Label, Rect, Response, Sense, Ui, UiBuilder, vec2};

use crate::composites::{live, measure};
use crate::controls::{IconButton, NumberField};
use crate::direction;
use crate::icons::Icon;
use crate::localized::{LocaleNumbers, Unit};
use crate::menu::{MenuEntry, MenuItem, PressMenu};
use crate::primitives::{Content, Divider, Text};
use crate::story::{Args, Level, Story, Surface};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};
use craft_theme::{ControlSize, TypeRole};

/// A status bar's sizes and colours, from [`StatusBar::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct StatusBarStyle {
    /// Height of the bar: `status_bar.height`.
    pub height: f32,
    /// Space at both ends of the bar: `status_bar.pad`.
    pub pad: f32,
    /// Between items: `status_bar.gap`.
    pub gap: f32,
    /// Added after the zoom field: `status_bar.group_gap`.
    pub group_gap: f32,
    /// Length of a divider: `status_bar.divider`.
    pub divider: f32,
    /// Width of the box a divider is drawn in: the line's own point between two gaps.
    pub divider_box: f32,
    /// Width of the zoom field's cell: `status_bar.field_width`.
    pub field_width: f32,
    /// Fill of the bar: `surface.chrome`.
    pub fill: Color32,
    /// Info text: `text.muted`.
    pub text: Color32,
    /// The status message: `text.faint`.
    pub message: Color32,
    /// An error and the render-path label: `status.warn.fg`.
    pub warning: Color32,
}

type Slot<'a> = Option<Box<dyn FnOnce(&mut Ui) + 'a>>;

/// The slots of a bar, filled by the closure given to [`StatusBar::show`].
#[derive(Default)]
pub struct StatusBarSlots<'a> {
    leading: Slot<'a>,
    trailing: Slot<'a>,
}

impl<'a> StatusBarSlots<'a> {
    /// The items at the reading start: zoom field, info text and menu button, separator and message.
    /// When the bar is too narrow the last of them is cut; the message is the one that gives way.
    pub fn leading(&mut self, add: impl FnOnce(&mut Ui) + 'a) {
        self.leading = Some(Box::new(add));
    }

    /// The items pinned at the reading end, listed in reading order: job progress, the render-path
    /// label. The group keeps its width however narrow the bar is.
    pub fn trailing(&mut self, add: impl FnOnce(&mut Ui) + 'a) {
        self.trailing = Some(Box::new(add));
    }
}

/// The bar at the bottom of the window, as in PhotoCraft: a full-width bar in `surface.chrome`
/// with a leading group (zoom field, info text, message) and a trailing group (job progress,
/// render path), items centred vertically. The app supplies every item.
///
/// ```
/// use craft_ui::controls::NumberField;
/// use craft_ui::localized::{LocaleNumbers, Unit};
/// use craft_ui::patterns::StatusBar;
/// # egui::__run_test_ui(|ui| {
/// let numbers = LocaleNumbers::from_tag("en-US");
/// StatusBar::new("status").label("Status bar").show(ui, |bar| {
///     bar.leading(|ui| {
///         StatusBar::field(ui, |ui| {
///             let _ = NumberField::new("zoom", "Zoom", 99.3, &numbers).unit(Unit::new("{n}%")).show(ui);
///         });
///         StatusBar::info(ui, "1200 px x 800 px (72 ppi)");
///         StatusBar::message(ui, "Saved", false);
///     });
///     bar.trailing(|ui| {
///         StatusBar::warning(ui, "CPU Rendering");
///     });
/// });
/// # });
/// ```
///
/// - Narrow: the trailing group keeps its width and the leading group is clipped at its end. A
///   [`message`](StatusBar::message) ends in an ellipsis and shows its full text as a tooltip.
/// - Both groups mirror in a right-to-left UI.
/// - **Keyboard and AccessKit:** every control in the bar is a Tab stop, in reading order (leading
///   group, then trailing group); the bar is not. The bar is a `Group` named by
///   [`label`](StatusBar::label); the message is in a polite live region. Dividers are not in the
///   tree.
pub struct StatusBar {
    id_salt: IdSalt,
    label: String,
}

impl StatusBar {
    /// A bar; `id_salt` keeps the width of its trailing group.
    pub fn new(id_salt: impl AsIdSalt) -> Self {
        Self { id_salt: IdSalt::new(id_salt), label: "Status bar".into() }
    }

    /// The bar's accessible name, translated by the host (default "Status bar").
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    pub fn style(tokens: &UiTokens) -> StatusBarStyle {
        let s = &tokens.component.status_bar;
        let c = &tokens.color;
        StatusBarStyle {
            height: s.height,
            pad: s.pad,
            gap: s.gap,
            group_gap: s.group_gap,
            divider: s.divider,
            divider_box: 2.0 * s.gap + 1.0,
            field_width: s.field_width,
            fill: c.surface.chrome.to_egui(),
            text: Content::Muted.color(tokens).to_egui(),
            message: Content::Faint.color(tokens).to_egui(),
            warning: c.status.warn.fg.to_egui(),
        }
    }

    /// Shows the bar across the available width. `add` fills the slots; the leading group is
    /// drawn first, so Tab and AccessKit visit it before the trailing one.
    pub fn show<'a>(self, ui: &mut Ui, add: impl FnOnce(&mut StatusBarSlots<'a>)) -> Response {
        let tokens = Tokens::of(ui);
        let style = StatusBar::style(&tokens);
        let mut slots = StatusBarSlots::default();
        add(&mut slots);
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), style.height), Sense::hover());
        ui.painter().rect_filled(rect, 0.0, style.fill);
        let inner = Rect::from_x_y_ranges(rect.x_range().shrink(style.pad), rect.y_range());
        let id = ui.make_persistent_id(self.id_salt);
        let builder = UiBuilder::new()
            .id_salt(id)
            .max_rect(inner)
            .layout(direction::row(ui))
            .accessibility_role(Role::Group)
            .accessibility_label(self.label.as_str());
        let mut bar = ui.new_child(builder);
        let trailing_id = id.with("trailing");
        let reserved = match slots.trailing {
            Some(_) => measure::trailing_width(ui.ctx(), trailing_id).map_or(0.0, |w| w + style.gap),
            None => 0.0,
        };
        if let Some(leading) = slots.leading {
            let x_range = if direction::is_rtl(ui) { inner.left() + reserved..=inner.right() } else { inner.left()..=inner.right() - reserved };
            let area = Rect::from_x_y_ranges(x_range, inner.y_range());
            let mut group = bar.new_child(UiBuilder::new().id_salt("leading").max_rect(area).layout(direction::row(ui)));
            group.set_clip_rect(area.intersect(bar.clip_rect()));
            group.spacing_mut().item_spacing.x = style.gap;
            leading(&mut group);
        }
        if let Some(trailing) = slots.trailing {
            measure::trailing_row(&mut bar, trailing_id, inner, Sense::hover(), style.gap, 0.0, trailing);
        }
        response
    }

    /// The zoom field's cell: a box `status_bar.field_width` wide for a [`NumberField`] of
    /// [`ControlSize::Sm`], followed by `status_bar.group_gap`.
    pub fn field<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
        let style = StatusBar::style(&Tokens::of(ui));
        let size = vec2(style.field_width, ui.available_height());
        let result = ui
            .allocate_ui_with_layout(size, direction::row(ui).with_main_justify(true), |ui| {
                ui.set_width(style.field_width);
                add(ui)
            })
            .inner;
        ui.add_space(style.group_gap);
        result
    }

    /// Info text in `text.muted`.
    pub fn info(ui: &mut Ui, text: &str) -> Response {
        ui.add(Text::new(text).content(Content::Muted))
    }

    /// The button that opens the info menu: a chevron pointing along the bar, away from the zoom
    /// field. `label` is its translated name and tooltip ("Show"). The host opens a
    /// [`PressMenu`] on the returned response.
    pub fn menu_button(ui: &mut Ui, label: &str) -> Response {
        ui.add(IconButton::new(Icon::ChevronRight, label).size(ControlSize::Sm))
    }

    /// A divider between items: a line `status_bar.divider` long in `line.divider`, in a box the
    /// width of two gaps and a point.
    pub fn separator(ui: &mut Ui) {
        let tokens = Tokens::of(ui);
        let style = StatusBar::style(&tokens);
        let (rect, _) = ui.allocate_exact_size(vec2(style.divider_box, style.divider), Sense::hover());
        Divider::paint(ui.painter(), rect.center_top(), rect.center_bottom(), &tokens);
    }

    /// The status message, after a separator: `text.faint`, or the warning colour for an error. It
    /// stays on one line, ends in an ellipsis when it does not fit, and is in a polite live region
    /// so a screen reader announces a new text. An empty message draws nothing, its separator
    /// included. Use it once per bar.
    pub fn message(ui: &mut Ui, text: &str, error: bool) {
        if text.is_empty() {
            return;
        }
        Self::separator(ui);
        let tokens = Tokens::of(ui);
        let style = StatusBar::style(&tokens);
        let mut format = Text::format(&tokens, TypeRole::Body, Content::Faint);
        format.color = if error { style.warning } else { style.message };
        format.valign = ui.text_valign();
        let id = ui.make_persistent_id("status-message");
        live::region(ui, id, Role::Status, live::Politeness::Polite, |ui, labels| {
            labels.add(text);
            ui.add(Label::new(LayoutJob::single_section(text.to_owned(), format)).truncate());
        });
    }

    /// A clickable text in the warning colour, for the trailing group: the render-path label
    /// ("CPU Rendering"). The host handles the click and gives it a tooltip.
    pub fn warning(ui: &mut Ui, text: &str) -> Response {
        let tokens = Tokens::of(ui);
        let mut format = Text::format(&tokens, TypeRole::Body, Content::Muted);
        format.color = StatusBar::style(&tokens).warning;
        format.valign = ui.text_valign();
        ui.add(Label::new(LayoutJob::single_section(text.to_owned(), format)).sense(Sense::click()))
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["StatusBar"];

const INFO: &[(&str, &str)] = &[("size", "Document size"), ("profile", "Document profile"), ("tool", "Current tool")];

#[derive(Clone, Debug)]
struct Demo {
    zoom: f64,
    info: usize,
}

impl Default for Demo {
    fn default() -> Self {
        Self { zoom: 99.3, info: 0 }
    }
}

fn info_text(index: usize) -> &'static str {
    ["1200 px x 800 px (72 ppi)", "sRGB IEC61966-2.1", "Brush"][index]
}

/// PhotoCraft's status bar: zoom, the chosen info and its menu, a message and the render path.
fn show_story(ui: &mut Ui, args: &mut Args) {
    let width = args.number("width") as f32;
    let (message, error, render_path) = (args.text("message").to_owned(), args.bool("error"), args.bool("render_path"));
    let state_id = ui.scope_id().with("status-bar-demo");
    let mut demo = ui.data(|d| d.get_temp::<Demo>(state_id)).unwrap_or_default();
    let numbers = LocaleNumbers::from_tag("en-US");
    ui.allocate_ui(vec2(width, 0.0), |ui| {
        ui.set_max_width(width);
        StatusBar::new("status-bar-demo").show(ui, |bar| {
            bar.leading(|ui| {
                StatusBar::field(ui, |ui| {
                    let edit = NumberField::new("demo-zoom", "Zoom", demo.zoom, &numbers)
                        .range(1.0..=6400.0)
                        .unit(Unit::new("{n}%"))
                        .scrub_wrap(true)
                        .size(ControlSize::Sm)
                        .show(ui);
                    if let Some(v) = edit.event.as_ref().and_then(|e| e.value()) {
                        demo.zoom = *v;
                    }
                });
                StatusBar::info(ui, info_text(demo.info));
                let button = StatusBar::menu_button(ui, "Show");
                let entries: Vec<MenuEntry<usize>> =
                    INFO.iter().enumerate().map(|(i, (_, label))| MenuItem::new(*label, i).radio(demo.info == i).into()).collect();
                if let Some(chosen) = PressMenu::new(&entries).size(ControlSize::Sm).show(ui, &button).chosen {
                    demo.info = chosen;
                }
                StatusBar::message(ui, &message, error);
            });
            if render_path {
                bar.trailing(|ui| {
                    StatusBar::warning(ui, "CPU Rendering");
                });
            }
        });
    });
    ui.data_mut(|d| d.insert_temp(state_id, demo));
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        Story::new(
            "patterns/status-bar/photocraft",
            Level::Patterns,
            "StatusBar",
            "PhotoCraft",
            || {
                Args::new()
                    .with_text("message", "Saved to Documents")
                    .with_bool("error", false)
                    .with_bool("render_path", true)
                    .with_number("width", 640.0, 160.0, 1200.0)
            },
            show_story,
        )
        .on(Surface::Chrome),
        Story::new(
            "patterns/status-bar/error",
            Level::Patterns,
            "StatusBar",
            "Error",
            || {
                Args::new()
                    .with_text("message", "Couldn't save: the disk is full")
                    .with_bool("error", true)
                    .with_bool("render_path", true)
                    .with_number("width", 640.0, 160.0, 1200.0)
            },
            show_story,
        )
        .on(Surface::Chrome),
        Story::new(
            "patterns/status-bar/narrow",
            Level::Patterns,
            "StatusBar",
            "Narrow",
            || {
                Args::new()
                    .with_text("message", "Exported 14 layers to the folder Documents/Exports/2026")
                    .with_bool("error", false)
                    .with_bool("render_path", true)
                    .with_number("width", 480.0, 160.0, 1200.0)
            },
            show_story,
        )
        .on(Surface::Chrome),
    ]
}
