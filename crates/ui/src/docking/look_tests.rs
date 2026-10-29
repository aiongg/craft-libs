//! [L] The dock look: token resolution per Size, parity of the `DockStyle` split, scope overrides,
//! the token path's accessible names, and the stories.

use craft_theme::{Appearance, ComponentField, Contrast, DockField, FloatField, Mode, Pins, Rgba, Size, TabField, TextSize, builtins, resolve};
use egui::{Context, FontId, Ui, Visuals};

use super::testing::DockDriver;
use super::{Column, DockArea, DockConfig, DockContent, DockLook, DockSpec, DockStyle, Floating, Group, Hit, Layout, Node, Permissions, Side};
use crate::theme::{FontFamilies, ToEgui as _, UiTokens, scope};

fn look_at(size: Size, mode: Mode) -> DockLook {
    let appearance = Appearance { mode, contrast: Contrast::Standard, size, text_size: TextSize::default(), reduced_motion: false };
    let theme = builtins().first().expect("a built-in theme");
    let tokens = resolve(theme, appearance, None).tokens;
    DockLook::from_tokens(&UiTokens::new(tokens, appearance, mode, FontFamilies::default()))
}

fn in_ui<R>(test: impl FnOnce(&mut Ui) -> R) -> R {
    let ctx = Context::default();
    let mut test = Some(test);
    let mut out = None;
    let mut full = ctx.run_ui(Default::default(), |ui| out = Some((test.take().unwrap())(ui)));
    full.textures_delta.clear();
    out.expect("the ui ran")
}

/// Sizes follow the Size preset: the title bar and column bar scale with the control height, the
/// iconic column with the panel rail, the tab strip with the tab family.
#[test]
fn the_look_follows_the_size_preset() {
    let sizes = |look: &DockLook| {
        [look.tab_height, look.gap, look.tab_padding, look.title_height, look.column_bar_height, look.iconic_width, look.icon_size, look.body_gap]
    };
    assert_eq!(sizes(&look_at(Size::Compact, Mode::Light)), [28.0, 4.0, 24.0, 14.0, 14.0, 36.0, 20.0, 0.0]);
    assert_eq!(sizes(&look_at(Size::Comfortable, Mode::Light)), [40.0, 6.0, 32.0, 18.0, 18.0, 44.0, 22.0, 6.0]);
}

/// Controls stacked in a panel body are `dock.body_gap` apart, in a docked panel and a floating one.
#[test]
fn stacked_controls_in_a_panel_body_are_the_body_gap_apart() {
    use craft_theme::{AppDefaults, ModeChoice, SuiteChoice, SystemAppearance, ThemeId, ThemePair};
    use egui_kittest::{Harness, kittest::Queryable as _};
    struct Stack;
    impl DockContent<u32> for Stack {
        fn body(&mut self, ui: &mut Ui, panel: &u32) {
            ui.label(format!("first {panel}"));
            ui.label(format!("second {panel}"));
        }
        fn logical_label(&self, panel: &u32) -> String {
            format!("Panel {panel}")
        }
        fn permissions(&self, _: &u32) -> Permissions {
            Permissions::default()
        }
    }
    let layout = Layout {
        root: Some(Node::Tabs { panels: vec![0], active: 0 }),
        floating: vec![Floating::single(vec![1], 0, [300.0, 20.0, 160.0, 160.0]).with_id(1)],
    };
    for size in [Size::Compact, Size::Comfortable] {
        let defaults = AppDefaults { theme: ThemePair::same(ThemeId::suite_default()), mode: ModeChoice::Light, size };
        let base =
            crate::theme::Base::resolve(&SuiteChoice::default().with_defaults(&defaults), SystemAppearance::default(), &defaults, None, &mut |id| {
                builtins().iter().find(|t| t.id == *id).cloned()
            });
        let layout = layout.clone();
        let mut harness = Harness::builder()
            .with_size(egui::vec2(520.0, 300.0))
            .with_context_setup(move |ctx: &Context| crate::theme::install(ctx, &base))
            .build_ui(move |ui| {
                DockArea::new(egui::Id::unique("body-gap")).show_content(ui, &layout, &DockConfig::default(), &mut Stack);
            });
        harness.run_steps(3);
        let expected = look_at(size, Mode::Light).body_gap;
        for panel in [0, 1] {
            let top = harness.get_by_label(&format!("first {panel}")).rect();
            let next = harness.get_by_label(&format!("second {panel}")).rect();
            assert!((next.top() - top.bottom() - expected).abs() < 0.5, "{size:?} panel {panel}: {top:?} {next:?}");
        }
    }
}

