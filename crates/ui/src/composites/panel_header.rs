//! [`PanelHeader`]: the title row of a flyout or floating panel.

use craft_theme::{ControlSize, TypeRole};
use egui::accesskit::Role;
use egui::{Color32, Label, Rect, Response, Sense, Ui, UiBuilder, WidgetInfo, vec2};

use crate::controls::IconButton;
use crate::direction;
use crate::icons::Icon;
use crate::primitives::Content;
use crate::story::{Args, Level, Story};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens, role_format};

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
    /// The title's type role: `title`.
    pub title_role: TypeRole,
    /// The title: `text.normal`.
    pub title: Color32,
}

/// What [`PanelHeader::show`] shows: the header's background and its buttons.
#[derive(Debug)]
pub struct PanelHeaderResponse {
    /// The background, which senses drags: the host starts a move or a tear-off from it.
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
/// button. It is `panel_header.height` high.
///
/// ```
/// use craft_ui::composites::PanelHeader;
/// # egui::__run_test_ui(|ui| {
/// let header = PanelHeader::new("Layers").float(true).close(true).show(ui, |_| {});
/// if header.close.is_some_and(|close| close.clicked()) { /* close the panel */ }
/// # });
/// ```
///
/// - The title is cut with an ellipsis and shows its full text as a tooltip when it does not fit.
/// - The buttons are icon buttons named "Panel menu", "Float panel" and "Close panel" (the host
///   passes translated names with the `*_label` methods). The menu button is on by default; the
///   float and close buttons are off until asked for.
/// - The background is not a Tab stop. The host gets its drag from
///   [`PanelHeaderResponse::header`]: moving and tearing off stay with the docking code.
/// - Docked panels keep the dock area's tab strip, not this header.
/// - **AccessKit:** the background is a `Header` named by the title; the buttons are named by
///   their labels.
#[must_use = "show the header with `show`"]
pub struct PanelHeader {
    title: String,
    menu: Option<String>,
    float: Option<String>,
    close: Option<String>,
    id_salt: egui::IdSalt,
}

impl PanelHeader {
    /// A header titled `title`, which is also its id.
    pub fn new(title: impl Into<String>) -> Self {
        let title = title.into();
        Self { id_salt: egui::IdSalt::new(&title), title, menu: Some("Panel menu".into()), float: None, close: None }
    }

    /// Keeps the header's state when its title changes, or tells two headers of one title apart.
    pub fn id_salt(mut self, id_salt: impl egui::AsIdSalt) -> Self {
        self.id_salt = egui::IdSalt::new(id_salt);
        self
    }

    /// Whether the panel menu button is shown; on by default.
    pub fn menu(mut self, menu: bool) -> Self {
        self.menu = menu.then(|| self.menu.unwrap_or_else(|| "Panel menu".into()));
        self
    }

    /// Shows the panel menu button with this accessible name.
    pub fn menu_label(mut self, label: impl Into<String>) -> Self {
        self.menu = Some(label.into());
        self
    }

    /// Whether the float button is shown.
    pub fn float(mut self, float: bool) -> Self {
        self.float = float.then(|| "Float panel".into());
        self
    }

    /// Shows the float button with this accessible name.
    pub fn float_label(mut self, label: impl Into<String>) -> Self {
        self.float = Some(label.into());
        self
    }

    /// Whether the close button is shown.
    pub fn close(mut self, close: bool) -> Self {
        self.close = close.then(|| "Close panel".into());
        self
    }

    /// Shows the close button with this accessible name.
    pub fn close_label(mut self, label: impl Into<String>) -> Self {
        self.close = Some(label.into());
        self
    }

    pub fn style(tokens: &UiTokens) -> PanelHeaderStyle {
        let p = &tokens.component.panel_header;
        PanelHeaderStyle {
            height: p.height,
            pad: p.pad,
            gap: p.gap,
            button: p.button,
            title_role: TypeRole::Title,
            title: Content::Normal.color(tokens).to_egui(),
        }
    }

    /// Shows the header across the available width. `add_actions` adds widgets between the title
    /// and the menu button; it runs in the trailing layout, so the first widget it adds is nearest
    /// the menu button.
    pub fn show(self, ui: &mut Ui, add_actions: impl FnOnce(&mut Ui)) -> PanelHeaderResponse {
        let tokens = Tokens::of(ui);
        let style = PanelHeader::style(&tokens);
        let (_, rect) = ui.allocate_space(vec2(ui.available_width(), style.height));
        let id = ui.make_persistent_id(self.id_salt);
        let header = ui.interact(rect, id, Sense::drag());
        header.widget_info(|| WidgetInfo::labeled(Role::Header, ui.is_enabled(), &self.title));

        // The title is placed once the trailing side's width is known, in a `Ui` made first so
        // that AccessKit lists it first.
        let inner = rect.shrink2(vec2(style.pad, 0.0));
        let mut title_ui = ui.new_child(UiBuilder::new().max_rect(inner));
        let mut trailing = ui.new_child(UiBuilder::new().max_rect(inner).layout(direction::trailing(ui)));
        trailing.spacing_mut().item_spacing.x = style.gap;
        let button = |ui: &mut Ui, icon: Icon, label: &Option<String>| {
            label.as_ref().map(|label| ui.add(IconButton::new(icon, label.as_str()).size(ControlSize::Sm).id_salt((id, label.as_str()))))
        };
        let close = button(&mut trailing, Icon::Close, &self.close);
        let float = button(&mut trailing, Icon::Float, &self.float);
        let menu = button(&mut trailing, Icon::More, &self.menu);
        add_actions(&mut trailing);

        let rtl = direction::is_rtl(ui);
        let used = trailing.min_rect();
        let end = if used.width() > 0.0 && used.width() < inner.width() {
            if rtl { used.right() + style.gap } else { used.left() - style.gap }
        } else if rtl {
            inner.left()
        } else {
            inner.right()
        };
        let start = direction::start_x(ui, inner);
        let band = Rect::from_x_y_ranges(start.min(end)..=start.max(end), rect.y_range());
        if band.width() > 0.0 {
            let format = role_format(&tokens, style.title_role, style.title);
            title_ui.scope_builder(UiBuilder::new().max_rect(band).layout(direction::row(ui)), |ui| {
                ui.add(Label::new(egui::text::LayoutJob::single_section(self.title.clone(), format)).truncate().selectable(false));
            });
        }
        PanelHeaderResponse { header, menu, float, close }
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["PanelHeader"];

pub(crate) fn stories() -> Vec<Story> {
    let args = || {
        Args::new()
            .with_bool("float", true)
            .with_bool("close", true)
            .with_bool("menu", true)
            .with_bool("actions", false)
            .with_number("width", 280.0, 60.0, 400.0)
    };
    vec![Story { id: "composites/panel-header/flyout", level: Level::Composites, component: "PanelHeader", name: "Flyout", args, show: show_flyout }]
}

/// A flyout's header in a panel `width` wide, with the buttons and an action chosen by args. The
/// line under it shows what the host got.
pub(super) fn show_flyout(ui: &mut Ui, args: &mut Args) {
    let (width, actions) = (args.number("width") as f32, args.bool("actions"));
    let header = PanelHeader::new("Brush settings").menu(args.bool("menu")).float(args.bool("float")).close(args.bool("close"));
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
