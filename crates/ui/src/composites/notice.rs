//! [`Notice`]: an inline status message.

use craft_theme::{Tone, TypeRole};
use egui::accesskit::Role;
use egui::text::LayoutJob;
use egui::{Align, AsIdSalt, Color32, IdSalt, Label, Response, Ui, UiBuilder, vec2};

use super::live;
use crate::direction;
use crate::icons::{self, IconSize};
use crate::story::{Level, Story, StoryArgs};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens, role_format};
use crate::typed_story;

/// A notice's sizes, type roles and colours, from [`Notice::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct NoticeStyle {
    /// Between the icon and the text: `notice.icon_gap`.
    pub icon_gap: f32,
    /// Side of the tone icon: `icon.sm`.
    pub icon: f32,
    /// The title's type role: `body_strong`.
    pub title_role: TypeRole,
    /// The message's type role: `caption`.
    pub message_role: TypeRole,
    /// Icon and title, and the message when there is no title: `notice.<tone>.fg`.
    pub fg: Color32,
    /// The message under a title: `text.muted`, as the bullet lines of a PhotoCraft notice.
    pub body: Color32,
}

/// An inline status message in a dialog or a panel: the tone's icon at the reading start, an
/// optional title in the tone's colour, and the message in the tone's colour, or in `text.muted`
/// under a title. It has no fill and no border.
///
/// ```
/// use craft_ui::composites::{Notice, Tone};
/// # egui::__run_test_ui(|ui| {
/// Notice::new("ICO is limited to 256 × 256 pixels").tone(Tone::Warn).show(ui);
/// Notice::new("Applies at the next launch.").title("Colour management").announce(false).show(ui);
/// # });
/// ```
///
/// - Neutral has no icon and reads `text.muted`; the other tones show their icon (`info`,
///   `success`, `warning`, `error`) and read `status.<tone>.fg` (PhotoCraft's `warning` and
///   `danger`). The text wraps and can be selected.
/// - Not interactive, not a Tab stop.
/// - **AccessKit:** a live region by default, as WCAG 4.1.3 asks of status messages that appear
///   after an action: `Status` and polite, or for [`Tone::Bad`] `Alert` and assertive, named by
///   the title and message. [`announce`](Notice::announce)`(false)` makes it a plain `Note`
///   for permanent text, which would otherwise be read every time its panel appears. The icon is
///   decorative and has no node. Give a notice drawn after widgets that come and go an
///   [`id_salt`](Notice::id_salt), so it keeps its node and is not announced again.
#[must_use = "show the notice with `show`"]
pub struct Notice {
    message: String,
    title: Option<String>,
    tone: Tone,
    announce: bool,
    id_salt: Option<IdSalt>,
}

impl Notice {
    /// An informational notice saying `message`.
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into(), title: None, tone: Tone::Info, announce: true, id_salt: None }
    }

    /// A title in the strong type role above the message.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into()).filter(|t| !t.is_empty());
        self
    }

    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    /// Whether a screen reader announces the notice when it appears; on by default.
    pub fn announce(mut self, announce: bool) -> Self {
        self.announce = announce;
        self
    }

    /// Keeps the live region's node when widgets before the notice come and go, so it is not
    /// announced again; else the `Ui`'s next automatic id.
    pub fn id_salt(mut self, id_salt: impl AsIdSalt) -> Self {
        self.id_salt = Some(IdSalt::new(id_salt));
        self
    }

    pub fn style(tokens: &UiTokens, tone: Tone) -> NoticeStyle {
        let n = &tokens.component.notice;
        NoticeStyle {
            icon_gap: n.icon_gap,
            icon: IconSize::Sm.points(tokens),
            title_role: TypeRole::BodyStrong,
            message_role: TypeRole::Caption,
            fg: n.fg(tone).to_egui(),
            body: tokens.color.text.muted.to_egui(),
        }
    }

    /// Shows the notice across the available width.
    pub fn show(self, ui: &mut Ui) -> Response {
        let tokens = Tokens::of(ui);
        let style = Notice::style(&tokens, self.tone);
        let body = |ui: &mut Ui, labels: &mut live::Labels| {
            let first_line = tokens.typography.get(self.title.as_ref().map_or(style.message_role, |_| style.title_role)).line_height;
            let mut row = direction::row(ui);
            row.cross_align = Align::Min;
            ui.scope_builder(UiBuilder::new().layout(row), |ui| {
                ui.spacing_mut().item_spacing.x = style.icon_gap;
                tone_icon(ui, self.tone, IconSize::Sm, first_line, style.fg);
                let width = ui.available_width();
                ui.allocate_ui_with_layout(vec2(width, 0.0), direction::column(ui), |ui| {
                    ui.set_min_width(width);
                    ui.spacing_mut().item_spacing.y = 0.0;
                    if let Some(title) = &self.title {
                        text(ui, &tokens, style.title_role, style.fg, title);
                        labels.add(title.as_str());
                    }
                    let message = if self.title.is_some() { style.body } else { style.fg };
                    text(ui, &tokens, style.message_role, message, &self.message);
                    labels.add(self.message.as_str());
                });
            })
        };
        if self.announce {
            let (role, politeness) = live::for_tone(self.tone);
            let salt = crate::controls::salt(ui, self.id_salt);
            let id = ui.make_persistent_id(salt);
            live::region(ui, id, role, politeness, body).response
        } else {
            let mut ignored = live::Labels::default();
            ui.scope_builder(UiBuilder::new().accessibility_role(Role::Note), |ui| body(ui, &mut ignored)).response
        }
    }
}

