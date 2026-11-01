//! Stories of the Navigator.
//!
//! Every story uses fakes: a generated gradient image put into a story-local `TextureCache` for
//! `NavigatorPreview::Texture`, a `CanvasView` the story keeps in egui memory, and story words
//! (the gallery's pseudo-locale test). Canvas stories show `.on(Surface::Canvas)`. A zoom the
//! user starts from the zoom row's buttons is applied at once; the stories run no zoom animation.
//!
//! | Story id | Shows |
//! |---|---|
//! | `patterns/navigator/document` | a landscape document, the view zoomed to 400 %; args: zoom, rotation, flip, proxy colour, preview waiting |
//! | `patterns/navigator/whole-document` | the fit view: the proxy covers the preview |
//! | `patterns/navigator/options-dialog` | Panel Options with the colour list and Custom… |

use std::sync::{Arc, Mutex, PoisonError};

use egui::{ColorImage, Context, Rect, Sense, Ui, UiBuilder, pos2, vec2};
use kurbo::{Point, Size};

use super::{Navigator, NavigatorLabels, NavigatorOptions, NavigatorOptionsDialog, NavigatorPreview, NavigatorSettings, ProxyColor};
use crate::canvas::textures::{Revision, TextureCache, TextureCacheOptions};
use crate::canvas::{CanvasView, NavigationOptions};
use crate::controls::Button;
use crate::dialog::{DialogHost, DialogLabels, DialogResult};
use crate::localized::{Labels as _, LocaleNumbers, SharedLabels as _};
use crate::story::{Level, Story, StoryArgs, StoryChoice, StoryLabels, Surface};
use crate::typed_story;
use craft_theme::Rgba;

/// The story document, in document units.
const DOC: Size = Size::new(1600.0, 1000.0);
/// The fake canvas the proxy shows, in points.
const CANVAS: egui::Vec2 = vec2(960.0, 600.0);
/// The panel the Navigator is shown in, in points.
const PANEL: egui::Vec2 = vec2(280.0, 340.0);
/// The generated preview picture, in pixels.
const PICTURE: [usize; 2] = [320, 200];

/// The proxy colours the stories offer; Custom… needs the dialog's picker.
#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum ColorArg {
    Default,
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Violet,
    Gray,
}

