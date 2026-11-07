//! `Env` and scopes, app values and `content_env`.

mod label;

pub use label::{Label, Word};

use crate::runtime::egui::memory::Service;
use crate::runtime::egui::{child, input as raw_input, pass, repaint, scale, stack, style};
use crate::runtime::frame::PerViewport;
use crate::style::{EnvKey, Ink, UiTokens};
use craft_behavior::{CanvasTransform, MotionMode};
use craft_host::Host;
use craft_host::target::Os;
pub use craft_i18n::Direction;
use craft_theme::{
    AppSizes, Appearance, Contrast, ControlSize, Mode, MotionTokens, SUITE_DEFAULT, Size, TextSize, ThemeDef, builtins, resolve_sized,
};
pub use craft_ui_macros::EnvValue;
use egui::{Context, Id, Ui, ViewportId};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::sync::Arc;

/// The interface scale: pixels per point.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Scale(pub f32);

impl Scale {
    /// `pixels_per_point` pixels per point.
    pub const fn new(pixels_per_point: f32) -> Self {
        Scale(pixels_per_point)
    }
}

impl Default for Scale {
    fn default() -> Self {
        Scale(1.0)
    }
}

/// The motion mode in effect, carried in `Env.motion`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct MotionPolicy {
    /// The level animations run at.
    pub mode: MotionMode,
    /// The durations and curves of the theme's motions; reduced motion keeps them.
    pub tokens: MotionTokens,
}

impl MotionPolicy {
    /// The policy that runs animations at `mode` with `tokens`.
    pub const fn new(mode: MotionMode, tokens: MotionTokens) -> Self {
        Self { mode, tokens }
    }
}

/// The id of an arena, carried in `Env.arena`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct ArenaId(pub Id);

impl ArenaId {
    /// The arena whose id is `id`.
    pub const fn new(id: Id) -> Self {
        ArenaId(id)
    }
}

/// An app-defined value inherited down the tree like the library's own.
pub trait EnvValue: Clone + PartialEq + Send + Sync + 'static {
    /// A value that changes how components look is folded into the style key, so cached styles follow it.
    const AFFECTS_STYLE: bool = false;

    /// The value where nothing above sets one; computed at its first `get`, outside every hook.
    fn root() -> Self;

    /// The hash of the value, folded into `EnvKey` when `AFFECTS_STYLE`. Written by `#[env(style)]`; a value
    /// that does not affect style returns `None`.
    #[doc(hidden)]
    fn style_hash(&self) -> Option<u64> {
        None
    }
}

/// Marker for values `style` may read. `Hash`, because the value is folded into the style key.
pub trait StyleValue: EnvValue + Hash {}

/// The app values a root, a scope or a component's `content_env` carries: one entry per `EnvValue` type, the
/// innermost wins.
#[derive(Clone, Default)]
pub struct AppValues {
    map: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
    /// The hash of each style-affecting value, by type.
    style: HashMap<TypeId, u64>,
}

impl AppValues {
    /// Sets the value of type `T`.
    pub fn set<T: EnvValue>(&mut self, value: T) -> &mut Self {
        debug_assert!(
            !T::AFFECTS_STYLE || value.style_hash().is_some(),
            "{} sets AFFECTS_STYLE but gives no style hash: use `#[env(style)]`",
            std::any::type_name::<T>()
        );
        match value.style_hash().filter(|_| T::AFFECTS_STYLE) {
            Some(hash) => self.style.insert(TypeId::of::<T>(), hash),
            None => self.style.remove(&TypeId::of::<T>()),
        };
        self.map.insert(TypeId::of::<T>(), Arc::new(value));
        self
    }

    /// The hash of every style-affecting value, independent of the order they were set in.
    pub(crate) fn style_hash(&self) -> u64 {
        self.style
            .iter()
            .map(|(ty, hash)| {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                (ty, hash).hash(&mut hasher);
                hasher.finish()
            })
            .fold(0, u64::wrapping_add)
    }

