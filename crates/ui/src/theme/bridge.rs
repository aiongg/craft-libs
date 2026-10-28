//! The theme in an egui context: the installed base, per-viewport scope stacks and the cache of
//! overridden tokens.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock};

use craft_theme::{
    AppDefaults, AppId, Appearance, Contrast, Mode, ModeChoice, Pins, Selection, Shown, Size, SuiteChoice, SystemAppearance, ThemeDef, ThemeId,
    ThemePair, Tokens, builtins, resolve, resolve_selection,
};
use egui::{Context, Id, Style, Theme, ThemePreference, Ui, ViewportId};

use super::convert::FontFamilies;
use super::tokens::UiTokens;
use crate::direction::{self, Direction, HasDirection};

/// What an app shows: the selection resolved for each mode, and what it was resolved from.
#[derive(Clone, Debug, PartialEq)]
pub struct Base {
    pub light: Shown,
    pub dark: Shown,
    /// Which of the two egui shows; `System` follows egui's system theme.
    pub mode: ModeChoice,
    pub inputs: Inputs,
}

/// The themes and app the two [`Shown`] of a [`Base`] came from, so scopes can re-resolve with
/// overrides. Each `Shown` holds its appearance.
#[derive(Clone, Debug, PartialEq)]
pub struct Inputs {
    pub light: ThemeDef,
    pub dark: ThemeDef,
    pub app: Option<AppId>,
}

impl Base {
    /// `sel` resolved for light and dark with [`resolve_selection`]; the mode egui shows is
    /// `sel.mode`.
    pub fn resolve(
        sel: &Selection,
        system: SystemAppearance,
        defaults: &AppDefaults,
        app: Option<&AppId>,
        lookup: &mut dyn FnMut(&ThemeId) -> Option<ThemeDef>,
    ) -> Base {
        let mut shown = |mode: Mode| {
            let choice = match mode {
                Mode::Light => ModeChoice::Light,
                Mode::Dark => ModeChoice::Dark,
            };
            let shown = resolve_selection(&Selection { mode: choice, ..sel.clone() }, system, mode, defaults, app, lookup);
            let theme = lookup(&shown.theme).or_else(|| builtin(&shown.theme)).unwrap_or_else(|| suite_default_theme().clone());
            (shown, theme)
        };
        let ((light, light_theme), (dark, dark_theme)) = (shown(Mode::Light), shown(Mode::Dark));
        Base { light, dark, mode: sel.mode, inputs: Inputs { light: light_theme, dark: dark_theme, app: app.cloned() } }
    }

    /// The suite default theme in both modes, standard contrast, compact, following the system
    /// mode: what [`Tokens::of`](super::TokensOf::of) shows before anything is installed.
    pub fn suite_default() -> Base {
        SUITE_DEFAULT_BASE.clone()
    }

    /// The tokens shown in `mode`.
    pub fn shown(&self, mode: Mode) -> &Shown {
        match mode {
            Mode::Light => &self.light,
            Mode::Dark => &self.dark,
        }
    }
}

impl Inputs {
    /// The theme resolved for `mode`.
    pub fn theme(&self, mode: Mode) -> &ThemeDef {
        match mode {
            Mode::Light => &self.light,
            Mode::Dark => &self.dark,
        }
    }
}

static SUITE_DEFAULT_BASE: LazyLock<Base> = LazyLock::new(|| {
    let defaults = AppDefaults { theme: ThemePair::same(ThemeId::suite_default()), mode: ModeChoice::System, size: Size::default() };
    Base::resolve(&SuiteChoice::default().with_defaults(&defaults), SystemAppearance::default(), &defaults, None, &mut builtin)
});

fn builtin(id: &ThemeId) -> Option<ThemeDef> {
    builtins().iter().find(|t| t.id == *id).cloned()
}

fn suite_default_theme() -> &'static ThemeDef {
    &SUITE_DEFAULT_BASE.inputs.light
}

