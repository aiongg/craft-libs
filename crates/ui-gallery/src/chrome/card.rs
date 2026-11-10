//! One card: a story live on its surface under a header with the component's name and, while the story differs from its
//! start, Reset; the Open command of an Overview card opens the component's tab. The story runs in an embedded shell,
//! so its dialogs open inside the card.

use crate::viewer::CardState;
use craft_behavior::IconId;
use craft_i18n::Direction;
use craft_theme::{ControlSize, Mode};
use craft_ui::contract::{CommandSet, Commands as _, Props as _, ShowContainer as _, Sizing, ThemeScope};
use craft_ui::layout_components::Stack;
use craft_ui::runtime::env::{WindowKey, with_env};
use craft_ui::runtime::pointer;
use craft_ui::sections::{HeaderCommand, Panel};
use craft_ui::shell::AppShell;
use craft_ui::testing::StoryRegistration;
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
    pub surface: craft_ui::contract::Surface,
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
        // A card entirely outside the visible area is not run: its region would have no room, and a page of cards
        // would run every story at once. It keeps the height it had.
        let remembered = Id::unique(("gallery.card.size", &self.key, state.resets));
        let known: Option<f32> = ui.data(|data| data.get_temp(remembered));
        if !ui.is_sizing_pass() {
            let (top, view) = (ui.cursor().min.y, ui.clip_rect());
            if top >= view.max.y || known.is_some_and(|height| top + height <= view.min.y) {
                ui.allocate_space(vec2(ui.available_width(), known.unwrap_or(0.0)));
                return events;
            }
        }
        let mut panel = Panel::new(self.title.to_owned()).sizing(Sizing::Content).command_scope(CardCommands::SCOPE).selected(self.selected);
        if let Some(note) = self.note {
            panel = panel.subtitle(format!("· {note}"));
        }
        let mut commands = Vec::new();
        if self.open {
            commands.push(HeaderCommand::new(IconId::new("open"), CardCommands::Open));
        }
        if self.usable {
            commands.push(HeaderCommand::new(IconId::new("check"), CardCommands::Use));
        }
        if state.changed() {
            commands.push(HeaderCommand::new(IconId::new("undo"), CardCommands::Reset));
        }
        let key = (self.key.clone(), state.resets);
        let shown = panel.commands(commands).show(ui, |ui, cx| {
            while let Some((command, _)) = cx.take_command::<CardCommands>() {
                match command {
                    CardCommands::Open => events.opened = true,
                    CardCommands::Reset => events.reset = true,
                    CardCommands::Use => events.used = true,
                }
            }
            self.story_body(ui, state, key.clone());
        });
        let rect = shown.response.rect;
        ui.data_mut(|data| data.insert_temp(remembered, rect.height()));
        events.rect = Some(rect);
        events.pressed = pointer::frame(ui.ctx()).press.is_some_and(|press| rect.contains(press.origin) && ui.clip_rect().contains(press.origin));
        events
    }

    /// The story on its surface, in an embedded shell of its own.
    fn story_body(&self, ui: &mut Ui, state: &mut CardState, key: (String, u32)) {
        let window = WindowKey::new(format!("card-{}-{}", self.key, state.resets));
        let (show, look, surface) = (self.story.story.show, self.look, self.surface);
        let on_panel = matches!(surface, craft_ui::contract::Surface::Panel);
        let title = self.title.to_owned();
        // The embedded shell fills the room it is given, so the card gives it the height its story took last pass (or its
        // largest dialog took, which the shell reports).
        let content = Id::unique(("gallery.card.content", &self.key, state.resets));
        let measured = Id::unique(("gallery.card.height", &window));
        let height: f32 = ui.data(|data| data.get_temp(measured)).unwrap_or_else(|| ui.ctx().content_rect().height());
        let width = ui.available_width();
        let (room, _) = ui.allocate_exact_size(vec2(width, height), egui::Sense::hover());
        let shown = ui
            .scope_builder(egui::UiBuilder::new().max_rect(room), |ui| {
                AppShell::embedded(window)
                    .body(move |ui, _, data: &mut craft_ui::testing::StoryData| {
                        let mut stack = Stack::new().id_salt(key.clone()).fill();
                        if let Some(mode) = look.mode {
                            stack = stack.theme(ThemeScope::mode(mode));
                        }
                        stack.show(ui, |ui, _| {
                            with_env(
                                ui,
                                |env| {
                                    env.direction = look.direction;
                                    env.size = look.size;
                                },
                                |ui| {
                                    if on_panel {
                                        Panel::new(title.clone()).headerless().sizing(Sizing::FillWidth).show(ui, |ui, _| {
                                            let _ = show(ui, data);
                                        });
                                    } else {
                                        let _ = show(ui, data);
                                    }
                                    // What the story took is what the card's body is as tall as.
                                    let taken = ui.min_rect().height();
                                    if !ui.is_sizing_pass() && taken.is_finite() {
                                        ui.data_mut(|data| data.insert_temp(content, taken));
                                    }
                                },
                            );
                        });
                    })
                    .show(ui, &mut state.data)
            })
            .inner;
        let wanted = shown.dialog.map_or(0.0, |dialog| dialog.y);
        let taken = ui.data(|data| data.get_temp::<f32>(content)).unwrap_or(0.0);
        // A story that draws nothing in flow (an overlay, a dialog) takes the viewport's height, as it did in the old gallery.
        let next = if taken.max(wanted) > 0.0 { taken.max(wanted) } else { ui.ctx().content_rect().height() };
        if (next - height).abs() > 0.5 {
            ui.data_mut(|data| data.insert_temp(measured, next));
            ui.ctx().request_repaint();
        }
        state.note_start();
    }
}
