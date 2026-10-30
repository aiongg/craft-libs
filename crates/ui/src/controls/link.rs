//! [`Link`]: text in the accent colour that opens a URL or reports a click.

use craft_theme::{Contrast, TypeRole};
use egui::accesskit::Role;
use egui::emath::GuiRounding as _;
use egui::text::LayoutJob;
use egui::{AsIdSalt, Color32, CursorIcon, IdSalt, OpenUrl, Response, Stroke, Ui, Widget};

use super::salt;
use crate::primitives::{Content, Corner, Gap, Interactive, Stack, Text};
use crate::story::{Level, Story, StoryArgs};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens, role_format};
use crate::typed_story;

/// A link's look, from [`Link::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct LinkStyle {
    /// `accent.solid`, as PhotoCraft's links; `accent.text` with high contrast, which meets the
    /// text contrast `accent.solid` doesn't.
    pub color: Color32,
    /// The underline, always drawn (WCAG 1.4.1): one point in the link's colour.
    pub stroke: Stroke,
}

/// A link: `ui.add(Link::new("Release notes").url("https://…"))`.
///
/// Drawn in the accent colour in a type role (body by default) and underlined, so colour isn't
/// its only cue. With a
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
        let color = if tokens.appearance.contrast == Contrast::High { c.accent.text } else { c.accent.solid }.to_egui();
        LinkStyle { color, stroke: Stroke::new(craft_theme::Stroke::W1.width(1.0), color) }
    }
}

impl Widget for Link {
    fn ui(self, ui: &mut Ui) -> Response {
        let tokens = Tokens::of(ui);
        let style = Link::style(&tokens);
        let mut format = role_format(&tokens, self.role, style.color);
        format.valign = ui.text_valign();
        format.underline = style.stroke;
        let galley = ui.painter().layout_job(LayoutJob::single_section(self.label.clone(), format));
        let response = Interactive::salted(salt(ui, self.id_salt), Role::Link, self.label.as_str())
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
            ui.painter().galley(response.rect.min.round_to_pixels(ui.pixels_per_point()), galley, style.color);
        }
        response
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Link"];

#[derive(StoryArgs, Debug, PartialEq)]
struct LinkArgs {
    label: String,
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!("controls/link/inline", Level::Controls, "Link", "Inline", LinkArgs { label: "release notes".into() }, show_inline),
        typed_story!("controls/link/alone", Level::Controls, "Link", "Alone", LinkArgs { label: "Show all shortcuts".into() }, show_alone),
    ]
}

/// A link inside a wrapping paragraph.
fn show_inline(ui: &mut Ui, args: &mut LinkArgs) {
    let label = args.label.clone();
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.add(Text::new("This version changes how layers are saved. Read the "));
        ui.add(Link::new(label).url("https://github.com/storytold/craft-libs/releases").id_salt("link-inline"));
        ui.add(Text::new(" before you open older documents."));
    });
}

/// Links on their own, one reporting clicks, one in a smaller role.
fn show_alone(ui: &mut Ui, args: &mut LinkArgs) {
    let state_id = ui.scope_id().with("link-alone-count");
    let mut count = ui.data(|d| d.get_temp::<u32>(state_id).unwrap_or_default());
    Stack::new(Gap::Stack).show(ui, |ui| {
        if ui.add(Link::new(args.label.as_str()).id_salt("link-alone")).clicked() {
            count += 1;
        }
        ui.add(Link::new("Privacy policy").role(TypeRole::Caption).url("https://example.com/privacy").id_salt("link-caption"));
        ui.add(Text::new(format!("Clicked {count} times")).content(Content::Muted));
    });
    ui.data_mut(|d| d.insert_temp(state_id, count));
}
