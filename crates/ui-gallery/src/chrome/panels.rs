//! The bodies of the viewer's panels: the component tree, the target component's stories, facts and decision record,
//! and the theme workspace's tokens, problems, detail and app sizes. The dock draws each panel's tab and frame.

use super::describe;
use super::overview;
use crate::theme::{AppLayerEditor, Problems, ThemeSubject, ThemeWorkspace, TokenBrowser, TokenDetail};
use crate::viewer::{self, Catalog, Cells, ComponentInfo, PanelId, TreeKey, TreeRow, TreeRows};
use craft_behavior::IconId;
use craft_behavior::{Selection, keyed};
use craft_theme::Explained;
use craft_theme::report::ContrastReport;
use craft_ui::buttons::Button;
use craft_ui::contract::{Bind, CollectionEvent, Pad, Scroll, SelectMode, Show as _, ShowCollection as _, ShowContainer as _};
use craft_ui::display::{Shade, Text};
use craft_ui::layout_components::{Row, Stack};
use craft_ui::list::{List, ListRow, OpenSet, RowEvent, TrailingFace, TrailingPart};
use craft_ui::runtime::env::Label;
use craft_ui::style::TypeRole;
use craft_ui::testing::{StoryData, StorySnapshot, story_props};
use craft_ui::text::TextField;
use egui::Ui;
use std::collections::HashMap;
use std::sync::Arc;

/// What the panels read and edit in one pass.
#[non_exhaustive]
pub struct Panels<'a> {
    /// What is listed.
    pub catalog: &'a Catalog,
    /// The text that narrows the tree.
    pub filter: &'a mut String,
    /// The selected row of the tree.
    pub selection: &'a mut Selection<TreeKey>,
    /// The open level groups.
    pub open: &'a mut OpenSet<TreeKey>,
    /// The story each component's card shows.
    pub story_of: &'a mut HashMap<&'static str, &'static str>,
    /// The shown stories' values.
    pub cells: &'a mut Cells,
    /// The values each shown story started with, which Reset goes back to.
    pub starts: &'a mut HashMap<usize, StorySnapshot>,
    /// The component whose tab the tree asked to open in this pass.
    pub open_request: &'a mut Option<&'static str>,
    /// The reset count the cards' id scopes carry.
    pub generation: u32,
    /// What the theme panels share.
    pub themes: &'a mut ThemeWorkspace,
    /// The theme and appearance the theme panels explain.
    pub subject: ThemeSubject<'a>,
    /// The row the Problems view picked in this pass, which the token browser brings into view.
    pub revealed: Option<crate::theme::Pick>,
}

/// `add` in the panel's padding, scrolling vertically. The body may run more than once in a pass, so `add` is `FnMut`.
fn scrolled(ui: &mut Ui, mut add: impl FnMut(&mut Ui)) {
    Stack::new().fill().pad(Pad::Regular).scroll(Scroll::vertical()).show(ui, |ui, _| add(ui));
}

/// `add` in the panel's padding, for a body that is a collection or scrolls itself. Like `scrolled`, `add` may run more
/// than once in a pass.
fn padded(ui: &mut Ui, mut add: impl FnMut(&mut Ui)) {
    Stack::new().fill().pad(Pad::Regular).show(ui, |ui, _| add(ui));
}

/// The tree row part that opens a component's tab.
const OPEN: TrailingPart = TrailingPart::new(0);

/// A muted line where a panel has nothing to show yet.
fn hint(ui: &mut Ui, text: &'static str) {
    Text::new(text).shade(Shade::Muted).show(ui);
}

/// A caption over a value.
fn fact(ui: &mut Ui, name: &'static str, value: impl Into<Label>) {
    let value = value.into();
    Stack::new().fill_width().show(ui, |ui, _| {
        Text::new(name).role(TypeRole::Caption).shade(Shade::Muted).show(ui);
        Text::new(value.clone()).show(ui);
    });
}

/// The values of the story at catalog index `index`: the one place the panels reach into the shown cells.
fn data_of(cells: &mut Cells, index: usize) -> &mut StoryData {
    cells.entry(index).or_default()
}

