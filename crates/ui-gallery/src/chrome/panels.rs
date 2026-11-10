//! The bodies of the viewer's panels: the component tree, the target component's stories, facts and decision record,
//! and the theme workspace's tokens, problems, detail and app sizes. The dock draws each panel's tab and frame.

use super::describe;
use super::overview;
use crate::theme::{AppLayerEditor, Problems, ThemeSubject, ThemeWorkspace, TokenBrowser, TokenDetail};
use crate::viewer::{self, Catalog, Cells, ComponentInfo, PanelId, TreeKey, TreeRow, TreeRows};
use craft_behavior::{Selection, keyed};
use craft_theme::Explained;
use craft_theme::report::ContrastReport;
use craft_theme::variant::Tone;
use craft_ui::contract::{Bind, CollectionEvent, Pad, Scroll, SelectMode, Show as _, ShowCollection as _, ShowContainer as _};
use craft_ui::display::{Shade, Text};
use craft_ui::layout_components::Stack;
use craft_ui::list::{List, ListRow, OpenSet};
use craft_ui::notices::Notice;
use craft_ui::runtime::env::Label;
use craft_ui::style::TypeRole;
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
    /// The open module groups.
    pub open: &'a mut OpenSet<TreeKey>,
    /// The story each component's card shows.
    pub story_of: &'a mut HashMap<&'static str, &'static str>,
    /// The shown stories' values.
    pub cells: &'a mut Cells,
    /// The reset count the cards' id scopes carry.
    pub generation: u32,
    /// What the theme panels share.
    pub themes: &'a mut ThemeWorkspace,
    /// The theme and appearance the theme panels explain.
    pub subject: ThemeSubject<'a>,
    /// The level the Overview shows, by its title.
    pub level: &'a mut String,
    /// The Variants view of the target story, while one is open.
    pub variants: &'a mut Option<crate::viewer::VariantAxis>,
    /// The root values; a Variants card that is used sets its value.
    pub env: &'a mut craft_ui::runtime::env::RootEnv,
    /// The component whose Open a card ran in this pass.
    pub opened: Option<&'static str>,
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

/// A caption over a value.
fn fact(ui: &mut Ui, name: &'static str, value: impl Into<Label>) {
    let value = value.into();
    Stack::new().show(ui, |ui, _| {
        Text::new(name).role(TypeRole::Caption).shade(Shade::Muted).show(ui);
        Text::new(value.clone()).show(ui);
    });
}

impl Panels<'_> {
    /// Shows the body of `panel`.
    pub fn show(&mut self, ui: &mut Ui, panel: PanelId) {
        match panel {
            PanelId::Components => self.components(ui),
            PanelId::Stories => self.stories(ui),
            PanelId::Properties => self.facts(ui),
            PanelId::Inspect => self.record(ui),
            PanelId::Overview => overview::overview(ui, self),
            PanelId::Component(type_name) => overview::component_tab(ui, self, type_name),
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
        let (filter, selection, open) = (&mut *self.filter, &mut *self.selection, &mut *self.open);
        Stack::new().fill().pad(Pad::Regular).show(ui, |ui, _| {
            TextField::search("Filter components", Bind::bound(&mut *filter)).show(ui);
            let listed = List::<TreeKey>::new("Components")
                .select(SelectMode::Single)
                .tree(true)
                .selection(Bind::bound(&mut *selection))
                .empty("No component matches the filter")
                .show_items(ui, &TreeRows::new(&shown), &mut |key, _| {
                    let Some(row) = by_key.get(key) else { return ListRow::spacer() };
                    let item = ListRow::new(row.label.clone()).depth(row.depth);
                    if row.group { item.group(row.open) } else { item }
                });
            for event in listed.events {
                if let CollectionEvent::Open(change) = event {
                    open.apply(&change.key, change.open, change.all, groups.iter());
                }
            }
        });
    }

    fn stories(&mut self, ui: &mut Ui) {
        let catalog = self.catalog;
        let Some(type_name) = self.selected().map(|component| component.type_name) else {
            return scrolled(ui, |ui| {
                Notice::new(Tone::Info, "Select a component to see its stories.").show(ui);
            });
        };
        let listed: Vec<usize> = (0..catalog.entries.len()).filter(|&at| catalog.entries[at].type_name == type_name).collect();
        let mut selection = Selection::default();
        selection.items.extend(self.target(type_name));
        let before = selection.items.clone();
        Stack::new().fill().pad(Pad::Regular).show(ui, |ui, _| {
            let source = keyed(&listed, |at| *at);
            List::<usize>::new("Stories")
                .select(SelectMode::Single)
                .selection(Bind::bound(&mut selection))
                .show_items(ui, &source, &mut |at, _| ListRow::new(catalog.entries[*at].registration.story.name));
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

    fn facts(&mut self, ui: &mut Ui) {
        let component = self.selected();
        let declared = component.and_then(|component| self.catalog.declared.get(component.type_name));
        scrolled(ui, |ui| {
            let Some(component) = component else {
                Notice::new(Tone::Info, "Select a component to see its facts.").show(ui);
                return;
            };
            fact(ui, "Type", component.type_name);
            fact(ui, "Stories", component.stories.join(", "));
            if let Some(declared) = declared {
                fact(ui, "Module", declared.module);
                fact(ui, "Purpose", declared.purpose);
            }
        });
    }

    fn record(&mut self, ui: &mut Ui) {
        let probe = self.selected().and_then(|component| self.catalog.declared.get(component.type_name)).and_then(|declared| declared.probe);
        let chosen = self.selected().is_some();
        scrolled(ui, |ui| {
            if !chosen {
                Notice::new(Tone::Info, "Select a component to see its decision record.").show(ui);
                return;
            }
            let Some(probe) = probe else {
                Notice::new(Tone::Info, "A pattern has no record of its own: its parts are the records.").show(ui);
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
