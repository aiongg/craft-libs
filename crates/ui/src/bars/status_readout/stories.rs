//! Stories of `StatusReadout`.

use super::{StatusItem, StatusReadout, StatusValues};
use crate::bars::{Bar, BarKind};
use crate::contract::{Bind, ShowContainer as _};
use crate::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;
use std::time::Duration;

fn values() -> StatusValues {
    StatusValues {
        document_sizes: Some((26_100_000, 31_400_000)),
        efficiency: Some(100),
        timing: Some(Duration::from_millis(400)),
        layer_count: Some(7),
        ..StatusValues::default()
    }
}

#[story(StatusReadout<'static>, covers = [], drive = Drive::Never)]
fn status_readout(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let values = values();
    Bar::new("Status", BarKind::Status)
        .show(ui, |ui, _| {
            StatusReadout::new(Bind::bound(data.value("zoom", Some(66.67f64))), StatusItem::DocumentSizes, &values).show(ui);
        })
        .output
        .into()
}
