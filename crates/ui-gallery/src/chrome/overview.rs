//! The centre tabs: the Overview (the cards of one level), a component's tab (every story of it) and the Variants view
//! that replaces the front one.

use super::card::{Card, CardEvents, CardStory, Look};
use super::panels::Panels;
use super::variants;
use crate::viewer::{ComponentInfo, PanelId, TreeKey, by_level, entry_of};
use craft_behavior::{ColumnWidth, Selection};
use craft_theme::variant::Tone;
use craft_ui::contract::{Bind, BodyCx, Entry, Gap, Lazy, Pad, Props as _, Scroll, Show as _, ShowContainer as _, StoryLayout};
use craft_ui::display::{Shade, Text};
use craft_ui::layout_components::{Grid, Row, Stack};
use craft_ui::notices::Notice;
use craft_ui::pickers::Select;
use craft_ui::runtime::env::WindowKey;
use craft_ui::shell::AppShell;
use craft_ui::style::SurfaceKind;
use craft_ui::testing::StoryRegistration;
use egui::{Align, Ui};

/// The narrowest a card may be before the cards go to one column: a dock panel of the apps is about this wide, and a
/// story drawn in less is no longer what the apps show.
const CARD_MIN_WIDTH: f32 = 290.0;

const ONE: &[ColumnWidth] = &[ColumnWidth::fill(1.0)];
const TWO: &[ColumnWidth] = &[ColumnWidth::fill(1.0); 2];

/// One story of a card to show.
pub(super) struct SpecStory {
    /// The index of the story in the catalog.
    pub index: usize,
    pub story: &'static StoryRegistration,
    /// The story's caption.
    pub caption: String,
}

impl SpecStory {
    /// The story at catalog index `index` of the component whose display name is `name`.
    pub fn of(p: &Panels<'_>, index: usize, name: &str) -> Self {
        let story = p.catalog.entries[index].registration;
        SpecStory { index, story, caption: story.story.caption(name) }
    }
}

/// The card of one component to show, before it is laid out.
pub(super) struct Spec {
    /// The id scope of the card.
    pub key: String,
    /// The registered name of the component.
    pub type_name: &'static str,
    /// The card's title.
    pub title: String,
    /// A note after the title.
    pub note: Option<String>,
    /// The stories the card shows, in order.
    pub stories: Vec<SpecStory>,
    /// How the stories are laid out together.
    pub layout: StoryLayout,
    pub surface: craft_ui::contract::Surface,
    pub open: bool,
    pub usable: bool,
    pub selected: bool,
    pub look: Look,
    /// Whether the card gates its story's wheel takers (`Card::gated`).
    pub gated: bool,
}

/// What the cards of a pass reported, by card.
pub(super) struct Reported {
    pub spec: usize,
    pub events: CardEvents,
}

/// The index in `catalog` of the story the component registered as `type_name` shows: the one picked for it, else its
/// first.
pub(super) fn target_story(p: &Panels<'_>, type_name: &'static str) -> Option<usize> {
    let entries = &p.catalog.entries;
    let mut stories = (0..entries.len()).filter(|&at| entries[at].type_name == type_name);
    let first = stories.clone().next();
    let picked = p.story_of.get(type_name).copied();
    picked.and_then(|name| stories.find(|&at| entries[at].registration.story.name == name)).or(first)
}

