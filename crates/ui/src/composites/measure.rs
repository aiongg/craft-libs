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

/// The groups shown on one pass, to catch two of one id.
#[derive(Clone, Debug, Default)]
struct Seen {
    pass: u64,
    ids: Vec<Id>,
}

fn stack_id(ctx: &Context, kind: &'static str) -> Id {
    Id::unique("craft_ui::composites::measure").with(kind).with(ctx.viewport_id())
}

/// Stores `measured` under `id` and, when it differs from the `used` width by more than
/// `tolerance`, discards the pass so the next one uses it.
fn remember(ctx: &Context, id: Id, used: f32, measured: f32, tolerance: f32, reason: &'static str) {
    let stored = ctx.data(|d| d.get_temp::<f32>(id));
    if stored.is_none_or(|s| (s - measured).abs() > tolerance) {
        ctx.data_mut(|d| d.insert_temp(id, measured));
    }
    if (used - measured).abs() > tolerance {
        ctx.request_discard(reason);
    }
}

/// Whether a group of `id` was already shown on this pass; records it.
fn shown_before(ctx: &Context, kind: &'static str, id: Id) -> bool {
    let pass = ctx.cumulative_pass_nr();
    let key = stack_id(ctx, kind).with("seen");
    ctx.data_mut(|d| {
        let seen = d.get_temp_mut_or_default::<Seen>(key);
        if seen.pass != pass {
            *seen = Seen { pass, ids: Vec::new() };
        }
        let before = seen.ids.contains(&id);
        if !before {
            seen.ids.push(id);
        }
        before
    })
}

/// Runs `add_contents` as a group of `kind` (`"FieldGroup"`, `"PropertyGroup"`; one context
/// stack per kind).
///
/// Two groups of one kind and `id_salt` under one parent would share the stored column and each
/// overwrite the other's, discarding a pass every frame: in a debug build that fails a
/// `debug_assert`; in a release build the second group uses the first's column and stores
/// nothing.
pub(super) fn group<R>(ui: &mut Ui, kind: &'static str, id_salt: IdSalt, add_contents: impl FnOnce(&mut Ui) -> R) -> egui::InnerResponse<R> {
    let id = ui.make_persistent_id(id_salt).with(kind).with("column");
    let ctx = ui.ctx().clone();
    let duplicate = shown_before(&ctx, kind, id);
    debug_assert!(
        !duplicate,
        "craft-ui: two {kind}s share one id under the same parent ({id:?}); give each its own id_salt, or each frame is laid out twice"
    );
    let column = ctx.data(|d| d.get_temp::<f32>(id));
    let stack = stack_id(&ctx, kind);
    ctx.data_mut(|d| d.get_temp_mut_or_default::<Vec<Collecting>>(stack).push(Collecting { layer: ui.layer_id(), column, widest: 0.0 }));
    let inner = ui.scope_builder(UiBuilder::new().id_salt(id_salt), add_contents);
    let widest = ctx.data_mut(|d| d.get_temp_mut_or_default::<Vec<Collecting>>(stack).pop()).map_or(0.0, |c| c.widest);
    if !duplicate {
        remember(&ctx, id, column.unwrap_or(0.0), widest, 0.1, kind);
    }
    inner
}

/// The column of a row whose label is `own` wide, and whether it is in a group of `kind`: the
/// group's measured column, or `own` outside one.
pub(super) fn column(ui: &Ui, kind: &'static str, own: f32) -> (f32, bool) {
    let layer = ui.layer_id();
    let stack = stack_id(ui.ctx(), kind);
    ui.data_mut(|d| match d.get_temp_mut_or_default::<Vec<Collecting>>(stack).last_mut().filter(|g| g.layer == layer) {
        Some(group) => {
            group.widest = group.widest.max(own);
            (group.column.unwrap_or(own), true)
        }
        None => (own, false),
    })
}
