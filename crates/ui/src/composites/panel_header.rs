//! [`PanelHeader`]: the title row of a flyout or floating panel.

use craft_theme::Stroke as Hairline;
use craft_theme::{ControlSize, TypeRole};
use egui::accesskit::Role;
use egui::emath::GuiRounding as _;
use egui::{Color32, Label, Rect, Response, Sense, Stroke, Ui, UiBuilder, WidgetInfo, vec2};

use super::measure;
use crate::controls::IconButton;
use crate::direction;
use crate::icons::Icon;
use crate::primitives::Content;
use crate::story::{Level, Story, StoryArgs};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens, role_format};
use crate::typed_story;

/// A panel header's height, padding, buttons and title, from [`PanelHeader::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PanelHeaderStyle {
    /// Height of the header: `panel_header.height`, the size preset's `panel_header`.
    pub height: f32,
    /// Between the header's ends and its content: `panel_header.pad`.
    pub pad: f32,
    /// Between the buttons and the actions: `panel_header.gap`.
    pub gap: f32,
    /// Side of the menu, float and close buttons: `panel_header.button`.
    pub button: f32,
    /// Space between the row and the rule under it: `space.gap.row`.
    pub rule_gap: f32,
    /// The rule under the row: `line.divider`, one physical pixel.
    pub rule: Stroke,
    /// Space under the rule: `space.pad.panel`.
    pub body_gap: f32,
    /// The title's type role: `title`.
    pub title_role: TypeRole,
    /// The title: `text.normal`.
    pub title: Color32,
}

/// What [`PanelHeader::show`] shows: the header's background and its buttons.
#[derive(Debug)]
pub struct PanelHeaderResponse {
    /// The background, which senses drags (not focus): the host starts a move or a tear-off from
    /// it.
    pub header: Response,
    /// The "Panel menu" button; the host opens its menu from it.
    pub menu: Option<Response>,
    /// The "Float panel" button.
    pub float: Option<Response>,
    /// The "Close panel" button.
    pub close: Option<Response>,
}

/// The title row of a flyout or a floating panel (DesignCraft's flyout header): the title at the
/// reading start, and at the other end the actions, a panel menu button, a float button and a close
/// button. It is `panel_header.height` high, with a hairline under it (PhotoCraft's flyout and
/// dialog title row: `space.gap.row` above the rule, `space.pad.panel` below).
///
/// ```
/// use craft_ui::composites::PanelHeader;
/// # egui::__run_test_ui(|ui| {
/// let header = PanelHeader::new("layers", "Layers").float(true).close(true).show(ui, |_| {});
/// if header.close.is_some_and(|close| close.clicked()) { /* close the panel */ }
/// # });
/// ```
///
/// - The title is cut with an ellipsis and shows its full text as a tooltip when it does not fit;
///   when the trailing content reaches the title's start, no title is drawn.
/// - The buttons are `panel_header.button` squares holding icon buttons named "Panel menu",
///   "Float panel" and "Close panel" (the host passes translated names with the `*_label`
///   methods). The menu button is on by default; the float and close buttons are off until asked
///   for.
/// - **Keyboard:** Tab visits the actions and the buttons in reading order (actions, menu, float,
///   close; mirrored right to left). The background is not a Tab stop.
/// - **Dragging.** The host gets the background's drag from [`PanelHeaderResponse::header`]:
///   moving and tearing off stay with the docking code. A press that starts on the actions or the
///   buttons belongs to them and never drags the panel.
/// - Docked panels keep the dock area's tab strip, not this header.
/// - **AccessKit:** the background is a `Header` named by the title; the buttons are named by
///   their labels and listed in reading order after the title.
#[must_use = "show the header with `show`"]
pub struct PanelHeader {
    title: String,
    id_salt: egui::IdSalt,
    menu: Slot,
    float: Slot,
    close: Slot,
}

/// One of the header's buttons: whether it is shown, and its accessible name.
struct Slot {
    shown: bool,
    label: String,
}

impl Slot {
    fn new(shown: bool, label: &str) -> Self {
        Self { shown, label: label.to_owned() }
    }
}

