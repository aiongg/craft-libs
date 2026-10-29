//! Notices: an inline status message with the tone's icon and colour, and no fill or border. The
//! family holds each tone's colour and the gap after the icon. [`Tone`] is shared with toasts.

use super::{ComponentField, ContrastPair, Family, FieldKind, OverrideValue, Semantic};
use crate::color::Rgba;
use crate::tokens::{ColorToken, ColorTokens, StatusColors, StatusKind, TokenRef};
use crate::values;

/// What a notice or toast says about its subject.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum Tone {
    /// No status: a plain message.
    #[default]
    Neutral,
    /// Information.
    Info,
    /// Success.
    Good,
    /// Needs attention.
    Warn,
    /// Failure.
    Bad,
}

impl Tone {
    /// Every tone, in path order.
    pub const ALL: [Tone; 5] = [Tone::Neutral, Tone::Info, Tone::Good, Tone::Warn, Tone::Bad];

    /// The path segment, e.g. `"good"`.
    pub const fn name(self) -> &'static str {
        match self {
            Tone::Neutral => "neutral",
            Tone::Info => "info",
            Tone::Good => "good",
            Tone::Warn => "warn",
            Tone::Bad => "bad",
        }
    }

    /// The tone whose [`name`](Tone::name) is `name`.
    pub fn from_name(name: &str) -> Option<Tone> {
        Tone::ALL.into_iter().find(|t| t.name() == name)
    }

    /// The status the tone reads its colours from; `None` for neutral.
    pub const fn status(self) -> Option<StatusKind> {
        match self {
            Tone::Neutral => None,
            Tone::Info => Some(StatusKind::Info),
            Tone::Good => Some(StatusKind::Good),
            Tone::Warn => Some(StatusKind::Warn),
            Tone::Bad => Some(StatusKind::Bad),
        }
    }

    /// The tone's status colours; `None` for neutral.
    pub(super) fn colors(self, c: &ColorTokens) -> Option<&StatusColors> {
        self.status().map(|s| c.status.get(s))
    }
}

/// Notice tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct NoticeTokens {
    /// Between the icon and the text: `space.gap.inline`.
    pub icon_gap: f32,
    pub(crate) fg: [Rgba; Tone::ALL.len()],
}

impl NoticeTokens {
    /// Icon, title and message of `tone`: `status.<tone>.fg`; neutral is `text.muted`.
    pub fn fg(&self, tone: Tone) -> Rgba {
        self.fg[tone as usize]
    }
}

/// An overridable notice field. Path: `<tone>.fg`, e.g. `notice.warn.fg`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NoticeField {
    /// `notice.<tone>.fg`
    Fg(Tone),
}

impl Family for NoticeTokens {
    type Field = NoticeField;
    const FIELDS: &'static [NoticeField] = &[
        NoticeField::Fg(Tone::Neutral),
        NoticeField::Fg(Tone::Info),
        NoticeField::Fg(Tone::Good),
        NoticeField::Fg(Tone::Warn),
        NoticeField::Fg(Tone::Bad),
    ];

    fn defaults(s: &Semantic<'_>) -> NoticeTokens {
        NoticeTokens { icon_gap: s.space.gap.inline, fg: Tone::ALL.map(|t| t.colors(s.color).map_or(s.color.text.muted, |c| c.fg)) }
    }

    fn path(f: NoticeField) -> String {
        let NoticeField::Fg(tone) = f;
        format!("{}.fg", tone.name())
    }

    fn from_path(path: &str) -> Option<NoticeField> {
        let (tone, part) = path.split_once('.')?;
        if part != "fg" {
            return None;
        }
        Tone::from_name(tone).map(NoticeField::Fg)
    }

    fn kind(_: NoticeField) -> FieldKind {
        FieldKind::Color
    }

    fn get(&self, f: NoticeField) -> OverrideValue {
        let NoticeField::Fg(tone) = f;
        OverrideValue::Color(self.fg(tone))
    }

    fn set(&mut self, f: NoticeField, v: OverrideValue) {
        let (NoticeField::Fg(tone), OverrideValue::Color(c)) = (f, v) else { return };
        self.fg[tone as usize] = c;
    }

    /// The text is drawn on the panel and on raised surfaces (dialogs, popups).
    fn pairs() -> Vec<ContrastPair> {
        Tone::ALL
            .into_iter()
            .flat_map(|tone| {
                [ColorToken::SurfacePanel, ColorToken::SurfaceRaised].map(|bg| ContrastPair {
                    fg: TokenRef::Component(ComponentField::Notice(NoticeField::Fg(tone))),
                    bg: TokenRef::Color(bg),
                    minimum: values::TEXT_MIN,
                })
            })
            .collect()
    }
}
