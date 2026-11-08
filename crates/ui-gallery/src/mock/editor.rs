//! `EditorMock`: a document editor window composed from library components.

use super::model::{Document, MockState, Panel, Tool};
use super::panels::{Panels, permissions};
use craft_behavior::dock::Workspace;
use craft_ui::bars::{Bar, BarKind, StatusItem, StatusReadout, StatusValues};
use craft_ui::contract::{Bind, Show as _, ShowContainer as _};
use craft_ui::dialog::{PromptChoice, StandardPrompt};
use craft_ui::docking::DockConfig;
use craft_ui::menu::{EditMenu, MenuBar, MenuItem, MenuTitle};
use craft_ui::patterns::dock::DockArea;
use craft_ui::rails::{LastUsed, ToolRail};
use craft_ui::runtime::env::{Label, WindowKey};
use craft_ui::shell::{AppShell, AppShellOutput, Title, TitleBar};
use egui::Ui;
use std::cell::RefCell;

/// The percent a zoom of 1 shows.
const PERCENT: f64 = 100.0;

/// The mock editor: the shell with its title bar and menus, options bar, tool rail and status bar around a dock whose
/// canvas group holds the document tabs, the canvas with its rulers and pasteboard, and the Navigator and layer panels.
///
/// - Tool: the rail picks the current tool; the options bar and the status bar show its name.
/// - Dialogs: Edit, Delete Layer opens the confirmation; Delete removes the current layer.
/// - Screen modes: the shell's own commands (F) and the reveal keys act on it.
#[derive(craft_ui::contract::Pattern)]
pub struct EditorMock<'a> {
    label: Label,
    state: &'a mut MockState,
}

/// What the shell's slots edit: everything but the dock's workspace.
struct Parts<'a> {
    document: &'a mut Document,
    tool: &'a mut Tool,
    last_used: &'a mut LastUsed<Tool>,
    confirm_delete: &'a mut bool,
}

impl<'a> EditorMock<'a> {
    /// A mock editor named `label` over `state`.
    pub fn new(label: impl Into<Label>, state: &'a mut MockState) -> Self {
        EditorMock { label: label.into(), state }
    }

    /// Shows the window in the room offered.
    pub fn show(self, ui: &mut Ui) -> AppShellOutput {
        let EditorMock { label, state } = self;
        let MockState { workspace, document, tool, last_used, confirm_delete } = state;
        let parts = RefCell::new(Parts { document, tool, last_used, confirm_delete });
        let workspace: &mut Workspace<Panel> = workspace;
        let shown = AppShell::new(WindowKey::main())
            .title_bar_if(true, |ui, _| {
                let parts = &mut *parts.borrow_mut();
                title_bar(ui, parts);
            })
            .options_bar(true, |ui, _| {
                let parts = parts.borrow();
                let _ = Bar::new("Options", BarKind::Options).show(ui, |ui, _| {
                    craft_ui::display::Text::new(format!("Tool: {}", parts.tool.name())).show(ui);
                });
            })
            .tools(true, |ui, _| {
                let parts = &mut *parts.borrow_mut();
                tool_rail(ui, parts);
            })
            .status(true, |ui, _| {
                let parts = parts.borrow();
                status_bar(ui, &parts);
            })
            .body_label(label)
            .body(|ui, _| {
                let mut parts = parts.borrow_mut();
                let mut panels = Panels { document: &mut *parts.document };
                let id = egui::Id::unique("mock editor dock");
                let output = DockArea::new(id, Bind::bound(&mut *workspace), &mut panels, DockConfig::default()).show(ui);
                for action in output.actions {
                    // A refused action leaves the workspace as it was.
                    let _ = workspace.apply(action, false, permissions);
                }
            })
            .show(ui);
        let parts = parts.into_inner();
        if *parts.confirm_delete {
            let name = parts.document.current_layer().map(|layer| layer.name.clone()).unwrap_or_default();
            if let Some(answer) = StandardPrompt::confirm_delete([name], 1).show(ui).answer {
                if answer.choice == PromptChoice::Proceed {
                    parts.document.delete_current();
                }
                *parts.confirm_delete = false;
            }
        }
        shown
    }
}

fn title_bar(ui: &mut Ui, parts: &mut Parts<'_>) {
    let title = if parts.document.dirty { Title::unsaved(parts.document.title.clone()) } else { Title::new(parts.document.title.clone()) };
    let mut delete = false;
    TitleBar::new().show_titled(
        ui,
        title,
        |ui, _| {
            let titles = vec![
                MenuTitle::new("&File", |ui| {
                    MenuItem::new("&New").show(ui);
                    MenuItem::new("&Open").show(ui);
                    MenuItem::new("&Save").show(ui);
                }),
                MenuTitle::new("&Edit", |ui| {
                    EditMenu.show(ui);
                    if MenuItem::new("&Delete Layer...").show(ui).ran {
                        delete = true;
                    }
                }),
            ];
            MenuBar::new("Main menu", titles).show(ui);
        },
        |_, _| {},
    );
    if delete {
        *parts.confirm_delete = true;
    }
}

fn tool_rail(ui: &mut Ui, parts: &mut Parts<'_>) {
    let slots = Tool::slots();
    let current = *parts.tool;
    let last_used = parts.last_used.clone();
    let output = ToolRail::new("Tools", &slots, &current, &last_used).show_rail(ui, |_, _| {});
    if let Some(tool) = output.select {
        *parts.tool = tool;
    }
    if let Some(last_used) = output.last_used {
        *parts.last_used = last_used;
    }
}

fn status_bar(ui: &mut Ui, parts: &Parts<'_>) {
    let document = &*parts.document;
    let mut values = StatusValues::default();
    values.document_dimensions = Some(format!("{} x {} px", document.size.width, document.size.height));
    values.current_tool = Some(parts.tool.name().to_owned());
    values.layer_count = Some(document.layers.len());
    let zoom = Some(document.view.zoom * PERCENT);
    let _ = Bar::new("Status", BarKind::Status).show(ui, |ui, _| {
        StatusReadout::new(Bind::given(zoom), StatusItem::DocumentDimensions, &values).show(ui);
    });
}
