//! [`Text`]: text in a type role and a content colour.

use craft_theme::TypeRole;
use egui::text::{LayoutJob, TextFormat};
use egui::{Label, Response, Role, Sense, Ui, Widget, WidgetInfo};

use super::{Content, ContentStoryArgs};
use crate::story::NoArgs;
use crate::story::{Level, Story, StoryArgs, StoryChoice};
use crate::text::elide::{ElideAt, elide_path, elided_galley, text_width};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens, role_format};
use crate::typed_story;

/// Text in a type role ([`TypeRole::Body`] by default) and a [`Content`] colour
/// ([`Content::Normal`] by default): `ui.add(Text::new("Layers").role(TypeRole::Title))`.
///
/// It is an egui [`Label`], so it wraps as labels do, can be selected, and its AccessKit node
/// carries the text. With [`truncate`](Text::truncate) it stays on one line and ends in an
/// ellipsis when it doesn't fit; the tooltip and the AccessKit node keep the full text.
///
/// [`elide`](Text::elide) and [`elide_path`](Text::elide_path) shorten it elsewhere than at the
/// end, by craft-ui's [`text::elide`](crate::text::elide()) rules; the tooltip and AccessKit node
/// keep the whole text there too:
///
/// ```ignore
/// ui.add(Text::new(file_name).elide(ElideAt::Middle));     // "IMG_2024…_final.psd"
/// ui.add(Text::new(folder).elide_path().max_width(240.0)); // "…/Pictures/Library"
/// ```
#[derive(Clone, Debug)]
pub struct Text {
    text: String,
    role: TypeRole,
    content: Content,
    truncate: bool,
    elide: Option<Elision>,
    max_width: Option<f32>,
    italic: bool,
}

/// How [`Text`] shortens what doesn't fit, beyond egui's own end truncation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Elision {
    At(ElideAt),
    Path,
}

impl Text {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), role: TypeRole::Body, content: Content::Normal, truncate: false, elide: None, max_width: None, italic: false }
    }

    pub fn role(mut self, role: TypeRole) -> Self {
        self.role = role;
        self
    }

    pub fn content(mut self, content: Content) -> Self {
        self.content = content;
        self
    }

    /// Slanted: an empty-list line.
    pub fn italic(mut self) -> Self {
        self.italic = true;
        self
    }

    /// One line, cut with an ellipsis when too long.
    pub fn truncate(mut self) -> Self {
        self.truncate = true;
        self
    }

    /// One line, shortened at `at` when too long ([`ElideAt::End`] keeps a right-to-left name's
    /// logical start, where egui's [`truncate`](Text::truncate) cuts at the visual right).
    pub fn elide(mut self, at: ElideAt) -> Self {
        self.elide = Some(Elision::At(at));
        self
    }

    /// One line, a path shortened by dropping leading folders first
    /// ([`elide_path`](crate::text::elide_path)).
    pub fn elide_path(mut self) -> Self {
        self.elide = Some(Elision::Path);
        self
    }

    /// At most this wide, even when the row has more room: a label that wraps at that width, or,
    /// with an elision or [`truncate`](Self::truncate), the one line it shortens to. Leaving room for what follows on the line is
    /// `.max_width(ui.available_width() - after)`.
    pub fn max_width(mut self, width: f32) -> Self {
        self.max_width = Some(width);
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
        format.italics = self.italic;
        if self.elide.is_some() || (self.max_width.is_some() && self.truncate) {
            return self.show_elided(ui, format);
        }
        if let Some(width) = self.max_width {
            // A wrapping label with a width cap: it wraps at the cap and stays selectable.
            return ui
                .scope(|ui| {
                    ui.set_max_width(width.min(ui.available_width()).max(1.0));
                    ui.add(Label::new(LayoutJob::single_section(self.text, format)))
                })
                .inner;
        }
        let label = Label::new(LayoutJob::single_section(self.text, format));
        ui.add(if self.truncate { label.truncate() } else { label })
    }
}

