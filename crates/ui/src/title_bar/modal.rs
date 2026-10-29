//! Keeping the bar draggable under a modal.
//!
//! An `egui::Modal` takes the input of everything below its backdrop, the bar's drag area
//! included. After all windows are drawn, the shell calls [`TitleBar::modal_drag_overlay`] while
//! a modal is open: it puts a layer above the modal over the bar's free gap (the one the last pass
//! measured) that moves the window.

use std::hash::Hash;

use egui::{Area, Context, Id, Order, PointerButton, Popup, Rect, Sense};

use super::{TitleBar, captions, drag, measure, resize};

/// The rects of the modal windows open in one window, which the drag overlay must not cover: a
/// dialog dragged into the title bar stays in front. Each modal window (a `Dialog`, an embedded
/// `PreferencesWindow`) calls [`push`](Self::push) on every frame it is open; the shell
/// [`take`](Self::take)s them for [`TitleBar::modal_drag_overlay`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ModalRects(Vec<Rect>);

/// The rects pushed on one pass.
#[derive(Clone, Debug, Default)]
struct Pushed {
    pass: u64,
    rects: Vec<Rect>,
}

fn key(window: impl Hash + std::fmt::Debug) -> Id {
    captions::window_scope(window).with("modal_rects")
}

impl ModalRects {
    /// Records that a modal window of `window` (the bar's `id_salt`, or its viewport id without
    /// one) covers `rect` on this pass.
    pub fn push(ctx: &Context, window: impl Hash + std::fmt::Debug, rect: Rect) {
        let (id, pass) = (key(window), ctx.cumulative_pass_nr());
        ctx.data_mut(|d| {
            let pushed = d.get_temp_mut_or_default::<Pushed>(id);
            if pushed.pass != pass {
                *pushed = Pushed { pass, rects: Vec::new() };
            }
            pushed.rects.push(rect);
        });
    }

    /// The rects pushed for `window` on this pass.
    pub fn take(ctx: &Context, window: impl Hash + std::fmt::Debug) -> Self {
        let (id, pass) = (key(window), ctx.cumulative_pass_nr());
        Self(ctx.data_mut(|d| d.remove_temp::<Pushed>(id)).filter(|p| p.pass == pass).map(|p| p.rects).unwrap_or_default())
    }

    /// The rects.
    pub fn rects(&self) -> &[Rect] {
        &self.0
    }

    fn intersects(&self, rect: Rect) -> bool {
        self.0.iter().any(|r| r.intersects(rect))
    }
}

impl TitleBar<'_> {
    /// Keeps the window of `window` (the bar's `id_salt`, or its viewport id without one) draggable
    /// by its bar's free gap while a modal is open. Call it after every window is drawn, only
    /// while a modal is open: a primary-button drag in the gap moves the window and a double-click
    /// maximizes or restores it.
    ///
    /// It does nothing when the last pass drew no titled bar (none, or under [`Frame::Native`]),
    /// in full screen, while a popup is open, or when a rect of `rects` meets the gap.
    ///
    /// [`Frame::Native`]: super::Frame::Native
    pub fn modal_drag_overlay(ctx: &Context, window: impl Hash + std::fmt::Debug, rects: &ModalRects) {
        let scope = captions::window_scope(window);
        let Some(gap) = measure::drawn(ctx, scope) else { return };
        let fullscreen = ctx.input(|i| i.viewport().fullscreen).unwrap_or(false);
        if !gap.is_positive() || fullscreen || Popup::is_any_open(ctx) || rects.intersects(gap) {
            return;
        }
        Area::new(scope.with("modal_drag")).order(Order::Tooltip).fixed_pos(gap.min).movable(false).sense(Sense::hover()).show(ctx, |ui| {
            let response = ui.allocate_rect(gap, Sense::click_and_drag());
            if response.drag_started_by(PointerButton::Primary) {
                resize::start_os_move(ctx);
            }
            if response.double_clicked() {
                drag::double_clicked(ctx);
            }
        });
    }
}
