//! Story of [`NoticeCenter`]: three notices, one with Don't Show Again, one error.

use craft_theme::Tone;
use egui::{Rect, Ui, vec2};

use super::NoticeCenter;
use crate::story::{Level, NoArgs, Story, Surface};
use crate::typed_story;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["NoticeCenter"];

pub(crate) fn stories() -> Vec<Story> {
    vec![typed_story!("patterns/shell/notice-center", Level::Patterns, "NoticeCenter", "Notices", NoArgs {}, show_center).on(Surface::Canvas)]
}

/// The center in the bottom corner of the story's area; notices that ended stay gone, and a
/// Reset button posts them again.
fn show_center(ui: &mut Ui, _args: &mut NoArgs) {
    let key = ui.scope_id().with("notice-center-story");
    let mut center: Option<NoticeCenter> = ui.data(|d| d.get_temp(key));
    let area = ui.available_rect_before_wrap();
    let rect = Rect::from_min_size(area.min, vec2(area.width(), area.height().min(360.0)));
    ui.allocate_rect(rect, egui::Sense::hover());
    let center = center.get_or_insert_with(|| {
        let mut center = NoticeCenter::new().auto_hide(None);
        let _ = center.post("Pasted from the clipboard", Vec::new(), Tone::Neutral, None, &[]);
        let _ = center.post(
            "Profile mismatch",
            vec!["The document uses its own profile.".to_owned()],
            Tone::Warn,
            Some("profile-mismatch".to_owned()),
            &[],
        );
        if let Some(id) = center.post("Couldn't save the document", vec!["The disk is full.".to_owned()], Tone::Bad, None, &[]).id {
            center.keep(id);
        }
        center
    });
    center.show(ui.ctx(), rect);
    let snapshot = center.clone();
    ui.data_mut(|d| d.insert_temp(key, snapshot));
}
