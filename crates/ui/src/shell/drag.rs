//! `WindowDrag`: the strip of a title bar that moves an undecorated window.

use crate::contract::{Common, Component, Cx, MeasureCx, NoValue, Outcome, PartsCx, Pointer as _, Props as _, Role};
use crate::runtime::windows::{WindowAction, WindowHost};
use crate::shell::title_bar::set_window_movable;
use crate::style::{ComponentStyle, StyleCx};
use craft_behavior::{Metrics, PartKey, PointerButton, PressPhase, Sense};
use craft_host::target::WindowState;
use egui::{Rect, Response, Vec2, ViewportId};

#[cfg(feature = "testing")]
mod stories;

/// The strip.
const STRIP: PartKey = PartKey::new("strip");

/// The keyboard path of the strip.
const REASON: &str = "the operating system moves an undecorated window";

/// The style of the drag strip.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct WindowDragStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// What the strip reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct WindowDragOutput {
    /// The response of the strip's root.
    pub response: Response,
    /// A primary press is held on the strip: from its press until its release, or until the operating system
    /// takes over with a move.
    pub pressed: bool,
    /// A second press of a double click went down on the strip. The caller maps it to a window command with
    /// `craft_window::title_bar::double_click_command`.
    pub double_clicked: bool,
}

impl Outcome for WindowDragOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// The strip of a title bar that moves an undecorated window: a primary press that moves past the click distance
/// hands the move to the operating system. The strip is a click-only area, so show it before the bar's controls: a
/// press on a control goes to the control.
///
/// - No move and no double click in full screen.
/// - It records whether the operating system may move the window from the title-bar band
///   (`shell::title_bar::window_movable`): not while the pointer is on a control or a panel over the strip.
#[must_use = "show the strip with `show_at`"]
#[derive(crate::contract::Component)]
pub struct WindowDrag {
    #[common]
    common: Common,
    viewport: ViewportId,
    live: bool,
    pressed: bool,
    double_clicked: bool,
}

impl WindowDrag {
    /// The strip of the window of `viewport`.
    pub fn new(viewport: ViewportId) -> Self {
        WindowDrag { common: Common::new(), viewport, live: false, pressed: false, double_clicked: false }.id_salt(("window_drag", viewport))
    }
}

impl Component for WindowDrag {
    const ROLE: Role = Role::None;
    const STOPS: bool = false;
    const OVERLAYS: bool = false;
    const SHOWS_LABEL: bool = false;
    type Style = WindowDragStyle;
    type Output = WindowDragOutput;

    fn style(_: &StyleCx<'_>, _: WindowDragStyleKey) -> WindowDragStyle {
        WindowDragStyle { metrics: Metrics::new() }
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::ZERO
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        let ctx = cx.ui.ctx().clone();
        let shell = crate::runtime::windows::shell_of(&ctx, self.viewport);
        self.live = shell.window.state != WindowState::Fullscreen;
        if self.live {
            // Click and drag, so a press that stops being a click is seen; the press-target rule hands a press on
            // a control shown after the strip to the control.
            cx.part(STRIP, rect).sense(Sense::CLICK | Sense::DRAG).exclusive().pointer_only(REASON);
        }
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let ctx = cx.core.ctx.clone();
        let rect = cx.core.rect;
        let covered = ctx.pointer_hover_pos().is_some_and(|at| rect.contains(at)) && cx.hover_pos(STRIP).is_none();
        set_window_movable(&ctx, !covered);
        self.pressed = false;
        self.double_clicked = false;
        if !self.live {
            return;
        }
        match cx.part(STRIP).press.phase {
            PressPhase::Down { start, .. } if start.button == PointerButton::Primary => {
                self.pressed = true;
                self.double_clicked = start.clicks >= 2;
            }
            PressPhase::DragStart { start, .. } if start.button == PointerButton::Primary => {
                self.pressed = !WindowHost::of(&ctx).act(self.viewport, WindowAction::StartMove);
            }
            PressPhase::Drag { start, .. } if start.button == PointerButton::Primary => self.pressed = true,
            _ => {}
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> WindowDragOutput {
        WindowDragOutput { response: cx.response.clone(), pressed: self.pressed, double_clicked: self.double_clicked }
    }
}