/// Installs `base` in `ctx`: egui's light and dark styles from its tokens, egui's theme
/// preference from its mode, and the input-modality tracking that shows
/// [focus rings](crate::primitives::FocusRing) after keyboard input. Call it every frame: it
/// does nothing while `base`, the font stack and the interface direction are unchanged, and
/// requests a repaint when anything changed.
pub fn install(ctx: &Context, base: &Base) {
    crate::primitives::focus_ring::track_modality(ctx);
    let env = Env::of(ctx);
    let unchanged = with_state(ctx, |s| s.installed.as_ref().is_some_and(|i| i.env == env && i.base == *base));
    if unchanged {
        return;
    }
    let installed = Arc::new(Installed::new(base.clone(), env));
    ctx.set_style_of(Theme::Light, Arc::clone(&installed.light.style));
    ctx.set_style_of(Theme::Dark, Arc::clone(&installed.dark.style));
    ctx.set_theme(match base.mode {
        ModeChoice::Light => ThemePreference::Light,
        ModeChoice::Dark => ThemePreference::Dark,
        ModeChoice::System => ThemePreference::System,
    });
    with_state(ctx, |s| s.installed = Some(installed));
    ctx.request_repaint();
}

/// Changes a [`scope`] makes to the tokens around it. Fields left `None` (and colours not pinned)
/// keep the outer value.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Overrides {
    pub mode: Option<Mode>,
    pub contrast: Option<Contrast>,
    pub theme: Option<ThemeDef>,
    pub size: Option<Size>,
    /// Pinned over the theme's colours (and, in high contrast, over its high-contrast pins).
    pub colors: Pins,
}

impl Overrides {
    /// `inner` over `self`.
    fn then(&self, inner: Overrides) -> Overrides {
        let mut colors = self.colors.clone();
        colors.0.extend(inner.colors.0);
        Overrides {
            mode: inner.mode.or(self.mode),
            contrast: inner.contrast.or(self.contrast),
            theme: inner.theme.or_else(|| self.theme.clone()),
            size: inner.size.or(self.size),
            colors,
        }
    }

    fn changes_tokens(&self) -> bool {
        self.contrast.is_some() || self.theme.is_some() || self.size.is_some() || !self.colors.0.is_empty()
    }
}

/// Shows `add_contents` with overridden tokens: [`Tokens::of`](super::TokensOf::of) inside it
/// returns them, and its child `Ui` has their style. Scopes nest; each overrides the one around
/// it. Resolved once per set of overrides and base, then cached.
pub fn scope<R>(ui: &mut Ui, overrides: impl FnOnce(&mut Overrides), add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    let mut own = Overrides::default();
    overrides(&mut own);
    let ctx = ui.ctx().clone();
    let viewport = ctx.viewport_id();
    let outer = with_state(&ctx, |s| s.viewports.get(&viewport).and_then(Viewport::top).map(|e| e.overrides.clone()));
    let overrides = match outer {
        Some(outer) => outer.then(own),
        None => own,
    };
    let mode = overrides.mode.unwrap_or_else(|| context_mode(&ctx));
    let tokens = scoped(&ctx, &overrides, mode);
    let style = Arc::clone(&tokens.style);
    let _pushed = Pushed::new(&ctx, viewport, Arc::new(Entry { overrides, tokens }));
    ui.scope(|ui| {
        ui.set_style(style);
        add_contents(ui)
    })
    .inner
}

/// The style of the innermost scope around `ui`, or `None` outside scopes, where egui's global
/// style is already the installed base's.
pub fn scoped_style(ui: &Ui) -> Option<Arc<Style>> {
    let viewport = ui.ctx().viewport_id();
    with_state(ui.ctx(), |s| s.viewports.get(&viewport).and_then(Viewport::top).map(|e| Arc::clone(&e.tokens.style)))
}

/// Gives a popup's `Ui` the style of the scope it was opened in. Popups start from egui's global
/// style but run inline, so the scope is still in effect: call this first thing in the popup's
/// contents. Does nothing outside scopes. Also for the root `Ui` of a deferred viewport after
/// [`Captured::install`].
pub fn apply_popup_style(ui: &mut Ui) {
    if let Some(style) = scoped_style(ui) {
        ui.set_style(style);
    }
}

/// The tokens in effect at a point of the UI, for a deferred viewport, whose contents run
/// outside the scopes that were open when it was shown.
#[derive(Clone, Debug, Default)]
pub struct Captured(Option<Arc<Entry>>);

