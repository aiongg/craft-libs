//! Stories of `StatusReadout`.

use super::{StatusItem, StatusReadout, StatusValues};
use crate::bars::{Bar, BarKind};
use crate::contract::{Bind, ShowContainer as _};
use crate::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;
use std::time::Duration;

fn values() -> StatusValues {
    let mut values = StatusValues::default();
    values.document_sizes = Some((26_100_000, 31_400_000));
    values.efficiency = Some(100);
    values.timing = Some(Duration::from_millis(400));
    values.layer_count = Some(7);
    values
}

#[story(StatusReadout<'static>, covers = [], drive = Drive::Never)]
fn status_readout(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let values = values();
    Bar::new("Status", BarKind::Status)
        .show(ui, |ui, cx| {
            StatusReadout::new(Bind::bound(data.value("zoom", Some(66.67f64))), StatusItem::DocumentSizes, &values).show_in(ui, cx);
        })
        .output
        .into()
}
