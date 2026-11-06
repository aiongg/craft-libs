//! The Components panel: a filter and the components that match it, in a tree with a group for each level.
use std::cell::Cell;

use craft_theme::ControlSize;
use craft_ui::behavior::{RowInfo, Selection, SelectionChange, SelectionMode};
use craft_ui::controls::{IconButton, SearchField};
use craft_ui::icons::Icon;
use craft_ui::list::{List, ListEvent, ListRow, TreeState};
use craft_ui::story::{Level, Story};
use craft_ui::theme::{Tokens, TokensOf};
use egui::{Rect, Ui};

use super::catalog::nav;
use crate::fields;

/// A row of the component list: a level's label, or a component.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Entry {
    Level(Level),
    Component(&'static str),
}

/// What the list reported in a frame.
#[derive(Default)]
pub(super) struct ComponentEvents {
    /// The component that was selected.
    pub(super) picked: Option<&'static str>,
    /// The component whose tab was asked for.
    pub(super) opened: Option<&'static str>,
}

#[derive(Default)]
pub(super) struct ComponentList {
    /// The text the list is filtered by; the Overview filters its levels by it too.
    pub(super) filter: String,
}

impl ComponentList {
    pub(super) fn new(filter: String) -> Self {
        Self { filter }
    }

    /// The components that have a story matching the filter, in a group for each level. `current`
    /// is the target's component. A click selects a component; Open, Enter and a double click open
    /// its tab. The open levels are a [`TreeState`] in egui's persisted memory; a level opens when
    /// the target moves into it, and while the filter has text every level with matches shows open
    /// without changing what is remembered.
    pub(super) fn show(&mut self, ui: &mut Ui, stories: &[Story], current: Option<&'static str>) -> ComponentEvents {
        let filter = SearchField::new("gallery.filter", "Filter components", &self.filter, "Clear filter")
            .placeholder("Filter components")
            .policy(fields::policy())
            .show(ui);
        fields::apply(&filter, &mut self.filter);
        let filter = self.filter.to_lowercase();
        let mut open = TreeState::load(ui.ctx(), "gallery.components");
        let seen = ui.scope_id().with("gallery.components.target");
        if ui.data(|d| d.get_temp::<Option<&'static str>>(seen)) != Some(current) {
            ui.data_mut(|d| d.insert_temp(seen, current));
            if let Some(story) = current.and_then(|c| stories.iter().find(|s| s.component == c)) {
                open.set(&Entry::Level(story.level), true);
            }
        }
        let filtering = !filter.is_empty();
        let mut rows: Vec<RowInfo<Entry>> = Vec::new();
        let mut levels: Vec<Entry> = Vec::new();
        for (level, components) in nav(stories, &filter) {
            let level = Entry::Level(level);
            let shown = filtering || open.is_open(&level);
            levels.push(level);
            rows.push(RowInfo::group(level, shown));
            if shown {
                rows.extend(components.into_iter().map(|c| RowInfo::leaf(Entry::Component(c)).at_depth(1)));
            }
        }
        let mut selection =
            current.map(Entry::Component).filter(|e| rows.iter().any(|r| r.key == *e)).map_or_else(Selection::default, Selection::only);
        let open_button = Tokens::of(ui).size.control.sm;
        let opened = Cell::new(None);
        let report = &opened;
        let out = List::new("gallery.components", "Components", rows.as_slice())
            .tree(true)
            .selection(&mut selection)
            .mode(SelectionMode::Single)
            .type_ahead(true)
            .show(ui, |_, entry| match entry {
                Entry::Level(level) => ListRow::new(*entry, level.title()),
                Entry::Component(component) => {
                    let (component, on) = (*component, current == Some(*component));
                    ListRow::new(*entry, component).trailing_item(open_button, move |ui| {
                        // The item is as tall as the row; the list's clip is as wide.
                        let row = Rect::from_x_y_ranges(ui.clip_rect().x_range(), ui.max_rect().y_range());
                        if (on || ui.rect_contains_pointer(row))
                            && ui.add(IconButton::new(Icon::Open, format!("Open {component}")).size(ControlSize::Sm)).clicked()
                        {
                            report.set(Some(component));
                        }
                    })
                }
            });
        let mut events = ComponentEvents { picked: None, opened: opened.get() };
        for event in out.events {
            match event {
                ListEvent::Selection(SelectionChange { row: Some(Entry::Component(c)), .. }) | ListEvent::Clicked(Entry::Component(c)) => {
                    events.picked = Some(c)
                }
                ListEvent::Activate(Entry::Component(c)) => events.opened = Some(c),
                ListEvent::Open(change) if !filtering => open.apply(&change, levels.iter()),
                _ => {}
            }
        }
        open.store(ui.ctx());
        events
    }
}
