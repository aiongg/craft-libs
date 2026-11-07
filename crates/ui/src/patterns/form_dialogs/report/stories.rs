//! Stories of `ReportDialog`.

use super::{ReportDialog, ReportNote};
use crate::contract::Bind;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use craft_theme::variant::Tone;
use egui::Ui;

#[story(ReportDialog<'static>, covers = [], drive = Drive::Never)]
fn report_dialog(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let notes = vec![
        ReportNote::new(Tone::Warn, "Font Garamond was substituted"),
        ReportNote::new(Tone::Info, "Page 4 has no bleed"),
        ReportNote::new(Tone::Bad, "Image logo.tif could not be read"),
    ];
    ReportDialog::new("Import Report", "Imported 12 pages.", notes).opened(Bind::bound(data.value("open", true))).show(ui).into()
}