    /// A clone of the value, else `T::root()`.
    pub(crate) fn get<T: EnvValue>(&self) -> T {
        self.map.get(&TypeId::of::<T>()).and_then(|value| value.downcast_ref::<T>()).cloned().unwrap_or_else(T::root)
    }

    /// Takes every value `other` sets over this set's.
    pub(crate) fn extend(&mut self, other: &AppValues) {
        for (ty, value) in &other.map {
            self.map.insert(*ty, Arc::clone(value));
            match other.style.get(ty) {
                Some(hash) => self.style.insert(*ty, *hash),
                None => self.style.remove(ty),
            };
        }
    }
}

/// The app's root values each frame.
#[derive(Clone)]
#[non_exhaustive]
pub struct RootEnv {
    /// The app's `sizes.json`.
    pub sizes: Arc<AppSizes>,
    /// The text-size step.
    pub text_size: TextSize,
    /// The layout direction.
    pub direction: Direction,
    /// The motion mode.
    pub motion: MotionMode,
    /// Light or dark.
    pub mode: Mode,
    /// Standard or high contrast.
    pub contrast: Contrast,
    /// The density preset.
    pub density: Size,
    /// The interface scale over the native pixels per point.
    pub scale: Scale,
    /// Whether Tab reaches every control.
    pub tab_all_controls: bool,
    /// Root app values.
    pub values: AppValues,
}

impl Default for RootEnv {
    fn default() -> Self {
        RootEnv {
            sizes: Arc::new(AppSizes::default()),
            text_size: TextSize::default(),
            direction: Direction::Ltr,
            motion: MotionMode::default(),
            mode: Mode::Light,
            contrast: Contrast::Standard,
            density: Size::default(),
            scale: Scale::default(),
            tab_all_controls: true,
            values: AppValues::default(),
        }
    }
}

impl RootEnv {
    /// The default root of `ctx`, built once so its sizes keep one identity for the token cache.
    pub(crate) fn fallback(ctx: &Context) -> RootEnv {
        #[derive(Default)]
        struct Fallback(Option<RootEnv>);
        Service::<Fallback>::of(ctx).with(|fallback| fallback.0.get_or_insert_with(RootEnv::default).clone())
    }

    /// A root with every value at its default.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the interface scale.
    pub fn scale(mut self, scale: Scale) -> Self {
        self.scale = scale;
        self
    }

    /// Sets the layout direction.
    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    /// Sets the motion mode.
    pub fn motion(mut self, motion: MotionMode) -> Self {
        self.motion = motion;
        self
    }

    /// Sets light or dark.
    pub fn mode(mut self, mode: Mode) -> Self {
        self.mode = mode;
        self
    }

    /// Sets standard or high contrast.
    pub fn contrast(mut self, contrast: Contrast) -> Self {
        self.contrast = contrast;
        self
    }

    /// Sets the density preset.
    pub fn density(mut self, density: Size) -> Self {
        self.density = density;
        self
    }
}

/// The inherited context of a `Ui`: one `Arc` clone per read.
#[derive(Clone)]
pub struct Env {
    scoped: Arc<Scoped>,
}

impl Deref for Env {
    type Target = Scoped;

    fn deref(&self) -> &Scoped {
        &self.scoped
    }
}

