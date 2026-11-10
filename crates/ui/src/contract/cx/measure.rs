//! `MeasureCx`: step 5's read view of a component.

use crate::contract::{Component, Resolved, Show, ValueOf};
use crate::primitives::{TextEnv, TextLayout, TextRun};
use crate::runtime::commands::{self, CommandRef, ResolvedCommand};
use crate::runtime::env::{EnvView, Label};
use crate::style::{ComponentStyle as _, Ink, UiTokens};
use craft_behavior::Proposal;
use egui::{Id, Ui, Vec2};
use std::fmt::Debug;
use std::hash::Hash;

/// What `measure` reads: step 5's view of the environment, the style, the resolved props and the shown value.
#[non_exhaustive]
pub struct MeasureCx<'a, C: Component> {
    /// What the inherited context holds that component code may read.
    pub env: EnvView<'a>,
    /// The component's cached style.
    pub style: &'a C::Style,
    /// `Common` after step 1.
    pub resolved: &'a Resolved<'a>,
    /// The size the parent proposes.
    pub proposal: Proposal,
    /// The width the parent offers the content: the parent's room less the component's padding, which the pipeline
    /// adds to what `measure` returns.
    pub room: Option<f32>,
    pub(crate) value: Option<&'a ValueOf<C>>,
    pub(crate) ui: &'a Ui,
    pub(crate) tokens: &'a UiTokens,
    pub(crate) ink: Ink,
    pub(crate) id: Id,
    pub(crate) enabled_size: (bool, craft_theme::ControlSize),
}

impl<'a, C: Component> MeasureCx<'a, C> {
    /// The shown value; it has the stage's lifetime, so a borrow of `cx` is not held.
    pub fn value(&self) -> Option<&'a ValueOf<C>> {
        self.value
    }
}

impl<C: Component> MeasureCx<'_, C> {
    /// The bound command as step 1 resolved it, with the scoped bindings of the scopes above the component.
    pub fn command(&self) -> Option<&ResolvedCommand> {
        self.resolved.command
    }

    /// `command` as this component shows it: the label, icon and chord with the scoped bindings of the scopes above
    /// the component, and the enabled state of its instance. `None` when the command is scoped and no instance of
    /// its scope is above the component.
    pub fn resolve_command(&self, command: impl Into<CommandRef>) -> Option<ResolvedCommand> {
        commands::resolve_scoped(self.ui.ctx(), self.env.command_chain(), &command.into())
    }

    /// The label as given.
    pub fn label(&self) -> Label {
        self.resolved.label_value.clone()
    }

    /// Lays `run` out in the content's room, as `Draw::text` will paint it.
    pub fn layout(&self, run: &TextRun<'_>) -> TextLayout {
        self.layout_at(run, self.room.unwrap_or(f32::INFINITY))
    }

    /// Lays `run` out `width` wide, for a text that paints in a band narrower than the room.
    pub fn layout_at(&self, run: &TextRun<'_>, width: f32) -> TextLayout {
        let env = TextEnv {
            tokens: self.tokens,
            direction: self.env.text_direction(),
            ink: crate::style::ink_color(self.tokens, self.ink),
            fonts: crate::fonts::stack_facts(self.ui.ctx()),
        };
        run.layout(self.ui.ctx(), &env, width)
    }

    /// Lays document `text` out `text.wrap` wide, as `ContentPainterCx::layout` will paint it: the galley a content
    /// painter's measure sizes its band by is the one its paint draws.
    pub fn layout_doc(&self, text: &crate::primitives::DocText<'_>) -> TextLayout {
        text.layout(self.ui.ctx(), self.tokens, craft_i18n::Direction::Ltr)
    }

    /// Measures a child under the composite's resolved `enabled` and `size`, as `PaintUi::child` will show it.
    pub fn child<D: Show>(&self, key: impl Hash + Debug, child: &mut D, proposal: Proposal) -> Vec2 {
        let id = self.id.with(key);
        crate::contract::measure_under(child, self.ui, Some(id), proposal, Some(self.enabled_size))
    }

    /// The size `face` has when `PaintUi::face` shows it as the look of a part: steps 1, 2 and 4 of the face run
    /// under the composite's resolved `enabled` and `size`. A composite measured at its least measures the face at
    /// its least; otherwise the face has no proposal.
    pub fn face<S: Show>(&self, mut face: S) -> Vec2 {
        let id = self.id.with("face");
        let proposal = if self.proposal == Proposal::Min { Proposal::Min } else { Proposal::Ideal };
        crate::contract::measure_under(&mut face, self.ui, Some(id), proposal, Some(self.enabled_size))
    }

    /// The icon side of the component's size.
    pub fn icon(&self) -> f32 {
        self.env.control(self.resolved.size).icon
    }

    /// The gap between the component's items.
    pub fn gap(&self) -> f32 {
        self.style.metrics().gap
    }
}
