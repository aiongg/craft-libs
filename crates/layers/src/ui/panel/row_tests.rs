//! Screen tests of the row mapping (L4): one row built with `row::build_row` and shown alone.

#![allow(clippy::expect_used, reason = "tests")]

use craft_theme::Size;
use craft_ui::canvas::textures::{FakeThumbSource, ThumbnailOptions, Thumbnails};
use craft_ui::direction::{self, Direction};
use craft_ui::list::{ListRow, ThumbSize};
use craft_ui::localized::Labels as _;
use craft_ui::theme::{Base, current, install, scope};
use egui::accesskit::Role;
use egui::{Context, Id, Rect, Shape, Ui, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::context::{PanelContext, ThumbHost, Trailing};
use super::labels::LayersLabels;
use super::row::{ask_near_thumbnails, build_row, row_description, row_height};
use super::style::LayersStyle;
use crate::ui::mock::MockLayers;
use crate::ui::options::LayersOptions;
use crate::ui::thumbs::LayerThumb;
use crate::ui::tree::{LayerInfo, LayerKind, LayerLocks, LayerPart, LayerRowKey, LayerSelection, LayerTree as _, MaskInfo};
use craft_ui::behavior::RowInfo;

/// What one pass of the row reported.
#[derive(Clone, Debug)]
struct Out {
    toggles: Vec<Rect>,
    label: Rect,
    min_width: f32,
    thumbs: Vec<(LayerPart, Rect)>,
}

struct Setup {
    tree: MockLayers,
    options: LayersOptions,
    key: LayerRowKey<u32>,
    width: f32,
    lock_column: bool,
    trailing: bool,
    out: Out,
}

impl Setup {
    fn new(tree: MockLayers, key: LayerRowKey<u32>) -> Self {
        let options = LayersOptions { thumbnail_size: ThumbSize::Medium, ..LayersOptions::default() };
        Self {
            tree,
            options,
            key,
            width: 420.0,
            lock_column: false,
            trailing: false,
            out: Out { toggles: Vec::new(), label: Rect::NOTHING, min_width: 0.0, thumbs: Vec::new() },
        }
    }
}

fn show(ui: &mut Ui, setup: &mut Setup) {
    let ctx = ui.ctx().clone();
    let tokens = current(&ctx);
    let labels = LayersLabels::of(ui);
    let trailing = setup.trailing.then(|| Trailing {
        width: 24.0,
        add: Box::new(|ui: &mut Ui, _: &u32, _: &LayerInfo| {
            ui.label("Target");
        }),
    });
    let cx = PanelContext {
        ctx: ctx.clone(),
        id: Id::NULL,
        tree: &setup.tree,
        selection: setup.tree.selection(),
        options: &setup.options,
        thumbs: None,
        labels: &labels,
        style: LayersStyle::of(&tokens),
        row_style: ListRow::<LayerRowKey<u32>>::style(&tokens),
        lock_column: setup.lock_column,
        visibility_key: None,
        trailing: trailing.as_ref(),
        sink: Default::default(),
    };
    let width = setup.width;
    let key = setup.key.clone();
    let response = ui
        .allocate_ui(vec2(width, 0.0), |ui| {
            ui.set_max_width(width);
            ui.spacing_mut().item_spacing.y = 0.0;
            build_row(&cx, 0, &key).show(ui)
        })
        .inner;
    setup.out = Out {
        toggles: response.toggles.iter().map(|r| r.rect).collect(),
        label: response.label,
        min_width: response.min_width,
        thumbs: cx.sink.borrow().thumbs.iter().map(|(_, part, rect)| (*part, *rect)).collect(),
    };
}

fn harness(setup: Setup, dir: Direction) -> Harness<'static, Setup> {
    let mut harness = HarnessBuilder::default()
        .with_size(vec2(480.0, 120.0))
        .with_context_setup(move |ctx: &Context| {
            install(ctx, &Base::suite_default());
            direction::apply(ctx, dir);
        })
        .build_ui_state(show, setup);
    harness.run();
    harness
}

/// Every shape one pass paints, flattened.
fn shapes(setup: &mut Setup) -> Vec<Shape> {
    fn flatten(shape: &Shape, out: &mut Vec<Shape>) {
        match shape {
            Shape::Vec(shapes) => shapes.iter().for_each(|s| flatten(s, out)),
            other => out.push(other.clone()),
        }
    }
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let output = ctx.run_ui(egui::RawInput::default(), |ui| show(ui, setup));
    let mut out = Vec::new();
    output.shapes.iter().for_each(|s| flatten(&s.shape, &mut out));
    output.drop_without_applying_deltas();
    out
}

