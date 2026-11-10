//! One card: a story live on its surface under a header with the component's name and, while the story differs from its
//! start, Reset; the Open command of an Overview card opens the component's tab. The story runs in an embedded shell,
//! so its dialogs open inside the card. [`StoryBody`] is the story part alone, which the component sheet renders too.

use crate::viewer::{CardState, DialogState};
use craft_behavior::IconId;
use craft_behavior::Proposal;
use craft_i18n::Direction;
use craft_theme::{ControlSize, Mode};
use craft_ui::buttons::Button;
use craft_ui::contract::Show as _;
use craft_ui::contract::{CommandSet, Commands as _, Pad, Props as _, ShowContainer as _, Sizing, Surface, ThemeScope};
use craft_ui::layout_components::Stack;
use craft_ui::runtime::env::{BodyMode, ShellCx, WindowKey, with_env};
use craft_ui::sections::{HeaderCommand, HeaderStrength, Panel};
use craft_ui::shell::AppShell;
use craft_ui::style::SurfaceKind;
use craft_ui::testing::{StoryData, StoryOutput, StoryRegistration};

/// A story's show function.
type StoryFn = fn(&mut Ui, &mut StoryData) -> StoryOutput;
use egui::{Id, Rect, Ui, vec2};

/// The header commands of a card.
#[derive(Clone, Copy, Debug, PartialEq, Eq, CommandSet)]
#[command(prefix = "galleryCard", scope = "gallery.card")]
#[non_exhaustive]
pub enum CardCommands {
    /// Open the component's tab.
    #[command(label = "Open", icon = "open")]
    Open,
    /// Give the story its start values.
    #[command(label = "Reset", icon = "undo")]
    Reset,
    /// Use the card's value, in a Variants view.
    #[command(label = "Use", icon = "check")]
    Use,
}

/// The appearance values a card sets for its story beyond the window's.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct Look {
    /// The mode the story's tokens resolve in.
    pub mode: Option<Mode>,
    /// The layout direction.
    pub direction: Option<Direction>,
    /// The control size.
    pub size: Option<ControlSize>,
}

/// What one card shows.
#[non_exhaustive]
pub struct Card<'a> {
    /// The id scope of the card: unique among the cards of a pass.
    pub key: String,
    /// The component's display name.
    pub title: &'a str,
    /// The story's title, when the card shows other than the component's first story.
    pub note: Option<&'a str>,
    /// The story.
    pub story: &'static StoryRegistration,
    /// The surface the story sits on.
    pub surface: Surface,
    /// The height a story that fills is given: the visible height of the view the cards scroll in, as the old
    /// gallery's tiles gave it.
    pub room: f32,
    /// Whether the header has Open.
    pub open: bool,
    /// Whether the header has Use.
    pub usable: bool,
    /// Whether the card has the selection ring.
    pub selected: bool,
    /// The values the card sets for its story.
    pub look: Look,
    /// Whether the story's wheel takers take the wheel only while focused or armed: a card among others in a
    /// scrolling overview.
    pub gated: bool,
}

/// What a card reported in a pass.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct CardEvents {
    /// A press went down in the card.
    pub pressed: bool,
    /// Open was run.
    pub opened: bool,
    /// Reset was run.
    pub reset: bool,
    /// Use was run.
    pub used: bool,
    /// The card's rect.
    pub rect: Option<Rect>,
}

impl Card<'_> {
    /// Shows the card in the room `ui` offers, with the state of its story.
    pub fn show(&self, ui: &mut Ui, state: &mut CardState) -> CardEvents {
        let mut events = CardEvents::default();
        // A story on the card's own surface sits inside the card's padding; any other surface runs edge to edge under the
        // header band.
        let pad = if self.surface == Surface::Panel { Pad::Regular } else { Pad::None };
        let mut panel = Panel::new(self.title.to_owned())
            .sizing(Sizing::Fill)
            .command_scope(CardCommands::SCOPE)
            .selected(self.selected)
            .pickable()
            .framed()
            .band(SurfaceKind::Background)
            .strength(HeaderStrength::Heading)
            .pad(pad);
        if let Some(note) = self.note {
            panel = panel.subtitle(format!("· {note}"));
        }
        let mut commands = Vec::new();
        if self.open {
            commands.push(HeaderCommand::new(IconId::new("open"), CardCommands::Open).size(ControlSize::Sm));
        }
        if self.usable {
            commands.push(HeaderCommand::new(IconId::new("check"), CardCommands::Use).size(ControlSize::Sm));
        }
        if state.changed() || state.dialog.shown {
            commands.push(HeaderCommand::new(IconId::new("undo"), CardCommands::Reset).size(ControlSize::Sm));
        }
        let shown = panel.commands(commands).show(ui, |ui, cx| {
            while let Some((command, _)) = cx.take_command::<CardCommands>() {
                match command {
                    CardCommands::Open => events.opened = true,
                    CardCommands::Reset => events.reset = true,
                    CardCommands::Use => events.used = true,
                }
            }
            events.pressed |= cx.surface_pressed();
            let body =
                StoryBody { look: self.look, gated: self.gated, ..StoryBody::new(self.key.clone(), self.story, self.surface, self.room, Pad::None) };
            body.show(ui, state);
        });
        events.rect = Some(shown.response.rect);
        events
    }
}

