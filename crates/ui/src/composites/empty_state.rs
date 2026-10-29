//! [`EmptyState`]: the message of an empty panel, list or canvas.

use craft_theme::{ButtonVariant, TypeRole};
use egui::accesskit::Role;
use egui::text::LayoutJob;
use egui::{Align, AsIdSalt, Color32, Id, IdSalt, Label, Layout, Rect, Response, Sense, Ui, UiBuilder, pos2, vec2};

use super::live;
use crate::controls::Button;
use crate::direction;
use crate::icons::{Icon, IconSize};
use crate::primitives::{Content, Icon as IconWidget};
use crate::story::{Args, Level, Story};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens, role_format};

/// An empty state's sizes, type roles and colours, from [`EmptyState::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct EmptyStateStyle {
    /// Widest the centred column grows: `empty_state.max_width`.
    pub max_width: f32,
    /// Above and below the inline message, and between a centred title and description:
    /// `gap.stack`.
    pub gap: f32,
    /// Before the actions and the footer of a centred empty state: `gap.section`.
    pub section_gap: f32,
    /// The inline message's type role: `body`.
    pub message_role: TypeRole,
    /// The inline message: `empty_state.text`.
    pub message: Color32,
    /// The centred title's type role: `heading`.
    pub title_role: TypeRole,
    /// The centred description's type role: `body`.
    pub description_role: TypeRole,
}

/// What [`EmptyState::show`] shows.
#[derive(Debug)]
pub struct EmptyStateResponse {
    /// The whole empty state.
    pub response: Response,
    /// The responses of the actions, in the order they were added.
    pub actions: Vec<Response>,
}

enum Kind<'a> {
    Inline { status: bool },
    Centred(Box<Centred<'a>>),
}

type Footer<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

#[derive(Default)]
struct Centred<'a> {
    description: Option<String>,
    icon: Option<Icon>,
    actions: Vec<Button>,
    footer: Option<Footer<'a>>,
}

/// What a panel, list or canvas says when it has nothing to show.
///
/// ```
/// use craft_ui::composites::EmptyState;
/// # egui::__run_test_ui(|ui| {
/// EmptyState::new("No document").show(ui);
/// # });
/// ```
///
/// - **Inline** ([`new`](EmptyState::new)): a panel's or list's message, one wrapping text at the
///   reading start with a `gap.stack` above and below, in `empty_state.text`. The wording is the
///   host's: a short fact ("No document"), or the fact and what to do. [`status`](EmptyState::status)
///   is for a list that empties as the user types (a palette, a search): the text is a polite
///   `Status` live region, so "No matching commands" is announced when it appears.
/// - **Centred** ([`centred`](EmptyState::centred)): no document in a canvas or workspace. A column
///   centred in the available rect, at most `empty_state.max_width` wide: an optional icon, the
///   title, a description, actions and a footer for the host's own content. The actions are laid
///   side by side, `gap.section` apart, when they fit at the width of the widest one, and stacked
///   otherwise; the first is primary. The branding, drop hint and background stay with the host.
///   No live region: the state appears with its screen, not after an action.
/// - **AccessKit:** the centred column is a `GenericContainer` named by its title, which is a
///   `Heading`.
#[must_use = "show the empty state with `show`"]
pub struct EmptyState<'a> {
    text: String,
    kind: Kind<'a>,
    id_salt: Option<IdSalt>,
}

impl<'a> EmptyState<'a> {
    /// An inline message.
    pub fn new(message: impl Into<String>) -> Self {
        Self { text: message.into(), kind: Kind::Inline { status: false }, id_salt: None }
    }

    /// A centred empty state with this title.
    pub fn centred(title: impl Into<String>) -> Self {
        Self { text: title.into(), kind: Kind::Centred(Box::default()), id_salt: None }
    }

    /// Announces the inline message when it appears. No effect on a centred state.
    pub fn status(mut self) -> Self {
        if let Kind::Inline { status } = &mut self.kind {
            *status = true;
        }
        self
    }

    /// [`status`](EmptyState::status) when `on`.
    pub fn status_if(self, on: bool) -> Self {
        if on { self.status() } else { self }
    }

