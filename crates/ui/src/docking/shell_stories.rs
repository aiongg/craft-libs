//! Stories of the step 3 docking items: the panel menu, the titled floating frame, the Window ›
//! Arrange rows and cells, and the workspace switcher.

use egui::{Rect, Ui, vec2};

use super::arrange::{Arrangement, ArrangeLabels, arrange_entries, arrange_rects};
use super::chrome::{ChromeStyle, FloatingFrame, TitleBar};
use super::panel_menu::{PanelMenu, PanelMenuLabels};
use super::workspaces::{NamedWorkspace, WorkspaceChrome, WorkspaceLabels, WorkspaceSet, WorkspaceSwitcher};
use super::{Permissions, Workspace};
use crate::controls::Button;
use crate::localized::Labels as _;
use crate::menu::{MenuEntry, MenuItem, PressMenu};
use crate::primitives::{Content, Text};
use crate::story::{Level, NoArgs, Story, StoryArgs, Surface};
use crate::typed_story;

/// The components these stories cover.
pub const COMPONENTS: &[&str] = &["PanelMenu", "FloatingFrame", "Arrange", "WorkspaceSwitcher"];

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!("patterns/dock/panel-menu", Level::Patterns, "PanelMenu", "Panel menu", NoArgs {}, show_panel_menu),
        typed_story!(
            "patterns/dock/titled-frame",
            Level::Patterns,
            "FloatingFrame",
            "Titled frame",
            FrameArgs { auto_height: true, contents: 80.0 },
            show_titled_frame
        )
        .on(Surface::Canvas),
        typed_story!(
            "patterns/dock/arrange",
            Level::Patterns,
            "Arrange",
            "Arrange",
            ArrangeArgs { documents: 4, groups: 1, floating: 0, cells: 4 },
            show_arrange
        ),
        typed_story!("patterns/dock/workspaces", Level::Patterns, "WorkspaceSwitcher", "Workspaces", NoArgs {}, show_workspaces),
    ]
}

fn show_panel_menu(ui: &mut Ui, _args: &mut NoArgs) {
    let entries: Vec<MenuEntry<String>> = vec![
        MenuItem::new("Show Thumbnails", "thumbnails".to_owned()).check(true).into(),
        MenuItem::new("Merge Down", "merge".to_owned()).into(),
    ];
    let words = PanelMenuLabels::of(ui);
    let targets = vec!["Properties".to_owned(), "Channels".to_owned()];
    let group_items = PanelMenu::group_items(Permissions::default(), &targets, true, &words);
    let last = ui.make_persistent_id("panel-menu-story-last");
    let button = ui.add(Button::new("≡"));
    let menu = PanelMenu { entries: &entries, group_items };
    if let Some(choice) = menu.show(ui, &button) {
        ui.data_mut(|d| d.insert_temp(last, format!("{choice:?}")));
    }
    let text = ui.data(|d| d.get_temp::<String>(last)).unwrap_or_else(|| "Nothing chosen".to_owned());
    ui.add(Text::new(text).content(Content::Muted));
}

#[derive(StoryArgs, Debug, PartialEq)]
struct FrameArgs {
    #[arg(name = "auto height")]
    auto_height: bool,
    #[arg(range = 20.0..=240.0)]
    contents: f32,
}

fn show_titled_frame(ui: &mut Ui, args: &mut FrameArgs) {
    let area = ui.allocate_space(vec2(ui.available_width().min(520.0), 300.0)).1;
    let id = ui.make_persistent_id("titled-frame-story");
    let rect_key = ui.make_persistent_id("titled-frame-rect");
    let start = Rect::from_min_size(area.min + vec2(24.0, 24.0), vec2(220.0, 140.0));
    let rect = ui.data(|d| d.get_temp::<Rect>(rect_key)).unwrap_or(start);
    let style = ChromeStyle::from_ui(ui);
    let bar = TitleBar::new(id.with("bar"), "Move window").title("Styles").close("Close window");
    let contents = args.contents;
    let shown = FloatingFrame::new(id, rect).bounds(area).auto_height(args.auto_height).title_bar(bar).show_titled::<_, ()>(
        ui.ctx(),
        &style,
        |ui, _| {
            ui.allocate_space(vec2(40.0, contents));
        },
    );
    let mut next = shown.moved.unwrap_or(rect);
    if let Some(height) = shown.content_height {
        next.set_height(height + style.title_height + style.body_inset());
    }
    ui.data_mut(|d| d.insert_temp(rect_key, next));
}

#[derive(StoryArgs, Debug, PartialEq)]
struct ArrangeArgs {
    #[arg(range = 0..=8)]
    documents: usize,
    #[arg(range = 1..=6)]
    groups: usize,
    #[arg(range = 0..=6)]
    floating: usize,
    #[arg(range = 2..=6)]
    cells: usize,
}

fn show_arrange(ui: &mut Ui, args: &mut ArrangeArgs) {
    let labels = ArrangeLabels::of(ui);
    let entries = arrange_entries(&labels, args.documents, args.groups, args.floating);
    let last = ui.make_persistent_id("arrange-story-last");
    let button = ui.add(Button::new(&labels.arrange));
    if let Some(chosen) = PressMenu::new(&entries).show(ui, &button).chosen {
        ui.data_mut(|d| d.insert_temp(last, chosen));
    }
    let arrangement = ui.data(|d| d.get_temp::<Arrangement>(last)).unwrap_or(Arrangement::TileAllVertically);
    ui.add(Text::new(format!("{arrangement:?}")).content(Content::Muted));
    let (_, area) = ui.allocate_space(vec2(240.0, 160.0));
    let painter = ui.painter_at(area);
    let color = ui.visuals().widgets.noninteractive.fg_stroke.color;
    if let Some(cells) = arrange_rects(arrangement, area, args.cells) {
        for cell in cells {
            painter.rect_stroke(cell.shrink(1.0), 0.0, egui::Stroke::new(1.0, color), egui::StrokeKind::Inside);
        }
    }
}

fn show_workspaces(ui: &mut Ui, _args: &mut NoArgs) {
    let named = |id: &str, name: &str| NamedWorkspace {
        id: id.to_owned(),
        name: name.to_owned(),
        layout: Workspace::<u32>::default(),
        chrome: WorkspaceChrome::default(),
        keeps_shortcuts: false,
        keeps_menus: false,
    };
    let mut set = WorkspaceSet::new(vec![named("essentials", "Essentials"), named("painting", "Painting"), named("photography", "Photography")]);
    set.saved = vec![named("mine", "Mine"), named("review", "Review")];
    let key = ui.make_persistent_id("workspaces-story");
    set.current = ui.data(|d| d.get_temp::<String>(key)).unwrap_or(set.current);
    let labels = WorkspaceLabels::of(ui);
    let (_, command) = WorkspaceSwitcher::new(&set, &labels).show(ui);
    if let Some(super::workspaces::WorkspaceCommand::Select(id)) = command {
        ui.data_mut(|d| d.insert_temp(key, id));
    }
}