/// Wrapping, selectable text in `role` and `color`.
pub(super) fn text(ui: &mut Ui, tokens: &UiTokens, role: TypeRole, color: Color32, text: &str) -> Response {
    ui.add(Label::new(LayoutJob::single_section(text.to_owned(), role_format(tokens, role, color))).wrap())
}

/// The icon of `tone` in a box of the icon's width and `line_height`, centred on the line. It has
/// no AccessKit node. Neutral has none: nothing is allocated.
pub(super) fn tone_icon(ui: &mut Ui, tone: Tone, size: IconSize, line_height: f32, color: Color32) {
    let icon = match tone {
        Tone::Neutral => return,
        Tone::Info => icons::Icon::Info,
        Tone::Good => icons::Icon::Success,
        Tone::Warn => icons::Icon::Warning,
        Tone::Bad => icons::Icon::Error,
    };
    let points = size.points(&Tokens::of(ui));
    let (_, rect) = ui.allocate_space(vec2(points, line_height.max(points)));
    if ui.is_rect_visible(rect) {
        icons::paint(ui, rect, icon, size, color);
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Notice"];

pub(crate) fn stories() -> Vec<Story> {
    vec![typed_story!(
        "composites/notice/tones",
        Level::Composites,
        "Notice",
        "Tones",
        TonesArgs { announce: true, title: true, width: 320.0 },
        show_tones
    )]
}

#[derive(StoryArgs, Debug, PartialEq)]
pub(super) struct TonesArgs {
    announce: bool,
    title: bool,
    #[arg(range = 120.0..=480.0)]
    width: f32,
}

/// The five tones on the panel and on a dialog surface.
pub(super) fn show_tones(ui: &mut Ui, args: &mut TonesArgs) {
    let (width, announce, title) = (args.width.min(ui.available_width()), args.announce, args.title);
    for (surface, name) in [(crate::primitives::SurfaceKind::Panel, "Panel"), (crate::primitives::SurfaceKind::Dialog, "Dialog")] {
        ui.add(crate::primitives::Text::new(name).role(TypeRole::Label).content(crate::primitives::Content::Muted));
        ui.allocate_ui(vec2(width, 0.0), |ui| {
            ui.set_max_width(width);
            let pad = crate::theme::Tokens::of(ui).space.pad.dialog;
            crate::primitives::Surface::new(surface).show(ui, |ui| {
                ui.set_width((width - 2.0 * pad).max(0.0));
                crate::primitives::Stack::new(crate::primitives::Gap::Stack).pad(crate::primitives::Pad::Dialog).show(ui, |ui| {
                    for tone in Tone::ALL {
                        let mut notice = Notice::new(format!("{} message that wraps onto a second line when the room runs out.", tone.name()))
                            .tone(tone)
                            .announce(announce);
                        if title {
                            notice = notice.title(tone.name());
                        }
                        notice.show(ui);
                    }
                });
            });
        });
    }
}
