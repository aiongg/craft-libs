//! Toasts: posted from a context, announced once, expired by their timer.

use craft_ui::contract::toast::ToastHost;
use craft_ui::runtime::toast::{self, Toast};
use craft_ui::testing::{Frames, trial};
use egui::Ui;

#[trial]
fn a_toast_posts_announces_once_and_expires() {
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| ToastHost::show(ui);
    frames.idle(2, &mut show);
    toast::post(&frames.ctx, Toast::new("Saved").timeout(Some(1.0)));
    let mut announced = 0;
    for _ in 0..30 {
        frames.frame(vec![], &mut show);
        announced += craft_ui::runtime::frame::take_output(&frames.ctx).announcements.iter().filter(|text| text.as_str() == "Saved").count();
    }
    assert_eq!(announced, 1, "a toast is announced once");
    assert_eq!(toast::cards(&frames.ctx).len(), 1, "the toast is still shown before its timer ends");
    for _ in 0..90 {
        frames.frame(vec![], &mut show);
    }
    assert!(toast::cards(&frames.ctx).is_empty(), "the toast expired");
}