/// One pass of `add` on `ctx`.
fn run(ctx: &Context, add: impl FnMut(&mut Ui)) {
    let mut add = add;
    ctx.run_ui(egui::RawInput::default(), |ui| add(ui)).drop_without_applying_deltas();
}

fn texts(shapes: &[Shape]) -> Vec<String> {
    shapes.iter().filter_map(|s| if let Shape::Text(t) = s { Some(t.galley.text().to_owned()) } else { None }).collect()
}

/// A tree of one layer, `edit` applied to its info.
fn one(name: &str, kind: LayerKind, edit: impl FnOnce(&mut LayerInfo)) -> MockLayers {
    let mut tree = MockLayers::sample();
    tree.layers.retain(|l| l.id == 4);
    let layer = &mut tree.layers[0];
    layer.parent = None;
    layer.info = LayerInfo::new(name, kind);
    edit(&mut layer.info);
    tree.selection = LayerSelection { layers: vec![4], active: Some(4), target: LayerPart::Pixels };
    tree
}

fn rect_of(harness: &Harness<'_, Setup>, label: &str) -> Rect {
    harness.get_by_label(label).rect()
}

fn busy() -> MockLayers {
    one("Busy", LayerKind::Pixel, |info| {
        info.masks = vec![
            MaskInfo { part: LayerPart::LayerMask, enabled: true, linked: true },
            MaskInfo { part: LayerPart::VectorMask, enabled: true, linked: false },
        ];
        info.linked = true;
        info.has_effects = true;
        info.locks = LayerLocks { all: true, ..LayerLocks::default() };
        info.blend_label = Some(String::from("Multiply"));
    })
}

// Eye, layer thumbnail, each mask with its chain, name, then the indicators, in reading order (and
// mirrored right to left).
#[test]
fn row_parts_follow_the_layer_info_in_reading_order() {
    for dir in [Direction::Ltr, Direction::Rtl] {
        let mut setup = Setup::new(busy(), LayerRowKey::Layer(4));
        setup.lock_column = true;
        let harness = harness(setup, dir);
        let out = &harness.state().out;
        let at = |r: Rect| if dir == Direction::Ltr { r.min.x } else { -r.max.x };
        let eye_lock = (at(out.toggles[0]), at(out.toggles[1]));
        assert!(eye_lock.0 < eye_lock.1, "the eye comes first, then the lock column ({dir:?})");
        let parts: Vec<LayerPart> = out.thumbs.iter().map(|(p, _)| *p).collect();
        assert_eq!(parts, [LayerPart::Pixels, LayerPart::LayerMask, LayerPart::VectorMask]);
        let thumbs: Vec<f32> = out.thumbs.iter().map(|(_, r)| at(*r)).collect();
        assert!(eye_lock.1 < thumbs[0] && thumbs[0] < thumbs[1] && thumbs[1] < thumbs[2], "{dir:?}: {thumbs:?}");
        let chain = at(rect_of(&harness, "Unlink layer mask of Busy"));
        assert!(thumbs[0] < chain && chain < thumbs[1], "the chain sits between the thumbnails ({dir:?})");
        let label = at(out.label);
        assert!(thumbs[2] < label, "the name follows the last mask ({dir:?})");
        let order = ["Unlock Busy", "Collapse effects Busy", "Expand effects Busy"];
        let lock = at(rect_of(&harness, order[0]));
        let effects = harness.query_by_label(order[1]).or_else(|| harness.query_by_label(order[2])).expect("effects triangle").rect();
        assert!(label < at(effects) && at(effects) < lock, "name, effects triangle, then the lock at the end ({dir:?})");
    }
}