/// The values of an env.
#[derive(Clone)]
#[non_exhaustive]
pub struct Scoped {
    /// The resolved tokens of the scope; read in `style` only.
    pub(crate) tokens: Arc<UiTokens>,
    /// The operating system.
    pub(crate) os: Os,
    /// Whether the window is active.
    pub(crate) active: bool,
    /// The text and icon ink of the surface the scope sits on, set by a container for its body.
    pub ink: Ink,
    /// The key of everything above that the style cache depends on.
    pub(crate) key: EnvKey,
    /// Ambient control size; the root default is `Md`.
    pub size: ControlSize,
    /// AND of every scope and gate above.
    pub enabled: bool,
    /// OR of every scope above.
    pub read_only: bool,
    /// The layout direction.
    pub direction: Direction,
    /// The motion mode in effect.
    pub motion: MotionPolicy,
    /// False inside the body of a component with an `Auto` id.
    pub ids_stable: bool,
    /// Root and viewport only: the pixels per point, read back once per pass.
    pub scale: Scale,
    /// The innermost arena: components shown under it join it.
    pub arena: Option<ArenaId>,
    /// From `ContainerProps.transform`.
    pub transform: Option<CanvasTransform>,
    pub(crate) host: Option<Arc<Host>>,
    pub(crate) values: AppValues,
}

/// The inherited group of an env, captured to open a popup or viewport: the layer-local fields are reset.
#[derive(Clone)]
pub struct EnvCapture(Env);

impl EnvCapture {
    /// The env a new egui layer or viewport starts with.
    #[expect(dead_code, reason = "used by the layers (F3)")]
    pub(crate) fn env(&self) -> Env {
        self.0.clone()
    }
}

impl Env {
    /// The root env of a frame: the app's values with the pixels per point read back from egui.
    pub(crate) fn from_root(ctx: &Context, root: &RootEnv, host: Option<Arc<Host>>, pixels_per_point: f32) -> Env {
        let tokens = root_tokens(ctx, root);
        let motion = MotionPolicy::new(motion_mode(ctx, root), tokens.tokens().motion);
        let os = raw_input::os(ctx);
        let scale = Scale(pixels_per_point);
        // F3-LATER: runtime::windows (`WindowInfo.active` per viewport)
        let active = true;
        Env {
            scoped: Arc::new(Scoped {
                key: EnvKey::new(tokens.key(), root.direction, scale, os, active, root.values.style_hash()),
                tokens,
                os,
                active,
                ink: Ink::default(),
                size: ControlSize::Md,
                enabled: true,
                read_only: false,
                direction: root.direction,
                motion,
                ids_stable: true,
                scale,
                arena: None,
                transform: None,
                host,
                values: root.values.clone(),
            }),
        }
    }

    /// The env of `ui`: the nearest entry up the `Ui` stack, else the running viewport's root env, so the `Ui`
    /// of an egui `Area` or `Window` reads the app's root values. For app panels (layer 8) and caller closures only.
    ///
    /// While apps still use `add_enabled_ui`, it also ANDs `ui.is_enabled()`.
    pub fn of(ui: &Ui) -> Env {
        let ctx = ui.ctx();
        let viewport = pass::current_viewport(ctx);
        let found = PerViewport::<ViewportEnvs>::with(ctx, viewport, |envs| {
            stack::nearest(ui, |node| envs.entries.get(&node).cloned()).or_else(|| envs.root.clone())
        });
        let env = found.unwrap_or_else(|| Env::from_root(ctx, &RootEnv::fallback(ctx), None, scale::pixels_per_point(ctx)));
        if style::is_enabled(ui) { env } else { env.derive(|s| s.enabled = false) }
    }

    /// The `Host` of the running viewport's root env.
    pub(crate) fn host_of(ctx: &Context) -> Option<Arc<Host>> {
        PerViewport::<ViewportEnvs>::with(ctx, pass::current_viewport(ctx), |envs| envs.root.as_ref().and_then(|env| env.host.clone()))
    }

    /// A clone of the innermost app value, else `T::root()`. Large values hold an `Arc`.
    pub fn get<T: EnvValue>(&self) -> T {
        self.values.get::<T>()
    }

    /// The inherited group only; layer-local fields start fresh where the capture is installed.
    pub fn capture(&self) -> EnvCapture {
        EnvCapture(self.derive(|s| {
            s.arena = None;
            s.transform = None;
            s.ink = Ink::default();
        }))
    }

