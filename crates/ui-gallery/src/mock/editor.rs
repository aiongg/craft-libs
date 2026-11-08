//! `EditorMock`: a document editor window composed from library components.

use super::model::{Document, MockState, Panel, Request, Tool};
use super::panels::Panels;
use super::tools::{Dialogs, Tools};
use craft_behavior::PixelSize;
use craft_behavior::dock::Workspace;
use craft_ui::bars::{Bar, BarKind, StatusItem, StatusReadout, StatusValues};
use craft_ui::contract::{Bind, Remember as _, Show as _, ShowContainer as _};
use craft_ui::dialog::{DialogResult, PromptChoice, StandardPrompt};
use craft_ui::docking::DockConfig;
use craft_ui::menu::{EditMenu, MenuBar, MenuItem, MenuTitle};
use craft_ui::patterns::dock::DockArea;
use craft_ui::patterns::form_dialogs::{DialogFields, HostedParamDialog, ImageSize, ImageSizeValues, Param, ParamDialog, ParamResult};
use craft_ui::rails::{LastUsed, ToolRail};
use craft_ui::runtime::commands::CommandRef;
use craft_ui::runtime::env::{Label, WindowKey};
use craft_ui::shell::{AppShell, AppShellOutput, Title, TitleBar};
use egui::Ui;
use std::cell::{Cell, RefCell};

/// The parameters of Gaussian Blur.
const BLUR: &str = r#"{"radius":0.1..1000=1}"#;

/// The command key of the Gaussian Blur dialog.
const BLUR_KEY: &str = "gaussianBlur";

/// The mock editor: the shell with its title bar and menus, options bar, tool rail and status bar around a dock whose
/// canvas group holds the document tabs, the canvas with its rulers and pasteboard, and the Navigator and layer panels.
///
/// - Tool: the rail picks the current tool; Hand and Zoom run in the canvas's tool host.
/// - Dialogs: Edit, Delete Layer and Filter, Gaussian Blur open on the dialog host; Image, Image Size shows its dialog.
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
    request: &'a mut Option<Request>,
    image_size: &'a mut ImageSizeValues,
    image_size_open: &'a mut bool,
}

impl<'a> EditorMock<'a> {
    /// A mock editor named `label` over `state`.
    pub fn new(label: impl Into<Label>, state: &'a mut MockState) -> Self {
        EditorMock { label: label.into(), state }
    }

    /// Shows the window in the room offered.
    pub fn show(self, ui: &mut Ui) -> AppShellOutput {
        let EditorMock { label, state } = self;
        let MockState { workspace, document, tool, last_used, request, image_size, image_size_open } = state;
        let parts = RefCell::new(Parts { document, tool, last_used, request, image_size, image_size_open });
        let workspace: &mut Workspace<Panel> = workspace;
        AppShell::new(WindowKey::main())
            .title_bar_if(true, |ui, _, _| {
                let parts = &mut *parts.borrow_mut();
                title_bar(ui, parts);
            })
            .options_bar(true, |ui, _, _| {
                let parts = parts.borrow();
                let _ = Bar::new("Options", BarKind::Options).show(ui, |ui, _| {
                    craft_ui::display::Text::new(format!("Tool: {}", parts.tool.name())).show(ui);
                });
            })
            .tools(true, |ui, _, _| {
                let parts = &mut *parts.borrow_mut();
                tool_rail(ui, parts);
            })
            .status(true, |ui, _, _| {
                let parts = parts.borrow();
                status_bar(ui, &parts);
            })
            .body_label(label)
            .body(|ui, cx, _| {
                let mut parts = parts.borrow_mut();
                let mut tools = cx.keep::<Tools>().clone();
                let dialogs = cx.keep::<Dialogs>().clone();
                let current = *parts.tool;
                let mut panels = Panels { document: &mut *parts.document, tool: current, tools: &mut tools };
                DockArea::new("Dock", Bind::bound(&mut *workspace), &mut panels, DockConfig::default()).show(ui);
                *cx.keep::<Tools>() = tools;
                if let Ok(mut host) = dialogs.0.lock() {
                    open_requested(&mut host, &mut parts);
                    host.run(ui);
                    answers(&mut host, &mut parts);
                }
                if *parts.image_size_open {
                    let parts = &mut *parts;
                    let shown = ImageSize::new(Bind::bound(&mut *parts.image_size)).opened(Bind::bound(&mut *parts.image_size_open)).show(ui);
                    if shown.result.is_some() {
                        parts.document.dirty = true;
                    }
                }
            })
            .show(ui, &mut ())
    }
}

/// Opens the dialog a menu asked for.
fn open_requested(host: &mut craft_ui::dialog::DialogHost, parts: &mut Parts<'_>) {
    match parts.request.take() {
        Some(Request::GaussianBlur) => {
            host.open(HostedParamDialog::new(BLUR_KEY, DialogFields::new(), |_| {
                ParamDialog::new(BLUR_KEY, "Gaussian Blur", Param::parse_list(BLUR))
            }));
        }
        Some(Request::DeleteLayer) => {
            let name = parts.document.current_layer().map(|layer| layer.name.clone()).unwrap_or_default();
            host.open(StandardPrompt::confirm_delete([name], 1));
        }
        Some(Request::ImageSize) => *parts.image_size_open = true,
        None => {}
    }
}

/// Applies the answers the dialogs gave in this pass.
fn answers(host: &mut craft_ui::dialog::DialogHost, parts: &mut Parts<'_>) {
    if host.take::<HostedParamDialog>().and_then(DialogResult::<ParamResult>::ok).is_some() {
        parts.document.dirty = true;
    }
    let confirmed = host.take::<StandardPrompt>().and_then(DialogResult::ok).is_some_and(|answer| answer.choice == PromptChoice::Proceed);
    if confirmed {
        parts.document.delete_current();
    }
}

fn title_bar(ui: &mut Ui, parts: &mut Parts<'_>) {
    let title = if parts.document.dirty { Title::unsaved(parts.document.title.clone()) } else { Title::new(parts.document.title.clone()) };
    let asked = Cell::new(None);
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
                        asked.set(Some(Request::DeleteLayer));
                    }
                }),
                MenuTitle::new("&Image", |ui| {
                    if MenuItem::new("Image &Size...").show(ui).ran {
                        asked.set(Some(Request::ImageSize));
                    }
                }),
                MenuTitle::new("Fil&ter", |ui| {
                    if MenuItem::new("&Gaussian Blur...").show(ui).ran {
                        asked.set(Some(Request::GaussianBlur));
                    }
                }),
            ];
            MenuBar::new("Main menu", titles).show(ui);
        },
        |_, _| {},
    );
    if asked.get().is_some() {
        *parts.request = asked.get();
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
    values.document_dimensions = Some(PixelSize::new(document.size.width as u32, document.size.height as u32));
    values.current_tool = Some(CommandRef::from(*parts.tool));
    values.layer_count = Some(document.layers.len());
    let _ = Bar::new("Status", BarKind::Status).show(ui, |ui, _| {
        StatusReadout::new(Bind::given(document.view), document.size, StatusItem::DocumentDimensions, &values).show(ui);
    });
}