/// The innermost scope around `ui`, if any, for [`Captured::install`] in a deferred viewport.
pub fn capture(ui: &Ui) -> Captured {
    let viewport = ui.ctx().viewport_id();
    Captured(with_state(ui.ctx(), |s| s.viewports.get(&viewport).and_then(Viewport::top).cloned()))
}

impl Captured {
    /// Makes the captured tokens the base of `ctx`'s current viewport: call it at the start of
    /// the viewport's callback, which the parent replaces every frame with a fresh capture.
    pub fn install(&self, ctx: &Context) {
        let viewport = ctx.viewport_id();
        with_state(ctx, |s| {
            let entry = s.viewports.entry(viewport).or_default();
            entry.root = self.0.clone();
            if entry.is_empty() {
                s.viewports.remove(&viewport);
            }
        });
    }
}

/// The tokens for `ctx`'s current viewport and theme.
pub(super) fn current(ctx: &Context) -> Arc<UiTokens> {
    let viewport = ctx.viewport_id();
    if let Some(top) = with_state(ctx, |s| s.viewports.get(&viewport).and_then(Viewport::top).map(|e| Arc::clone(&e.tokens))) {
        return top;
    }
    let mode = context_mode(ctx);
    Arc::clone(base(ctx).tokens(mode))
}

fn context_mode(ctx: &Context) -> Mode {
    match ctx.theme() {
        Theme::Light => Mode::Light,
        Theme::Dark => Mode::Dark,
    }
}

/// The installed base, else the suite default for the context's fonts and direction.
fn base(ctx: &Context) -> Arc<Installed> {
    if let Some(installed) = with_state(ctx, |s| s.installed.clone()) {
        return installed;
    }
    let env = Env::of(ctx);
    if let Some(fallback) = with_state(ctx, |s| s.fallback.clone().filter(|f| f.env == env)) {
        return fallback;
    }
    let fallback = Arc::new(Installed::new(Base::suite_default(), env));
    with_state(ctx, |s| s.fallback = Some(Arc::clone(&fallback)));
    fallback
}

/// The tokens of `overrides` in `mode`, over the current base.
fn scoped(ctx: &Context, overrides: &Overrides, mode: Mode) -> Arc<UiTokens> {
    let base = base(ctx);
    if !overrides.changes_tokens() {
        return Arc::clone(base.tokens(mode));
    }
    let generation = base.generation;
    let hit = with_state(ctx, |s| {
        s.cache.iter().find(|c| c.generation == generation && c.mode == mode && c.overrides == *overrides).map(|c| Arc::clone(&c.tokens))
    });
    if let Some(tokens) = hit {
        return tokens;
    }
    let tokens = Arc::new(base.resolve(overrides, mode));
    with_state(ctx, |s| {
        s.cache.retain(|c| c.generation == generation);
        if s.cache.len() >= CACHE_SIZE {
            s.cache.remove(0);
        }
        s.cache.push(Cached { generation, mode, overrides: overrides.clone(), tokens: Arc::clone(&tokens) });
    });
    tokens
}

/// Override sets kept per base; a UI has a handful of distinct scopes.
const CACHE_SIZE: usize = 32;

fn with_state<R>(ctx: &Context, f: impl FnOnce(&mut State) -> R) -> R {
    ctx.data_mut(|d| f(d.get_temp_mut_or_default::<State>(Id::unique("craft_ui::theme"))))
}

#[derive(Clone, Default)]
struct State {
    installed: Option<Arc<Installed>>,
    /// The suite default, built on first use while nothing is installed.
    fallback: Option<Arc<Installed>>,
    viewports: HashMap<ViewportId, Viewport>,
    cache: Vec<Cached>,
}

/// What the styles depend on beyond the tokens.
#[derive(Clone, Debug, PartialEq)]
struct Env {
    fonts: FontFamilies,
    direction: Direction,
}

impl Env {
    fn of(ctx: &Context) -> Env {
        Env { fonts: FontFamilies::installed(ctx), direction: ctx.direction() }
    }
}