    /// What component code sees of this env.
    pub fn view(&self) -> EnvView<'_> {
        EnvView { scoped: &self.scoped }
    }

    /// A child entry: the one way an env changes.
    pub(crate) fn derive(&self, f: impl FnOnce(&mut Scoped)) -> Env {
        let mut scoped = (*self.scoped).clone();
        f(&mut scoped);
        scoped.key = EnvKey::new(scoped.tokens.key(), scoped.direction, scoped.scale, scoped.os, scoped.active, scoped.values.style_hash());
        Env { scoped: Arc::new(scoped) }
    }

    /// Installs `env` for `ui` and everything below it; `restore` puts back what it replaced.
    pub(crate) fn install_for(ui: &Ui, env: Env) -> Installed {
        let viewport = pass::current_viewport(ui.ctx());
        let node = stack::node_id(ui);
        let previous = PerViewport::<ViewportEnvs>::with(ui.ctx(), viewport, |envs| envs.entries.insert(node, env));
        Installed { viewport, node, previous }
    }

    /// Puts back the entry `install_for` replaced.
    pub(crate) fn restore(ctx: &Context, installed: Installed) {
        PerViewport::<ViewportEnvs>::with(ctx, installed.viewport, |envs| match installed.previous {
            Some(previous) => {
                envs.entries.insert(installed.node, previous);
            }
            None => {
                envs.entries.remove(&installed.node);
            }
        });
    }

    /// Records `env` as the root env of `viewport`, which `Env::of` falls back to for a `Ui` with no entry above it.
    pub(crate) fn set_root(ctx: &Context, viewport: ViewportId, env: Env) {
        PerViewport::<ViewportEnvs>::with(ctx, viewport, |envs| envs.root = Some(env));
    }
}

/// One viewport's env entries by the `Ui` they are installed for, and its root env. The lock is held only to
/// install or look up an entry or clone an `Arc`.
#[derive(Default)]
struct ViewportEnvs {
    entries: HashMap<Id, Env>,
    root: Option<Env>,
}

/// What `install_for` replaced.
pub(crate) struct Installed {
    viewport: ViewportId,
    node: Id,
    previous: Option<Env>,
}

/// Restores the replaced entry when dropped, so a panic in a scope's body leaves no entry behind.
struct Restore<'a> {
    ctx: &'a Context,
    installed: Option<Installed>,
}

impl Drop for Restore<'_> {
    fn drop(&mut self) {
        if let Some(installed) = self.installed.take() {
            Env::restore(self.ctx, installed);
        }
    }
}

/// The mode the root runs animations at: the app's, and off while egui's animation time is zero.
fn motion_mode(ctx: &Context, root: &RootEnv) -> MotionMode {
    if repaint::animation_time(ctx) > 0.0 { root.motion } else { root.motion.lowered_by(MotionMode::Off) }
}

/// The tokens the root resolves to, cached by the inputs that determine them.
fn root_tokens(ctx: &Context, root: &RootEnv) -> Arc<UiTokens> {
    /// The newest few inputs; an app swaps its sizes rarely.
    const KEPT: usize = 4;
    #[derive(Default)]
    struct Cache {
        entries: Vec<(TokenInputs, Arc<UiTokens>)>,
    }
    let inputs =
        TokenInputs { sizes: root.sizes.clone(), mode: root.mode, contrast: root.contrast, density: root.density, text_size: root.text_size };
    let service = Service::<Cache>::of(ctx);
    if let Some(hit) = service.with(|cache| cache.entries.iter().find(|(key, _)| key.same(&inputs)).map(|(_, tokens)| tokens.clone())) {
        return hit;
    }
    let appearance = Appearance { mode: root.mode, contrast: root.contrast, size: root.density, text_size: root.text_size, reduced_motion: false };
    let themes = builtins();
    let theme: &ThemeDef = themes.iter().find(|theme| theme.id.as_str() == SUITE_DEFAULT).unwrap_or(&themes[0]);
    let tokens = Arc::new(UiTokens::new(Arc::new(resolve_sized(theme, appearance, None, &root.sizes).tokens)));
    service.with(|cache| {
        if cache.entries.len() == KEPT {
            cache.entries.remove(0);
        }
        cache.entries.push((inputs, tokens.clone()));
    });
    tokens
}