    /// A description under the title of a centred state.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        if let Kind::Centred(c) = &mut self.kind {
            c.description = Some(description.into());
        }
        self
    }

    /// An icon above the title of a centred state.
    pub fn icon(mut self, icon: Icon) -> Self {
        if let Kind::Centred(c) = &mut self.kind {
            c.icon = Some(icon);
        }
        self
    }

    /// Adds an action to a centred state. The first is the primary button, the others secondary.
    pub fn action(mut self, button: Button) -> Self {
        if let Kind::Centred(c) = &mut self.kind {
            c.actions.push(button);
        }
        self
    }

    /// The host's own content under the actions of a centred state.
    pub fn footer(mut self, footer: impl FnOnce(&mut Ui) + 'a) -> Self {
        if let Kind::Centred(c) = &mut self.kind {
            c.footer = Some(Box::new(footer));
        }
        self
    }

    /// Keeps the state's own memory (a status message's live region, a centred column's height)
    /// when widgets before it come and go; else the `Ui`'s next automatic id.
    pub fn id_salt(mut self, id_salt: impl AsIdSalt) -> Self {
        self.id_salt = Some(IdSalt::new(id_salt));
        self
    }

    pub fn style(tokens: &UiTokens) -> EmptyStateStyle {
        let e = &tokens.component.empty_state;
        EmptyStateStyle {
            max_width: e.max_width,
            gap: tokens.space.gap.stack,
            section_gap: tokens.space.gap.section,
            message_role: TypeRole::Body,
            message: e.text.to_egui(),
            title_role: TypeRole::Heading,
            description_role: TypeRole::Body,
        }
    }

    pub fn show(self, ui: &mut Ui) -> EmptyStateResponse {
        let tokens = Tokens::of(ui);
        let style = EmptyState::style(&tokens);
        let salt = crate::controls::salt(ui, self.id_salt);
        let id = ui.make_persistent_id(salt);
        match self.kind {
            Kind::Inline { status } => {
                let response = show_inline(ui, &tokens, &style, &self.text, status.then_some(id));
                EmptyStateResponse { response, actions: Vec::new() }
            }
            Kind::Centred(centred) => show_centred(ui, &tokens, &style, &self.text, id, *centred),
        }
    }
}

fn label(tokens: &UiTokens, role: TypeRole, color: Color32, text: &str, halign: Align) -> Label {
    let mut job = LayoutJob::single_section(text.to_owned(), role_format(tokens, role, color));
    job.halign = halign;
    Label::new(job).wrap()
}

/// The inline message; with `status`, in a live region of that id.
fn show_inline(ui: &mut Ui, tokens: &UiTokens, style: &EmptyStateStyle, text: &str, status: Option<Id>) -> Response {
    let column = Layout { cross_justify: false, ..direction::column(ui) };
    ui.scope_builder(UiBuilder::new().layout(column), |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        ui.add_space(style.gap);
        let message = label(tokens, style.message_role, style.message, text, direction::start_align(ui));
        if let Some(id) = status {
            live::region(ui, id, Role::Status, live::Politeness::Polite, |ui, labels| {
                ui.add(message);
                labels.add(text);
            });
        } else {
            ui.add(message);
        }
        ui.add_space(style.gap);
    })
    .response
}

fn show_centred(ui: &mut Ui, tokens: &UiTokens, style: &EmptyStateStyle, title: &str, id: Id, centred: Centred<'_>) -> EmptyStateResponse {
    let area = ui.available_rect_before_wrap();
    let width = area.width().min(style.max_width);
    // Vertical centring needs the column's height: the last pass's, discarded when it changes.
    let height_id = id.with("height");
    let height = ui.data(|d| d.get_temp::<f32>(height_id));
    let top = area.top() + height.map_or(0.0, |h| ((area.height() - h) / 2.0).max(0.0));
    let column = Rect::from_min_size(pos2(area.center().x - width / 2.0, top), vec2(width, (area.bottom() - top).max(0.0)));
    let mut actions = Vec::new();
    let mut title_id = None;
    let inner = ui.scope_builder(
        UiBuilder::new().max_rect(column).layout(Layout::top_down(Align::Center)).accessibility_role(Role::GenericContainer),
        |ui| {
            let id = ui.unique_id();
            ui.spacing_mut().item_spacing.y = 0.0;
            if let Some(icon) = centred.icon {
                ui.add(IconWidget::new(icon).size(IconSize::Lg).content(Content::Faint));
                ui.add_space(style.gap);
            }
            let heading = ui.add(label(tokens, style.title_role, Content::Normal.color(tokens).to_egui(), title, Align::Center));
            title_id = Some((id, heading.id));
            if let Some(description) = &centred.description {
                ui.add_space(style.gap);
                ui.add(label(tokens, style.description_role, Content::Muted.color(tokens).to_egui(), description, Align::Center));
            }
            if !centred.actions.is_empty() {
                ui.add_space(style.section_gap);
                actions = show_actions(ui, style, width, centred.actions);
            }
            if let Some(footer) = centred.footer {
                ui.add_space(style.section_gap);
                footer(ui);
            }
        },
    );
    if let Some((container, heading)) = title_id {
        let ctx = ui.ctx();
        ctx.accesskit_node_builder(heading, |node| {
            // A heading is named by its text, not valued by it as a plain label is.
            node.set_role(Role::Heading);
            node.clear_value();
            node.set_label(title);
        });
        ctx.accesskit_node_builder(container, |node| node.push_labelled_by(heading.accesskit_id()));
    }
    let measured = inner.response.rect.height();
    if height.is_none_or(|h| (h - measured).abs() > 0.5) {
        ui.data_mut(|d| d.insert_temp(height_id, measured));
        ui.ctx().request_discard("empty state height changed");
    }
    let response = ui.allocate_rect(area, Sense::hover());
    EmptyStateResponse { response, actions }
}

