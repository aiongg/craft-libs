//! The panels of the viewer: the component tree, the shown stories, the component's facts and its decision record,
//! and the theme workspace's tokens, problems, detail and app sizes.

use crate::theme::{AppLayerEditor, Problems, ThemeSubject, ThemeWorkspace, TokenBrowser, TokenDetail};
use crate::viewer::{
    self, COMPONENTS, Catalog, CellFilter, Cells, ComponentInfo, DETAIL, FACTS, GalleryCommands, OVERRIDES, PROBLEMS, RECORD, STORIES, TOKENS,
    TreeKey, TreeRows,
};
use craft_behavior::{GroupId, Selection};
use craft_i18n::Direction;
use craft_theme::ControlSize;
use craft_theme::Explained;
use craft_theme::report::ContrastReport;
use craft_theme::variant::Tone;
use craft_ui::contract::{Bind, CollectionEvent, Props as _, SelectMode, Show as _, ShowCollection as _, ShowContainer as _};
use craft_ui::display::{Shade, Text};
use craft_ui::layout_components::{Row, Stack};
use craft_ui::list::{List, ListRow, OpenSet};
use craft_ui::menu::MenuItem;
use craft_ui::notices::Notice;
use craft_ui::runtime::env::{Label, with_env};
use craft_ui::sections::Panel;
use craft_ui::style::TypeRole;
use craft_ui::text::TextField;
use egui::{Rect, Ui};
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
    /// The shown cells' values.
    pub cells: &'a mut Cells,
    /// The reset count the cells' id scopes carry.
    pub generation: u32,
    /// Which cells of a story are shown.
    pub cell_filter: CellFilter,
    /// What the theme panels share.
    pub themes: &'a mut ThemeWorkspace,
    /// The theme and appearance the theme panels explain.
    pub subject: ThemeSubject<'a>,
    /// The row the Problems view picked in this pass, which the token browser brings into view.
    pub revealed: Option<crate::theme::Pick>,
}