/// What the resolved root tokens depend on. The sizes are compared by identity, as `set_input` does.
struct TokenInputs {
    sizes: Arc<AppSizes>,
    mode: Mode,
    contrast: Contrast,
    density: Size,
    text_size: TextSize,
}

impl TokenInputs {
    fn same(&self, other: &TokenInputs) -> bool {
        Arc::ptr_eq(&self.sizes, &other.sizes)
            && self.mode == other.mode
            && self.contrast == other.contrast
            && self.density == other.density
            && self.text_size == other.text_size
    }
}

/// What component code sees: everything in `Env` except tokens, size, enabled, read-only, arena, transform
/// and `ids_stable`.
#[derive(Clone, Copy)]
pub struct EnvView<'a> {
    scoped: &'a Scoped,
}

impl<'a> EnvView<'a> {
    /// The layout direction.
    pub fn direction(&self) -> Direction {
        self.scoped.direction
    }

    /// The motion mode in effect.
    pub fn motion(&self) -> MotionPolicy {
        self.scoped.motion
    }

    /// The pixels per point.
    pub fn scale(&self) -> Scale {
        self.scoped.scale
    }

    /// The app's `Host`, from `FrameInput.host`.
    pub fn host(&self) -> Option<&'a Arc<Host>> {
        self.scoped.host.as_ref()
    }

    /// A clone of the innermost app value, else `T::root()`.
    pub fn get<T: EnvValue>(&self) -> T {
        self.scoped.values.get::<T>()
    }
}

/// Changes to the inherited values for a stretch of code.
#[derive(Default)]
#[non_exhaustive]
pub struct Overrides {
    /// The ambient control size.
    pub size: Option<ControlSize>,
    /// Only `false` has an effect.
    pub enabled: Option<bool>,
    /// Only `true` has an effect.
    pub read_only: Option<bool>,
    /// Can only lower the mode.
    pub motion: Option<MotionMode>,
    /// Also the child `Ui`'s direction and layout.
    pub direction: Option<Direction>,
    /// App values set for the scope.
    pub values: AppValues,
}

impl Overrides {
    fn apply(self, scoped: &mut Scoped) {
        if let Some(size) = self.size {
            scoped.size = size;
        }
        if self.enabled == Some(false) {
            scoped.enabled = false;
        }
        if self.read_only == Some(true) {
            scoped.read_only = true;
        }
        if let Some(motion) = self.motion {
            scoped.motion.mode = scoped.motion.mode.lowered_by(motion);
        }
        if let Some(direction) = self.direction {
            scoped.direction = direction;
        }
        scoped.values.extend(&self.values);
    }
}

/// Runs `body` in a child `Ui` with `set`'s changes applied to the inherited values. A scope that disables
/// disables its child `Ui`; one that changes direction builds the child with an explicit layout and never
/// inherits the parent's.
pub fn with_env<R>(ui: &mut Ui, set: impl FnOnce(&mut Overrides), body: impl FnOnce(&mut Ui) -> R) -> R {
    let mut overrides = Overrides::default();
    set(&mut overrides);
    let outer = Env::of(ui);
    let inner = outer.derive(|scoped| overrides.apply(scoped));
    let direction = (inner.direction != outer.direction).then(|| inner.direction.is_rtl());
    let disabled = !inner.enabled;
    let ctx = ui.ctx().clone();
    child::scope(ui, disabled, direction, |child| {
        let _restore = Restore { ctx: &ctx, installed: Some(Env::install_for(child, inner)) };
        body(child)
    })
}
