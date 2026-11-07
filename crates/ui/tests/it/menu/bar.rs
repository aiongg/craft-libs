//! The menu bar's own rules: which titles show in a room, and which help entries a query finds.

use craft_ui::menu::bar::{MAX_RESULTS, TitleFit, fit_titles, search_order};
use craft_ui::testing::trial;

const TITLES: [f32; 3] = [40.0, 50.0, 60.0];
const MORE: f32 = 20.0;

#[trial]
fn titles_that_do_not_fit_go_into_an_overflow_menu() {
    let fit = |room| {
        let TitleFit { shown, overflow, .. } = fit_titles(&TITLES, MORE, room);
        (shown, overflow)
    };
    assert_eq!(fit(150.0), (3, false));
    assert_eq!(fit(120.0), (2, true));
    assert_eq!(fit(50.0), (0, true), "none fit beside the overflow title");
}

#[trial]
fn help_search_ranks_by_label_and_matches_the_path_and_the_english_label() {
    let texts = [["Curves…", "", "Image › Adjustments"], ["Levels…", "", "Image › Adjustments"], ["All", "", "Select"], ["Level Horizon", "", "Select"]];
    assert_eq!(search_order(" LEV ", &texts), [1, 3], "a label starting with the query, in menu order");
    assert_eq!(search_order("adjust", &texts), [0, 1], "the path matches");
    assert_eq!(search_order("levels", &[["Niveaux…", "Levels…", "Image"]]), [0], "the English label matches");
    assert_eq!(search_order("level horizon", &texts), [3], "an exact label ranks first and every word must occur");
    assert!(search_order("  ", &texts).is_empty());
}

#[trial]
fn help_search_lists_at_most_the_limit() {
    let labels: Vec<String> = (0..40).map(|at| format!("Row {at}")).collect();
    let texts: Vec<[&str; 3]> = labels.iter().map(|label| [label.as_str(), "", "Many"]).collect();
    assert_eq!(search_order("row", &texts).len(), MAX_RESULTS);
}
