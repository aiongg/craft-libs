//! One card: a story live on its surface under a header with the component's name and, while the story differs from its
//! start, Reset; the Open command of an Overview card opens the component's tab. The story runs in an embedded shell,
//! so its dialogs open inside the card.

use crate::viewer::{CardState, DialogState};
use craft_behavior::IconId;
use craft_behavior::Proposal;
use craft_i18n::Direction;
use craft_theme::{ControlSize, Mode};
use craft_ui::buttons::Button;
use craft_ui::contract::Show as _;
use craft_ui::contract::{CommandSet, Commands as _, Pad, Props as _, ShowContainer as _, Sizing, Surface, ThemeScope};
use craft_ui::layout_components::Stack;
use craft_ui::runtime::env::{WindowKey, with_env};
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
            .sizing(Sizing::FillWidth)
            .command_scope(CardCommands::SCOPE)
            .selected(self.selected)
            .pickable()
            .band(SurfaceKind::Background)
            .strength(HeaderStrength::Section)
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
            self.story_body(ui, state);
        });
        events.rect = Some(shown.response.rect);
        events
    }

    /// The story on its surface, in an embedded shell of its own so its dialogs stay in the card. The shell is as tall
    /// as the story measures in the card's room: its content height, or the whole room for a story that fills it or
    /// shows nothing in flow (a dialog).
    fn story_body(&self, ui: &mut Ui, state: &mut CardState) {
        let window = WindowKey::new(format!("card-{}-{}", self.key, state.resets));
        let host = Host {
            id: craft_ui::testing::memory::global("gallery.card.story", (&self.key, state.resets)),
            room: self.room,
            look: self.look,
            surface: self.surface,
        };
        let show = self.story.story.show;
        let took = host.stack().measure_in(ui, Proposal::Ideal, |ui, _| host.content(ui, show, &mut state.data));
        // A story that shows nothing in flow takes no more than its bare surface; under its own id, so its record does
        // not stand for the story's.
        let bare = host.stack().id(craft_ui::testing::memory::sub(host.id, "bare")).measure_in(ui, Proposal::Ideal, |_, _| {});
        let height = if took.y > bare.y { took.y } else { self.room };
        // A story that opens a dialog starts with it closed behind a button, as on the old gallery's Overview tiles.
        if state.dialog.hidden {
            if Button::new("Show dialog").show(ui).clicks == 0 {
                return;
            }
            ui.ctx().request_repaint();
            state.dialog = DialogState { known: true, hidden: false, shown: false };
            if let Some(open) = state.data.present::<bool>("open") {
                *open = true;
            }
        }
        let rect = Rect::from_min_size(ui.available_rect_before_wrap().min, vec2(ui.available_width(), height));
        let shown = AppShell::embedded(window)
            .body(move |ui, _, data: &mut StoryData| {
                host.stack().show(ui, |ui, _| host.content(ui, show, data));
            })
            .show_at(ui, rect, &mut state.data);
        let was_shown = state.dialog.shown;
        // A pass records the dialog's rect only some of the time, so it is the story's own `open` flag that says the
        // dialog was closed (a story that cannot close its dialog has none).
        let closed = state.data.present::<bool>("open").is_some_and(|open| !*open);
        let dialog = &mut state.dialog;
        let first_seen = shown.dialog.is_some() && !dialog.known;
        match (shown.dialog.is_some(), dialog.known) {
            (true, false) => *dialog = DialogState { known: true, hidden: true, shown: false },
            (true, true) => dialog.shown = true,
            (false, _) => {}
        }
        if dialog.known && !dialog.hidden && closed && dialog.shown {
            *dialog = DialogState { known: true, hidden: true, shown: false };
        }
        if dialog.hidden || dialog.shown != was_shown {
            ui.ctx().request_repaint();
        }
        // The card's start is the story with its dialog closed, so a dialog closed by its own button is back at the
        // start and Reset hides.
        if first_seen {
            if let Some(open) = state.data.present::<bool>("open") {
                *open = false;
            }
            state.start = None;
        }
        state.note_start();
    }
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
        // On the card's own panel surface the card's padding is the story's margin; another surface keeps a margin of its
        // own inside it.
        let pad = if kind == SurfaceKind::Panel { Pad::None } else { Pad::Regular };
        let mut stack = Stack::new().id(self.id).fill_width().room_height(self.room).surface(kind).pad(pad);
        if let Some(mode) = self.look.mode {
            stack = stack.theme(ThemeScope::mode(mode));
        }
        stack
    }

    /// The story under the card's direction and control size.
    fn content(&self, ui: &mut Ui, show: StoryFn, data: &mut StoryData) {
        let look = self.look;
        with_env(
            ui,
            |env| {
                env.direction = look.direction;
                env.size = look.size;
            },
            |ui| {
                let _ = show(ui, data);
            },
        );
    }
}