struct Installed {
    base: Base,
    env: Env,
    light: Arc<UiTokens>,
    dark: Arc<UiTokens>,
    /// Distinguishes cache entries of different bases.
    generation: u64,
}

impl Installed {
    fn new(base: Base, env: Env) -> Installed {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let light = Arc::new(ui_tokens(base.light.resolved.tokens.clone(), base.light.appearance, base.light.resolved.variant_used, &env));
        let dark = Arc::new(ui_tokens(base.dark.resolved.tokens.clone(), base.dark.appearance, base.dark.resolved.variant_used, &env));
        Installed { base, env, light, dark, generation: NEXT.fetch_add(1, Ordering::Relaxed) }
    }

    fn tokens(&self, mode: Mode) -> &Arc<UiTokens> {
        match mode {
            Mode::Light => &self.light,
            Mode::Dark => &self.dark,
        }
    }

    /// Re-resolves the base's theme for `mode` with `overrides`.
    fn resolve(&self, overrides: &Overrides, mode: Mode) -> UiTokens {
        let shown = self.base.shown(mode);
        let mut theme = overrides.theme.clone().unwrap_or_else(|| self.base.inputs.theme(mode).clone());
        let appearance = Appearance {
            mode,
            contrast: overrides.contrast.unwrap_or(shown.appearance.contrast),
            size: overrides.size.unwrap_or(shown.appearance.size),
            ..shown.appearance
        };
        let app = self.base.inputs.app.as_ref();
        if !overrides.colors.0.is_empty() {
            // The last layer resolution applies: high-contrast pins, else the app's, else the theme's.
            let pins = match (appearance.contrast, app) {
                (Contrast::High, _) => &mut theme.high_contrast,
                (Contrast::Standard, Some(app)) => &mut theme.apps.entry(app.clone()).or_default().pins,
                (Contrast::Standard, None) => &mut theme.pins,
            };
            for layer in [&mut pins.light, &mut pins.dark] {
                layer.0.extend(overrides.colors.0.iter().map(|(k, v)| (*k, *v)));
            }
        }
        let resolved = resolve(&theme, appearance, app);
        ui_tokens(resolved.tokens, appearance, resolved.variant_used, &self.env)
    }
}

fn ui_tokens(tokens: Tokens, appearance: Appearance, variant: Mode, env: &Env) -> UiTokens {
    let mut ui_tokens = UiTokens::new(tokens, appearance, variant, env.fonts.clone());
    direction::set_on(Arc::make_mut(&mut ui_tokens.style), env.direction);
    ui_tokens
}

#[derive(Clone, Default)]
struct Viewport {
    /// From [`Captured::install`].
    root: Option<Arc<Entry>>,
    stack: Vec<Arc<Entry>>,
}

impl Viewport {
    fn top(&self) -> Option<&Arc<Entry>> {
        self.stack.last().or(self.root.as_ref())
    }

    fn is_empty(&self) -> bool {
        self.root.is_none() && self.stack.is_empty()
    }
}

#[derive(Debug)]
struct Entry {
    /// Every scope's overrides up to this one, combined.
    overrides: Overrides,
    tokens: Arc<UiTokens>,
}

#[derive(Clone)]
struct Cached {
    generation: u64,
    mode: Mode,
    overrides: Overrides,
    tokens: Arc<UiTokens>,
}

/// A pushed scope; pops it when dropped, also when the scope's contents panic.
struct Pushed {
    ctx: Context,
    viewport: ViewportId,
    depth: usize,
}

impl Pushed {
    fn new(ctx: &Context, viewport: ViewportId, entry: Arc<Entry>) -> Pushed {
        let depth = with_state(ctx, |s| {
            let stack = &mut s.viewports.entry(viewport).or_default().stack;
            stack.push(entry);
            stack.len() - 1
        });
        Pushed { ctx: ctx.clone(), viewport, depth }
    }
}

impl Drop for Pushed {
    fn drop(&mut self) {
        let (viewport, depth) = (self.viewport, self.depth);
        with_state(&self.ctx, |s| {
            if let Some(v) = s.viewports.get_mut(&viewport) {
                v.stack.truncate(depth);
                if v.is_empty() {
                    s.viewports.remove(&viewport);
                }
            }
        });
    }
}