/// The selected component's registered name.
pub(super) fn selected_type(p: &Panels<'_>) -> Option<&'static str> {
    p.selection.items.iter().rev().find_map(|key| match key {
        TreeKey::Component(type_name) => Some(*type_name),
        _ => None,
    })
}

/// The window's background (an embedded shell, which paints it and keeps its cards' dialogs in the tab) with the
/// panel's padding around `add`.
pub(super) fn on_background(ui: &mut Ui, key: &str, mut add: impl FnMut(&mut Ui, &mut BodyCx<'_>)) {
    AppShell::embedded(WindowKey::new(format!("gallery-centre-{key}")))
        .body(|ui, cx, _: &mut ()| {
            Stack::new().fill().surface(SurfaceKind::Background).pad(Pad::Regular).show(ui, |ui, _| add(ui, cx));
        })
        .show(ui, &mut ());
}

/// The cards in one or two columns, scrolling vertically; each row is as tall as its tallest card. A row out of view
/// does not run (`Lazy`), except the one the selection scrolls to. The cards report into `reported`.
pub(super) fn grid(
    ui: &mut Ui,
    scroll_id: impl std::hash::Hash + std::fmt::Debug,
    cards: &[Spec],
    p: &mut Panels<'_>,
    one_column: bool,
) -> Vec<Reported> {
    let mut reported = Vec::new();
    let columns = if one_column || ui.available_width() < 2.0 * CARD_MIN_WIDTH { 1 } else { 2 };
    let generation = p.generation;
    // A story that fills its room gets the view's height, as on the old gallery's tiles.
    let room = ui.available_height();
    Stack::new().fill().scroll(Scroll::vertical()).gap(Gap::CardRow).id_salt(scroll_id).show(ui, |ui, _| {
        reported.clear();
        for (row, chunk) in cards.chunks(columns).enumerate() {
            let target = chunk.iter().any(|spec| *p.scroll_to == Some(spec.type_name));
            let key = (generation, columns, chunk[0].key.as_str());
            Lazy::new(key).keep(target).show(ui, |ui| {
                Grid::new(if columns == 1 { ONE } else { TWO }).gap(Gap::CardColumn).fill_width().show(ui, |ui, _| {
                    for (at, spec) in chunk.iter().enumerate() {
                        let stories: Vec<CardStory<'_>> =
                            spec.stories.iter().map(|story| CardStory::new(story.index, story.story, &story.caption)).collect();
                        // Each card is its own id scope: a story's `id_salt` and auto ids must not meet another card's.
                        let card = Card {
                            key: format!("{}-{generation}", spec.key),
                            title: &spec.title,
                            note: spec.note.as_deref(),
                            stories: &stories,
                            layout: spec.layout,
                            surface: spec.surface,
                            room,
                            open: spec.open,
                            usable: spec.usable,
                            selected: spec.selected,
                            look: spec.look,
                            gated: spec.gated,
                        };
                        let events = card.show(ui, p.cells);
                        reported.push(Reported { spec: row * columns + at, events });
                    }
                });
            });
        }
    });
    reported
}

/// Applies what the cards reported: a press selects the card's component (and the story of a card of one story), Open
/// opens its tab, Reset gives the card's stories their start values.
pub(super) fn apply(ui: &mut Ui, p: &mut Panels<'_>, cards: &[Spec], reported: Vec<Reported>) {
    let scroll = *p.scroll_to;
    let mut scrolled = false;
    for Reported { spec, events } in reported {
        let Some(spec) = cards.get(spec) else { continue };
        if events.reset {
            for story in &spec.stories {
                p.cells.entry(story.index).or_default().reset();
            }
        }
        if events.opened {
            *p.open_request = Some(spec.type_name);
        }
        if events.pressed || events.opened {
            *p.selection = Selection::single(TreeKey::Component(spec.type_name));
            if let [story] = spec.stories.as_slice() {
                p.story_of.insert(spec.type_name, story.story.story.name);
            }
        }
        if scroll == Some(spec.type_name)
            && let Some(rect) = events.rect
        {
            ui.scroll_to_rect(rect, Some(Align::TOP));
            scrolled = true;
        }
    }
    if scrolled {
        *p.scroll_to = None;
    }
}

/// The level the selected component sits on, and a request to bring its card into view, when the selection changed
/// since the last pass of the Overview.
fn follow_selection(p: &mut Panels<'_>) {
    let selected = selected_type(p);
    if *p.followed == Some(selected) {
        return;
    }
    *p.followed = Some(selected);
    if let Some(type_name) = selected
        && let Some(entry) = entry_of(type_name)
    {
        *p.level = entry.level.title().to_owned();
        *p.scroll_to = Some(type_name);
    }
}

/// The Overview: the level select over the cards of the level's components.
pub(super) fn overview(ui: &mut Ui, p: &mut Panels<'_>) {
    if variants::show(ui, p, PanelId::Overview) {
        return;
    }
    follow_selection(p);
    let groups = by_level(p.catalog, p.filter);
    let Some(shown) = groups.iter().position(|(level, _)| level.title() == p.level.as_str()).or(if groups.is_empty() { None } else { Some(0) })
    else {
        on_background(ui, "overview", |ui, _| {
            Notice::new(Tone::Info, "No components match the filter.").show(ui);
        });
        return;
    };
    let level = groups[shown].0;
    let entries: Vec<Entry<craft_ui::contract::Level>> = groups.iter().map(|(level, _)| Entry::new(*level, level.title())).collect();
    let selected = selected_type(p);
    let cards: Vec<Spec> = groups[shown].1.iter().filter_map(|component| component_card(p, component, selected)).collect();
    let mut chosen = level;
    let mut reported = Vec::new();
    on_background(ui, "overview", |ui, _| {
        Row::new().show(ui, |ui, _| {
            Text::new("Level:").shade(Shade::Muted).show(ui);
            Select::new("Level", Bind::bound(&mut chosen), &entries).show(ui);
        });
        reported = grid(ui, level.title(), &cards, p, false);
    });
    if chosen != level {
        *p.level = chosen.title().to_owned();
    }
    apply(ui, p, &cards, reported);
}

/// The Overview's card of `component`: every story of it, in catalog order.
fn component_card(p: &Panels<'_>, component: &ComponentInfo, selected: Option<&'static str>) -> Option<Spec> {
    let entry = entry_of(component.type_name)?;
    let stories: Vec<SpecStory> = (0..p.catalog.entries.len())
        .filter(|&at| p.catalog.entries[at].type_name == component.type_name)
        .map(|at| SpecStory::of(p, at, &entry.name))
        .collect();
    if stories.is_empty() {
        return None;
    }
    Some(Spec {
        key: component.type_name.to_owned(),
        type_name: component.type_name,
        title: entry.name.clone(),
        note: None,
        stories,
        layout: entry.story_layout,
        surface: entry.surface,
        open: true,
        usable: false,
        selected: selected == Some(component.type_name),
        look: Look::default(),
        gated: true,
    })
}

/// A component's tab: every story of the component as a card in one column.
pub(super) fn component_tab(ui: &mut Ui, p: &mut Panels<'_>, type_name: &'static str) {
    if variants::show(ui, p, PanelId::Component(type_name)) {
        return;
    }
    let Some(entry) = entry_of(type_name) else {
        on_background(ui, "component", |ui, _| {
            Notice::new(Tone::Info, "The component has no stories.").show(ui);
        });
        return;
    };
    let target = target_story(p, type_name);
    let chosen = selected_type(p) == Some(type_name);
    let cards: Vec<Spec> = (0..p.catalog.entries.len())
        .filter(|&at| p.catalog.entries[at].type_name == type_name)
        .map(|at| {
            let story = SpecStory::of(p, at, &entry.name);
            Spec {
                key: format!("{type_name}/{}", story.story.story.name),
                type_name,
                title: story.caption.clone(),
                note: None,
                stories: vec![story],
                layout: StoryLayout::Stacked,
                surface: entry.surface,
                open: false,
                usable: false,
                selected: chosen && target == Some(at),
                look: Look::default(),
                gated: false,
            }
        })
        .collect();
    let mut reported = Vec::new();
    on_background(ui, "component", |ui, _| reported = grid(ui, ("component", type_name), &cards, p, true));
    apply(ui, p, &cards, reported);
}
