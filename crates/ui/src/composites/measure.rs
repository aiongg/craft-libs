//! The widest-label measuring shared by [`FieldGroup`](super::FieldGroup) and
//! [`PropertyGroup`](super::PropertyGroup): a group keeps the widest label of the last pass,
//! its rows use it, and a change discards the pass so the column never shows at a stale width.

use egui::{Context, Id, IdSalt, LayerId, Ui, UiBuilder};

/// A group whose rows are being laid out.
#[derive(Clone, Copy, Debug)]
struct Collecting {
    layer: LayerId,
    /// The column measured on the last pass.
    column: Option<f32>,
    /// The widest label so far on this pass.
    widest: f32,
}

fn stack_id(ctx: &Context, tag: &'static str) -> Id {
    Id::unique(tag).with(ctx.viewport_id())
}

/// Runs `add_contents` as a group of kind `tag` (one context stack per kind).
pub(super) fn group<R>(ui: &mut Ui, tag: &'static str, id_salt: IdSalt, add_contents: impl FnOnce(&mut Ui) -> R) -> egui::InnerResponse<R> {
    let id = ui.make_persistent_id(id_salt).with(tag).with("column");
    let ctx = ui.ctx().clone();
    let column = ctx.data(|d| d.get_temp::<f32>(id));
    let stack = stack_id(&ctx, tag);
    ctx.data_mut(|d| d.get_temp_mut_or_default::<Vec<Collecting>>(stack).push(Collecting { layer: ui.layer_id(), column, widest: 0.0 }));
    let inner = ui.scope_builder(UiBuilder::new().id_salt(id_salt), add_contents);
    let widest = ctx.data_mut(|d| d.get_temp_mut_or_default::<Vec<Collecting>>(stack).pop()).map_or(0.0, |c| c.widest);
    if column.is_none_or(|c| (c - widest).abs() > 0.1) {
        ctx.data_mut(|d| d.insert_temp(id, widest));
        if column.is_some() || widest > 0.0 {
            ctx.request_discard(tag);
        }
    }
    inner
}

/// The column of a row whose label is `own` wide, and whether it is in a group of kind `tag`:
/// the group's measured column, or `own` outside one.
pub(super) fn column(ui: &Ui, tag: &'static str, own: f32) -> (f32, bool) {
    let layer = ui.layer_id();
    let stack = stack_id(ui.ctx(), tag);
    ui.data_mut(|d| match d.get_temp_mut_or_default::<Vec<Collecting>>(stack).last_mut().filter(|g| g.layer == layer) {
        Some(group) => {
            group.widest = group.widest.max(own);
            (group.column.unwrap_or(own), true)
        }
        None => (own, false),
    })
}
