//! A window opened inside an embedded shell shows in the shell's region: no OS window opens, so the host window is
//! never blocked by an app-modal window it cannot reach.

use crate::windows_kit::Windows;
use craft_ui::contract::Show as _;
use craft_ui::display::Text;
use craft_ui::runtime::env::WindowKey;
use craft_ui::shell::{AppShell, ViewportWindow};
use craft_ui::testing::trial;
use egui::{Rect, Ui, ViewportId, pos2};

const FRAMES: usize = 4;

fn window(ui: &mut Ui) {
    let _ = ViewportWindow::new("Export Log").show(ui, |ui, _| {
        let _ = Text::new("The export finished.").show(ui);
    });
}

fn children(windows: &Windows, ui: &mut dyn FnMut(&mut Ui)) -> usize {
    let mut most = 0;
    for _ in 0..FRAMES {
        let out = windows.frame(Vec::new(), Vec::new(), &mut *ui);
        most = most.max(out.builders.iter().filter(|(id, _)| *id != ViewportId::ROOT).count());
    }
    most
}

#[trial]
fn a_viewport_window_in_an_embedded_shell_opens_no_os_window() {
    let embedded = children(&Windows::new(), &mut |ui| {
        let rect = Rect::from_min_max(pos2(10.0, 10.0), pos2(390.0, 290.0));
        AppShell::embedded(WindowKey::new("card")).body(|ui, _, _: &mut ()| window(ui)).show_at(ui, rect, &mut ());
    });
    assert_eq!(embedded, 0, "the window shows in the shell's region");
    let plain = children(&Windows::new(), &mut |ui| window(ui));
    assert_eq!(plain, 1, "outside an embedded shell the same window is an OS window");
}
