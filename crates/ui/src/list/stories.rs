//! Stories of the list family: `FindingsList`.

use super::{Finding, FindingRow, Findings, FindingsList, Severity};
use crate::contract::ShowCollection as _;
use crate::testing::show_as_only_item;
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

#[story(FindingRow<'static>, covers = [Default], drive = Drive::Never)]
fn finding_row(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let finding = Finding::new(Severity::Warning, "The font Avenir is missing");
    show_as_only_item(ui, FindingRow::new(&finding)).response.into()
}
