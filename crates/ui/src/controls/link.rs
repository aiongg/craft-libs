//! [`Link`]: text in the accent colour that opens a URL or reports a click.

use std::sync::Arc;

use craft_theme::{TypeRole, contrast_ratio};
use egui::accesskit::Role;
use egui::emath::GuiRounding as _;
use egui::text::LayoutJob;
use egui::{AsIdSalt, Color32, CursorIcon, IdSalt, OpenUrl, Response, Stroke, Ui, Widget};

use super::salt;
use crate::primitives::{Content, Corner, Gap, Interactive, Stack, Text};
use crate::story::{Args, Level, Story};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens, role_format};

/// The contrast between link text and the text around it below which colour alone doesn't set a
/// link apart, so it is always underlined (WCAG 1.4.1, technique G183).
pub const DISTINCT_FROM_TEXT: f64 = 3.0;

/// When a link is underlined.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LinkUnderline {
    /// Under the pointer: `accent.text` stands out from `text.normal` by colour.
    Hover,
    /// Always: `accent.text` is below [`DISTINCT_FROM_TEXT`] against `text.normal`.
    Always,
}

impl LinkUnderline {
    /// The name in dumps, e.g. `"hover"`.
    pub const fn name(self) -> &'static str {
        match self {
            LinkUnderline::Hover => "hover",
            LinkUnderline::Always => "always",
        }
    }
}

/// A link's look, from [`Link::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct LinkStyle {
    /// `accent.text`.
    pub color: Color32,
    pub underline: LinkUnderline,
    /// The underline: one point in the link's colour.
    pub stroke: Stroke,
}

/// A link: `ui.add(Link::new("Release notes").url("https://…"))`.
///
/// Drawn in `accent.text` in a type role (body by default), underlined on hover, and always when
/// `accent.text` is too close to `text.normal` to tell apart by colour. With a
/// [`url`](Link::url) a click opens it (in a new tab with a modifier held or a middle click, on
/// the web); without one the caller acts on [`clicked`](Response::clicked). AccessKit role
/// `Link`, with the URL. Enter and Space activate it, as a link in the suite's apps does.
#[derive(Clone, Debug)]
pub struct Link {
    label: String,
    url: Option<String>,
    role: TypeRole,
    id_salt: Option<IdSalt>,
}

impl Link {
    pub fn new(label: impl Into<String>) -> Self {
        Self { label: label.into(), url: None, role: TypeRole::Body, id_salt: None }
    }

    /// The URL a click opens.
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// The type role, to match the text around it; [`TypeRole::Body`] by default.
    pub fn role(mut self, role: TypeRole) -> Self {
        self.role = role;
        self
    }

    /// Keeps focus when the layout around it changes; else the `Ui`'s next automatic id.
    pub fn id_salt(mut self, id_salt: impl AsIdSalt) -> Self {
        self.id_salt = Some(IdSalt::new(id_salt));
        self
    }

    pub fn style(tokens: &UiTokens) -> LinkStyle {
        let c = &tokens.color;
        let underline = if contrast_ratio(c.accent.text, c.text.normal) < DISTINCT_FROM_TEXT { LinkUnderline::Always } else { LinkUnderline::Hover };
        let color = Content::Accent.color(tokens).to_egui();
        LinkStyle { color, underline, stroke: Stroke::new(craft_theme::Stroke::W1.width(1.0), color) }
    }
}

impl Widget for Link {
    fn ui(self, ui: &mut Ui) -> Response {
        let tokens = Tokens::of(ui);
        let style = Link::style(&tokens);
        let format = role_format(&tokens, self.role, style.color);
        let galley = ui.painter().layout_job(LayoutJob::single_section(self.label.clone(), format.clone()));
        let response = Interactive::new(salt(ui, self.id_salt), Role::Link, self.label.as_str())
            .state_layer(false)
            .corner(Corner::Control)
            .show(ui, galley.size())
            .on_hover_cursor(CursorIcon::PointingHand);
        if let Some(url) = &self.url {
            ui.ctx().accesskit_node_builder(response.id, |node| node.set_url(url.as_str()));
            let new_tab = response.middle_clicked() || (response.clicked() && ui.input(|i| i.modifiers.any()));
            if response.clicked() || response.middle_clicked() {
                ui.ctx().open_url(OpenUrl { url: url.clone(), new_tab });
            }
        }
        if ui.is_rect_visible(response.rect) {
            let underlined = style.underline == LinkUnderline::Always || response.hovered();
            let galley = if underlined {
                let mut format = format;
                format.underline = style.stroke;
                ui.painter().layout_job(LayoutJob::single_section(self.label, format))
            } else {
                galley
            };
            ui.painter().galley(response.rect.min.round_to_pixels(ui.pixels_per_point()), Arc::clone(&galley), style.color);
        }
        response
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Link"];

pub(crate) fn stories() -> Vec<Story> {
    vec![
        Story {
            id: "controls/link/inline",
            level: Level::Controls,
            component: "Link",
            name: "Inline",
            args: || Args::new().with_text("label", "release notes"),
            show: show_inline,
        },
        Story {
            id: "controls/link/alone",
            level: Level::Controls,
            component: "Link",
            name: "Alone",
            args: || Args::new().with_text("label", "Show all shortcuts"),
            show: show_alone,
        },
    ]
}

/// A link inside a wrapping paragraph.
fn show_inline(ui: &mut Ui, args: &mut Args) {
    let label = args.text("label").to_owned();
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.add(Text::new("This version changes how layers are saved. Read the "));
        ui.add(Link::new(label).url("https://github.com/storytold/craft-libs/releases").id_salt("link-inline"));
        ui.add(Text::new(" before you open older documents."));
    });
}

/// Links on their own, one reporting clicks, one in a smaller role.
fn show_alone(ui: &mut Ui, args: &mut Args) {
    let state_id = ui.scope_id().with("link-alone-count");
    let mut count = ui.data(|d| d.get_temp::<u32>(state_id).unwrap_or_default());
    Stack::new(Gap::Stack).show(ui, |ui| {
        if ui.add(Link::new(args.text("label")).id_salt("link-alone")).clicked() {
            count += 1;
        }
        ui.add(Link::new("Privacy policy").role(TypeRole::Caption).url("https://example.com/privacy").id_salt("link-caption"));
        ui.add(Text::new(format!("Clicked {count} times")).content(Content::Muted));
    });
    ui.data_mut(|d| d.insert_temp(state_id, count));
}
