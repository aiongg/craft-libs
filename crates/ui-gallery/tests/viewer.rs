//! The viewer's own logic: what it saves and restores, and how the filter narrows the component tree.

use craft_behavior::Selection;
use craft_theme::Mode;
use craft_ui::list::OpenSet;
use craft_ui::testing::{TRIALS, trial};
use craft_ui_gallery::viewer::{Catalog, ComponentInfo, GallerySettings, PanelShares, TreeKey, Viewer, rows};
use libtest_mimic::{Arguments, Trial};

fn components() -> Vec<ComponentInfo> {
    vec![
        ComponentInfo::new("craft_ui::buttons::button::Button<'_>", "craft_ui::buttons::button", vec!["button", "button_long_text"]),
        ComponentInfo::new("craft_ui::buttons::link::Link", "craft_ui::buttons::link", vec!["link"]),
        ComponentInfo::new("craft_ui::display::text::Text", "craft_ui::display::text", vec!["text"]),
        ComponentInfo::new("crate::Undeclared", "", vec!["undeclared"]),
    ]
}

fn viewer(settings: &GallerySettings) -> Viewer {
    Viewer::new(Catalog::listing(components()), settings)
}

#[trial]
fn a_saved_layout_restores_the_shares() {
    let mut shares = PanelShares::default();
    shares.columns = [1.0, 4.0, 2.5];
    shares.right = [3.0, 1.0];
    assert_eq!(PanelShares::load(&shares.save()), shares);
    assert_eq!(PanelShares::of_tree(&shares.tree()), Some(shares));
}

#[trial]
fn a_layout_that_cannot_be_read_costs_only_the_layout() {
    for layout in ["{not json", "", r#"{"columns":[1.0,0.0,1.0],"right":[1.0,1.0]}"#] {
        let mut settings = GallerySettings::default();
        settings.mode = "dark".to_owned();
        settings.filter = "link".to_owned();
        settings.layout = layout.to_owned();
        let restored = viewer(&settings);
        assert_eq!(restored.layout(), PanelShares::default(), "layout {layout:?}");
        assert_eq!(restored.appearance.mode, Mode::Dark);
        assert_eq!(restored.browser.filter, "link");
    }
}

#[trial]
fn settings_restore_the_state_they_were_saved_from() {
    let mut before = viewer(&GallerySettings::default());
    before.appearance.mode = Mode::Dark;
    before.browser.filter = "u".to_owned();
    before.browser.selection = Selection::single(TreeKey::Component("craft_ui::buttons::link::Link"));
    before.browser.open.set(&TreeKey::Module("craft_ui::buttons::link"), true);
    let saved = GallerySettings::of(&before);
    let text = serde_json::to_string(&saved).unwrap_or_default();
    let read: GallerySettings = serde_json::from_str(&text).unwrap_or_default();
    let after = viewer(&read);
    assert_eq!(GallerySettings::of(&after), saved);
    assert_eq!(after.selected_type(), Some("craft_ui::buttons::link::Link"));
    assert_eq!(after.open_modules(), vec!["craft_ui::buttons::link".to_owned()]);
}

#[trial]
fn a_name_this_build_does_not_know_keeps_the_default() {
    let mut settings = GallerySettings::default();
    settings.mode = "sepia".to_owned();
    settings.selected = "gone::Type".to_owned();
    let restored = viewer(&settings);
    assert_eq!(restored.appearance, viewer(&GallerySettings::default()).appearance);
    assert_eq!(restored.selected_type(), None);
}

#[trial]
fn the_filter_shows_the_groups_that_hold_a_match_open() {
    let closed = OpenSet::new();
    let all = rows(&components(), "", &closed);
    assert_eq!(all.len(), 4, "closed groups show alone");
    assert!(all.iter().all(|row| row.group));

    let by_type = rows(&components(), "LINK", &closed);
    let labels: Vec<_> = by_type.iter().map(|row| row.label.as_str()).collect();
    assert_eq!(labels, ["craft_ui::buttons::link", "Link"]);

    let by_story = rows(&components(), "long_text", &closed);
    assert_eq!(by_story.last().map(|row| row.label.as_str()), Some("Button<'_>"));

    let none = rows(&components(), "no such component", &closed);
    assert!(none.is_empty());
}

fn main() {
    let args = Arguments::from_args();
    let trials: Vec<Trial> = TRIALS
        .iter()
        .map(|trial| {
            let run = trial.run;
            Trial::test(trial.full_name(), move || {
                run();
                Ok(())
            })
        })
        .collect();
    libtest_mimic::run(&args, trials).exit();
}
