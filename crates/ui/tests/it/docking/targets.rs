//! `DockTargets` and `DockSpot`: a dock's registry resolves a pointer to the spot a drop would land on, one pass
//! after the spots were registered.

use craft_behavior::DockSide;
use craft_theme::{Contrast, ControlSize, Mode};
use craft_ui::docking::{DockSizes, DockSpot, DockTargets, ZoneRule};
use craft_ui::style::Variant;
use craft_ui::testing::memory::global;
use craft_ui::testing::style::{tokens, with_cx};
use craft_ui::testing::{Frames, trial};
use egui::{Context, Id, Pos2, Rect, Vec2};

const GROUP: u32 = 4;
const LAYOUT: Rect = Rect { min: Pos2::new(0.0, 0.0), max: Pos2::new(400.0, 300.0) };
const STRIP_HEIGHT: f32 = 24.0;

fn dock_id(name: &'static str) -> Id {
    global("dock registry", name)
}

fn sizes(frames: &Frames) -> DockSizes {
    with_cx(&frames.ctx, &tokens(Mode::Dark, Contrast::Standard), Variant::SECONDARY, ControlSize::Md, DockSizes::style)
}

fn strip() -> Rect {
    Rect::from_min_size(LAYOUT.min, Vec2::new(LAYOUT.width(), STRIP_HEIGHT))
}

fn body() -> Rect {
    Rect::from_min_max(Pos2::new(LAYOUT.left(), strip().bottom()), LAYOUT.max)
}

/// One group the way group rendering registers it: the strip, the sides and the middle of the body.
fn register_group(ctx: &Context, dock: Id, anchor: u32, rule: ZoneRule, sizes: &DockSizes) {
    for (side, rect) in rule.zones(body(), sizes, false) {
        DockTargets::register(ctx, dock, DockSpot::group(anchor, side), rect);
    }
    DockTargets::register(ctx, dock, DockSpot::tab(anchor, 0), strip());
}

/// Registers in one frame with `register`, then resolves in the next with `probe`.
fn registered_then(frames: &mut Frames, register: impl Fn(&Context), probe: impl Fn(&Context)) {
    frames.frame(Vec::new(), |ui| register(ui.ctx()));
    frames.frame(Vec::new(), |ui| {
        register(ui.ctx());
        probe(ui.ctx());
    });
}

#[trial]
fn a_pointer_resolves_to_a_tab_insertion_a_group_side_or_the_group() {
    let mut frames = Frames::new();
    let sizes = sizes(&frames);
    let dock = dock_id("dock");
    for rule in [ZoneRule::Fraction, ZoneRule::Depth] {
        registered_then(
            &mut frames,
            |ctx| register_group(ctx, dock, GROUP, rule, &sizes),
            |ctx| {
                let at = |pos: Pos2| DockTargets::<u32>::resolve(ctx, dock, pos).map(|(spot, _)| spot);
                assert_eq!(at(strip().center()), Some(DockSpot::tab(GROUP, 0)), "{rule:?}: the strip");
                let left = Pos2::new(body().left() + 2.0, body().center().y);
                assert_eq!(at(left), Some(DockSpot::group(GROUP, DockSide::Start)), "{rule:?}: the start side");
                let bottom = Pos2::new(body().center().x, body().bottom() - 2.0);
                assert_eq!(at(bottom), Some(DockSpot::group(GROUP, DockSide::Bottom)), "{rule:?}: the bottom side");
                assert_eq!(at(body().center()), Some(DockSpot::group(GROUP, DockSide::Center)), "{rule:?}: the middle");
            },
        );
    }
}

#[trial]
fn a_spot_is_resolved_in_the_pass_after_its_registration() {
    let mut frames = Frames::new();
    let dock = dock_id("dock");
    let mut first = true;
    let mut unresolved_before = false;
    frames.frame(Vec::new(), |ui| {
        if std::mem::take(&mut first) {
            unresolved_before = DockTargets::<u32>::resolve(ui.ctx(), dock, strip().center()).is_none();
        }
        DockTargets::register(ui.ctx(), dock, DockSpot::tab(GROUP, 0), strip());
    });
    assert!(unresolved_before, "a spot registered in a pass is not read in the same pass");
    frames.frame(Vec::new(), |ui| {
        assert_eq!(DockTargets::<u32>::resolve(ui.ctx(), dock, strip().center()), Some((DockSpot::tab(GROUP, 0), strip())));
    });
}

#[trial]
fn the_side_zones_of_a_body_do_not_overlap_and_stay_inside_it() {
    let frames = Frames::new();
    let sizes = sizes(&frames);
    for rule in [ZoneRule::Fraction, ZoneRule::Depth] {
        for rtl in [false, true] {
            let zones = rule.zones(body(), &sizes, rtl);
            for (at, (_, rect)) in zones.iter().enumerate() {
                assert!(body().contains_rect(*rect), "{rule:?}: zone {at} stays inside the body");
                for (_, other) in &zones[at + 1..] {
                    assert!(!rect.shrink(0.01).intersects(*other), "{rule:?}: zone {at} does not overlap another");
                }
            }
        }
    }
    let depth = ZoneRule::Depth.zones(body(), &sizes, false)[0].1.width();
    assert!(depth <= sizes.zone_depth, "a side zone is at most the named depth");
}

#[trial]
fn an_edge_band_beats_a_group_side_near_the_layouts_border() {
    let mut frames = Frames::new();
    let sizes = sizes(&frames);
    let dock = dock_id("dock");
    let band = Rect::from_min_max(LAYOUT.min, Pos2::new(LAYOUT.left() + sizes.edge_band, LAYOUT.bottom()));
    registered_then(
        &mut frames,
        |ctx| {
            register_group(ctx, dock, GROUP, ZoneRule::Fraction, &sizes);
            DockTargets::register(ctx, dock, DockSpot::<u32>::Edge(DockSide::Start), band);
        },
        |ctx| {
            let near = Pos2::new(LAYOUT.left() + 2.0, body().center().y);
            assert_eq!(DockTargets::<u32>::resolve(ctx, dock, near).map(|(spot, _)| spot), Some(DockSpot::Edge(DockSide::Start)));
            let inside = Pos2::new(LAYOUT.left() + sizes.edge_band + 2.0, body().center().y);
            assert_eq!(
                DockTargets::<u32>::resolve(ctx, dock, inside).map(|(spot, _)| spot),
                Some(DockSpot::group(GROUP, DockSide::Start)),
                "past the band the group's side"
            );
        },
    );
}

#[trial]
fn a_blocked_rect_hides_the_spots_registered_below_it() {
    let mut frames = Frames::new();
    let sizes = sizes(&frames);
    let dock = dock_id("dock");
    let window = Rect::from_center_size(body().center(), Vec2::splat(80.0));
    registered_then(
        &mut frames,
        |ctx| {
            register_group(ctx, dock, GROUP, ZoneRule::Fraction, &sizes);
            DockTargets::register(ctx, dock, DockSpot::<u32>::Blocked, window);
        },
        |ctx| {
            assert_eq!(DockTargets::<u32>::resolve(ctx, dock, window.center()), None, "a rect that takes no drop hides the group under it");
            assert!(DockTargets::<u32>::resolve(ctx, dock, strip().center()).is_some(), "outside the rect the group still takes the drop");
        },
    );
}
