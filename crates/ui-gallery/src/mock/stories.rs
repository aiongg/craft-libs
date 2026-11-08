//! The story of the mock editor window.

use super::{EditorMock, MockState};
use crate::theme::{AppLayerEditor, Pick, Problems, ThemeSubject, ThemeWorkspace, TokenBrowser, TokenDetail};
use craft_theme::{AppSizes, Appearance, Contrast, Mode, Size, TextSize, builtins};
use craft_ui::contract::Bind;
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;
use std::sync::LazyLock;

#[story(EditorMock<'static>, covers = [], drive = Drive::Never)]
fn editor_window(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let state = data.value("mock", MockState::default());
    EditorMock::new("Editor", state).show(ui).into()
}

/// The built-in theme the theme panels' stories explain, with its explanation and contrast report.
fn workspace() -> &'static ThemeWorkspace {
    static WORKSPACE: LazyLock<ThemeWorkspace> = LazyLock::new(|| {
        let mut workspace = ThemeWorkspace::new();
        workspace.refresh(&ThemeSubject::new(&builtins()[0], appearance()));
        workspace
    });
    &WORKSPACE
}

fn appearance() -> Appearance {
    Appearance { mode: Mode::Light, contrast: Contrast::Standard, size: Size::Compact, text_size: TextSize::Small, reduced_motion: false }
}

#[story(TokenBrowser<'static>, covers = [], drive = Drive::Never)]
fn token_browser(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    if let Some(explained) = workspace().explained() {
        TokenBrowser::new(explained, Bind::bound(data.value("pick", None::<Pick>))).show(ui);
    }
    StoryOutput::default()
}

#[story(TokenDetail<'static>, covers = [], drive = Drive::Never)]
fn token_detail(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    if let Some(explained) = workspace().explained() {
        TokenDetail::new(explained).show(ui);
    }
    StoryOutput::default()
}

#[story(Problems<'static>, covers = [], drive = Drive::Never)]
fn problems(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    if let (Some(explained), Some(report)) = (workspace().explained(), workspace().report()) {
        Problems::new(report, explained, &ThemeSubject::new(&builtins()[0], appearance())).show(ui);
    }
    StoryOutput::default()
}

#[story(AppLayerEditor<'static>, covers = [], drive = Drive::Never)]
fn app_layer_editor(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    if let Some(explained) = workspace().explained() {
        AppLayerEditor::new(Bind::bound(data.value("sizes", AppSizes::default())), explained).show(ui);
    }
    StoryOutput::default()
}
