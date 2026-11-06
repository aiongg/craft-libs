//! Story of the align bar: the six align buttons and the ⋯ menu, with the distribute commands in
//! the menu only or also as a second row of buttons.

use egui::Ui;

use super::{AlignAction, AlignBar, AlignLabels, DistributeButtons};
use crate::localized::Labels as _;
use crate::primitives::{Content, Text};
use crate::story::{Level, Story, StoryArgs, StoryChoice, StoryLabels};
use crate::typed_story;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["AlignBar"];

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum DistributeArg {
    Menu,
    Row,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct AlignBarArgs {
    distribute: DistributeArg,
    #[arg(name = "distribute enabled")]
    distribute_enabled: bool,
}

pub(crate) fn stories() -> Vec<Story> {
    vec![typed_story!(
        "patterns/align-bar/bar",
        Level::Patterns,
        "AlignBar",
        "Align bar",
        AlignBarArgs { distribute: DistributeArg::Menu, distribute_enabled: true },
        show_align_bar
    )]
}

/// The bar with every align command enabled; distribute commands are enabled with the
/// `distribute enabled` argument (they need three layers). The last pick shows under the bar.
fn show_align_bar(ui: &mut Ui, args: &mut AlignBarArgs) {
    let key = ui.make_persistent_id("craft_ui::patterns::align_stories::picked");
    let distribute = match args.distribute {
        DistributeArg::Menu => DistributeButtons::Menu,
        DistributeArg::Row => DistributeButtons::Row,
    };
    let distribute_enabled = args.distribute_enabled;
    let bar = AlignBar::new("align-story").distribute(distribute).enabled(|action| action.is_align() || distribute_enabled).show(ui);
    if let Some(action) = bar.picked {
        ui.data_mut(|d| d.insert_temp(key, action));
    }
    if let Some(action) = ui.data(|d| d.get_temp::<AlignAction>(key)) {
        let labels = AlignLabels::of(ui);
        let text = craft_i18n::fmt(&StoryLabels::of(ui).picked, &[("action", action.label(&labels))]);
        ui.add(Text::new(text).content(Content::Muted));
    }
}
