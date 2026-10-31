use egui::{Color32, Context, CornerRadius, Event, Id, Pos2, RawInput, Rect, Shadow, Shape, Ui, epaint::ClippedShape, pos2, vec2};

use super::DockLook;
use super::chrome::{ChromeStyle, FloatingFrame, TitleBar, drop_preview_with};

const RECT: Rect = Rect { min: pos2(100.0, 100.0), max: pos2(400.0, 300.0) };

fn run(ctx: &Context, events: Vec<Event>, add: impl FnOnce(&mut Ui)) -> Vec<ClippedShape> {
    let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))), events, ..Default::default() };
    let mut add = Some(add);
    let mut output = ctx.run_ui(input, |ui| (add.take().unwrap())(ui));
    output.textures_delta.clear();
    output.shapes
}

fn rects(shapes: &[ClippedShape]) -> Vec<egui::epaint::RectShape> {
    shapes.iter().filter_map(|s| if let Shape::Rect(rect) = &s.shape { Some(rect.clone()) } else { None }).collect()
}

/// A frame drawn from a style with radius and shadow paints the shadow first, then a rounded
/// fill and a rounded outline, and its title bar rounds only its top corners. The legacy styles
/// keep square corners and no shadow.
#[test]
fn a_floating_frame_rounds_its_fill_outline_and_title_bar_and_casts_the_shadow() {
    let radius = CornerRadius::same(6);
    let shadow = Shadow { offset: [0, 4], blur: 12, spread: 1, color: Color32::from_black_alpha(90) };
    let chrome = |style: ChromeStyle| {
        let ctx = Context::default();
        let id = Id::unique("frame");
        let draw = || {
            run(&ctx, vec![], |ui| {
                FloatingFrame::new(id, RECT).resizable(false).show(ui.ctx(), &style, |ui, inner| {
                    let bar = Rect::from_min_size(inner.min, vec2(inner.width(), 14.0));
                    TitleBar::new(id.with("title"), "Move").title("Title").show(ui, bar, &style);
                });
            })
        };
        draw();
        rects(&draw())
    };
    let base = ChromeStyle { radius: CornerRadius::ZERO, shadow: Shadow::NONE, ..crate::docking::testing::default_chrome() };
    let token = ChromeStyle { radius, shadow, ..base.clone() };

    let shapes = chrome(token.clone());
    let cast = shapes.iter().position(|r| r.blur_width > 0.0).expect("a shadow shape");
    assert_eq!(cast, 0, "the shadow is under the fill");
    let fill = shapes.iter().find(|r| r.fill == token.background && r.rect == RECT).expect("fill");
    assert_eq!(fill.corner_radius, radius);
    let outline = shapes.iter().find(|r| r.stroke == token.stroke && r.fill == Color32::TRANSPARENT).expect("outline");
    assert_eq!(outline.corner_radius, radius);
    let title = shapes.iter().find(|r| r.fill == token.title_background).expect("title bar");
    assert_eq!(title.corner_radius, CornerRadius { nw: 6, ne: 6, sw: 0, se: 0 });

    let square = chrome(base);
    assert!(square.iter().all(|r| r.blur_width == 0.0), "no shadow");
    assert!(square.iter().all(|r| r.corner_radius == CornerRadius::ZERO), "square corners");
}

/// `ChromeStyle::from_look` takes the title, grip, focus and frame values of the look, so a
/// floating panel outside docking can draw the same chrome with `ChromeStyle::themed`.
#[test]
fn the_chrome_style_is_built_from_the_look() {
    let mut look = crate::docking::testing::default_look();
    look.title_background = Color32::RED;
    look.title_text = Color32::GREEN;
    look.title_muted = Color32::BLUE;
    look.grip = Color32::YELLOW;
    look.drop_width = 3.0;
    look.floating_radius = CornerRadius::same(5);
    let style = ChromeStyle::from_look(&look, &super::DockConfig::default());
    assert_eq!(
        (style.title_background, style.title_text, style.title_muted, style.grip, style.drop_width, style.radius),
        (Color32::RED, Color32::GREEN, Color32::BLUE, Color32::YELLOW, 3.0, CornerRadius::same(5))
    );
    let ctx = Context::default();
    run(&ctx, vec![], |ui| {
        let themed = ChromeStyle::themed(ui);
        let look = DockLook::of(ui);
        assert_eq!(
            (themed.background, themed.shadow, themed.title_background),
            (look.floating_background, look.floating_shadow, look.title_background)
        );
    });
}

/// The diagonal grip is `drop_width` thick in the `grip` colour while the handle is hovered.
#[test]
fn the_grip_uses_the_grip_color_and_the_drop_width() {
    let style = ChromeStyle { grip: Color32::YELLOW, drop_width: 3.0, ..crate::docking::testing::default_chrome() };
    let ctx = Context::default();
    let id = Id::unique("frame");
    let corner = RECT.max - vec2(4.0, 4.0);
    let draw = |events| run(&ctx, events, |ui| drop(FloatingFrame::new(id, RECT).show(ui.ctx(), &style, |_, _| ())));
    draw(vec![]);
    draw(vec![Event::PointerMoved(corner)]);
    let shapes = draw(vec![Event::PointerMoved(corner + vec2(0.5, 0.0))]);
    let grip = shapes.iter().find_map(|s| if let Shape::LineSegment { stroke, .. } = &s.shape { Some(*stroke) } else { None }).expect("grip");
    assert_eq!((grip.color, grip.width), (Color32::YELLOW, 3.0));
}