/// A story on its surface, as a card's body shows it, and what showed.
#[non_exhaustive]
pub struct StoryBody {
    /// The id scope of the story: unique among the stories of a pass.
    pub key: String,
    /// The story.
    pub story: &'static StoryRegistration,
    /// The surface the story sits on.
    pub surface: Surface,
    /// The height a story that fills is given.
    pub room: f32,
    /// The values the body sets for its story.
    pub look: Look,
    /// Whether the story's wheel takers take the wheel only while focused or armed.
    pub gated: bool,
    /// The margin of a story on the panel surface: none in a card, whose padding is that margin.
    pub panel_pad: Pad,
}

/// What a story body showed.
#[derive(Clone, Copy, Debug, Default)]
#[non_exhaustive]
pub struct BodyShown {
    /// Where the story shows in flow: its surface, or the whole room for a story that takes no room in flow; `None`
    /// for a story that shows only a dialog.
    pub flow: Option<Rect>,
}

impl StoryBody {
    /// The body of `story` on `surface`, in the window's look, with ungated wheel takers.
    pub fn new(key: impl Into<String>, story: &'static StoryRegistration, surface: Surface, room: f32, panel_pad: Pad) -> Self {
        StoryBody { key: key.into(), story, surface, room, look: Look::default(), gated: false, panel_pad }
    }

    /// The story on its surface, in an embedded shell of its own so its dialogs stay in the card. The shell is as tall
    /// as the story measures in the card's room: its content height, or the whole room for a story that fills it or
    /// shows nothing in flow (a dialog).
    ///
    /// A story that opens a dialog, a modal or a window starts closed behind a button, and nothing of what it opens
    /// paints before the button is pressed. Two records say what the story shows (`ShellCx::dialog_shown`): a measure
    /// that runs the story under a shell record of its own before anything paints, and the card's shell after it ran
    /// the story, for a dialog in an embedded shell of the story's own, whose body runs in real runs only. When the
    /// shell's record changes what the card shows, the card discards the pass, so nothing of that pass paints. The
    /// button turns the story's `open` value on and runs it, which opens the real thing (an OS window where the
    /// component uses one). Once nothing shows any more, the exit fade included, the card shows the button again.
    pub fn show(&self, ui: &mut Ui, state: &mut CardState) -> BodyShown {
        let window = WindowKey::new(format!("card-{}-{}", self.key, state.resets));
        let host = Host {
            id: craft_ui::testing::memory::global("gallery.card.story", (&self.key, state.resets)),
            room: self.room,
            look: self.look,
            surface: self.surface,
            gated: self.gated,
            panel_pad: self.panel_pad,
        };
        let show = self.story.story.show;
        let at = ui.available_rect_before_wrap().min;
        let record = ShellCx::detached(window.clone(), BodyMode::Normal, Rect::from_min_size(at, vec2(ui.available_width(), self.room)));
        let took = host.stack().measure_in(ui, Proposal::Ideal, |ui, _| host.content(ui, show, &mut state.data, Some(&record)));
        // A story that shows nothing in flow takes no more than its bare surface; under its own id, so its record does
        // not stand for the story's.
        let bare = host.stack().id(craft_ui::testing::memory::sub(host.id, "bare")).measure_in(ui, Proposal::Ideal, |_, _| {});
        let height = if took.y > bare.y { took.y } else { self.room };
        if record.dialog_shown() && !state.dialog.painted {
            close_at_start(state);
        }
        if state.dialog.hidden {
            state.note_start();
            if Button::new("Show dialog").show(ui).clicks == 0 {
                return BodyShown::default();
            }
            ui.ctx().request_repaint();
            state.show_dialog();
        }
        // A story that has not painted yet runs only in a pass that can still be discarded: what its shell's record
        // finds open would paint otherwise. The next frame's first pass runs it.
        let runs = state.dialog.painted || craft_ui::runtime::frame::discardable(ui.ctx());
        let ran = std::cell::Cell::new(false);
        let flow = std::cell::Cell::new(None);
        let rect = Rect::from_min_size(ui.available_rect_before_wrap().min, vec2(ui.available_width(), height));
        let shown = AppShell::embedded(window)
            .body(|ui, _, data: &mut StoryData| {
                if runs {
                    ran.set(!ui.is_sizing_pass());
                    let surface = host.stack().show(ui, |ui, _| host.content(ui, show, data, None));
                    flow.set(Some(surface.output.rect));
                }
            })
            .show_at(ui, rect, &mut state.data);
        if !runs {
            ui.ctx().request_repaint();
        }
        // The shell runs its body only in real runs (not in the card's own measuring runs, nor scrolled out of sight), so
        // its record counts only when the story ran.
        if ran.get() {
            let dialog = &mut state.dialog;
            if shown.dialog && !dialog.painted {
                close_at_start(state);
                craft_ui::runtime::frame::discard_pass(ui.ctx(), "gallery: a card's story showed a dialog the measure did not see");
            } else if dialog.known && !dialog.hidden && dialog.shown && !shown.dialog {
                *dialog = DialogState { known: true, hidden: true, shown: false, painted: true };
                craft_ui::runtime::frame::discard_pass(ui.ctx(), "gallery: a card's dialog closed");
            } else {
                dialog.shown |= dialog.known && shown.dialog;
                dialog.painted = true;
            }
        }
        state.note_start();
        // A story that takes no room in flow paints in the shell's whole room, unless what it shows is a dialog.
        let flow = if took.y > bare.y { flow.get() } else { (!state.dialog.known).then_some(rect) };
        BodyShown { flow }
    }
}