impl Text {
    /// One line no wider than `max_width` (else the room left), laid out with
    /// [`elided_galley`](crate::text::elided_galley) (or the shortened path), drawn as a label
    /// whose hover text and AccessKit node are the whole text when it was shortened. With `truncate` and
    /// a width but no elision, an end elision inside that width.
    // pc: crates/ui-egui/src/widgets/fields.rs:26 elided_label_sensed
    // lightcraft: crates/ui-egui/src/dialogs/settings/widgets.rs:31 path_label
    fn show_elided(self, ui: &mut Ui, format: TextFormat) -> Response {
        let room = ui.available_width();
        let max_width = self.max_width.map_or(room, |w| w.min(room)).max(1.0);
        let color = format.color;
        let font = format.font_id.clone();
        let (shown, at) = match self.elide {
            Some(Elision::Path) => (elide_path(&self.text, max_width, |s| text_width(ui, s, &font)), ElideAt::End),
            Some(Elision::At(at)) => (std::borrow::Cow::Borrowed(self.text.as_str()), at),
            None => (std::borrow::Cow::Borrowed(self.text.as_str()), ElideAt::End),
        };
        let shortened = shown != self.text;
        let laid_out = elided_galley(ui, &shown, format, max_width, at);
        let (rect, response) = ui.allocate_exact_size(laid_out.galley.size(), Sense::hover());
        ui.painter().galley(rect.min, laid_out.galley, color);
        response.widget_info(|| WidgetInfo::labeled(Role::Label, ui.is_enabled(), &self.text));
        if shortened || laid_out.elided { response.on_hover_text(&self.text) } else { response }
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Text"];

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum RoleArg {
    Caption,
    Label,
    Body,
    #[choice(name = "body_strong")]
    BodyStrong,
    Title,
    Heading,
    #[choice(name = "body_large")]
    BodyLarge,
    Mono,
}

impl RoleArg {
    fn role(self) -> TypeRole {
        match self {
            RoleArg::Caption => TypeRole::Caption,
            RoleArg::Label => TypeRole::Label,
            RoleArg::Body => TypeRole::Body,
            RoleArg::BodyStrong => TypeRole::BodyStrong,
            RoleArg::Title => TypeRole::Title,
            RoleArg::Heading => TypeRole::Heading,
            RoleArg::BodyLarge => TypeRole::BodyLarge,
            RoleArg::Mono => TypeRole::Mono,
        }
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
struct TruncateArgs {
    role: RoleArg,
    text: String,
    #[arg(range = 40.0..=400.0)]
    width: f32,
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!("primitives/text/roles", Level::Primitives, "Text", "Roles", ContentStoryArgs::default(), show_roles),
        typed_story!("primitives/text/contents", Level::Primitives, "Text", "Contents", NoArgs {}, show_contents),
        typed_story!(
            "primitives/text/truncate",
            Level::Primitives,
            "Text",
            "Truncate",
            TruncateArgs {
                role: RoleArg::Body,
                text: "A long layer name that does not fit in the panel and ends in an ellipsis".into(),
                width: 160.0,
            },
            show_truncate
        ),
        typed_story!(
            "primitives/text/elide",
            Level::Primitives,
            "Text",
            "Elide",
            ElideArgs { text: "IMG_2024_summer_holiday_final_edit.psd".into(), width: 180.0 },
            show_elide
        ),
    ]
}

/// Every role in the chosen content colour.
fn show_roles(ui: &mut Ui, args: &mut ContentStoryArgs) {
    let content = args.content.content();
    crate::story::variant_list(ui, "text-roles", |form| {
        for index in 0..RoleArg::OPTIONS.len() {
            let (role, name) = (RoleArg::from_index(index).role(), RoleArg::OPTIONS[index]);
            crate::story::variant_row(form, name, |ui| ui.add(Text::new("The quick brown fox").role(role).content(content)));
        }
    });
}

/// Every content colour on the surface it is meant for.
fn show_contents(ui: &mut Ui, _: &mut NoArgs) {
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

#[derive(StoryArgs, Debug, PartialEq)]
struct ElideArgs {
    text: String,
    #[arg(range = 40.0..=400.0)]
    width: f32,
}

/// One line each, shortened at the end, the middle and the start, a path by dropping leading
/// folders, and a right-to-left name that keeps its logical start.
fn show_elide(ui: &mut Ui, args: &mut ElideArgs) {
    let width = args.width.min(ui.available_width());
    let path = "/home/ada/Pictures/2026/summer holiday/IMG_2024_summer_holiday_final_edit.psd";
    let right_to_left = "שם קובץ ארוך מאוד שלא נכנס בשורה אחת של הפאנל.psd";
    use crate::localized::Labels as _;
    let words = crate::story::StoryLabels::of(ui);
    crate::story::variant_list(ui, "text-elide", |form| {
        for (name, at) in [(&words.end, ElideAt::End), (&words.middle, ElideAt::Middle), (&words.start, ElideAt::Start)] {
            crate::story::variant_row(form, name, |ui| ui.add(Text::new(args.text.as_str()).elide(at).max_width(width)));
        }
        crate::story::variant_row(form, &words.elide_path, |ui| ui.add(Text::new(path).elide_path().max_width(width)));
        crate::story::variant_row(form, &words.elide_rtl, |ui| ui.add(Text::new(right_to_left).elide(ElideAt::End).max_width(width)));
    });
}

fn show_truncate(ui: &mut Ui, args: &mut TruncateArgs) {
    let width = args.width.min(ui.available_width());
    let text = Text::new(args.text.as_str()).role(args.role.role()).truncate();
    ui.allocate_ui(egui::vec2(width, 0.0), |ui| {
        ui.set_max_width(width);
        ui.add(text);
    });
}