/// The unmigrated flat style splits into the default config and the visuals look, field by field.
#[test]
fn a_dock_style_from_visuals_splits_into_the_default_config_and_the_visuals_look() {
    for visuals in [Visuals::dark(), Visuals::light()] {
        let (config, look) = DockStyle::from_visuals(&visuals).split();
        assert_eq!(format!("{config:?}"), format!("{:?}", DockConfig::default()));
        assert_eq!(look, DockLook::from_visuals(&visuals));
    }
}

/// Light and dark resolve to different strips and active tabs, and neither mode's strip or drop
/// colour is the colour `DockLook::from_visuals` takes from egui. (The drop colour is the accent,
/// which is the same in both modes of the default theme.)
#[test]
fn the_themed_look_differs_between_modes_and_from_the_visuals_look() {
    let (light, dark) = (look_at(Size::Compact, Mode::Light), look_at(Size::Compact, Mode::Dark));
    for (name, a, b) in [("strip", light.tab_strip, dark.tab_strip), ("active", light.tab_active, dark.tab_active)] {
        assert_ne!(a, b, "{name}");
    }
    for (look, visuals) in [(&light, Visuals::light()), (&dark, Visuals::dark())] {
        let plain = DockLook::from_visuals(&visuals);
        assert_ne!(look.tab_strip, plain.tab_strip);
        assert_ne!(look.drop, plain.drop);
    }
}

/// The config as a `DockSpec` reads the look of the ui's theme, so a scope that pins `tab.strip`
/// recolors the strip and leaves the config alone.
#[test]
fn the_config_spec_follows_a_scope_override_of_the_tab_strip() {
    let pinned = Rgba([12, 34, 56, 255]);
    let config = DockConfig::default();
    let (outside, inside) = in_ui(|ui| {
        let outside = config.parts(ui).1.tab_strip;
        let colors: Pins = [(ComponentField::Tab(TabField::Strip), pinned)].into_iter().collect();
        let inside = scope(ui, |o| o.colors = colors, |ui| config.parts(ui).1.into_owned());
        (outside, inside)
    });
    assert_ne!(outside, pinned.to_egui());
    assert_eq!(inside.tab_strip, pinned.to_egui());
}

/// `min_tab_width: None` is the label font's width plus the tab padding, so it follows a font
/// change without a second setting.
#[test]
fn the_default_minimum_tab_width_follows_the_label_font() {
    let mut look = look_at(Size::Compact, Mode::Light);
    let config = DockConfig { min_tab_width: None, ..DockConfig::default() };
    let small = config.tab_width_floor(&look);
    look.font = FontId::proportional(look.font.size * 2.0);
    assert!(config.tab_width_floor(&look) > small);
    let fixed = DockConfig { min_tab_width: Some(60.0), ..DockConfig::default() };
    assert_eq!(fixed.tab_width_floor(&look), 60.0);
}

/// `valid` rejects non-finite or out-of-range values in the look and in the config.
#[test]
fn a_look_or_config_with_a_non_finite_value_is_invalid() {
    let look = look_at(Size::Compact, Mode::Light);
    let config = DockConfig::default();
    assert!(look.valid() && config.valid(&look));
    for corrupt in [
        (|l: &mut DockLook| l.tab_height = f32::NAN) as fn(&mut DockLook),
        |l| l.title_height = f32::INFINITY,
        |l| l.tab_fill = f32::NAN,
        |l| l.tab_height = 0.0,
    ] {
        let mut bad = look.clone();
        corrupt(&mut bad);
        assert!(!bad.valid());
        assert!(!super::look::valid(&config, &bad));
    }
    let mut bad = config.clone();
    bad.min_pane = f32::NAN;
    assert!(!bad.valid(&look));
}