impl Panels<'_> {
    /// Shows the body of `panel`.
    pub fn show(&mut self, ui: &mut Ui, panel: PanelId) {
        match panel {
            PanelId::Components => self.components(ui),
            PanelId::Stories => self.stories(ui),
            PanelId::Properties => self.properties(ui),
            PanelId::Inspect => self.record(ui),
            PanelId::Overview => {
                let selected = self.selected().map(|component| component.type_name);
                scrolled(ui, |ui| overview::stories(ui, self.catalog, self.cells, self.generation, selected));
            }
            PanelId::Component(type_name) => scrolled(ui, |ui| overview::stories(ui, self.catalog, self.cells, self.generation, Some(type_name))),
            PanelId::Preview => {}
            PanelId::Problems => self.problems(ui),
            PanelId::Tokens => self.tokens(ui),
            PanelId::Detail => self.detail(ui),
            PanelId::AppSizes => self.overrides(ui),
        }
    }

    /// The selected component.
    fn selected(&self) -> Option<&ComponentInfo> {
        let type_name = self.selection.items.iter().rev().find_map(|key| match key {
            TreeKey::Component(type_name) => Some(*type_name),
            _ => None,
        })?;
        self.catalog.component(type_name)
    }

    fn components(&mut self, ui: &mut Ui) {
        let shown = viewer::rows(&self.catalog.components, self.filter.as_str(), &*self.open);
        let groups = viewer::groups(&shown);
        let by_key: HashMap<&TreeKey, &TreeRow> = shown.iter().map(|row| (&row.key, row)).collect();
        let before = self.selected().map(|component| component.type_name);
        let pointer = ui.ctx().pointer_hover_pos();
        let mut opened = None;
        let (filter, selection, open) = (&mut *self.filter, &mut *self.selection, &mut *self.open);
        Stack::new().fill().pad(Pad::Regular).show(ui, |ui, _| {
            TextField::search("Filter components", Bind::bound(&mut *filter)).show(ui);
            let listed = List::<TreeKey>::new("Components")
                .select(SelectMode::Single)
                .tree(true)
                .selection(Bind::bound(&mut *selection))
                .empty("No component matches the filter")
                .show_items(ui, &TreeRows::new(&shown), &mut |key, cx| {
                    let Some(row) = by_key.get(key) else { return ListRow::spacer() };
                    let item = ListRow::new(row.label.clone()).depth(row.depth);
                    if row.group {
                        return item.group(row.open);
                    }
                    // The Open icon shows on the selected row and the row under the pointer.
                    if cx.row.selected || pointer.is_some_and(|at| cx.rect.contains(at)) {
                        item.trailing_part(OPEN, TrailingFace::Icon(IconId::new("open").into()), format!("Open {}", row.label))
                    } else {
                        item
                    }
                });
            for event in listed.events {
                match event {
                    CollectionEvent::Open(change) => open.apply(&change.key, change.open, change.all, groups.iter()),
                    CollectionEvent::Activate(TreeKey::Component(type_name)) => opened = Some(type_name),
                    CollectionEvent::Item(TreeKey::Component(type_name), RowEvent::Trailing { .. }) => opened = Some(type_name),
                    _ => {}
                }
            }
        });
        if let Some(type_name) = opened {
            *self.open_request = Some(type_name);
        }
        // Selecting a component opens its level.
        let now = self.selected().map(|component| (component.type_name, component.level));
        if let Some((type_name, level)) = now
            && Some(type_name) != before
        {
            self.open.set(&TreeKey::Level(level), true);
        }
    }

    fn stories(&mut self, ui: &mut Ui) {
        let catalog = self.catalog;
        let Some(component) = self.selected() else {
            return scrolled(ui, |ui| {
                hint(ui, "Select a component");
            });
        };
        let (type_name, titles) = (component.type_name, component.titles.clone());
        let listed: Vec<usize> = (0..catalog.entries.len()).filter(|&at| catalog.entries[at].type_name == type_name).collect();
        let mut selection = Selection::default();
        selection.items.extend(self.target(type_name));
        let before = selection.items.clone();
        Stack::new().fill().pad(Pad::Regular).show(ui, |ui, _| {
            let source = keyed(&listed, |at| *at);
            List::<usize>::new("Stories").select(SelectMode::Single).selection(Bind::bound(&mut selection)).show_items(ui, &source, &mut |at, _| {
                ListRow::new(listed.iter().position(|shown| shown == at).and_then(|place| titles.get(place)).cloned().unwrap_or_default())
            });
        });
        if selection.items != before
            && let Some(picked) = selection.items.first()
        {
            self.story_of.insert(type_name, catalog.entries[*picked].registration.story.name);
        }
    }

    /// The index of the story the component registered as `type_name` shows: the one picked for it, else its first.
    fn target(&self, type_name: &'static str) -> Option<usize> {
        let mut stories = (0..self.catalog.entries.len()).filter(|&at| self.catalog.entries[at].type_name == type_name);
        let picked = self.story_of.get(type_name).copied();
        let first = stories.clone().next();
        picked.and_then(|name| stories.find(|&at| self.catalog.entries[at].registration.story.name == name)).or(first)
    }

    /// The display title of the story at catalog index `at` of `component`.
    fn title(&self, component: &ComponentInfo, at: usize) -> String {
        let place = self.catalog.entries[..at].iter().filter(|entry| entry.type_name == component.type_name).count();
        component.titles.get(place).cloned().unwrap_or_default()
    }

    fn properties(&mut self, ui: &mut Ui) {
        let component = self.selected().cloned();
        let target = component.as_ref().and_then(|component| self.target(component.type_name));
        let (Some(component), Some(index)) = (component, target) else {
            return scrolled(ui, |ui| {
                hint(ui, "Select a component");
            });
        };
        let title = self.title(&component, index);
        let declared = self.catalog.declared.get(component.type_name).copied();
        let starts = &mut *self.starts;
        let data = data_of(self.cells, index);
        // The values exist once the story has run; the first pass that finds them records the start.
        if !starts.contains_key(&index) && !data.changed_since(&StoryData::default().snapshot()).is_empty() {
            starts.insert(index, data.snapshot());
        }
        let changed = starts.get(&index).is_some_and(|start| !data.changed_since(start).is_empty());
        let mut reset = false;
        scrolled(ui, |ui| {
            Row::new().show(ui, |ui, _| {
                Text::new(component.name.clone()).role(TypeRole::Heading).show(ui);
                Text::new(title.clone()).shade(Shade::Muted).show(ui);
                if changed {
                    reset = Button::new("Reset").show(ui).clicks > 0;
                }
            });
            story_props(ui, &mut *data);
            fact(ui, "Type", component.type_name);
            if let Some(declared) = declared {
                fact(ui, "Module", declared.module);
                fact(ui, "Purpose", declared.purpose);
            }
        });
        if reset && let Some(start) = starts.get(&index) {
            data.restore(start);
        }
    }

    fn record(&mut self, ui: &mut Ui) {
        let probe = self.selected().and_then(|component| self.catalog.declared.get(component.type_name)).and_then(|declared| declared.probe);
        let chosen = self.selected().is_some();
        scrolled(ui, |ui| {
            if !chosen {
                hint(ui, "Select a component");
                return;
            }
            let Some(probe) = probe else {
                hint(ui, "A pattern has no record of its own: its parts are the records.");
                return;
            };
            fact(ui, "Shape", describe::shape(probe.shape));
            fact(ui, "Role", describe::role(probe.role));
            fact(ui, "Input", describe::input(probe.input));
            fact(ui, "Keys it keeps", describe::keys(&probe.keys));
            fact(ui, "Opens a popup", probe.overlay.to_string());
            fact(ui, "Shows its label", probe.shows_label.to_string());
            for exemption in probe.exempt {
                fact(ui, "Exempt from", format!("{}: {}", exemption.case, exemption.reason));
            }
        });
    }

    /// A panel over the explanation and report of the subject.
    fn theme_panel(&mut self, ui: &mut Ui, mut body: impl FnMut(&mut Self, &mut Ui, &Arc<Explained>, &Arc<ContrastReport>)) {
        self.themes.refresh(&self.subject);
        let (Some(explained), Some(report)) = (self.themes.explained().cloned(), self.themes.report().cloned()) else { return };
        padded(ui, |ui| body(self, ui, &explained, &report));
    }

    fn problems(&mut self, ui: &mut Ui) {
        self.theme_panel(ui, |panels, ui, explained, report| {
            let picked = Problems::new(report, explained, &panels.subject).show(ui).picked;
            if let Some(pick) = picked {
                panels.themes.pick = Some(pick.clone());
                panels.revealed = Some(pick);
            }
        });
    }

    fn tokens(&mut self, ui: &mut Ui) {
        self.theme_panel(ui, |panels, ui, explained, _| {
            let mut browser = TokenBrowser::new(explained, Bind::bound(&mut panels.themes.pick));
            if let Some(pick) = &panels.revealed {
                browser = browser.reveal(pick);
            }
            browser.show(ui);
        });
    }

    fn detail(&mut self, ui: &mut Ui) {
        self.theme_panel(ui, |panels, ui, explained, _| {
            let mut detail = TokenDetail::new(explained);
            if let Some(pick) = &panels.themes.pick {
                detail = detail.pick(pick);
            }
            detail.show(ui);
        });
    }

    fn overrides(&mut self, ui: &mut Ui) {
        self.theme_panel(ui, |panels, ui, explained, _| {
            let pick = panels.themes.pick.clone();
            panels.themes.edit_sizes(|sizes| {
                let mut editor = AppLayerEditor::new(Bind::bound(sizes), explained);
                if let Some(pick) = &pick {
                    editor = editor.pick(pick);
                }
                editor.show(ui)
            });
        });
    }
}
