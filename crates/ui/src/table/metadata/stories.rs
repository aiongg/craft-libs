//! Stories of `MetadataTable`: grouped fields, the check column, a search with no match and no fields at all.

use super::{MetadataGroup, MetadataRow, MetadataTable};
use crate::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

fn groups() -> Vec<MetadataGroup> {
    vec![
        MetadataGroup::new(
            "Camera",
            vec![
                MetadataRow::new("Make", "Hasselblad"),
                MetadataRow::new("Exposure Time", "1/250 s").key("exif:ExposureTime"),
                MetadataRow::new("Description", "A long free-text description that runs past the width of its column and wraps"),
            ],
        ),
        MetadataGroup::new("XMP", vec![MetadataRow::new("Creator", "A. Photographer").key("dc:creator"), MetadataRow::new("File", "IMG_0001.RAW").locked()]),
    ]
}

#[story(MetadataTable<'static>, covers = [Default], drive = Drive::Never)]
fn metadata_table(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    MetadataTable::new("Metadata", &groups()).show(ui).into()
}

#[story(MetadataTable<'static>, covers = [], drive = Drive::Never)]
fn metadata_table_chooser(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let mut filter = data.value("filter", String::new()).clone();
    let mut chosen = data.value("chosen", vec!["Make".to_owned()]).clone();
    let out = MetadataTable::new("Metadata", &groups()).filter(&mut filter).choose(&mut chosen).show(ui);
    *data.value("filter", String::new()) = filter;
    *data.value("chosen", Vec::new()) = chosen;
    out.into()
}

#[story(MetadataTable<'static>, covers = [Empty], drive = Drive::Never)]
fn metadata_table_no_match(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let mut filter = data.value("filter", "zzz".to_owned()).clone();
    let out = MetadataTable::new("Metadata", &groups()).filter(&mut filter).show(ui);
    *data.value("filter", String::new()) = filter;
    out.into()
}