impl PanelHeader {
    /// A header titled `title`. `id_salt` is its id, so that its state and the buttons' focus
    /// don't depend on the translated title.
    pub fn new(id_salt: impl egui::AsIdSalt, title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            id_salt: egui::IdSalt::new(id_salt),
            menu: Slot::new(true, "Panel menu"),
            float: Slot::new(false, "Float panel"),
            close: Slot::new(false, "Close panel"),
        }
    }

    /// Whether the panel menu button is shown; on by default.
    pub fn menu(mut self, menu: bool) -> Self {
        self.menu.shown = menu;
        self
    }

    /// Shows the panel menu button with this accessible name.
    pub fn menu_label(mut self, label: impl Into<String>) -> Self {
        self.menu = Slot { shown: true, label: label.into() };
        self
    }

    /// Whether the float button is shown.
    pub fn float(mut self, float: bool) -> Self {
        self.float.shown = float;
        self
    }

    /// Shows the float button with this accessible name.
    pub fn float_label(mut self, label: impl Into<String>) -> Self {
        self.float = Slot { shown: true, label: label.into() };
        self
    }

    /// Whether the close button is shown.
    pub fn close(mut self, close: bool) -> Self {
        self.close.shown = close;
        self
    }

    /// Shows the close button with this accessible name.
    pub fn close_label(mut self, label: impl Into<String>) -> Self {
        self.close = Slot { shown: true, label: label.into() };
        self
    }

    pub fn style(tokens: &UiTokens, pixels_per_point: f32) -> PanelHeaderStyle {
        let p = &tokens.component.panel_header;
        PanelHeaderStyle {
            height: p.height,
            pad: p.pad,
            gap: p.gap,
            button: p.button,
            rule_gap: tokens.space.gap.row,
            rule: Stroke::new(Hairline::Hairline.width(pixels_per_point), tokens.color.line.divider.to_egui()),
            body_gap: tokens.space.pad.panel,
            title_role: TypeRole::Title,
            title: Content::Normal.color(tokens).to_egui(),
        }
    }

    /// Shows the header across the available width. `add_actions` adds widgets in a row in
    /// reading order, ending `panel_header.gap` before the menu button.
    pub fn show(self, ui: &mut Ui, add_actions: impl FnOnce(&mut Ui)) -> PanelHeaderResponse {
        let tokens = Tokens::of(ui);
        let style = PanelHeader::style(&tokens, ui.pixels_per_point());
        let (_, rect) = ui.allocate_space(vec2(ui.available_width(), style.height));
        let id = ui.make_persistent_id(self.id_salt);
        // Drags only, without focus: the background is not a Tab stop.
        let header = ui.interact(rect, id, Sense::DRAG);
        header.widget_info(|| WidgetInfo::labeled(Role::Header, ui.is_enabled(), &self.title));

        // The title is placed once the trailing side's width is known, in a `Ui` made first so
        // that AccessKit lists it first.
        let inner = rect.shrink2(vec2(style.pad, 0.0));
        let mut title_ui = ui.new_child(UiBuilder::new().max_rect(inner));
        let slots = [(Icon::More, "menu", &self.menu), (Icon::Float, "float", &self.float), (Icon::Close, "close", &self.close)];
        let count = slots.iter().filter(|(_, _, slot)| slot.shown).count() as f32;
        let estimate = (count * (style.button + style.gap) - style.gap).max(0.0);
        // The row senses drags so that a press on its widgets never reaches the background.
        let ([menu, float, close], used) = measure::trailing_row(ui, id.with("trailing"), inner, Sense::DRAG, style.gap, estimate, |row| {
            add_actions(row);
            slots.map(|(icon, salt, slot)| {
                slot.shown.then(|| {
                    let (_, square) = row.allocate_space(vec2(style.button, style.button));
                    IconButton::new(icon, slot.label.as_str()).size(ControlSize::Sm).id_salt((id, salt)).show_at(row, square)
                })
            })
        });

        let start = direction::start_x(ui, inner);
        let end = if used.width() > 0.0 { direction::start_x(ui, used) - direction::forward(ui) * style.gap } else { direction::end_x(ui, inner) };
        if direction::forward(ui) * (end - start) >= 1.0 {
            let band = Rect::from_x_y_ranges(start.min(end)..=start.max(end), rect.y_range());
            let format = role_format(&tokens, style.title_role, style.title);
            title_ui.scope_builder(UiBuilder::new().max_rect(band).layout(direction::row(ui)), |ui| {
                ui.add(Label::new(egui::text::LayoutJob::single_section(self.title.clone(), format)).truncate().selectable(false));
            });
        }
        ui.add_space(style.rule_gap);
        let (_, rule) = ui.allocate_space(vec2(ui.available_width(), style.rule.width));
        let y = (rule.top() + style.rule.width / 2.0).round_to_pixel_center(ui.pixels_per_point());
        ui.painter().hline(rule.x_range(), y, style.rule);
        ui.add_space(style.body_gap);
        PanelHeaderResponse { header, menu, float, close }
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["PanelHeader"];

pub(crate) fn stories() -> Vec<Story> {
    vec![typed_story!(
        "composites/panel-header/flyout",
        Level::Composites,
        "PanelHeader",
        "Flyout",
        FlyoutArgs { float: true, close: true, menu: true, actions: false, width: 280.0 },
        show_flyout
    )]
}

#[derive(StoryArgs, Debug, PartialEq)]
pub(super) struct FlyoutArgs {
    pub(super) float: bool,
    pub(super) close: bool,
    pub(super) menu: bool,
    pub(super) actions: bool,
    #[arg(range = 60.0..=400.0)]
    pub(super) width: f32,
}

/// A flyout's header in a panel `width` wide, with the buttons and an action chosen by args. The
/// line under it shows what the host got.
pub(super) fn show_flyout(ui: &mut Ui, args: &mut FlyoutArgs) {
    let (width, actions) = (args.width.min(ui.available_width()), args.actions);
    let header = PanelHeader::new("brush-settings", "Brush settings").menu(args.menu).float(args.float).close(args.close);
    ui.allocate_ui(vec2(width, 0.0), |ui| {
        ui.set_max_width(width);
        let shown = header.show(ui, |ui| {
            if actions {
                ui.add(IconButton::new(Icon::Plus, "Add preset").size(ControlSize::Sm));
            }
        });
        let dragged = shown.header.dragged();
        let mut text = if dragged { "dragging the header" } else { "drag the header to move the panel" }.to_owned();
        for (name, response) in [("menu", &shown.menu), ("float", &shown.float), ("close", &shown.close)] {
            if response.as_ref().is_some_and(Response::clicked) {
                text = format!("clicked {name}");
            }
        }
        let id = ui.scope_id().with("panel-header-last");
        if text.starts_with("clicked") {
            ui.data_mut(|d| d.insert_temp(id, text.clone()));
        }
        let shown_text = ui.data(|d| d.get_temp::<String>(id)).filter(|_| !dragged).unwrap_or(text);
        ui.label(egui::RichText::new(shown_text).weak());
    });
}
