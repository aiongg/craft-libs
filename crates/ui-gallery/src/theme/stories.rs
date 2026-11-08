//! Stories of the theme panels, over the first built-in theme.

use super::{AppLayerEditor, Pick, Problems, ThemeSubject, ThemeWorkspace, TokenBrowser, TokenDetail};
use craft_theme::report::ContrastReport;
use craft_theme::{AppSizes, Appearance, Contrast, Explained, Mode, Size, TextSize, builtins};
use craft_ui::contract::Bind;
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;
use std::sync::LazyLock;

fn appearance() -> Appearance {
    Appearance { mode: Mode::Light, contrast: Contrast::Standard, size: Size::Compact, text_size: TextSize::Small, reduced_motion: false }
}

/// The explanation and contrast report of the first built-in theme; a built-in theme always explains.
fn explained() -> (&'static Explained, &'static ContrastReport) {
    static WORKSPACE: LazyLock<ThemeWorkspace> = LazyLock::new(|| {
        let mut workspace = ThemeWorkspace::new();
        workspace.refresh(&ThemeSubject::new(&builtins()[0], appearance()));
        workspace
    });
    let explained = WORKSPACE.explained().expect("a built-in theme explains");
    (explained, WORKSPACE.report().expect("a built-in theme has a contrast report"))
}

#[story(TokenBrowser<'static>, covers = [])]
fn token_browser(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    TokenBrowser::new(explained().0, Bind::bound(data.value("pick", None::<Pick>))).show(ui);
    StoryOutput::default()
}

// A read-only view of the pick: nothing in it takes input.
#[story(TokenDetail<'static>, covers = [], drive = Drive::Never)]
fn token_detail(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    TokenDetail::new(explained().0).show(ui);
    StoryOutput::default()
}

// It reports a pick through its output and binds no value, so there is no value for a drive to change.
#[story(Problems<'static>, covers = [], drive = Drive::Never)]
fn problems(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let (explained, report) = explained();
    Problems::new(report, explained, &ThemeSubject::new(&builtins()[0], appearance())).show(ui);
    StoryOutput::default()
}

// Its rows only appear for a pick made in the browser, and none is made here.
#[story(AppLayerEditor<'static>, covers = [], drive = Drive::Never)]
fn app_layer_editor(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    AppLayerEditor::new(Bind::bound(data.value("sizes", AppSizes::default())), explained().0).show(ui);
    StoryOutput::default()
}