/// The rows every panel's menu offers.
fn panel_menu(ui: &mut Ui) {
    MenuItem::command(GalleryCommands::ResetStories).show(ui);
    MenuItem::command(GalleryCommands::ResetLayout).show(ui);
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
    /// Shows the panel of `group` at `rect`.
    pub fn show(&mut self, ui: &mut Ui, group: GroupId, rect: Rect) {
        if group == COMPONENTS {
            self.components(ui, rect);
        } else if group == STORIES {
            self.stories(ui, rect);
        } else if group == FACTS {
            self.facts(ui, rect);
        } else if group == RECORD {
            self.record(ui, rect);
        } else if group == PROBLEMS {
            self.problems(ui, rect);
        } else if group == TOKENS {
            self.tokens(ui, rect);
        } else if group == DETAIL {
            self.detail(ui, rect);
        } else if group == OVERRIDES {
            self.overrides(ui, rect);
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

    fn components(&mut self, ui: &mut Ui, rect: Rect) {
        let shown = viewer::rows(&self.catalog.components, self.filter.as_str(), &*self.open);
        let groups = viewer::groups(&shown);
        let (filter, selection, open) = (&mut *self.filter, &mut *self.selection, &mut *self.open);
        Panel::new("Components").collection_body().menu(|ui, _| panel_menu(ui)).show_at(ui, rect, |ui, _| {
            TextField::search("Filter components", Bind::bound(&mut *filter)).show(ui);
            let listed = List::<TreeKey>::new("Components")
                .select(SelectMode::Single)
                .tree(true)
                .selection(Bind::bound(&mut *selection))
                .empty("No component matches the filter")
                .show_items(ui, &TreeRows::new(&shown), &mut |key, _| {
                    let Some(row) = shown.iter().find(|row| row.key == *key) else { return ListRow::spacer() };
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

    fn stories(&mut self, ui: &mut Ui, rect: Rect) {
        let catalog = self.catalog;
        let selected = self.selected().map(|component| component.type_name);
        let (cells, generation, shown) = (&mut *self.cells, self.generation, self.cell_filter);
        let directions: Vec<_> = [Direction::Ltr, Direction::Rtl].into_iter().filter(|d| shown.direction.is_none_or(|only| only == *d)).collect();
        let sizes: Vec<_> = [ControlSize::Sm, ControlSize::Md].into_iter().filter(|s| shown.size.is_none_or(|only| only == *s)).collect();
        Panel::new("Stories").menu(|ui, _| panel_menu(ui)).show_at(ui, rect, |ui, _| {
            let Some(type_name) = selected else {
                Notice::new(Tone::Info, "Select a component to see its stories.").show(ui);
                return;
            };
            for (index, entry) in catalog.entries.iter().enumerate().filter(|(_, entry)| entry.type_name == type_name) {
                Text::new(entry.registration.story.name).role(TypeRole::Heading).show(ui);
                Row::new().wrap(true).show(ui, |ui, _| {
                    for direction in &directions {
                        for size in &sizes {
                            let data = cells.entry((index, *direction, *size)).or_default();
                            // Each cell is its own id scope: a story's `id_salt` and auto ids must not meet in another cell.
                            Stack::new().id_salt((index, direction.is_rtl(), *size, generation)).show(ui, |ui, _| {
                                let name = if direction.is_rtl() { "RTL" } else { "LTR" };
                                Text::new(format!("{name}, {size:?}")).role(TypeRole::Caption).shade(Shade::Muted).show(ui);
                                with_env(
                                    ui,
                                    |env| {
                                        env.direction = Some(*direction);
                                        env.size = Some(*size);
                                    },
                                    |ui| {
                                        let _ = (entry.registration.story.show)(ui, data);
                                    },
                                );
                            });
                        }
                    }
                });
            }
        });
    }

    fn facts(&mut self, ui: &mut Ui, rect: Rect) {
        let component = self.selected();
        let declared = component.and_then(|component| self.catalog.declared.get(component.type_name));
        Panel::new("Properties").menu(|ui, _| panel_menu(ui)).show_at(ui, rect, |ui, _| {
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

    fn record(&mut self, ui: &mut Ui, rect: Rect) {
        let probe = self.selected().and_then(|component| self.catalog.declared.get(component.type_name)).and_then(|declared| declared.probe);
        let chosen = self.selected().is_some();
        Panel::new("Decision record").menu(|ui, _| panel_menu(ui)).show_at(ui, rect, |ui, _| {
            if !chosen {
                Notice::new(Tone::Info, "Select a component to see its decision record.").show(ui);
                return;
            }
            let Some(probe) = probe else {
                Notice::new(Tone::Info, "A pattern has no record of its own: its parts are the records.").show(ui);
                return;
            };
            fact(ui, "Shape", format!("{:?}", probe.shape));
            fact(ui, "Role", format!("{:?}", probe.role));
            fact(ui, "Input", format!("{:?}", probe.input));
            fact(ui, "Keys it keeps", format!("{:?}", probe.keys));
            fact(ui, "Opens a popup", probe.overlay.to_string());
            fact(ui, "Shows its label", probe.shows_label.to_string());
            for exemption in probe.exempt {
                fact(ui, "Exempt from", format!("{}: {}", exemption.case, exemption.reason));
            }
        });
    }

    /// The panel of `title` over the explanation and report of the subject, when the workspace has them.
    fn theme_panel(
        &mut self,
        ui: &mut Ui,
        rect: Rect,
        title: &'static str,
        mut body: impl FnMut(&mut Self, &mut Ui, &Arc<Explained>, &Arc<ContrastReport>),
    ) {
        let (Some(explained), Some(report)) = (self.themes.explained().cloned(), self.themes.report().cloned()) else { return };
        Panel::new(title).menu(|ui, _| panel_menu(ui)).show_at(ui, rect, |ui, _| body(self, ui, &explained, &report));
    }

    fn problems(&mut self, ui: &mut Ui, rect: Rect) {
        self.theme_panel(ui, rect, "Problems", |panels, ui, explained, report| {
            let picked = Problems::new(report, explained, &panels.subject).show(ui).picked;
            if let Some(pick) = picked {
                panels.themes.pick = Some(pick.clone());
                panels.revealed = Some(pick);
            }
        });
    }

    fn tokens(&mut self, ui: &mut Ui, rect: Rect) {
        self.theme_panel(ui, rect, "Tokens", |panels, ui, explained, _| {
            let mut browser = TokenBrowser::new(explained, Bind::bound(&mut panels.themes.pick));
            if let Some(pick) = &panels.revealed {
                browser = browser.reveal(pick);
            }
            browser.show(ui);
        });
    }

    fn detail(&mut self, ui: &mut Ui, rect: Rect) {
        self.theme_panel(ui, rect, "Detail", |panels, ui, explained, _| {
            let mut detail = TokenDetail::new(explained);
            if let Some(pick) = &panels.themes.pick {
                detail = detail.pick(pick);
            }
            detail.show(ui);
        });
    }

    fn overrides(&mut self, ui: &mut Ui, rect: Rect) {
        self.theme_panel(ui, rect, "App sizes", |panels, ui, explained, _| {
            let pick = panels.themes.pick.clone();
            let mut editor = AppLayerEditor::new(Bind::bound(&mut panels.themes.sizes), explained);
            if let Some(pick) = &pick {
                editor = editor.pick(pick);
            }
            editor.show(ui);
        });
    }
}