struct Names;

impl DockContent<u32> for Names {
    fn body(&mut self, ui: &mut Ui, _: &u32) {
        ui.label("Body");
    }
    fn logical_label(&self, panel: &u32) -> String {
        format!("Panel {panel}")
    }
    fn permissions(&self, _: &u32) -> Permissions {
        Permissions::default()
    }
}

/// A dock column of one group per panel, iconic when `iconic`.
fn dock_column(iconic: bool, panels: [u32; 2]) -> Column<u32> {
    let mut column = Column::new(panels.map(|panel| Group::new(vec![panel], 0)).to_vec(), 100.0);
    column.iconic = iconic;
    column
}

/// The accessible names of tabs, drag bars, column bars, iconic cells, dividers, resize handles
/// and title bar buttons are the same whether the area draws from a `DockStyle` or from the token path.
#[test]
fn the_accessible_names_are_the_same_on_the_token_path() {
    use egui_kittest::{Harness, kittest::NodeT as _};
    let layout = Layout {
        root: Some(Node::Split {
            axis: crate::layout::SplitAxis::Horizontal,
            size: crate::layout::SplitSize::FixedSecond(200.0),
            first: Box::new(Node::Tabs { panels: vec![0], active: 0 }),
            second: Box::new(Node::Split {
                axis: crate::layout::SplitAxis::Horizontal,
                size: crate::layout::SplitSize::FixedSecond(120.0),
                first: Box::new(Node::Tabs { panels: vec![1, 2], active: 0 }),
                second: Box::new(Node::Dock { side: Side::End, columns: vec![dock_column(false, [5, 6]), dock_column(true, [7, 8])] }),
            }),
        }),
        floating: vec![Floating::single(vec![3, 4], 0, [20.0, 20.0, 160.0, 160.0]).with_id(1)],
    };
    let names = |token_path: bool| {
        let layout = layout.clone();
        let mut harness = Harness::builder().with_size(egui::vec2(520.0, 360.0)).build_ui(move |ui| {
            let area = DockArea::new(egui::Id::unique("names"));
            if token_path {
                area.show_content(ui, &layout, &DockConfig::default(), &mut Names);
            } else {
                area.show_content(ui, &layout, &DockStyle::from_ui(ui), &mut Names);
            }
        });
        harness.run_steps(3);
        let mut names: Vec<String> = harness
            .root()
            .children_recursive()
            .map(|node| node.accesskit_node())
            .filter_map(|node| node.label().map(|label| format!("{:?} {label}", node.role())))
            .collect();
        names.sort();
        names
    };
    let (flat, tokens) = (names(false), names(true));
    assert!(flat.iter().any(|n| n.contains("Panel 1")), "{flat:?}");
    for iconic in ["Panel 7", "Panel 8"] {
        assert!(flat.iter().any(|n| n.contains(iconic)), "{iconic} in {flat:?}");
    }
    assert_eq!(flat, tokens);
}

/// Every dock story names its controls, in both directions and at three widths.
#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
#[test]
fn the_dock_stories_are_fully_labelled_in_both_locales() {
    use crate::testing::a11y;
    use egui_kittest::Harness;
    let stories: Vec<_> = crate::story::registry().into_iter().filter(|s| s.id.starts_with("patterns/dock-area/")).collect();
    assert_eq!(stories.len(), 4);
    for story in stories {
        for locale in ["en", "ar"] {
            for width in [360.0, 640.0, 960.0] {
                let mut args = story.defaults();
                args.set_choice("locale", locale);
                if let Some(arg) = args.iter_mut().find(|a| a.name == "width")
                    && let crate::story::ArgValue::Number { value, .. } = &mut arg.value
                {
                    *value = f64::from(width);
                }
                let show = story.show;
                let mut harness = Harness::builder().with_size(egui::vec2(width + 20.0, 420.0)).build_ui(move |ui| show(ui, &mut args));
                harness.run_steps(3);
                a11y::assert_labelled(&harness);
                if story.id.ends_with("/docked") {
                    use egui_kittest::kittest::Queryable as _;
                    harness.get_by_label("Layers");
                }
            }
        }
    }
}