/// Puts a story whose dialog shows at its start behind its button, with the dialog closed. The card's start is the
/// story with its dialog closed, so a dialog closed by its own button is back at the start and Reset hides.
fn close_at_start(state: &mut CardState) {
    state.dialog = DialogState { known: true, hidden: true, shown: false, painted: false };
    if let Some(open) = state.data.present::<bool>("open") {
        *open = false;
    }
    state.start = None;
}

/// What a story runs in inside its card: the same in the measure and in the shell, ids included, so the measure reads
/// the state the story shows.
#[derive(Clone, Copy)]
struct Host {
    id: Id,
    /// The height a story that fills is given.
    room: f32,
    look: Look,
    surface: Surface,
    gated: bool,
    panel_pad: Pad,
}

impl Host {
    /// The story's surface over the card's width, with the card's mode.
    fn stack(&self) -> Stack<'static> {
        let kind = match self.surface {
            Surface::Panel => SurfaceKind::Panel,
            Surface::Canvas => SurfaceKind::Canvas,
            Surface::Chrome => SurfaceKind::Chrome,
            // A story that paints its own surface sits on the background, as on the old gallery's cards.
            _ => SurfaceKind::Background,
        };
        // On the panel surface the margin is the body's `panel_pad` (in a card, the card's padding is the margin); another
        // surface keeps a margin of its own inside it.
        let pad = if kind == SurfaceKind::Panel { self.panel_pad } else { Pad::Regular };
        let mut stack = Stack::new().id(self.id).fill_width().room_height(self.room).surface(kind).pad(pad);
        if let Some(mode) = self.look.mode {
            stack = stack.theme(ThemeScope::mode(mode));
        }
        stack
    }

    /// The story under the card's direction and control size; in a measure, under `record`, the shell that records
    /// what the story opens.
    fn content(&self, ui: &mut Ui, show: StoryFn, data: &mut StoryData, record: Option<&ShellCx>) {
        let (look, gated) = (self.look, self.gated);
        with_env(
            ui,
            |env| {
                env.direction = look.direction;
                env.size = look.size;
                env.wheel_gated = Some(gated);
                if let Some(record) = record {
                    record.put(&mut env.values);
                }
            },
            |ui| {
                let _ = show(ui, data);
            },
        );
    }
}
