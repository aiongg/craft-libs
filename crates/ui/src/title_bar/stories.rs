//! Stories: the bar in each frame and caption family, with the app's menus at the leading end and
//! icon buttons at the trailing end so the title's squeeze is visible.

use craft_window::Layout;
use egui::{Id, Ui, vec2};

use super::{CaptionFamily, Captions, Frame, LeadingEnd, MacInset, Title, TitleBar, WindowState};
use crate::controls::IconButton;
use crate::direction::{self, Direction, HasDirection as _};
use crate::icons::Icon;
use crate::menu::MenuBar;
use crate::menu::bar::story_bar;
use crate::story::{Level, Story, StoryArgs, Surface};
use crate::typed_story;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["TitleBar"];

#[derive(StoryArgs, Debug, PartialEq)]
struct BarArgs {
    maximized: bool,
    inactive: bool,
    title: String,
    unsaved: bool,
    #[arg(range = 120.0..=1400.0)]
    width: f32,
    rtl: bool,
}

impl Default for BarArgs {
    fn default() -> Self {
        Self { maximized: false, inactive: false, title: "Poster.psd".into(), unsaved: true, width: 720.0, rtl: false }
    }
}

pub(crate) fn stories() -> Vec<Story> {
    // The frame is built inside each story's non-capturing closure.
    macro_rules! bar_story {
        ($id:expr, $name:expr, $frame:expr) => {
            typed_story!($id, Level::Patterns, "TitleBar", $name, BarArgs::default(), |ui: &mut Ui, args: &mut BarArgs| show(ui, args, $frame))
                .on(Surface::Chrome)
        };
    }
    vec![
        bar_story!("patterns/title-bar/windows", "Windows", captions(CaptionFamily::Windows)),
        bar_story!("patterns/title-bar/adwaita", "Adwaita", captions(CaptionFamily::Adwaita)),
        bar_story!("patterns/title-bar/breeze", "Breeze", captions(CaptionFamily::Breeze)),
        bar_story!("patterns/title-bar/generic", "Generic", captions(CaptionFamily::Generic)),
        bar_story!("patterns/title-bar/mac-inset", "Mac inset", Frame::Inset(MacInset::default())),
        bar_story!("patterns/title-bar/native", "Native", Frame::Native),
    ]
}

fn captions(family: CaptionFamily) -> Frame {
    Frame::Integrated(Captions { family, layout: Layout::default() })
}

/// The bar `width` wide, as a picture of a window in the story's state.
fn show(ui: &mut Ui, args: &mut BarArgs, frame: Frame) {
    let ctx = ui.ctx().clone();
    let width = args.width;
    let state = WindowState { maximized: args.maximized, active: !args.inactive };
    let outer = ctx.direction();
    direction::apply(&ctx, if args.rtl { Direction::Rtl } else { Direction::Ltr });
    ui.allocate_ui(vec2(width, 0.0), |ui| {
        ui.set_max_width(width);
        let menus = story_bar();
        let title = Title::new(args.title.as_str()).unsaved(args.unsaved);
        TitleBar::new(&frame).id_salt("craft-ui-title-bar-story").preview(state).separator(true).title(title).show_titled(
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
}