/// `drop_preview_with` takes the outline width and fill factor from the look.
#[test]
fn a_drop_preview_follows_the_look() {
    let mut look = crate::docking::testing::default_look();
    look.drop = Color32::from_rgb(200, 100, 50);
    look.drop_width = 3.0;
    look.drop_fill = 0.5;
    let ctx = Context::default();
    let shapes = run(&ctx, vec![], |ui| {
        drop_preview_with(ui.painter(), RECT, &look, true);
        drop_preview_with(ui.painter(), RECT, &look, false);
    });
    let rects = rects(&shapes);
    assert_eq!(rects[0].stroke.width, 3.0);
    assert_eq!(rects[0].stroke.color, look.drop);
    assert_eq!(rects[0].fill, look.drop.gamma_multiply(0.5));
    assert_eq!(rects[1].fill, Color32::TRANSPARENT);
}

/// A resize cannot shrink the frame below the style's title bar and room for the contents, so the minimum height follows the style's title height.
#[test]
fn the_minimum_height_follows_the_title_height() {
    let minimum = |style: ChromeStyle| {
        let ctx = Context::default();
        let id = Id::unique("frame");
        let bottom = pos2(RECT.center().x, RECT.bottom() - 1.0);
        let mut smallest = f32::MAX;
        let pointer = |pos| Event::PointerMoved(pos);
        let press = |pressed| Event::PointerButton { pos: bottom, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
        let steps =
            [vec![], vec![pointer(bottom)], vec![press(true)], vec![pointer(bottom - vec2(0.0, 20.0))], vec![pointer(bottom - vec2(0.0, 400.0))]];
        for events in steps {
            run(&ctx, events, |ui| {
                if let Some(rect) = FloatingFrame::new(id, RECT).show(ui.ctx(), &style, |_, _| ()).resized {
                    smallest = smallest.min(rect.height());
                }
            });
        }
        smallest
    };
    let themed = crate::docking::testing::default_chrome();
    assert_eq!(themed.title_height, 14.0);
    assert_eq!(minimum(themed.clone()), 14.0 + 20.0);
    assert_eq!(minimum(ChromeStyle { title_height: 18.0, ..themed }), 18.0 + 20.0);
}

/// A resizable frame hands its contents the whole rect: the title bar starts at the frame's top,
/// so its rounded top corners follow the frame's, and the resize band is a hit area over it.
#[test]
fn a_resizable_frame_with_a_radius_draws_its_title_bar_from_the_top() {
    let radius = CornerRadius::same(6);
    let style = ChromeStyle { radius, ..crate::docking::testing::default_chrome() };
    let ctx = Context::default();
    let id = Id::unique("frame");
    let draw = || {
        let mut contents = None;
        let shapes = run(&ctx, vec![], |ui| {
            FloatingFrame::new(id, RECT).show(ui.ctx(), &style, |ui, inner| {
                contents = Some(inner);
                let bar = Rect::from_min_size(inner.min, vec2(inner.width(), style.title_height));
                TitleBar::new(id.with("title"), "Move").title("Title").show(ui, bar, &style);
            });
        });
        (contents.unwrap(), rects(&shapes))
    };
    draw();
    let (contents, shapes) = draw();
    assert_eq!(contents, RECT);
    let title = shapes.iter().find(|r| r.fill == style.title_background).expect("title bar");
    assert_eq!(title.rect.top(), RECT.top());
    assert_eq!(title.corner_radius, CornerRadius { nw: 6, ne: 6, sw: 0, se: 0 });
    let inset = style.body_inset();
    assert!((inset - 6.0 * (1.0 - std::f32::consts::FRAC_1_SQRT_2)).abs() < 1e-6, "{inset}");
    assert_eq!(ChromeStyle { radius: CornerRadius::ZERO, ..style }.body_inset(), 0.0);
}

/// A focused edge handle shows the focus stroke on its band; the corner grip turns to the focus
/// color. Hover leaves the edge handles unpainted.
#[test]
fn a_focused_resize_handle_shows_the_focus_stroke() {
    let style = ChromeStyle { focus: Color32::GOLD, focus_width: 2.0, ..crate::docking::testing::default_chrome() };
    let ctx = Context::default();
    let id = Id::unique("frame");
    let draw = |ctx: &Context| run(ctx, vec![], |ui| drop(FloatingFrame::new(id, RECT).show(ui.ctx(), &style, |_, _| ())));
    draw(&ctx);
    assert!(rects(&draw(&ctx)).iter().all(|r| r.stroke.color != Color32::GOLD), "no focus yet");

    ctx.memory_mut(|m| m.request_focus(id.with("resize-left")));
    draw(&ctx);
    let left = Rect::from_min_max(pos2(RECT.left(), RECT.top() + 12.0), pos2(RECT.left() + 4.0, RECT.bottom() - 12.0));
    let focus = rects(&draw(&ctx)).into_iter().find(|r| r.rect == left).expect("the focused left handle is outlined");
    assert_eq!((focus.stroke.color, focus.stroke.width), (Color32::GOLD, 2.0));

    ctx.memory_mut(|m| m.request_focus(id.with("resize")));
    draw(&ctx);
    let shapes = draw(&ctx);
    let grip = shapes.iter().find_map(|s| if let Shape::LineSegment { stroke, .. } = &s.shape { Some(*stroke) } else { None }).expect("grip");
    assert_eq!(grip.color, Color32::GOLD);
}

/// `ChromeStyle::themed` mirrors the title bar in a right-to-left ui.
#[test]
fn the_themed_chrome_follows_the_direction_of_the_ui() {
    let ctx = Context::default();
    let themed = |ctx: &Context| {
        let mut rtl = false;
        run(ctx, vec![], |ui| rtl = ChromeStyle::themed(ui).right_to_left);
        rtl
    };
    assert!(!themed(&ctx));
    crate::direction::apply(&ctx, crate::direction::Direction::Rtl);
    assert!(themed(&ctx));
}
