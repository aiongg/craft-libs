//! `measure_slot`: a closure measured in a measuring run.

use crate::contract::Ui;
use craft_ui_rt::Vec2;

/// Runs `body` as a measuring run in a column `width` wide and returns the size it took. Nothing is placed in `ui`
/// and no node is reached: kept children answer from their size cache. The closure sees the env of `ui`.
pub fn measure_slot(ui: &mut Ui, width: f32, body: impl FnOnce(&mut Ui)) -> Vec2 {
    let mut body = Some(body);
    ui.measure_column(width, &mut |ui| {
        if let Some(body) = body.take() {
            body(ui);
        }
    })
}