// A hidden layer's eye cell is empty and its name is drawn with `list.text.dimmed`.
#[test]
fn hidden_layer_has_an_empty_eye_and_a_faint_name() {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let dimmed = ListRow::<u8>::style(&current(&ctx)).dimmed;
    let colored = |setup: &mut Setup| {
        let hex = format!("#{:02X}_{:02X}_{:02X}", dimmed.r(), dimmed.g(), dimmed.b());
        shapes(setup).iter().any(|shape| matches!(shape, Shape::Text(t) if t.galley.text() == "Ghost" && format!("{t:?}").contains(&hex)))
    };
    let hidden = one("Ghost", LayerKind::Pixel, |info| {
        info.visible = false;
        info.dimmed = true;
    });
    assert!(colored(&mut Setup::new(hidden.clone(), LayerRowKey::Layer(4))), "the name is faint");
    let shown = one("Ghost", LayerKind::Pixel, |_| {});
    assert!(!colored(&mut Setup::new(shown.clone(), LayerRowKey::Layer(4))), "a visible layer's name is not");

    let state = |tree| harness(Setup::new(tree, LayerRowKey::Layer(4)), Direction::Ltr).get_by_label("Visibility Ghost").accesskit_node().toggled();
    assert_eq!(state(hidden), Some(egui::accesskit::Toggled::False));
    assert_eq!(state(shown), Some(egui::accesskit::Toggled::True));
}

// The blend label drops before the name gets narrower than `list.min_label`; the other indicators
// stay.
#[test]
fn blend_label_drops_before_the_name_gets_narrow() {
    let mut setup = Setup::new(busy(), LayerRowKey::Layer(4));
    assert!(texts(&shapes(&mut setup)).iter().any(|t| t == "Multiply"), "the label shows with room");
    let least = setup.out.min_width;
    setup.width = least + 2.0;
    let drawn = shapes(&mut setup);
    assert!(!texts(&drawn).iter().any(|t| t == "Multiply"), "the label drops at the least width");
    assert!(texts(&drawn).iter().any(|t| t == "fx"), "the fx mark stays");
    let harness = harness(setup, Direction::Ltr);
    harness.get_by_label("Unlock Busy");
    let min_label = ListRow::<u8>::style(&current(&harness.ctx)).min_label;
    assert!(harness.state().out.label.width() >= min_label - 0.5, "the name keeps list.min_label");
}

fn bracket_segments(shapes: &[Shape]) -> Vec<[egui::Pos2; 2]> {
    shapes
        .iter()
        .filter_map(|s| match s {
            Shape::LineSegment { points, stroke } if (stroke.width - 1.5).abs() < f32::EPSILON => Some(*points),
            _ => None,
        })
        .collect()
}

// The active row's target thumbnail (pixels, layer mask, vector mask) carries the corner brackets;
// other rows have none.
#[test]
fn target_brackets_frame_the_active_rows_target_part() {
    for (part, index) in [(LayerPart::Pixels, 0), (LayerPart::LayerMask, 1), (LayerPart::VectorMask, 2)] {
        let mut tree = busy();
        tree.selection.target = part;
        tree.layers[0].info.locks = LayerLocks::default();
        let mut setup = Setup::new(tree, LayerRowKey::Layer(4));
        let segments = bracket_segments(&shapes(&mut setup));
        assert_eq!(segments.len(), 8, "four corners of two arms for {part:?}");
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        let outset = ListRow::<u8>::style(&current(&ctx)).bracket_outset;
        let target = setup.out.thumbs[index].1.expand(outset);
        let hull = segments.iter().flatten().fold(Rect::NOTHING, |hull, p| hull.union(Rect::from_min_max(*p, *p)));
        assert!((hull.min - target.min).length() < 1.0 && (hull.max - target.max).length() < 1.0, "{part:?}: {hull:?} vs {target:?}");
    }
    let mut tree = busy();
    tree.selection.active = None;
    let mut setup = Setup::new(tree, LayerRowKey::Layer(4));
    assert!(bracket_segments(&shapes(&mut setup)).is_empty(), "a row that is not active has no brackets");
}

// A disabled mask is crossed out in `layers.mask.disabled`; badges show only with
// `show_layer_mask_badges`.
#[test]
fn disabled_mask_is_crossed_out_and_badges_follow_the_setting() {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let style = LayersStyle::of(&current(&ctx));
    let crosses = |shapes: &[Shape]| {
        shapes
            .iter()
            .filter(
                |s| matches!(s, Shape::LineSegment { stroke, .. } if stroke.color == style.mask_disabled && stroke.width == style.mask_cross_width),
            )
            .count()
    };
    let badges = |shapes: &[Shape]| {
        shapes
            .iter()
            .filter(|s| matches!(s, Shape::Rect(r) if r.fill == style.mask_badge_fill && (r.rect.width() - style.mask_badge).abs() < 0.01))
            .count()
    };
    let disabled = one("Off", LayerKind::Adjustment, |info| {
        info.masks = vec![MaskInfo { part: LayerPart::LayerMask, enabled: false, linked: true }];
    });
    let enabled = one("On", LayerKind::Adjustment, |info| {
        info.masks = vec![MaskInfo { part: LayerPart::LayerMask, enabled: true, linked: true }];
    });
    let mut setup = Setup::new(disabled, LayerRowKey::Layer(4));
    let drawn = shapes(&mut setup);
    assert_eq!(crosses(&drawn), 2, "two diagonals");
    assert_eq!(badges(&drawn), 0, "badges are off by default");
    assert_eq!(crosses(&shapes(&mut Setup::new(enabled.clone(), LayerRowKey::Layer(4)))), 0);
    let mut with_badges = Setup::new(enabled, LayerRowKey::Layer(4));
    with_badges.options.show_layer_mask_badges = true;
    assert_eq!(badges(&shapes(&mut with_badges)), 1, "one badge on the mask thumbnail");
}

