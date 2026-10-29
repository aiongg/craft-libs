//! [`Button`]: push buttons, and the face (icon, label, spinner) toggle buttons share.

use std::sync::Arc;

use craft_theme::{ButtonVariant, ControlSize, States, TypeRole};
use egui::accesskit::Role;
use egui::emath::GuiRounding as _;
use egui::text::{LayoutJob, TextWrapping};
use egui::{AsIdSalt, Color32, CornerRadius, Galley, IdSalt, Rect, Response, Sense, Ui, Vec2, Widget, pos2, vec2};

use super::{SIZE_NAMES, Spinner, TooltipExt as _, icon_size, salt, size_arg};
use crate::direction;
use crate::icons::{self, Icon};
use crate::primitives::{Corner, Interactive, Row, Stack, StateColors, StateLayer, SurfaceKind, Text};
use crate::story::{Args, Level, Story};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens, role_format};

/// A push button's sizes and colours, from [`Button::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ButtonStyle {
    /// `button.height`.
    pub height: f32,
    /// At the start and end: `button.pad_x`.
    pub pad_x: f32,
    /// Between icon and label: `button.gap`.
    pub gap: f32,
    /// Side of the icon: `button.icon`.
    pub icon: f32,
    /// `button.label`.
    pub label: TypeRole,
    /// `Corner::Button`: `button.radius`.
    pub radius: CornerRadius,
    /// Raised, or none for a ghost button, which rests transparent with no edge.
    pub surface: Option<SurfaceKind>,
    /// `button.<variant>.fill`.
    pub fill: States<Color32>,
    /// `button.<variant>.border`, over a flat surface's border; a bevel keeps its colours.
    pub border: States<Color32>,
    /// `button.<variant>.text`: label, icon and spinner.
    pub text: States<Color32>,
}

impl ButtonStyle {
    /// The fill and border, for [`Interactive::colors`].
    pub fn colors(&self) -> StateColors {
        StateColors { fill: self.fill, border: Some(self.border) }
    }
}

/// A push button: `ui.add(Button::new("Export").variant(ButtonVariant::Primary))`.
///
/// Colours come from `button.<variant>.*`; primary, secondary and danger buttons sit on a raised
/// [`Surface`](crate::primitives::Surface), so a bevel theme bevels them and they press in, and
/// ghost buttons rest transparent. A leading [`icon`](Button::icon) sits at the reading start and
/// mirrors in a right-to-left interface when directional. A label wider than the room left is
/// cut with an ellipsis and shown whole in a tooltip.
///
/// [`loading`](Button::loading) shows a spinner in place of the icon (or of the label, without
/// an icon) at the same width, ignores clicks and Enter, keeps the focus, and reports the button
/// busy.
#[derive(Clone, Debug)]
pub struct Button {
    label: String,
    variant: ButtonVariant,
    size: ControlSize,
    icon: Option<Icon>,
    loading: bool,
    enabled: bool,
    min_width: f32,
    id_salt: Option<IdSalt>,
}

impl Button {
    /// A secondary, standard-size button; `label` is also its accessible name.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            variant: ButtonVariant::Secondary,
            size: ControlSize::Md,
            icon: None,
            loading: false,
            enabled: true,
            min_width: 0.0,
            id_salt: None,
        }
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// A leading icon.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Working: a spinner, no activation, AccessKit busy.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Whether it takes input; a disabled button draws `button.<variant>.*.disabled`.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// The least width in points, at most the room the `Ui` has; the face stays centred. For
    /// buttons that share a width.
    pub fn min_width(mut self, width: f32) -> Self {
        self.min_width = width;
        self
    }

    /// The width the button takes in a `Ui` with room to spare, before [`min_width`](Button::min_width).
    pub fn natural_width(&self, ui: &Ui) -> f32 {
        let tokens = Tokens::of(ui);
        let style = Button::style(&tokens, self.variant, self.size);
        let face = Face { label: &self.label, icon: self.icon, size: self.size, loading: self.loading };
        face.natural_width(ui, &tokens, &style)
    }

    /// Keeps focus and animations when the layout around it changes; else the `Ui`'s next
    /// automatic id.
    pub fn id_salt(mut self, id_salt: impl AsIdSalt) -> Self {
        self.id_salt = Some(IdSalt::new(id_salt));
        self
    }

    pub fn style(tokens: &UiTokens, variant: ButtonVariant, size: ControlSize) -> ButtonStyle {
        let b = &tokens.component.button;
        let colors = b.colors(variant);
        ButtonStyle {
            height: b.height.get(size),
            pad_x: b.pad_x.get(size),
            gap: b.gap,
            icon: b.icon.get(size),
            label: b.label,
            radius: Corner::Button.radius(tokens),
            surface: (variant != ButtonVariant::Ghost).then_some(SurfaceKind::Raised),
            fill: colors.fill.to_egui(),
            border: colors.border.to_egui(),
            text: colors.text.to_egui(),
        }
    }
}

