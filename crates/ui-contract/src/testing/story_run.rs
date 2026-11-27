//! A story shown on a [`Harness`]: the state the harness's view runs over (the story's data and what each build output),
//! the constructors that show a story or a viewer's story body, and the purity probe.

use super::story::{Story, StoryData, StoryOutput};
use crate::contract::rt::harness::{Harness, HarnessBuilder};
use crate::contract::{Id, Ui};
use crate::runtime::env::{Overrides, with_env};
use craft_ui_rt::record::RecordingRenderer;
use craft_ui_rt::{Renderer, Vec2};

/// The room of a test, named by the kind of host the component sits in. A test that needs another size gives it to
/// [`Harness::room`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Room {
    /// A docked side panel.
    Panel,
    /// A dialog's window.
    Dialog,
    /// An application window.
    Window,
}

impl Room {
    /// The room's size in points.
    pub fn size(self) -> Vec2 {
        match self {
            Room::Panel => Vec2::new(360.0, 600.0),
            Room::Dialog => Vec2::new(800.0, 600.0),
            Room::Window => Vec2::new(1120.0, 720.0),
        }
    }
}

/// A build in which a story's drawing code changed state it must leave alone, as a harness under
/// [`Harness::probe_purity`] sees it: in a rebuild with no input ([`Harness::rerun_check`]), where nothing may change.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Impure {
    /// The build, from 1.
    pub pass: usize,
    /// Whether it happened in a measuring run (`true`) or in the real run of a rebuild (`false`).
    pub measuring: bool,
    /// The story values that changed.
    pub changed: Vec<&'static str>,
    /// The effects (commands, announcements, toasts, requests) a measuring run skipped. Empty for a rebuild.
    pub effects: Vec<&'static str>,
}

/// The state a story's harness runs its view over.
#[derive(Default)]
pub struct StoryRun {
    data: StoryData,
    outputs: Vec<StoryOutput>,
    /// The id of the latest output, which stays when the outputs are taken.
    root_id: Option<Id>,
    builds: usize,
    probed: bool,
    impure: Vec<Impure>,
}

/// A harness over a story's [`StoryRun`].
pub type StoryHarness<R = RecordingRenderer> = Harness<StoryRun, R>;

/// One `show` of a story, inside the env scope when there is one.
fn shown(ui: &mut Ui, scope: Option<fn(&mut Overrides)>, show: fn(&mut Ui, &mut StoryData) -> StoryOutput, data: &mut StoryData) -> StoryOutput {
    match scope {
        Some(scope) => with_env(ui, scope, |ui| show(ui, data)),
        None => show(ui, data),
    }
}

impl<R: Renderer> Harness<StoryRun, R> {
    /// A harness that shows `story` with `builder`'s size and look on `renderer`, inside an env scope `scope` changes
    /// (disabled, read-only, or both) when there is one. Its data and outputs are read with [`Harness::data`] and
    /// [`Harness::take_outputs`].
    pub fn story_on(builder: HarnessBuilder, renderer: R, story: &Story, scope: Option<fn(&mut Overrides)>) -> Self {
        let show = story.show;
        let run = StoryRun { data: StoryData::of(story), ..StoryRun::default() };
        builder.build_with(
            renderer,
            move |ui, run: &mut StoryRun| {
                run.builds += 1;
                let output = shown(ui, scope, show, &mut run.data);
                run.root_id = output.id;
                run.outputs.push(output);
            },
            run,
        )
    }

    /// A harness that shows a story through `show`, which hosts it and keeps its values: a viewer's story body in its
    /// shell. `show` returns the story's output in a build that ran the story, which [`Harness::take_outputs`] and
    /// [`Harness::root_id`] then give as for [`Harness::story`], and `None` in a build that did not. The values live
    /// with the host, so [`Harness::data`] is empty.
    pub fn hosted_on(builder: HarnessBuilder, renderer: R, mut show: impl FnMut(&mut Ui) -> Option<StoryOutput> + 'static) -> Self {
        builder.build_with(
            renderer,
            move |ui, run: &mut StoryRun| {
                run.builds += 1;
                if let Some(output) = show(ui) {
                    run.root_id = output.id;
                    run.outputs.push(output);
                }
            },
            StoryRun::default(),
        )
    }

    /// The story's data, to read or change: the view builds again in the next step.
    pub fn data(&mut self) -> &mut StoryData {
        &mut self.state_mut().data
    }

    /// The outputs the story gave since the last call, one per build.
    pub fn take_outputs(&mut self) -> Vec<StoryOutput> {
        std::mem::take(&mut self.state_mut_quiet().outputs)
    }

    /// The component id of the latest shown output, kept when the outputs are taken.
    pub fn root_id(&self) -> Option<Id> {
        self.state().root_id
    }

    /// How many times the story's view has built.
    pub fn builds(&self) -> usize {
        self.state().builds
    }

    /// The rect the shown component took in the window.
    pub fn shown(&self) -> Option<craft_ui_rt::Rect> {
        self.rect_of(self.root_id()?)
    }

    /// Turns the purity probe on: [`Harness::rerun_check`] then compares the story's values around a rebuild. Set before
    /// the first step.
    pub fn probe_purity(&mut self) -> &mut Self {
        self.state_mut_quiet().probed = true;
        self
    }

    /// Under [`Harness::probe_purity`]: builds the story again with no input and records an [`Impure`] when a value changed.
    /// Without the probe it only steps.
    pub fn rerun_check(&mut self) -> &mut Self {
        let probed = self.state().probed;
        let before = probed.then(|| self.state().data.snapshot());
        self.rebuild().step();
        if let Some(before) = before {
            let (changed, pass) = (self.state().data.changed_values_since(&before), self.state().builds);
            if !changed.is_empty() {
                self.state_mut_quiet().impure.push(Impure { pass, measuring: false, changed, effects: Vec::new() });
            }
        }
        self
    }

    /// The impure builds seen so far under [`Harness::probe_purity`]; see [`Impure`].
    pub fn impure(&self) -> Vec<Impure> {
        self.state().impure.clone()
    }

    /// The room of a host kind: the same as [`Harness::room`] with that kind's size.
    pub fn in_room(&mut self, room: Room) -> &mut Self {
        self.room(room.size())
    }
}

impl Harness<StoryRun, RecordingRenderer> {
    /// A harness that shows `story` in a room of 400 by 300 points on the recording renderer.
    pub fn story(story: &Story) -> Self {
        Self::story_on(Harness::builder(), RecordingRenderer::default(), story, None)
    }

    /// [`Harness::story`] inside an env scope `scope` changes: disabled, read-only, or both.
    pub fn story_scoped(story: &Story, scope: fn(&mut Overrides)) -> Self {
        Self::story_on(Harness::builder(), RecordingRenderer::default(), story, Some(scope))
    }

    /// [`Harness::hosted_on`] on the recording renderer.
    pub fn hosted(show: impl FnMut(&mut Ui) -> Option<StoryOutput> + 'static) -> Self {
        Self::hosted_on(Harness::builder(), RecordingRenderer::default(), show)
    }
}

impl<S, R: Renderer> Harness<S, R> {
    /// Gives the app-scope command states the shown stories set ([`Story::command_states`]), as an app's states do.
    /// A panic when the harness has no command board.
    pub fn command_states(&mut self, states: &super::story::StoryCommands) -> &mut Self {
        for (command, state) in states.iter() {
            self.command_state(command.clone(), state.clone());
        }
        self
    }
}
