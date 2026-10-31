//! The appearance page in a harness.

use craft_theme::{ThemeDef, ThemeId, Variants, builtins};
use craft_ui::testing::a11y;
use craft_ui::theme::{Base, install};
use egui::Key;
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use super::*;

struct Page {
    client: SuiteClient,
    draft: SelectionChange,
    favorites: Vec<ThemeId>,
    changed: bool,
    toggled: Option<ThemeId>,
}

fn harness(draft: SelectionChange, favorites: Vec<ThemeId>) -> Harness<'static, Page> {
    let page = Page { client: SuiteClient::memory(), draft, favorites, changed: false, toggled: None };
    Harness::builder().with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, page: &mut Page| {
            let out = AppearancePage::new(&page.client, &mut page.draft).favorites(&page.favorites).show(ui);
            page.changed |= out.changed;
            page.toggled = page.toggled.take().or(out.favorite_toggled);
        },
        page,
    )
}

/// The first two built-in themes that have a light variant.
fn light_themes() -> Vec<&'static ThemeDef> {
    builtins().iter().filter(|t| t.variants != Variants::DarkOnly).take(2).collect()
}

#[allow(clippy::expect_used)]
fn chip<'h>(harness: &'h Harness<'_, Page>, name: &'h str) -> egui_kittest::Node<'h> {
    // The light row comes first.
    harness.get_all_by_role_and_label(Role::RadioButton, name).next().expect("a chip of that name")
}

// Picking a chip writes the light or dark slot into the draft only; the client is unchanged until
// the app applies.
#[test]
fn picking_a_chip_edits_the_draft_only() {
    let themes = light_themes();
    let mut harness = harness(SelectionChange::default(), Vec::new());
    harness.run();
    chip(&harness, &themes[1].name).click();
    harness.run();
    let page = harness.state();
    assert_eq!(page.draft.theme_light.as_ref(), Some(&themes[1].id));
    assert_eq!(page.draft.theme_dark, None);
    assert!(page.changed);
    assert_eq!(page.client.shown().theme_light, None);
}

// A chip row is one Tab stop; arrows move and pick, and the selected chip has the 2 px accent
// outline.
#[test]
fn chip_rows_are_radio_groups() {
    let themes = light_themes();
    let draft = SelectionChange { theme_light: Some(themes[0].id.clone()), theme_dark: Some(themes[0].id.clone()), ..SelectionChange::default() };
    let mut harness = harness(draft, Vec::new());
    harness.run();
    let stops = a11y::tab_order(&mut harness);
    let chips = stops.iter().filter(|stop| stop.role == Role::RadioButton).count();
    assert_eq!(chips, 2, "one Tab stop for the light row and one for the dark row: {stops:?}");

    chip(&harness, &themes[0].name).focus();
    harness.run();
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert_eq!(harness.state().draft.theme_light.as_ref(), Some(&themes[1].id));
    let accent = craft_ui::theme::current(&harness.ctx).color.accent.solid.to_egui();
    let (_, rings) = a11y::focus_rings(&harness).expect("the picked chip has the focus");
    assert!(rings.iter().any(|ring| ring.stroke.width == 2.0 && ring.stroke.color == accent), "the selected chip's 2 px accent outline: {rings:?}");
    assert!(chip(&harness, &themes[1].name).is_focused());
}

// A click on the star badge reports favorite_toggled and does not pick the theme.
#[test]
fn star_click_toggles_favorite_without_picking() {
    let themes = light_themes();
    let mut harness = harness(SelectionChange::default(), vec![themes[1].id.clone()]);
    harness.run();
    harness.get_all_by_role_and_label(Role::Button, &format!("Remove from Favorites: {}", themes[1].name)).next().expect("a star").click();
    harness.run();
    let page = harness.state();
    assert_eq!(page.toggled.as_ref(), Some(&themes[1].id));
    assert_eq!(page.draft, SelectionChange::default());
}