impl Widget for Button {
    fn ui(self, ui: &mut Ui) -> Response {
        let tokens = Tokens::of(ui);
        let style = Button::style(&tokens, self.variant, self.size);
        let face = Face { label: &self.label, icon: self.icon, size: self.size, loading: self.loading };
        let mut laid = face.layout(ui, &tokens, &style);
        laid.size.x = laid.size.x.max(self.min_width.min(ui.available_width()));
        let mut element = Interactive::salted(salt(ui, self.id_salt), Role::Button, self.label.as_str())
            .colors(style.colors())
            .corner(Corner::Button)
            .enabled(self.enabled)
            .busy(self.loading);
        if let Some(kind) = style.surface {
            element = element.surface(kind);
        }
        if self.loading {
            element = element.sense(Sense::FOCUSABLE).state_layer(false);
        }
        let response = element.show(ui, laid.size);
        let color = content_color(ui, &style.text, &response, !self.loading);
        face.paint(ui, response.rect, &laid, &style, color);
        laid.tooltip(response, &self.label)
    }
}

/// `colors` for `response` now: animated with the state layer, or still at rest (disabled when
/// disabled).
pub(crate) fn content_color(ui: &Ui, colors: &States<Color32>, response: &Response, animated: bool) -> Color32 {
    match (animated, response.enabled()) {
        (true, _) => StateLayer::color(ui, colors, response),
        (false, true) => colors.rest,
        (false, false) => colors.disabled,
    }
}

/// What a push or toggle button shows: an optional leading icon (or spinner) and a label.
pub(crate) struct Face<'a> {
    pub label: &'a str,
    pub icon: Option<Icon>,
    pub size: ControlSize,
    pub loading: bool,
}

/// A face laid out in the room the `Ui` has.
pub(crate) struct Laid {
    galley: Arc<Galley>,
    /// The button's size.
    pub size: Vec2,
    truncated: bool,
}

impl Laid {
    /// `response` with the whole label in a tooltip when it was cut.
    pub(crate) fn tooltip(&self, response: Response, label: &str) -> Response {
        if self.truncated { response.tooltip(label) } else { response }
    }
}

impl Face<'_> {
    /// The face's width with all the room it needs.
    pub(crate) fn natural_width(&self, ui: &Ui, tokens: &UiTokens, style: &ButtonStyle) -> f32 {
        self.fixed(style) + self.label(ui, tokens, style, TextWrapping::no_max_width()).size().x
    }

    pub(crate) fn layout(&self, ui: &Ui, tokens: &UiTokens, style: &ButtonStyle) -> Laid {
        let fixed = self.fixed(style);
        let full = self.label(ui, tokens, style, TextWrapping::no_max_width());
        let room = (ui.available_width() - fixed).max(0.0);
        let (galley, truncated) =
            if full.size().x > room { (self.label(ui, tokens, style, TextWrapping::truncate_at_width(room)), true) } else { (full, false) };
        Laid { size: vec2(fixed + galley.size().x, style.height), galley, truncated }
    }

    /// The padding and the icon with its gap: the width that is not the label.
    fn fixed(&self, style: &ButtonStyle) -> f32 {
        2.0 * style.pad_x + if self.icon.is_some() { style.icon + style.gap } else { 0.0 }
    }

    /// The label laid out with `wrap`.
    fn label(&self, ui: &Ui, tokens: &UiTokens, style: &ButtonStyle, wrap: TextWrapping) -> Arc<Galley> {
        // The text colour depends on the state, known only after the interaction: painted later.
        let mut job = LayoutJob::single_section(self.label.to_owned(), role_format(tokens, style.label, Color32::PLACEHOLDER));
        job.wrap = wrap;
        ui.painter().layout_job(job)
    }

    /// Paints the face centred in `rect`, icon at the reading start.
    pub(crate) fn paint(&self, ui: &Ui, rect: Rect, laid: &Laid, style: &ButtonStyle, color: Color32) {
        if !ui.is_rect_visible(rect) {
            return;
        }
        let ppp = ui.pixels_per_point();
        let text = laid.galley.size();
        let icon = vec2(style.icon, style.icon);
        let spinner = self.loading.then(|| icon_size(self.size).stroke());
        let Some(lead) = self.icon else {
            match spinner {
                // Without an icon the spinner takes the label's place; the width stays.
                Some(stroke) => Spinner::paint(ui, Rect::from_center_size(rect.center(), icon), stroke, color),
                None => paint_galley(ui, &laid.galley, rect.center() - text / 2.0, color),
            }
            return;
        };
        let content = style.icon + style.gap + text.x;
        let start = rect.center().x - content / 2.0;
        let (icon_x, text_x) = if direction::is_rtl(ui) { (start + content - style.icon, start) } else { (start, start + style.icon + style.gap) };
        let icon_rect = Rect::from_min_size(pos2(icon_x, rect.center().y - style.icon / 2.0), icon).round_to_pixels(ppp);
        match spinner {
            Some(stroke) => Spinner::paint(ui, icon_rect, stroke, color),
            None => icons::paint(ui, icon_rect, lead, icon_size(self.size), color),
        }
        paint_galley(ui, &laid.galley, pos2(text_x, rect.center().y - text.y / 2.0), color);
    }
}

