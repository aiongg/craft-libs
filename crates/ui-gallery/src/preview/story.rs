//! The mock app as stories: the photo editor at a fixed height, and the Canvas shell with its
//! slots, screen modes and reveal.

use craft_ui::direction::{self, Direction, HasDirection as _};
use craft_ui::shell::{AppShell, ScreenMode};
use craft_ui::story::{Level, NoArgs, Story, StoryArgs, StoryChoice, Surface};
use craft_ui::typed_story;
use egui::{Sense, Ui, UiBuilder, vec2};

use super::{AppPreview, ShellView};

fn shell_args(mode: ModeArg, reveal: bool, rtl: bool) -> ShellArgs {
    ShellArgs { size: SizeArg::Small, mode, reveal, rtl, title: true, options: true, tools: true, dock: true, status: true, tabs: true }
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!("patterns/app-preview/photo-editor", Level::Patterns, "AppPreview", "App preview", NoArgs {}, show).on(Surface::Own),
        typed_story!("shells/canvas/standard", Level::Shells, "CanvasShell", "Standard", shell_args(ModeArg::Standard, false, false), show_shell)
            .on(Surface::Own),
        typed_story!("shells/canvas/body_only", Level::Shells, "CanvasShell", "Body only", shell_args(ModeArg::FullScreen, false, false), show_shell)
            .on(Surface::Own),
        typed_story!("shells/canvas/revealed", Level::Shells, "CanvasShell", "Revealed", shell_args(ModeArg::FullScreen, true, false), show_shell)
            .on(Surface::Own),
        typed_story!("shells/canvas/rtl", Level::Shells, "CanvasShell", "Right to left", shell_args(ModeArg::Standard, false, true), show_shell)
            .on(Surface::Own),
    ]
}

/// The story's height in points.
const HEIGHT: f32 = 800.0;

/// The app's state lives in egui's temporary memory under the story's id: `AppPreview` is
/// `Clone`, and the story function is a plain `fn` with nowhere to keep it.
fn show(ui: &mut Ui, _args: &mut NoArgs) {
    let key = ui.make_persistent_id("app-preview-state");
    let mut preview = ui.data_mut(|d| d.remove_temp::<AppPreview>(key)).unwrap_or_default();
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), HEIGHT), Sense::hover());
    ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| preview.show(ui));
    ui.data_mut(|d| d.insert_temp(key, preview));
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum SizeArg {
    /// The least window the app lays out at.
    Small,
    Large,
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum ModeArg {
    Standard,
    #[choice(name = "Full screen with menu bar")]
    WithMenuBar,
    #[choice(name = "Full screen")]
    FullScreen,
}

impl ModeArg {
    fn mode(self) -> ScreenMode {
        match self {
            Self::Standard => ScreenMode::Standard,
            Self::WithMenuBar => ScreenMode::FullScreenWithMenuBar,
            Self::FullScreen => ScreenMode::FullScreen,
        }
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
struct ShellArgs {
    size: SizeArg,
    #[arg(name = "screen mode")]
    mode: ModeArg,
    /// Tab reveal in full screen; Tab with the canvas focused toggles it.
    #[arg(name = "tab reveal")]
    reveal: bool,
    rtl: bool,
    #[arg(name = "title bar")]
    title: bool,
    #[arg(name = "options bar")]
    options: bool,
    #[arg(name = "tool rail")]
    tools: bool,
    dock: bool,
    #[arg(name = "status bar")]
    status: bool,
    #[arg(name = "document tabs")]
    tabs: bool,
}

/// The mock app as a Canvas shell in the story's rect, with the slots the arguments show.
fn show_shell(ui: &mut Ui, args: &mut ShellArgs) {
    let ctx = ui.ctx().clone();
    let outer = ctx.direction();
    direction::apply(&ctx, if args.rtl { Direction::Rtl } else { Direction::Ltr });
    let key = ui.make_persistent_id("app-preview-shell-state");
    let mut preview = ui.data_mut(|d| d.remove_temp::<AppPreview>(key)).unwrap_or_default();
    let height = match args.size {
        SizeArg::Small => 600.0,
        SizeArg::Large => 800.0,
    };
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let view = ShellView {
        title: args.title,
        options: args.options,
        tools: args.tools,
        dock: args.dock,
        status: args.status,
        tabs: args.tabs,
        mode: args.mode.mode(),
    };
    ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
        let window = AppPreview::window_key(ui);
        // The checkbox and the Tab key both set the reveal; a Tab press during the pass wins.
        if args.reveal != AppShell::reveal_active(ui.ctx(), &window) {
            AppShell::toggle_reveal(ui.ctx(), &window);
        }
        let before = AppShell::reveal_active(ui.ctx(), &window);
        preview.show_with(ui, view);
        let after = AppShell::reveal_active(ui.ctx(), &window);
        if after != before {
            args.reveal = after;
        }
    });
    ui.data_mut(|d| d.insert_temp(key, preview));
    direction::apply(&ctx, outer);
}
