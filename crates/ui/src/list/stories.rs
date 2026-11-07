//! Stories of the list family: `FindingsList` and `ListRow`.

use super::{Finding, FindingRow, Findings, FindingsList, ListRow, RowInk, Severity};
use crate::contract::{Props as _, Show as _, ShowCollection as _};
use crate::primitives::IconSpec;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

fn sample(count: usize) -> Vec<Finding> {
    let all = [
        Finding::new(Severity::Info, "Layer 3 has no name"),
        Finding::new(Severity::Warning, "The font Avenir is missing"),
        Finding::new(Severity::Error, "Page 2 overflows its frame"),
    ];
    all.into_iter().cycle().take(count).collect()
}

fn show(ui: &mut Ui, findings: &[Finding]) -> StoryOutput {
    let source = Findings::new(findings);
    FindingsList::new("Findings").show_items(ui, &source, &mut |key, _| FindingRow::new(&findings[*key])).into()
}

#[story(FindingsList<'static>, covers = [Default, Many], drive = Drive::Never)]
fn findings_list(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    show(ui, &sample(30))
}

#[story(FindingsList<'static>, covers = [One], drive = Drive::Never)]
fn findings_list_one(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    show(ui, &sample(1))
}

#[story(FindingsList<'static>, covers = [Empty], drive = Drive::Never)]
fn findings_list_empty(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    show(ui, &[])
}

#[story(ListRow<'static>, covers = [Default], drive = Drive::Activate)]
fn list_row(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    ListRow::new("Background").show(ui).response.into()
}

#[story(ListRow<'static>, covers = [LongText], drive = Drive::Never)]
fn list_row_long_text(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let label = "A layer whose name is far longer than the room its row gives the label";
    ListRow::new(label).detail("A second line under it").show(ui).response.into()
}

#[story(ListRow<'static>, covers = [], drive = Drive::Never)]
fn list_row_tree_group(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    ListRow::new("Group 1").depth(1).group(true).icon(IconSpec::svg(craft_behavior::IconId::new("eye"))).show(ui).response.into()
}

#[story(ListRow<'static>, covers = [], drive = Drive::Never)]
fn list_row_clipped_dimmed(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    ListRow::new("Shadow").clipped(true).ink(RowInk::Dimmed).italic(true).lines("Clipped to the layer below, hidden").show(ui).response.into()
}
