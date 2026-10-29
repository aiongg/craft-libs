//! [`Text`]: text in a type role and a content colour.

use craft_theme::TypeRole;
use egui::text::{LayoutJob, TextFormat};
use egui::{Label, Response, Ui, Widget};

use super::{CONTENT_NAMES, Content};
use crate::story::{Args, Level, Story};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens, role_format};

/// Text in a type role ([`TypeRole::Body`] by default) and a [`Content`] colour
/// ([`Content::Normal`] by default): `ui.add(Text::new("Layers").role(TypeRole::Title))`.
///
/// It is an egui [`Label`], so it wraps as labels do, can be selected, and its AccessKit node
/// carries the text. With [`truncate`](Text::truncate) it stays on one line and ends in an
/// ellipsis when it doesn't fit; the tooltip and the AccessKit node keep the full text.
#[derive(Clone, Debug)]
pub struct Text {
    text: String,
    role: TypeRole,
    content: Content,
    truncate: bool,
}

impl Text {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), role: TypeRole::Body, content: Content::Normal, truncate: false }
    }

    pub fn role(mut self, role: TypeRole) -> Self {
        self.role = role;
        self
    }

    pub fn content(mut self, content: Content) -> Self {
        self.content = content;
        self
    }

    /// One line, cut with an ellipsis when too long.
    pub fn truncate(mut self) -> Self {
        self.truncate = true;
        self
    }

    /// The format of `role` in `content`: the role's font and line height, the content colour.
    pub fn format(tokens: &UiTokens, role: TypeRole, content: Content) -> TextFormat {
        role_format(tokens, role, content.color(tokens).to_egui())
    }
}

impl Widget for Text {
    fn ui(self, ui: &mut Ui) -> Response {
        let mut format = Text::format(&Tokens::of(ui), self.role, self.content);
        // A label job keeps its own vertical alignment (the bottom, by default) instead of the
        // `Ui`'s, so in a row taller than the line it would sit below the centred widgets
        // beside it, such as a link.
        format.valign = ui.text_valign();
        let label = Label::new(LayoutJob::single_section(self.text, format));
        ui.add(if self.truncate { label.truncate() } else { label })
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Text"];

const ROLE_NAMES: &[&str] = &["caption", "label", "body", "body_strong", "title", "heading", "mono"];
const ROLES: [TypeRole; 7] =
    [TypeRole::Caption, TypeRole::Label, TypeRole::Body, TypeRole::BodyStrong, TypeRole::Title, TypeRole::Heading, TypeRole::Mono];

pub(crate) fn stories() -> Vec<Story> {
    vec![
        Story::new("primitives/text/roles", Level::Primitives, "Text", "Roles", || Args::new().with_choice("content", CONTENT_NAMES, 0), show_roles),
        Story::new("primitives/text/contents", Level::Primitives, "Text", "Contents", Args::new, show_contents),
        Story::new(
            "primitives/text/truncate",
            Level::Primitives,
            "Text",
            "Truncate",
            || {
                Args::new()
                    .with_choice("role", ROLE_NAMES, 2)
                    .with_text("text", "A long layer name that does not fit in the panel and ends in an ellipsis")
                    .with_number("width", 160.0, 40.0, 400.0)
            },
            show_truncate,
        ),
    ]
}

fn role_arg(args: &Args) -> TypeRole {
    let name = args.choice("role");
    ROLE_NAMES.iter().position(|n| *n == name).map_or(TypeRole::Body, |i| ROLES[i])
}

/// Every role in the chosen content colour.
fn show_roles(ui: &mut Ui, args: &mut Args) {
    let content = Content::from_name(args.choice("content")).unwrap_or_default();
    crate::story::variant_list(ui, "text-roles", 90.0, |ui| {
        for (role, name) in ROLES.into_iter().zip(ROLE_NAMES) {
            crate::story::variant_row(ui, name, |ui| ui.add(Text::new("The quick brown fox").role(role).content(content)));
        }
    });
}

/// Every content colour on the surface it is meant for.
fn show_contents(ui: &mut Ui, _: &mut Args) {
    let tokens = Tokens::of(ui);
    let c = &tokens.color;
    super::Stack::new(super::Gap::Inline).show(ui, |ui| {
        for &content in Content::ALL {
            let fill = match content {
                Content::OnAccent => Some(c.accent.solid),
                Content::OnStatus => Some(c.status.bad.solid),
                Content::OnSelected => Some(c.state.selected),
                _ => None,
            };
            let text = Text::new(content.name()).role(TypeRole::Label).content(content);
            match fill {
                Some(fill) => {
                    egui::Frame::new().fill(fill.to_egui()).inner_margin(crate::theme::margin(tokens.space.pad.control)).show(ui, |ui| ui.add(text));
                }
                None => {
                    ui.add(text);
                }
            }
        }
    });
}

fn show_truncate(ui: &mut Ui, args: &mut Args) {
    let width = args.number("width") as f32;
    let text = Text::new(args.text("text")).role(role_arg(args)).truncate();
    ui.allocate_ui(egui::vec2(width, 0.0), |ui| {
        ui.set_max_width(width);
        ui.add(text);
    });
}
