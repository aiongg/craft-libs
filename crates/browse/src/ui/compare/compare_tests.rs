//! `CompareTile` through egui_kittest (task B8).

use craft_ui::canvas::CanvasView;
use craft_ui::canvas::textures::Revision;
use craft_ui::canvas::zoom::fit_view;
use craft_ui::theme::{Base, Tokens, TokensOf as _, install};
use egui::accesskit::Role;
use egui::{ColorImage, Event, Modifiers, MouseWheelSource, MouseWheelUnit, Rect, TouchPhase, Ui, pos2, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};
use kurbo::{Point, Size};

use super::caption::CaptionPainted;
use super::*;

const IMAGE: [u32; 2] = [800, 400];

struct Host {
    view: CanvasView,
    state: RenderState,
    tiles: Vec<CompareResponse>,
    caption: Option<CaptionPainted>,
}

fn picture(key: u64) -> CompareImage<'static> {
    CompareImage::Picture {
        key,
        revision: Revision(1),
        size: IMAGE,
        make: Box::new(|| ColorImage::filled([IMAGE[0] as usize, IMAGE[1] as usize], egui::Color32::GRAY)),
    }
}

fn harness(ui: impl FnMut(&mut Ui, &mut Host) + 'static) -> Harness<'static, Host> {
    let host = Host { view: CanvasView::new(Rect::ZERO, Point::ZERO, 0.0), state: RenderState::Ready, tiles: Vec::new(), caption: None };
    HarnessBuilder::default().with_size([700.0, 300.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(ui, host)
}

fn panes() -> [Rect; 2] {
    [Rect::from_min_size(pos2(20.0, 20.0), vec2(320.0, 260.0)), Rect::from_min_size(pos2(360.0, 20.0), vec2(320.0, 260.0))]
}

// A wheel over the left tile zooms; the right tile, drawn after it from the same view, shows the same zoom and centre.
#[test]
fn compare_tiles_share_zoom_and_pan() {
    let [left, right] = panes();
    let mut harness = harness(move |ui, host| {
        host.tiles.clear();
        for (i, rect) in [left, right].into_iter().enumerate() {
            let shown = CompareTile::new(("compare", i), &mut host.view, picture(i as u64)).show(ui, rect);
            host.tiles.push(shown);
        }
    });
    harness.run();
    let fitted = harness.state().tiles[0].image_rect.size();
    assert!((harness.state().tiles[1].image_rect.size() - fitted).length() < 0.5, "both panes start fitted alike");
    harness.hover_at(left.center());
    harness.run();
    harness.event(Event::MouseWheel {
        unit: MouseWheelUnit::Point,
        delta: vec2(0.0, 120.0),
        phase: TouchPhase::Move,
        modifiers: Modifiers::NONE,
        source: MouseWheelSource::Unknown,
    });
    harness.run();
    let [a, b] = [&harness.state().tiles[0], &harness.state().tiles[1]];
    assert!(a.image_rect.width() > fitted.x * 1.05 || a.image_rect.width() < fitted.x * 0.95, "the wheel zoomed the left tile");
    assert!((a.image_rect.size() - b.image_rect.size()).length() < 0.5, "the right tile follows: {:?} {:?}", a.image_rect, b.image_rect);
    assert!(
        ((a.image_rect.center() - left.min) - (b.image_rect.center() - right.min)).length() < 0.5,
        "equal panes show the same centre: {:?} {:?}",
        a.image_rect,
        b.image_rect
    );
}

// `toggle_zoom` from Fit gives 100 %, and back.
#[test]
fn double_click_toggles_fit_and_one_to_one() {
    let area = Rect::from_min_size(pos2(0.0, 0.0), vec2(300.0, 200.0));
    let size = Size::new(f64::from(IMAGE[0]), f64::from(IMAGE[1]));
    let mut view = CanvasView::new(area, Point::ZERO, 1.0);
    fit_view(&mut view, size, 1.0);
    let fit = view.zoom;
    assert!(fit < 1.0);
    CompareTile::toggle_zoom(&mut view, IMAGE, area, 1.0);
    assert!((view.zoom - 1.0).abs() < 1e-9, "100 %: {}", view.zoom);
    CompareTile::toggle_zoom(&mut view, IMAGE, area, 1.0);
    assert!((view.zoom - fit).abs() < 1e-9, "back to Fit: {} vs {fit}", view.zoom);
    // On a 2x display 100 % is two points per image pixel's worth of device pixels.
    CompareTile::toggle_zoom(&mut view, IMAGE, area, 2.0);
    assert!(view.zoom > 0.0);
}

// A pane 150 wide drops the file name; 400 wide shows it.
#[test]
fn the_caption_drops_the_name_when_narrow() {
    for (width, shown) in [(150.0, false), (400.0, true)] {
        let mut harness = harness(move |ui, host| {
            let style = CompareStyle::of(&Tokens::of(ui));
            let strip = Rect::from_min_size(pos2(0.0, 0.0), vec2(width, style.caption));
            let caption = CompareCaption::new("Select", "P1020.jpg").rating(Some(3)).flag(Some(crate::ui::grid::TileFlag::Picked));
            host.caption = Some(caption.paint(ui, strip, &style, &CompareLabels::default()));
        });
        harness.run();
        let painted = harness.state().caption.clone().expect("painted");
        assert_eq!(painted.name_shown, shown, "{width} wide");
        assert_eq!(painted.name, "Select: P1020.jpg, 3 of 5 stars, picked");
    }
}

// Pending not overdue: no pill; overdue: Rendering…; Failed: Render failed with the message as tooltip.
#[test]
fn the_pills_follow_the_render_state() {
    let [left, _] = panes();
    let mut harness = harness(move |ui, host| {
        let state = host.state.clone();
        let shown = CompareTile::new(("compare", 0), &mut host.view, picture(0))
            .caption(CompareCaption::new("Select", "P1020.jpg"))
            .state(state)
            .show(ui, left);
        host.tiles = vec![shown];
    });
    let description = |harness: &Harness<'_, Host>| {
        let tile = harness.get_by(|n| n.role() == Role::Image);
        tile.accesskit_node().description()
    };
    harness.run();
    assert_eq!(description(&harness), None);
    harness.state_mut().state = RenderState::Pending { overdue: false };
    harness.run();
    assert_eq!(description(&harness), None, "a render that is not overdue shows no pill");
    harness.state_mut().state = RenderState::Pending { overdue: true };
    harness.step();
    harness.step();
    assert_eq!(description(&harness).as_deref(), Some("Rendering…"));
    harness.state_mut().state = RenderState::Failed("out of memory".into());
    harness.run();
    assert_eq!(description(&harness).as_deref(), Some("Render failed"));
    let top = harness.state().tiles[0].image_rect;
    harness.hover_at(pos2(top.center().x, top.top() + 14.0));
    harness.run();
    harness.run();
    assert!(harness.query_by_label("out of memory").is_some(), "the message is the pill's tooltip");
}
