//! The overview (egui_kittest).

use craft_ui::testing::{HarnessExt as _, symbols};
use craft_ui::theme::{Base, Tokens, TokensOf as _, install};
use egui::{Color32, Pos2, Rect, Ui, pos2, vec2};
use egui_kittest::Harness;

use super::overview::{OverviewRun, TimelineOverview, overview_runs};
use crate::time::{Tick, TimeRange, TimeView};

fn secs(s: f64) -> Tick {
    Tick::from_seconds_f64(s)
}

/// The pixel quarters each run covers, painted in order: later runs win.
fn covered(runs: &[OverviewRun]) -> Vec<(i32, Color32)> {
    let mut columns: Vec<(i32, Color32)> = Vec::new();
    for run in runs {
        for x in (run.rect.min.x * 4.0).round() as i32..(run.rect.max.x * 4.0).round() as i32 {
            columns.retain(|column| column.0 != x);
            columns.push((x, run.color));
        }
    }
    columns.sort_by_key(|column| column.0);
    columns
}

// 5000 touching clips of one colour become a few runs covering the same pixels.
#[test]
fn runs_merge_touching_clips_of_one_colour() {
    let (grey, red) = (Color32::from_gray(120), Color32::from_rgb(200, 40, 40));
    let mut clips = Vec::new();
    let mut at = 0.0;
    for index in 0..5000u32 {
        let length = 0.01 + f64::from(index % 7) * 0.003;
        let color = if (2000..2100).contains(&index) { red } else { grey };
        clips.push((TimeRange::new(secs(at), secs(length)), color));
        at += length + if index == 3000 { 30.0 } else { 0.0 };
    }
    // The runs fill the widget's own area: no inset or tail to copy.
    let mut tokens = None;
    egui::__run_test_ui(|ui| tokens = Some(Tokens::of(ui)));
    let tokens = tokens.unwrap();
    let outer = Rect::from_min_size(pos2(8.0, 4.0), vec2(900.0, 56.0));
    let rect = TimelineOverview::area(outer, &tokens);
    let runs = overview_runs(std::slice::from_ref(&clips), secs(at), outer, &tokens);
    assert!(runs.len() <= 6, "{} rects for 5000 clips", runs.len());

    let width = secs(at + 10.0).0 as f32;
    let unmerged: Vec<OverviewRun> = clips
        .iter()
        .map(|&(range, color)| {
            let from = rect.min.x + range.start.0 as f32 / width * rect.width();
            let to = (rect.min.x + range.end().0 as f32 / width * rect.width()).max(from + 1.0);
            OverviewRun { rect: Rect::from_min_max(pos2(from, rect.min.y + 0.5), pos2(to, rect.max.y - 0.5)), color }
        })
        .collect();
    assert_eq!(covered(&runs), covered(&unmerged));
}

struct Overview {
    view: TimeView,
    rect: Rect,
}

fn show_overview(ui: &mut Ui, app: &mut Overview) {
    app.rect = TimelineOverview::new("overview", &[], &mut app.view, 400.0, secs(50.0)).show(ui).rect;
}

// a click sets view.start so the box is centred at the pointer.
#[test]
fn click_centres_the_view() {
    let mut harness = Harness::<Overview>::builder()
        .with_size([480.0, 120.0])
        .with_context_setup(|ctx| {
            install(ctx, &Base::suite_default());
            symbols::install(ctx);
        })
        .build_ui_state(show_overview, Overview { view: TimeView::new(Tick::ZERO, 100.0), rect: Rect::NOTHING });
    harness.run();
    let rect = harness.state().rect;
    let click: Pos2 = rect.center();
    harness.click_at(click);
    // 60 s in all (content end plus 10 s); the box is 4 s wide, so its start is 2 s before the
    // pointer's time, 30 s.
    let start = harness.state().view.start.seconds();
    assert!((start - 28.0).abs() < 0.2, "{start}");
}
