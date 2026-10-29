//! [L] The dock look: token resolution per Size, parity of the `DockStyle` split, scope overrides,
//! the token path's accessible names, and the stories.

use craft_theme::{Appearance, ComponentField, Contrast, Mode, Pins, Rgba, Size, TabField, TextSize, builtins, resolve};
use egui::{Context, FontId, Ui, Visuals};

use super::{DockArea, DockConfig, DockContent, DockLook, DockSpec, DockStyle, Floating, Layout, Node, Permissions};
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
    let sizes =
        |look: &DockLook| [look.tab_height, look.gap, look.tab_padding, look.title_height, look.column_bar_height, look.iconic_width, look.icon_size];
    assert_eq!(sizes(&look_at(Size::Compact, Mode::Light)), [26.0, 4.0, 24.0, 14.0, 14.0, 36.0, 20.0]);
    assert_eq!(sizes(&look_at(Size::Comfortable, Mode::Light)), [30.0, 6.0, 32.0, 18.0, 18.0, 44.0, 22.0]);
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

/// Light and dark resolve to different strips, active tabs and drop colours, none of them the
/// colours `DockLook::from_visuals` takes from egui.
#[test]
fn the_themed_look_differs_between_modes_and_from_the_visuals_look() {
    let (light, dark) = (look_at(Size::Compact, Mode::Light), look_at(Size::Compact, Mode::Dark));
    for (name, a, b) in [("strip", light.tab_strip, dark.tab_strip), ("active", light.tab_active, dark.tab_active), ("drop", light.drop, dark.drop)] {
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

/// The accessible names of tabs, drag bars, dividers, resize handles and title bar buttons are
/// the same whether the area draws from a `DockStyle` or from the token path.
#[test]
fn the_accessible_names_are_the_same_on_the_token_path() {
    use egui_kittest::{Harness, kittest::NodeT as _};
    let layout = Layout {
        root: Some(Node::Split {
            axis: crate::layout::SplitAxis::Horizontal,
            size: crate::layout::SplitSize::FixedSecond(200.0),
            first: Box::new(Node::Tabs { panels: vec![0], active: 0 }),
            second: Box::new(Node::Tabs { panels: vec![1, 2], active: 0 }),
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
