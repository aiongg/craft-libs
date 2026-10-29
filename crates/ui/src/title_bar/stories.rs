//! Stories: the bar in each frame and caption family, with the app's menus at the leading end and
//! icon buttons at the trailing end so the title's squeeze is visible.

use craft_window::Layout;
use egui::{Context, Id, Ui, vec2};

use super::{CaptionFamily, Captions, Frame, LeadingEnd, MacInset, Title, TitleBar};
use crate::controls::IconButton;
use crate::direction::{self, Direction, HasDirection as _};
use crate::icons::Icon;
use crate::menu::MenuBar;
use crate::menu::bar::story_bar;
use crate::story::{Args, Level, Story};

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["TitleBar"];

pub(crate) fn stories() -> Vec<Story> {
    let story = |id, name, show: fn(&mut Ui, &mut Args)| Story { id, level: Level::Patterns, component: "TitleBar", name, args, show };
    vec![
        story("patterns/title-bar/windows", "Windows", |ui, args| show(ui, args, captions(CaptionFamily::Windows))),
        story("patterns/title-bar/adwaita", "Adwaita", |ui, args| show(ui, args, captions(CaptionFamily::Adwaita))),
        story("patterns/title-bar/breeze", "Breeze", |ui, args| show(ui, args, captions(CaptionFamily::Breeze))),
        story("patterns/title-bar/generic", "Generic", |ui, args| show(ui, args, captions(CaptionFamily::Generic))),
        story("patterns/title-bar/mac-inset", "Mac inset", |ui, args| show(ui, args, Frame::Inset(MacInset::default()))),
        story("patterns/title-bar/native", "Native", |ui, args| show(ui, args, Frame::Native)),
    ]
}

fn args() -> Args {
    Args::new()
        .with_bool("maximized", false)
        .with_bool("inactive", false)
        .with_text("title", "Poster.psd")
        .with_bool("unsaved", true)
        .with_number("width", 720.0, 120.0, 1400.0)
        .with_bool("rtl", false)
}

fn captions(family: CaptionFamily) -> Frame {
    Frame::Integrated(Captions { family, layout: Layout::default() })
}

/// The bar `width` wide. The window's maximized and focused state, which the caption buttons
/// read from the viewport, are set for the story's frame and restored after it.
fn show(ui: &mut Ui, args: &mut Args, frame: Frame) {
    let ctx = ui.ctx().clone();
    let width = args.number("width") as f32;
    let (maximized, inactive) = (args.bool("maximized"), args.bool("inactive"));
    let before = viewport_state(&ctx, Some((Some(maximized), Some(!inactive))));
    let outer = ctx.direction();
    direction::apply(&ctx, if args.bool("rtl") { Direction::Rtl } else { Direction::Ltr });
    ui.allocate_ui(vec2(width, 0.0), |ui| {
        ui.set_max_width(width);
        let menus = story_bar();
        let title = Title::new(args.text("title")).unsaved(args.bool("unsaved"));
        TitleBar::new(&frame).id_salt("craft-ui-title-bar-story").separator(true).title(title).show_titled(
            ui,
            |ui, _| {
                MenuBar::new(&menus).id(Id::unique("craft-ui-title-bar-story-menus")).help_search(menus.len() - 1).show(ui);
                LeadingEnd::from_ui(ui)
            },
            |ui, _| {
                ui.add(IconButton::new(Icon::Search, "Search"));
                ui.add(IconButton::new(Icon::Plus, "New"));
            },
        );
    });
    direction::apply(&ctx, outer);
    viewport_state(&ctx, before);
}

/// Sets the viewport's `(maximized, focused)` when given, and returns what it was.
fn viewport_state(ctx: &Context, set: Option<(Option<bool>, Option<bool>)>) -> Option<(Option<bool>, Option<bool>)> {
    ctx.input_mut(|i| {
        let id = i.raw.viewport_id;
        let info = i.raw.viewports.entry(id).or_default();
        let was = (info.maximized, info.focused);
        if let Some((maximized, focused)) = set {
            (info.maximized, info.focused) = (maximized, focused);
        }
        Some(was)
    })
}