impl ColorArg {
    fn proxy(self) -> ProxyColor {
        match self {
            ColorArg::Default => ProxyColor::Default,
            ColorArg::Red => ProxyColor::Red,
            ColorArg::Orange => ProxyColor::Orange,
            ColorArg::Yellow => ProxyColor::Yellow,
            ColorArg::Green => ProxyColor::Green,
            ColorArg::Blue => ProxyColor::Blue,
            ColorArg::Violet => ProxyColor::Violet,
            ColorArg::Gray => ProxyColor::Gray,
        }
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
struct DocumentArgs {
    #[arg(range = 10.0..=1600.0)]
    zoom: f64,
    #[arg(range = -180.0..=180.0)]
    rotation: f64,
    flip: bool,
    #[arg(name = "proxy color")]
    proxy: ColorArg,
    #[arg(name = "preview waiting")]
    waiting: bool,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct WholeArgs {
    #[arg(name = "proxy color")]
    proxy: ColorArg,
    #[arg(name = "preview waiting")]
    waiting: bool,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct OptionsArgs {
    open: bool,
    #[arg(name = "proxy color")]
    proxy: ColorArg,
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Navigator"];

/// The three stories above.
pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "patterns/navigator/document",
            Level::Patterns,
            "Navigator",
            "Document",
            DocumentArgs { zoom: 400.0, rotation: 0.0, flip: false, proxy: ColorArg::Default, waiting: false },
            show_document
        )
        .on(Surface::Canvas),
        typed_story!(
            "patterns/navigator/whole-document",
            Level::Patterns,
            "Navigator",
            "Whole document",
            WholeArgs { proxy: ColorArg::Default, waiting: false },
            show_whole
        )
        .on(Surface::Canvas),
        typed_story!(
            "patterns/navigator/options-dialog",
            Level::Patterns,
            "Navigator",
            "Options dialog",
            OptionsArgs { open: true, proxy: ColorArg::Default },
            show_options
        )
        .on(Surface::Canvas),
    ]
}

/// The view a story keeps between passes, with the args it was seeded from.
#[derive(Clone, Copy)]
struct Kept {
    view: CanvasView,
    seeded: [f64; 4],
}

/// The generated document picture: a diagonal colour gradient with a lighter frame, so the
/// proxy reads against both bright and dark areas.
fn picture() -> ColorImage {
    let [width, height] = PICTURE;
    let mut rgba = Vec::with_capacity(width * height * 4);
    for y in 0..height {
        for x in 0..width {
            let (u, v) = (x as f32 / (width - 1) as f32, y as f32 / (height - 1) as f32);
            let edge = x < 4 || y < 4 || x >= width - 4 || y >= height - 4;
            let shade = if edge { 1.0 } else { 0.85 };
            let channel = |value: f32| (value * shade * 255.0) as u8;
            rgba.extend_from_slice(&[channel(0.15 + 0.7 * u), channel(0.25 + 0.5 * v), channel(0.9 - 0.6 * u * v), 255]);
        }
    }
    ColorImage::from_rgba_unmultiplied(PICTURE, &rgba)
}

/// The preview picture, made once into the story's own texture cache.
fn texture(ctx: &Context) -> crate::canvas::textures::CachedTexture {
    let cache: TextureCache<&'static str> = TextureCache::in_context(ctx, "craft_ui::canvas::navigator::stories", TextureCacheOptions::default());
    cache.get_or_put(ctx, "document", Revision(1), picture)
}

/// The fake canvas rectangle in points, at the origin.
fn canvas_rect() -> Rect {
    Rect::from_min_size(pos2(0.0, 0.0), CANVAS)
}

/// Shows the Navigator in a fixed-size panel on the view the story keeps. `seed` is called with
/// the args as numbers and gives the view when they change; the centre the user dragged to is kept.
fn show_navigator(ui: &mut Ui, name: &str, seeded: [f64; 4], seed: impl FnOnce(Point) -> CanvasView, proxy: ColorArg, waiting: bool) {
    let ctx = ui.ctx().clone();
    let key = ui.scope_id().with(("navigator-story-view", name));
    let kept: Option<Kept> = ctx.data(|d| d.get_temp(key));
    let mut view = match kept {
        Some(kept) if kept.seeded == seeded => kept.view,
        Some(kept) => seed(kept.view.center),
        None => seed(Point::new(DOC.width * 0.35, DOC.height * 0.4)),
    };
    let numbers = LocaleNumbers::of(ui);
    let mut settings = NavigatorSettings { options: NavigatorOptions { proxy_color: proxy.proxy() } };
    let navigation = NavigationOptions::default();
    let preview = if waiting { NavigatorPreview::Waiting } else { NavigatorPreview::Texture(texture(&ctx)) };
    let (panel, _) = ui.allocate_exact_size(PANEL, Sense::hover());
    ui.scope_builder(UiBuilder::new().max_rect(panel), |ui| {
        Navigator::new(("navigator-story", name), DOC, &numbers).preview(preview).navigation(&navigation).settings(&mut settings).show(ui, &mut view);
    });
    ctx.data_mut(|d| d.insert_temp(key, Kept { view, seeded }));
}

/// A landscape document under a zoomed view: the proxy is a small rectangle (a quadrilateral when
/// the view is rotated), dragged with the pointer, with the zoom row under the picture.
fn show_document(ui: &mut Ui, args: &mut DocumentArgs) {
    let ppp = f64::from(ui.pixels_per_point());
    let seeded = [args.zoom, args.rotation, f64::from(args.flip), ppp];
    let (zoom, rotation, flip) = (args.zoom, args.rotation, args.flip);
    let seed = |center| CanvasView { rect: canvas_rect(), center, zoom: zoom / 100.0 / ppp, rotation, flip_x: flip, aspect: 1.0 };
    show_navigator(ui, "document", seeded, seed, args.proxy, args.waiting);
}

/// The fit view: the canvas shows the whole document, so the proxy covers the picture.
fn show_whole(ui: &mut Ui, args: &mut WholeArgs) {
    let zoom = (f64::from(CANVAS.x) / DOC.width).min(f64::from(CANVAS.y) / DOC.height);
    let seed = |_| CanvasView {
        rect: canvas_rect(),
        center: Point::new(DOC.width / 2.0, DOC.height / 2.0),
        zoom,
        rotation: 0.0,
        flip_x: false,
        aspect: 1.0,
    };
    show_navigator(ui, "whole", [0.0; 4], seed, args.proxy, args.waiting);
}

type SharedHost = Arc<Mutex<DialogHost>>;

/// Panel Options on a [`DialogHost`] kept in egui memory. The colour picker is the story's: a
/// request for Custom… is answered at once with a fixed colour. The answer shows as text under
/// the tile.
fn show_options(ui: &mut Ui, args: &mut OptionsArgs) {
    let ctx = ui.ctx().clone();
    if !crate::story::claim(&ctx, "navigator-options", ui.scope_id()) {
        return;
    }
    let host_key = ui.make_persistent_id("craft_ui::canvas::navigator::stories::host");
    let (result_key, signature_key) = (host_key.with("result"), host_key.with("signature"));
    let shared: SharedHost = ui.data_mut(|d| d.get_temp_mut_or_insert_with(host_key, SharedHost::default).clone());
    let mut host = shared.lock().unwrap_or_else(PoisonError::into_inner);
    let initial = args.proxy.proxy();
    if crate::story::dialog_shown(ui, args.open) {
        let previous: Option<ProxyColor> = ui.data(|d| d.get_temp::<Option<ProxyColor>>(signature_key)).flatten();
        if previous.is_some_and(|previous| previous != initial) {
            host.cancel::<NavigatorOptionsDialog>(&ctx);
            host.take::<NavigatorOptionsDialog>();
        }
        ui.data_mut(|d| d.insert_temp(signature_key, Some(initial)));
        if !host.is_open::<NavigatorOptionsDialog>() {
            host.open(NavigatorOptionsDialog::new().initial(NavigatorOptions { proxy_color: initial }));
            ui.data_mut(|d| d.remove_temp::<String>(result_key));
        }
        if let Some(dialog) = host.get_mut::<NavigatorOptionsDialog>()
            && let Some(request) = dialog.pick_color()
        {
            dialog.set_color(&request.key, Rgba::rgb(0xe0, 0x6c, 0x1f).into());
        }
        host.show(&ctx);
        if let Some(result) = host.take::<NavigatorOptionsDialog>() {
            let text = match result {
                DialogResult::Ok(options) => {
                    let value = options.proxy_color.label(&NavigatorLabels::shared_ctx(&ctx)).to_owned();
                    craft_i18n::fmt(&StoryLabels::of(ui).result_ok, &[("value", &value)])
                }
                DialogResult::Cancel => DialogLabels::of_ctx(&ctx).cancel.clone(),
            };
            ui.data_mut(|d| d.insert_temp(result_key, text));
            crate::story::dialog_closed(ui, &mut args.open);
        }
    } else {
        host.cancel::<NavigatorOptionsDialog>(&ctx);
        host.take::<NavigatorOptionsDialog>();
        ui.data_mut(|d| d.insert_temp(signature_key, None::<ProxyColor>));
        if !crate::story::in_overview(&ctx) && ui.add(Button::new("Show dialog")).clicked() {
            args.open = true;
        }
    }
    if let Some(text) = ui.data(|d| d.get_temp::<String>(result_key)) {
        ui.label(text);
    }
}