/// Side by side when they fit at the widest one's width, else stacked at that width.
fn show_actions(ui: &mut Ui, style: &EmptyStateStyle, width: f32, buttons: Vec<Button>) -> Vec<Response> {
    let common = buttons.iter().map(|b| b.natural_width(ui)).fold(0.0, f32::max);
    let count = buttons.len() as f32;
    let side_by_side = count * common + (count - 1.0) * style.section_gap <= width;
    let block = if side_by_side { count * common + (count - 1.0) * style.section_gap } else { common.min(width) };
    let layout = if side_by_side { direction::row(ui) } else { Layout::top_down(Align::Center) };
    let mut responses = Vec::new();
    ui.allocate_ui_with_layout(vec2(block, 0.0), layout, |ui| {
        let spacing = if side_by_side { &mut ui.spacing_mut().item_spacing.x } else { &mut ui.spacing_mut().item_spacing.y };
        *spacing = if side_by_side { style.section_gap } else { style.gap };
        for (i, button) in buttons.into_iter().enumerate() {
            let variant = if i == 0 { ButtonVariant::Primary } else { ButtonVariant::Secondary };
            responses.push(ui.add(button.variant(variant).min_width(common.min(width))));
        }
    });
    responses
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["EmptyState"];

pub(crate) fn stories() -> Vec<Story> {
    vec![
        Story::new(
            "composites/empty-state/panel",
            Level::Composites,
            "EmptyState",
            "Panel",
            || Args::new().with_text("search", "zzz").with_bool("status", true).with_number("width", 260.0, 100.0, 400.0),
            show_panel,
        ),
        Story::new(
            "composites/empty-state/canvas",
            Level::Composites,
            "EmptyState",
            "Canvas",
            || Args::new().with_number("width", 640.0, 200.0, 800.0).with_number("height", 360.0, 200.0, 600.0),
            show_canvas,
        ),
    ]
}

/// A list filtered by the search text: matches show as rows, none shows the inline empty state.
fn show_panel(ui: &mut Ui, args: &mut Args) {
    const ITEMS: [&str; 3] = ["New layer", "Open file", "Save as"];
    let width = args.number("width") as f32;
    let needle = args.text("search").to_lowercase();
    ui.allocate_ui(vec2(width, 0.0), |ui| {
        ui.set_max_width(width);
        let matches: Vec<&str> = ITEMS.into_iter().filter(|item| item.to_lowercase().contains(&needle)).collect();
        if matches.is_empty() {
            let empty = EmptyState::new("No matching commands");
            empty.status_if(args.bool("status")).show(ui);
        } else {
            for item in matches {
                ui.label(item);
            }
        }
    });
}

/// The no-document screen: title, description, two actions and a footer.
fn show_canvas(ui: &mut Ui, args: &mut Args) {
    let size = vec2(args.number("width") as f32, args.number("height") as f32);
    ui.allocate_ui(size, |ui| {
        ui.set_min_size(size);
        EmptyState::centred("No document")
            .description("Create a new document or open an existing file.")
            .icon(Icon::Image)
            .action(Button::new("New document…"))
            .action(Button::new("Open…"))
            .footer(|ui| {
                ui.label("Drop an image here to open it");
            })
            .show(ui);
    });
}
