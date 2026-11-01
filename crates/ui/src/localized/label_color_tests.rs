use std::collections::HashSet;

use craft_theme::LabelColor;

use super::{LabelColorLabels, Labels as _};

#[test]
fn every_label_colour_has_its_own_name_and_none_is_none() {
    let labels = LabelColorLabels::english();
    let names: HashSet<_> = LabelColor::ALL.into_iter().map(|c| labels.name(Some(c)).to_owned()).collect();
    assert_eq!(names.len(), LabelColor::ALL.len());
    assert_eq!(labels.name(Some(LabelColor::Seafoam)), "Seafoam");
    assert_eq!(labels.name(None), "None");
}