fn effects_tree() -> MockLayers {
    let mut tree = MockLayers::sample();
    tree.selection.active = None;
    tree
}

// Effects rows sit under their layer at `list.row_height_dense`, their eye under the layer's eye.
#[test]
fn effect_rows_sit_under_the_layer_with_the_eye_indented() {
    let layer = harness(Setup::new(effects_tree(), LayerRowKey::Layer(7)), Direction::Ltr);
    let effect = harness(Setup::new(effects_tree(), LayerRowKey::Effects(7, Some(0))), Direction::Ltr);
    assert_eq!(effect.get_by_label("Drop Shadow").accesskit_node().description().as_deref(), Some("effect of Styled"));
    assert!(effect.state().out.toggles[0].min.x > layer.state().out.toggles[0].min.x, "the sub-row's eye is indented under the layer's");
    let tree = effects_tree();
    let mut setup = Setup::new(tree, LayerRowKey::Effects(7, None));
    let height = {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        let mut height = 0.0;
        run(&ctx, |ui| {
            with_cx(ui, &mut setup, |cx| {
                height = row_height(cx, &RowInfo::leaf(LayerRowKey::Effects(7, None)), ThumbSize::Large);
            });
        });
        height
    };
    let dense = {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        current(&ctx).component.list.row_height_dense
    };
    assert_eq!(height, dense, "an effects row is dense whatever the thumbnail size");
    assert_eq!(dense, 22.0);
}

// A Smart Filters header with `mask` shows the filter-mask thumbnail; at thumbnail size None it
// shows none.
#[test]
fn smart_filter_header_shows_the_shared_mask_thumbnail() {
    let mut setup = Setup::new(effects_tree(), LayerRowKey::Filters(6, None));
    let _ = shapes(&mut setup);
    assert_eq!(setup.out.thumbs.iter().map(|(p, _)| *p).collect::<Vec<_>>(), [LayerPart::FilterMask]);
    setup.options.thumbnail_size = ThumbSize::None;
    let _ = shapes(&mut setup);
    assert!(setup.out.thumbs.is_empty(), "none at thumbnail size None");
    let mut item = Setup::new(effects_tree(), LayerRowKey::Filters(6, Some(0)));
    let _ = shapes(&mut item);
    assert!(item.out.thumbs.is_empty(), "a filter row has no thumbnail");
}

// The trailing furniture is drawn in its cell and the name keeps at least `list.min_label`.
#[test]
fn trailing_slot_hosts_app_furniture_without_losing_the_name() {
    let mut setup = Setup::new(busy(), LayerRowKey::Layer(4));
    setup.trailing = true;
    let _ = shapes(&mut setup);
    setup.width = setup.out.min_width + 1.0;
    let harness = harness(setup, Direction::Ltr);
    let furniture = rect_of(&harness, "Target");
    let out = &harness.state().out;
    assert!(furniture.min.x >= out.label.max.x - 0.5, "the furniture sits after the name");
    let min_label = ListRow::<u8>::style(&current(&harness.ctx)).min_label;
    assert!(out.label.width() >= min_label - 0.5, "the name keeps list.min_label");
}

