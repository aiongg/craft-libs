//! Sizing passes and `request_discard`.

use egui::{Context, Layout, Rect, Ui, UiBuilder, Vec2};

/// The reason of the rerun that delivers an update a given binding held in the pass before. That rerun writes the update to
/// the host, so the purity check of the suite's fixture lets it change state.
pub(crate) const HELD_UPDATE_RERUN: &str = "craft_ui: an update is held for the next pass";

/// The reason of the rerun layout memory asks for when a kept size changed.
pub(crate) const LAYOUT_RERUN: &str = "craft_ui: layout memory changed";

/// The pass that asked for a discard last, and whether layout memory was its only reason.
#[derive(Clone, Copy)]
struct Asked {
    pass: u64,
    layout_only: bool,
}

fn asked_id(ctx: &Context) -> egui::Id {
    egui::Id::NULL.with(("craft_ui::discard_asked", super::pass::current_viewport(ctx)))
}

fn running_pass(ctx: &Context) -> u64 {
    super::pass::pass_nr(ctx, super::pass::current_viewport(ctx))
}

/// Asks egui to discard the running pass and run it again.
pub(crate) fn request_discard(ctx: &Context, reason: &'static str) {
    let (id, pass, layout) = (asked_id(ctx), running_pass(ctx), reason == LAYOUT_RERUN);
    ctx.data_mut(|data| {
        let asked = data.get_temp_mut_or(id, Asked { pass, layout_only: layout });
        if asked.pass == pass {
            asked.layout_only &= layout;
        } else {
            *asked = Asked { pass, layout_only: layout };
        }
    });
    ctx.request_discard(reason);
}

/// Whether the running pass reruns a discarded one, and something other than layout memory asked for it (an edit, a
/// held update, egui itself): the rerun may show state the discarded pass did not.
pub(crate) fn rerun_beyond_layout(ctx: &Context) -> bool {
    if ctx.current_pass_index() == 0 {
        return false;
    }
    let (id, pass) = (asked_id(ctx), running_pass(ctx));
    let asked = ctx.data(|data| data.get_temp::<Asked>(id));
    !asked.is_some_and(|asked| asked.pass + 1 == pass && asked.layout_only)
}

/// Whether a discard requested in the running pass would be honoured: egui runs at most `Options::max_passes` passes a
/// frame.
pub(crate) fn discardable(ctx: &Context) -> bool {
    ctx.current_pass_index() + 1 < ctx.options(|options| options.max_passes.get())
}

/// Runs `f` in an invisible sizing pass `width` wide, from the parent's cursor, and returns what it took.
/// The parent allocates nothing.
pub(crate) fn run_sized<R>(ui: &mut Ui, width: f32, layout: Layout, f: impl FnOnce(&mut Ui) -> R) -> (R, Vec2) {
    let room = Rect::from_min_size(ui.cursor().min, Vec2::new(width, f32::INFINITY));
    let mut child = ui.new_child(UiBuilder::new().max_rect(room).layout(layout).sizing_pass().invisible());
    let inner = f(&mut child);
    (inner, child.min_rect().size())
}
