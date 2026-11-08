//! `ChordRecorder`: the focusable field that records shortcuts. It shows the text it was given, starts recording on
//! a click or Enter, and while recording reports the chords of the pass. It never decides what a chord means.
//!
//! A component's key use is fixed, so the recorder is two components over one look: the idle field with the plain
//! widget keys (Tab leaves, Enter and Space start recording) and the recording field with `KeyUse::RECORDER`
//! (every key is a chord, Escape included). Both share the field's id, so focus stays on it when recording starts.

use craft_behavior::{Metrics, Sense};
use craft_keymap::Chord;
use craft_theme::Stateful;
use craft_ui::contract::{
    Access, Common, Component, Cx, Focus as _, MeasureCx, Name, NoValue, Outcome, PaintUi, Pointer as _, Props as _, Role, Show as _, Styled as _,
};
use craft_ui::primitives::{TextRun, Wrap};
use craft_ui::runtime::env::Label;
use craft_ui::runtime::focus::KeyUse;
use craft_ui::style::{ComponentStyle, FocusLook, Ink, StyleCx, SurfaceKind, SurfaceLook, TextLook, TypeRole};
use egui::{Rect, Response, Ui, Vec2};

#[cfg(feature = "screen-testing")]
mod stories;

/// What the recorder reports in one pass.
#[derive(Debug)]
#[non_exhaustive]
pub(crate) struct RecorderOutput {
    /// The field's response.
    pub response: Response,
    /// The chords pressed this pass while recording, in order.
    pub chords: Vec<Chord>,
    /// A click or Enter asked to start recording.
    pub started: bool,
}

impl Outcome for RecorderOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// The style of the field: a well, the text in the normal ink and an accent well while recording.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub(crate) struct FieldStyle {
    #[style(metrics)]
    metrics: Metrics,
    #[style(surface)]
    surface: SurfaceLook,
    #[style(focus)]
    focus: FocusLook,
    #[style(text)]
    text: TextLook,
    /// Around the text.
    pad: f32,
}

fn field_style(cx: &StyleCx<'_>, recording: bool) -> FieldStyle {
    let well = cx.surface(SurfaceKind::Well);
    let fill = if recording { cx.tone_fill(craft_theme::variant::Tone::Accent) } else { cx.color(cx.tokens.color.surface.sunken) };
    let control = cx.control();
    FieldStyle {
        metrics: Metrics::new().min_size(Vec2::splat(control.height)),
        surface: SurfaceLook::new(Stateful::new(fill)).kind(SurfaceKind::Well).radius(well.radius),
        focus: cx.focus_look(),
        text: TextLook::new(TypeRole::Label, Stateful::new(cx.ink(Ink::OnPanel))),
        pad: cx.tokens.space.pad.chip,
    }
}

macro_rules! field {
    ($name:ident, $keys:expr, $recording:expr) => {
        #[derive(craft_ui::contract::Component)]
        struct $name {
            #[common]
            common: Common,
            shown: Label,
            chords: Vec<Chord>,
            started: bool,
        }

        impl Component for $name {
            const ROLE: Role = Role::BUTTON;
            const OVERLAYS: bool = false;
            const KEYS: KeyUse = $keys;
            type Style = FieldStyle;
            type Output = RecorderOutput;

            fn style(cx: &StyleCx<'_>, _: Self::StyleKey) -> FieldStyle {
                field_style(cx, $recording)
            }

            fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
                let text = cx.layout(&TextRun::new(self.shown.as_str()).role(cx.style.text.role).wrap(Wrap::Line)).size();
                text + Vec2::splat(cx.style.pad * 2.0)
            }

            fn sense(&self) -> Sense {
                Sense::CLICK | Sense::FOCUS
            }

            fn update(&mut self, cx: &mut Cx<'_, Self>) {
                if $recording {
                    self.chords.extend(cx.intents().take_chords());
                } else if cx.activation().is_some() {
                    self.started = true;
                }
            }

            fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
                access.name(Name::Text(cx.resolved.label.to_owned()));
                access.value(self.shown.as_str().to_owned());
            }

            fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
                let run = TextRun::new(self.shown.as_str()).role(cx.style.text.role).color(cx.resolve(&cx.style.text.color)).wrap(Wrap::Elide);
                ui.draw().text(cx.content_rect, &run);
            }

            fn finish(self, cx: Cx<'_, Self>) -> RecorderOutput {
                RecorderOutput { response: cx.response.clone(), chords: self.chords, started: self.started }
            }
        }
    };
}

field!(IdleField, KeyUse::of(craft_behavior::Input::Pointer), false);
field!(RecordingField, KeyUse::RECORDER, true);

/// The field that records: its accessible name, whether it records now and the text it shows (the recording
/// prompt, or the chord).
pub(crate) struct ChordRecorder {
    label: Label,
    recording: bool,
    shown: Label,
    salt: Option<&'static str>,
}

impl ChordRecorder {
    /// A recorder named `label` showing `shown`.
    pub(crate) fn new(label: impl Into<Label>, recording: bool, shown: impl Into<Label>) -> Self {
        ChordRecorder { label: label.into(), recording, shown: shown.into(), salt: None }
    }

    /// Salts the field's id.
    #[must_use]
    pub(crate) fn id_salt(mut self, salt: &'static str) -> Self {
        self.salt = Some(salt);
        self
    }

    /// Shows the field.
    pub(crate) fn show(self, ui: &mut Ui) -> RecorderOutput {
        let ChordRecorder { label, recording, shown, salt } = self;
        let common = Common::new();
        if recording {
            let mut field = RecordingField { common, shown, chords: Vec::new(), started: false }.label(label);
            if let Some(salt) = salt {
                field = field.id_salt(salt);
            }
            field.show(ui)
        } else {
            let mut field = IdleField { common, shown, chords: Vec::new(), started: false }.label(label);
            if let Some(salt) = salt {
                field = field.id_salt(salt);
            }
            field.show(ui)
        }
    }

    /// Shows the field as a child of a component's paint at `rect`. Idle and recording share `key`, so focus
    /// stays on the field when recording starts.
    pub(crate) fn child(self, ui: &mut PaintUi<'_>, key: impl std::hash::Hash + std::fmt::Debug, rect: Rect) -> RecorderOutput {
        let ChordRecorder { label, recording, shown, .. } = self;
        let common = Common::new();
        if recording {
            ui.child(key, rect, RecordingField { common, shown, chords: Vec::new(), started: false }.label(label))
        } else {
            ui.child(key, rect, IdleField { common, shown, chords: Vec::new(), started: false }.label(label))
        }
    }
}