/// Pins distinct colors on the active tab, focus and current-icon tokens: the tab strip, drag bar and
/// icon cell painters read them, each from its own token.
#[test]
fn the_painters_read_their_own_tokens() {
    let hover = Rgba([10, 200, 30, 255]);
    let focus = Rgba([200, 10, 220, 255]);
    let selected = Rgba([20, 60, 230, 255]);
    let selected_text = Rgba([250, 120, 10, 255]);
    let pins: Pins = [
        (ComponentField::Tab(TabField::Active), hover),
        (ComponentField::Float(FloatField::Focus), focus),
        (ComponentField::Dock(DockField::IconSelected), selected),
        (ComponentField::Dock(DockField::IconSelectedText), selected_text),
    ]
    .into_iter()
    .collect();
    let layout = Layout {
        root: Some(Node::Split {
            axis: crate::layout::SplitAxis::Horizontal,
            size: crate::layout::SplitSize::FixedSecond(340.0),
            first: Box::new(Node::Tabs { panels: vec![0], active: 0 }),
            second: Box::new(Node::Dock {
                side: Side::End,
                columns: vec![Column::new(vec![Group::new(vec![1, 2], 0)], 240.0), dock_column(true, [3, 4])],
            }),
        }),
        floating: Vec::new(),
    };
    let mut d = DockDriver::new(layout, Names).themed(pins);
    let rect_of = |d: &DockDriver<u32, Names>, hit: Hit<u32>| d.hit(&hit).unwrap_or_else(|| panic!("{hit:?}"));
    let rects = |d: &DockDriver<u32, Names>| -> Vec<egui::epaint::RectShape> {
        d.shapes().iter().filter_map(|s| if let egui::Shape::Rect(r) = &s.shape { Some(r.clone()) } else { None }).collect()
    };
    let wanted = |color: Rgba| color.to_egui();

    // A tab under the pointer is filled with `tab.active`, as the selected tab is.
    let tab = rect_of(&d, Hit::Tab(2));
    assert_ne!(tab, rect_of(&d, Hit::Tab(1)));
    assert!(!rects(&d).iter().any(|r| r.rect == tab && r.fill == wanted(hover)), "resting tab");
    d.move_to(tab.center());
    d.step(vec![]);
    assert!(rects(&d).iter().any(|r| r.rect == tab && r.fill == wanted(hover)), "hovered tab");
    d.leave();

    // Keyboard focus on a drag bar is drawn with `float.focus`, at the focus width.
    let width = look_at(Size::Compact, Mode::Light).focus_width;
    let bar = rect_of(&d, Hit::DragBar(1));
    d.ctx.memory_mut(|m| m.request_focus(DockArea::<u32>::hit_id(d.id, &Hit::DragBar(1))));
    d.step(vec![]);
    d.step(vec![]);
    let stroked = rects(&d).iter().any(|r| r.stroke.color == wanted(focus) && r.stroke.width == width && r.rect == bar.shrink(1.0));
    assert!(stroked, "drag bar focus outline");

    // A tab shows the tab component's focus ring (`line.focus`, not `float.focus`) while the
    // keyboard has the focus.
    let tab = rect_of(&d, Hit::Tab(1));
    d.ctx.memory_mut(|m| m.request_focus(DockArea::<u32>::hit_id(d.id, &Hit::Tab(1))));
    let press = egui::Event::Key { key: egui::Key::F20, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE };
    d.step(vec![press]);
    d.step(vec![]);
    let ring = rects(&d)
        .iter()
        .any(|r| r.stroke.width > 0.0 && r.stroke.color != wanted(focus) && r.rect.intersects(tab) && r.rect.size() != tab.size() && r.fill.a() == 0);
    assert!(ring, "tab focus ring");

    // The current icon cell is filled with `dock.icon.selected`, its letter in the selected text color.
    d.ctx.memory_mut(|m| m.surrender_focus(DockArea::<u32>::hit_id(d.id, &Hit::DragBar(1))));
    let cell = rect_of(&d, Hit::Icon(3));
    d.click(cell.center());
    d.leave();
    d.settle();
    let drawn = rects(&d);
    assert!(drawn.iter().any(|r| r.rect == cell.shrink(2.0) && r.fill == wanted(selected)), "current icon cell");
    assert!(drawn.iter().any(|r| r.stroke.color == wanted(selected_text)), "the letter box in the selected text color");
}

