//! Stories of the appearance page and the theme chip.

use std::sync::{Arc, Mutex, PoisonError};

use craft_theme::{ThemeId, builtins};
use craft_ui::story::{Level, Story, StoryArgs};
use craft_ui::typed_story;
use egui::Ui;

use super::{AppearancePage, ThemeChip};
use crate::{SelectionChange, SuiteClient};

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["AppearancePage", "ThemeChip"];

#[derive(StoryArgs, Debug, PartialEq)]
struct PageArgs {
    favorites: bool,
    #[arg(name = "ui font size row")]
    text_size: bool,
    #[arg(range = 360.0..=720.0)]
    width: f32,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct ChipArgs {
    /// The built-in theme, by its place in the picker order.
    #[arg(range = 0..=9)]
    theme: usize,
    dark: bool,
}

pub(super) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "patterns/appearance-page/page",
            Level::Patterns,
            "AppearancePage",
            "Page",
            PageArgs { favorites: true, text_size: true, width: 560.0 },
            show_page
        ),
        typed_story!("composites/theme-chip/states", Level::Composites, "ThemeChip", "States", ChipArgs { theme: 1, dark: false }, show_chips),
    ]
}

/// The page's state: a suite in memory, the draft the page edits and the favourites it toggles.
struct Page {
    client: SuiteClient,
    draft: SelectionChange,
    favorites: Vec<ThemeId>,
    /// The `favorites` arg the favourites were last set from.
    seeded: bool,
}

type Shared = Arc<Mutex<Page>>;

/// The appearance page over `SuiteClient::memory()`. Picks edit the draft, which is shown under
/// the page; nothing reaches a suite on disk.
fn show_page(ui: &mut Ui, args: &mut PageArgs) {
    let key = ui.make_persistent_id("craft_theme_store::ui::page-story");
    let shared: Shared = ui.data(|d| d.get_temp::<Shared>(key)).unwrap_or_else(|| {
        Arc::new(Mutex::new(Page { client: SuiteClient::memory(), draft: SelectionChange::default(), favorites: Vec::new(), seeded: false }))
    });
    {
        let mut page = shared.lock().unwrap_or_else(PoisonError::into_inner);
        if page.seeded != args.favorites {
            page.favorites = if args.favorites { builtins().iter().skip(1).step_by(3).map(|t| t.id.clone()).collect() } else { Vec::new() };
            page.seeded = args.favorites;
        }
        let page = &mut *page;
        ui.allocate_ui(egui::vec2(args.width.min(ui.available_width()), 0.0), |ui| {
            let out = AppearancePage::new("appearance", &page.client, &mut page.draft).favorites(&page.favorites).text_size(args.text_size).show(ui);
            if let Some(id) = out.favorite_toggled {
                if let Some(at) = page.favorites.iter().position(|f| *f == id) {
                    page.favorites.remove(at);
                } else {
                    page.favorites.push(id);
                }
            }
        });
        ui.separator();
        ui.label(format!("Draft: {:?}", page.draft));
    }
    ui.data_mut(|d| d.insert_temp(key, shared));
}

/// One theme's chip in its four states: plain, selected, favourite, and both.
fn show_chips(ui: &mut Ui, args: &mut ChipArgs) {
    let themes = builtins();
    let theme = &themes[args.theme.min(themes.len() - 1)];
    ui.horizontal(|ui| {
        for (state, (selected, favorite)) in [(false, false), (true, false), (false, true), (true, true)].into_iter().enumerate() {
            // A chip's id comes from its theme and mode, so chips of one theme need a scope each.
            ui.push_id(state, |ui| ThemeChip::new(theme, args.dark).selected(selected).favorite(favorite).show(ui));
        }
    });
}