fn with_cx(ui: &mut Ui, setup: &mut Setup, run: impl FnOnce(&PanelContext<'_, MockLayers>)) {
    let tokens = current(ui.ctx());
    let labels = LayersLabels::of(ui);
    let cx = PanelContext {
        ctx: ui.ctx().clone(),
        id: Id::NULL,
        tree: &setup.tree,
        selection: setup.tree.selection(),
        options: &setup.options,
        thumbs: None,
        labels: &labels,
        style: LayersStyle::of(&tokens),
        row_style: ListRow::<LayerRowKey<u32>>::style(&tokens),
        lock_column: false,
        visibility_key: None,
        trailing: None,
        sink: Default::default(),
    };
    run(&cx);
}

// `row_height` gives the 8-cell table: None, Small, Medium, Large at Compact (24, 32, 32, 56) and
// Comfortable (30, 40, 42, 56).
#[test]
fn row_height_follows_size_preset_and_thumbnail_setting() {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let mut setup = Setup::new(MockLayers::sample(), LayerRowKey::Layer(1));
    let mut heights = |size: Size| {
        let mut out = Vec::new();
        run(&ctx, |ui| {
            scope(
                ui,
                |o| o.size = Some(size),
                |ui| {
                    with_cx(ui, &mut setup, |cx| {
                        for thumb in ThumbSize::ALL {
                            out.push(row_height(cx, &RowInfo::leaf(LayerRowKey::Layer(1)), thumb));
                        }
                    });
                },
            );
        });
        out
    };
    assert_eq!(heights(Size::Compact), [24.0, 32.0, 32.0, 56.0]);
    assert_eq!(heights(Size::Comfortable), [30.0, 40.0, 42.0, 56.0]);
}

// The row's AccessKit description holds the kind word, the flags that hold and the target words.
#[test]
fn rows_are_list_box_options_with_state_in_the_description() {
    let labels = LayersLabels::english();
    let mut info = busy().layers[0].info.clone();
    info.visible = false;
    assert_eq!(
        row_description(&labels, &info, Some(LayerPart::LayerMask)),
        "pixel layer, locked, linked, layer mask, vector mask, effects, hidden, layer mask targeted"
    );
    assert_eq!(row_description(&labels, &LayerInfo::new("Plain", LayerKind::Group), None), "group");

    // The warning is read with the row, after the app's own description.
    let mut warned = LayerInfo::new("Warned", LayerKind::Pixel);
    warned.description = Some(String::from("opacity 50%"));
    warned.warning = Some(String::from("Missing font"));
    assert_eq!(row_description(&labels, &warned, None), "pixel layer, opacity 50%, Missing font");

    let mut tree = busy();
    tree.selection.target = LayerPart::VectorMask;
    let harness = harness(Setup::new(tree, LayerRowKey::Layer(4)), Direction::Ltr);
    let row = harness.get_by_role(Role::ListBoxOption);
    assert_eq!(
        row.accesskit_node().description().as_deref(),
        Some("pixel layer, locked, linked, layer mask, vector mask, effects, vector mask targeted")
    );
}

// Only `info` of the rows the list draws is asked for, in a tree of 10,000 layers.
#[test]
fn tree_with_ten_thousand_layers_builds_rows_in_view_only() {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let tree = MockLayers::flat(10_000);
    let setup = Setup::new(tree, LayerRowKey::Layer(1));
    let rows: Vec<RowInfo<LayerRowKey<u32>>> = (1..=10_000).map(|id| RowInfo::leaf(LayerRowKey::Layer(id))).collect();
    let thumbnails = Thumbnails::<LayerThumb<u32>>::in_context(&ctx, "layers-test", ThumbnailOptions::default());
    let source = FakeThumbSource::<LayerThumb<u32>>::new();
    run(&ctx, |ui| {
        let tokens = current(ui.ctx());
        let labels = LayersLabels::of(ui);
        let cx = PanelContext {
            ctx: ui.ctx().clone(),
            id: Id::NULL,
            tree: &setup.tree,
            selection: setup.tree.selection(),
            options: &setup.options,
            thumbs: Some(ThumbHost { thumbnails: &thumbnails, source: &source }),
            labels: &labels,
            style: LayersStyle::of(&tokens),
            row_style: ListRow::<LayerRowKey<u32>>::style(&tokens),
            lock_column: false,
            visibility_key: None,
            trailing: None,
            sink: Default::default(),
        };
        for (at, row) in rows.iter().enumerate().take(120).skip(100) {
            let _ = build_row(&cx, at, &row.key);
        }
        assert_eq!(cx.sink.borrow().built, Some((100, 119)));
        ask_near_thumbnails(&cx, &rows);
    });
    let calls = setup.tree.take_info_calls();
    assert!((20..=100).contains(&calls), "info runs for the built rows and one viewport around them, not for 10,000: {calls}");
}