fn paint_galley(ui: &Ui, galley: &Arc<Galley>, pos: egui::Pos2, color: Color32) {
    ui.painter().galley(pos.round_to_pixels(ui.pixels_per_point()), Arc::clone(galley), color);
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Button"];

const VARIANT_NAMES: &[&str] = &["primary", "secondary", "ghost", "danger"];

const ICON_NAMES: &[&str] = &["none", "save", "chevron_right"];

pub(crate) fn variant_arg(name: &str) -> ButtonVariant {
    ButtonVariant::ALL.into_iter().find(|v| v.name() == name).unwrap_or(ButtonVariant::Secondary)
}

pub(crate) fn icon_arg(name: &str) -> Option<Icon> {
    Icon::ALL.into_iter().find(|i| i.name() == name)
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        Story {
            id: "controls/button/variants",
            level: Level::Controls,
            component: "Button",
            name: "Variants",
            args: || Args::new().with_text("label", "Button").with_choice("icon", ICON_NAMES, 0),
            show: show_variants,
        },
        Story {
            id: "controls/button/playground",
            level: Level::Controls,
            component: "Button",
            name: "Playground",
            args: || {
                Args::new()
                    .with_choice("variant", VARIANT_NAMES, 0)
                    .with_choice("size", SIZE_NAMES, 1)
                    .with_choice("icon", ICON_NAMES, 1)
                    .with_bool("disabled", false)
                    .with_bool("loading", false)
                    .with_bool("narrow", false)
                    .with_text("label", "Save the document as a copy")
            },
            show: show_playground,
        },
    ]
}

/// Every variant at each size (rows) in the rest, disabled and loading states.
fn show_variants(ui: &mut Ui, args: &mut Args) {
    let label = args.text("label").to_owned();
    let icon = icon_arg(args.choice("icon"));
    Stack::new(crate::primitives::Gap::Stack).show(ui, |ui| {
        for variant in ButtonVariant::ALL {
            for size in super::SIZES {
                let size_name = super::size_name(size);
                Row::new(crate::primitives::Gap::Inline).show(ui, |ui| {
                    ui.add(Text::new(format!("{} {size_name}", variant.name())).content(crate::primitives::Content::Muted));
                    for (state, enabled, loading) in [("rest", true, false), ("disabled", false, false), ("loading", true, true)] {
                        let mut button = Button::new(label.as_str()).variant(variant).size(size).enabled(enabled).loading(loading).id_salt((
                            "button-variants",
                            variant.name(),
                            size_name,
                            state,
                        ));
                        if let Some(icon) = icon {
                            button = button.icon(icon);
                        }
                        ui.add(button);
                    }
                });
            }
        }
    });
}

/// One button with every property in the args; clicks count up.
fn show_playground(ui: &mut Ui, args: &mut Args) {
    let state_id = ui.scope_id().with("button-playground-count");
    let mut count = ui.data(|d| d.get_temp::<u32>(state_id).unwrap_or_default());
    let mut button = Button::new(args.text("label"))
        .variant(variant_arg(args.choice("variant")))
        .size(size_arg(args.choice("size")))
        .enabled(!args.bool("disabled"))
        .loading(args.bool("loading"))
        .id_salt("button-playground");
    if let Some(icon) = icon_arg(args.choice("icon")) {
        button = button.icon(icon);
    }
    let width = if args.bool("narrow") { 120.0 } else { ui.available_width() };
    ui.allocate_ui(vec2(width, 0.0), |ui| {
        ui.set_max_width(width);
        if ui.add(button).clicked() {
            count += 1;
        }
    });
    ui.data_mut(|d| d.insert_temp(state_id, count));
    ui.add(Text::new(format!("Clicked {count} times")).content(crate::primitives::Content::Muted));
}