/// A floating window with rounded corners draws its title bar from the frame's top and keeps its
/// body inside the rounded bottom corners; a square window fills the frame. The top resize band
/// lies over the title bar.
#[test]
fn a_rounded_floating_window_insets_its_body_below_a_title_bar_from_the_top() {
    let at = |radius: u8| {
        let layout = Layout {
            root: Some(Node::Tabs { panels: vec![0], active: 0 }),
            floating: vec![Floating::single(vec![1], 0, [100.0, 100.0, 300.0, 200.0]).with_id(1)],
        };
        let d = DockDriver::new(layout, Names).with_look(move |l| l.floating_radius = egui::CornerRadius::same(radius));
        let hit = |hit: Hit<u32>| d.hit(&hit).unwrap_or_else(|| panic!("{hit:?}"));
        (hit(Hit::Title(1)), hit(Hit::Tab(1)), hit(Hit::Resize(1, super::ResizeEdge::Top)))
    };
    let title_height = DockLook::from_visuals(&Visuals::dark()).title_height;
    let (title, tab, band) = at(0);
    assert_eq!((title.left(), title.top()), (100.0, 100.0));
    assert_eq!((tab.left(), tab.top()), (100.0, 100.0 + title_height));
    assert_eq!((band.top(), band.bottom()), (100.0, 100.0 + super::chrome::RESIZE_BAND), "the band lies over the title bar");

    let inset = 10.0 * (1.0 - std::f32::consts::FRAC_1_SQRT_2);
    let (title, tab, _) = at(10);
    assert_eq!((title.left(), title.top()), (100.0, 100.0));
    assert!((tab.left() - (100.0 + inset)).abs() < 1e-4, "{tab:?}");
    assert_eq!(tab.top(), 100.0 + title_height);
}

/// A docked Compact slider row is PhotoCraft's 46 points: field 24, slider 18 and the row's 4, with
/// no item spacing between them (`panels/panel_docking.rs:1229`).
#[test]
fn a_docked_slider_row_is_the_photocraft_pitch() {
    use craft_theme::{AppDefaults, ModeChoice, SuiteChoice, SystemAppearance, ThemeId, ThemePair};
    use egui::accesskit::Role;
    use egui_kittest::{Harness, kittest::Queryable as _};
    struct Rows;
    impl DockContent<u32> for Rows {
        fn body(&mut self, ui: &mut Ui, _: &u32) {
            let numbers = crate::localized::LocaleNumbers::from_tag("en");
            for id in ["a", "b"] {
                let _ = crate::composites::SliderRow::new(id, id, 10.0, 0.0..=100.0, &numbers).show(ui);
            }
        }
        fn logical_label(&self, _: &u32) -> String {
            "Panel".into()
        }
        fn permissions(&self, _: &u32) -> Permissions {
            Permissions::default()
        }
    }
    let defaults = AppDefaults { theme: ThemePair::same(ThemeId::suite_default()), mode: ModeChoice::Light, size: Size::Compact };
    let base =
        crate::theme::Base::resolve(&SuiteChoice::default().with_defaults(&defaults), SystemAppearance::default(), &defaults, None, &mut |id| {
            builtins().iter().find(|t| t.id == *id).cloned()
        });
    let layout = Layout { root: Some(Node::Tabs { panels: vec![0], active: 0 }), floating: vec![] };
    let mut harness = Harness::builder()
        .with_size(egui::vec2(400.0, 400.0))
        .with_context_setup(move |ctx: &Context| crate::theme::install(ctx, &base))
        .build_ui(move |ui| {
            DockArea::new(egui::Id::unique("pitch")).show_content(ui, &layout, &DockConfig::default(), &mut Rows);
        });
    harness.run_steps(3);
    let fields: Vec<_> = harness.query_all(egui_kittest::kittest::by().role(Role::SpinButton)).map(|n| n.rect()).collect();
    assert_eq!(fields[1].top() - fields[0].top(), 46.0);
}
