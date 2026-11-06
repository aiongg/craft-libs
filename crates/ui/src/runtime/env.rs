//! `Env` and scopes, app values and `content_env`.

use crate::runtime::egui::data::Service;
use crate::runtime::egui::{scale, stack, style};
use craft_behavior::{CanvasTransform, MotionMode};
use craft_host::Host;
use craft_theme::{AppSizes, ControlSize, TextSize};
use egui::{Context, Id, Ui, ViewportId};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::hash::Hash;
use std::ops::Deref;
use std::sync::Arc;

/// The layout direction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Direction {
    /// Left to right.
    #[default]
    Ltr,
    /// Right to left.
    Rtl,
}

impl Direction {
    /// Whether the direction is right to left.
    pub const fn is_rtl(self) -> bool {
        matches!(self, Direction::Rtl)
    }
}

/// The interface scale: pixels per point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scale(pub f32);

impl Default for Scale {
    fn default() -> Self {
        Scale(1.0)
    }
}

/// The motion mode in effect, carried in `Env.motion`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MotionPolicy {
    /// The level animations run at.
    pub mode: MotionMode,
}

/// The id of an arena, carried in `Env.arena`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ArenaId(pub Id);

/// An app-defined value inherited down the tree like the library's own.
pub trait EnvValue: Clone + PartialEq + Send + Sync + 'static {
    /// A value that changes how components look is folded into the style key, so cached styles follow it.
    const AFFECTS_STYLE: bool = false;

    /// The value where nothing above sets one; computed at its first `get`, outside every hook.
    fn root() -> Self;
}

/// Marker for values `style` may read. `Hash`, because the value is folded into the style key.
pub trait StyleValue: EnvValue + Hash {}

/// The app values a root, a scope or a component's `content_env` carries: one entry per `EnvValue` type, the
/// innermost wins.
#[derive(Clone, Default)]
pub struct AppValues {
    map: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
}

impl AppValues {
    /// Sets the value of type `T`.
    pub fn set<T: EnvValue>(&mut self, value: T) -> &mut Self {
        self.map.insert(TypeId::of::<T>(), Arc::new(value));
        self
    }

    /// A clone of the value, else `T::root()`.
    pub(crate) fn get<T: EnvValue>(&self) -> T {
        self.map.get(&TypeId::of::<T>()).and_then(|value| value.downcast_ref::<T>()).cloned().unwrap_or_else(T::root)
    }

    /// Takes every value `other` sets over this set's.
    pub(crate) fn extend(&mut self, other: &AppValues) {
        for (ty, value) in &other.map {
            self.map.insert(*ty, Arc::clone(value));
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
            direction: Direction::default(),
            motion: MotionMode::default(),
            scale: Scale::default(),
            tab_all_controls: true,
            values: AppValues::default(),
        }
    }
}

impl RootEnv {
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
    #[doc(hidden)]
    pub fn env(&self) -> Env {
        self.0.clone()
    }
}

impl Env {
    /// The root env of a frame: the app's values with the pixels per point read back from egui.
    pub(crate) fn from_root(root: &RootEnv, host: Option<Arc<Host>>, pixels_per_point: f32) -> Env {
        Env {
            scoped: Arc::new(Scoped {
                size: ControlSize::Md,
                enabled: true,
                read_only: false,
                direction: root.direction,
                motion: MotionPolicy { mode: root.motion },
                ids_stable: true,
                scale: Scale(pixels_per_point),
                arena: None,
                transform: None,
                host,
                values: root.values.clone(),
            }),
        }
    }

    /// The env of `ui`: the nearest entry up the `Ui` stack. For app panels (layer 8) and caller closures only.
    ///
    /// While apps still use `add_enabled_ui`, it also ANDs `ui.is_enabled()`.
    pub fn of(ui: &Ui) -> Env {
        let viewport = ui.ctx().viewport_id();
        let store = EnvStore::of(ui.ctx());
        let found = stack::nearest(ui, |node| store.with(|entries| entries.0.get(&(viewport, node)).cloned()));
        let env = found.unwrap_or_else(|| Env::from_root(&RootEnv::default(), None, scale::pixels_per_point(ui.ctx())));
        if style::is_enabled(ui) { env } else { env.derive(|s| s.enabled = false) }
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
        Env { scoped: Arc::new(scoped) }
    }

    /// Installs `env` for `ui` and everything below it; `restore` puts back what it replaced.
    pub(crate) fn install_for(ui: &Ui, env: Env) -> Installed {
        let key = (ui.ctx().viewport_id(), stack::node_id(ui));
        let previous = EnvStore::of(ui.ctx()).with(|entries| entries.0.insert(key, env));
        Installed { key, previous }
    }

    /// Puts back the entry `install_for` replaced.
    pub(crate) fn restore(ctx: &Context, installed: Installed) {
        EnvStore::of(ctx).with(|entries| match installed.previous {
            Some(previous) => {
                entries.0.insert(installed.key, previous);
            }
            None => {
                entries.0.remove(&installed.key);
            }
        });
    }
}

/// The env entries by the `Ui` they are installed for, per viewport. The service lock is held only to
/// install or look up an entry or clone an `Arc`.
#[derive(Default)]
struct EnvStore(HashMap<(ViewportId, Id), Env>);

impl EnvStore {
    fn of(ctx: &Context) -> Service<EnvStore> {
        Service::of(ctx)
    }
}

/// Drops the env entries of viewports egui no longer reports.
pub(crate) fn prune_viewports(ctx: &Context, live: &[ViewportId]) {
    EnvStore::of(ctx).with(|entries| entries.0.retain(|(viewport, _), _| live.contains(viewport)));
}

/// What `install_for` replaced.
pub(crate) struct Installed {
    key: (ViewportId, Id),
    previous: Option<Env>,
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

/// Runs `body` with `set`'s changes applied to the inherited values. A scope that changes direction builds
/// its child `Ui` with an explicit layout and never inherits the parent's.
pub fn with_env<R>(ui: &mut Ui, set: impl FnOnce(&mut Overrides), body: impl FnOnce(&mut Ui) -> R) -> R {
    let mut overrides = Overrides::default();
    set(&mut overrides);
    let outer = Env::of(ui);
    let inner = outer.derive(|scoped| overrides.apply(scoped));
    let ctx = ui.ctx().clone();
    if inner.direction == outer.direction {
        let installed = Env::install_for(ui, inner);
        let result = body(ui);
        Env::restore(&ctx, installed);
        return result;
    }
    style::scope_direction(ui, inner.direction.is_rtl(), |child| {
        let installed = Env::install_for(child, inner);
        let result = body(child);
        Env::restore(&ctx, installed);
        result
    })
}
