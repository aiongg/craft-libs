//! `Env` and scopes, app values and `content_env`.

mod folder;
mod label;
mod shell;
mod units;

pub use label::{Label, Message, Word};
pub(crate) use shell::ShellEntry;
pub use shell::{BodyMode, RevealEdge, ShellCx, ShellRegions, SlotKey, WindowKey};
pub use units::{PointSize, UnitKind, UnitWords, Units};

use crate::runtime::commands::CommandChain;
use crate::runtime::egui::memory::{FastMap, Service};
use crate::runtime::egui::{child, pass, repaint, scale, stack, style};
use crate::runtime::frame::PerViewport;
use crate::runtime::windows;
use crate::style::{EnvKey, Ink, UiTokens};
use craft_behavior::{CanvasTransform, MotionMode};
use craft_host::Host;
use craft_host::target::{Os, ShellInfo};
pub use craft_i18n::Direction;
pub use craft_i18n::{AgeStyle, Count, Counted, WordRows, WordSet, WordTemplate, Words};
use craft_i18n::{CalendarDate, DateFormat, Format};
use craft_theme::{
    AppSizes, Appearance, Contrast, ControlSize, Mode, MotionTokens, SUITE_DEFAULT, Size, TextSize, ThemeDef, ThemeId, builtins, resolve_sized,
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
    entries: Arc<ValueEntries>,
}

#[derive(Clone, Default)]
struct ValueEntries {
    map: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
    /// The hash of each style-affecting value, by type.
    style: HashMap<TypeId, u64>,
    /// The hash of every style-affecting value, independent of the order they were set in.
    style_hash: u64,
}

impl ValueEntries {
    fn rehash(&mut self) {
        self.style_hash = self
            .style
            .iter()
            .map(|(ty, hash)| {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                (ty, hash).hash(&mut hasher);
                hasher.finish()
            })
            .fold(0, u64::wrapping_add);
    }
}

impl AppValues {
    /// Sets the value of type `T`.
    pub fn set<T: EnvValue>(&mut self, value: T) -> &mut Self {
        debug_assert!(
            !T::AFFECTS_STYLE || value.style_hash().is_some(),
            "{} sets AFFECTS_STYLE but gives no style hash: use `#[env(style)]`",
            std::any::type_name::<T>()
        );
        let entries = Arc::make_mut(&mut self.entries);
        match value.style_hash().filter(|_| T::AFFECTS_STYLE) {
            Some(hash) => entries.style.insert(TypeId::of::<T>(), hash),
            None => entries.style.remove(&TypeId::of::<T>()),
        };
        entries.map.insert(TypeId::of::<T>(), Arc::new(value));
        entries.rehash();
        self
    }

    /// The hash of every style-affecting value, independent of the order they were set in.
    pub(crate) fn style_hash(&self) -> u64 {
        self.entries.style_hash
    }

    /// A clone of the value, else `T::root()`.
    pub(crate) fn get<T: EnvValue>(&self) -> T {
        self.entries.map.get(&TypeId::of::<T>()).and_then(|value| value.downcast_ref::<T>()).cloned().unwrap_or_else(T::root)
    }

    /// Takes every value `other` sets over this set's.
    pub(crate) fn extend(&mut self, other: &AppValues) {
        if other.entries.map.is_empty() {
            return;
        }
        let entries = Arc::make_mut(&mut self.entries);
        for (ty, value) in &other.entries.map {
            entries.map.insert(*ty, Arc::clone(value));
            match other.entries.style.get(ty) {
                Some(hash) => entries.style.insert(*ty, *hash),
                None => entries.style.remove(ty),
            };
        }
        entries.rehash();
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
    /// Whether a bare Tab toggles the window's chrome in place of moving focus where the key stage's gate allows it.
    pub tab_toggles_chrome: bool,
    /// The theme the tokens resolve from; an id no built-in theme has resolves to the suite default.
    pub theme: ThemeId,
    /// Root app values.
    pub values: AppValues,
    /// Today's date in the host's time zone; the runtime reads no wall clock.
    pub today: Option<CalendarDate>,
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
            tab_toggles_chrome: false,
            theme: ThemeId::suite_default(),
            values: AppValues::default(),
            today: None,
        }
    }
}

impl RootEnv {
    /// The default root of `ctx`, built once so its sizes keep one identity for the token cache.
    pub(crate) fn fallback(ctx: &Context) -> RootEnv {
        #[derive(Default)]
        struct Fallback(Option<RootEnv>);
        Service::<Fallback>::run(ctx, |fallback| fallback.0.get_or_insert_with(RootEnv::default).clone())
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

    /// Sets the theme.
    pub fn theme(mut self, theme: ThemeId) -> Self {
        self.theme = theme;
        self
    }

    /// The appearance the root's tokens resolve under: the mode, contrast, density and text size.
    pub fn appearance(&self) -> Appearance {
        Appearance { mode: self.mode, contrast: self.contrast, size: self.density, text_size: self.text_size, reduced_motion: false }
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

    /// Sets today's date, as the host's clock and time zone give it.
    pub fn today(mut self, date: CalendarDate) -> Self {
        self.today = Some(date);
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
    /// The light or dark mode the tokens were resolved in: the app's, until a scope that resolves its tokens in
    /// another mode sets it with them.
    pub(crate) mode: Mode,
    /// The operating system.
    pub(crate) os: Os,
    /// Whether the window is active.
    pub(crate) active: bool,
    /// The platform and the running viewport's window facts.
    pub(crate) shell: Arc<ShellInfo>,
    /// The keyboard layout as the platform reports it and typing has shown it.
    pub(crate) keyboard: Arc<craft_keymap::Layout>,
    /// The text and icon ink of the surface the scope sits on, set by a container for its body.
    pub ink: Ink,
    /// The built-in text of the scope.
    pub(crate) words: Arc<Words>,
    /// How the scope writes numbers: the host's catalog format, else the system region's.
    pub(crate) format: Arc<Format>,
    /// How the scope writes dates: the host's catalog format, else the system region's.
    pub(crate) dates: Arc<DateFormat>,
    /// Today's date as the host supplied it.
    pub(crate) today: Option<CalendarDate>,
    /// `RootEnv::tab_toggles_chrome`.
    pub(crate) tab_toggles_chrome: bool,
    /// The line height per point of text of the leading face of the interface stack, as the frame began: control
    /// heights depend on it, so the key covers it.
    pub(crate) line_ratio: f32,
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
    /// The roving group of the innermost body with `ContainerProps.roving`: components shown under it put
    /// their stops in it instead of the scope's Tab order.
    pub(crate) roving: Option<Id>,
    /// From `ContainerProps.transform`.
    pub transform: Option<CanvasTransform>,
    /// The `Grid` or spread `Row` body whose cursor places the children in flow.
    pub(crate) grid: Option<Id>,
    /// The body that encloses the scope is a content-sized column: it takes its width from its widest child that is
    /// not fill-width, so a fill-width child takes the width the others settle on and never sets it.
    pub(crate) content_column: bool,
    /// The innermost scrolling body that encloses the scope: a collection under it owns no scroll of its own, and
    /// page and end moves a component leaves untaken go to it.
    pub(crate) scroll_owner: Option<Id>,
    /// The id of the collection item the scope shows: its root is not a Tab stop (17.3).
    pub(crate) item: Option<Id>,
    /// The roving group of the collection row the scope shows: the stops of its components and parts join it, so
    /// Tab leaves the collection and Forward enters the row (17.5).
    pub(crate) row: Option<Id>,
    /// What the collection's item step recorded for the item it shows this pass.
    pub(crate) item_facts: Option<ItemFacts>,
    /// The container instances above that receive scoped commands.
    pub(crate) commands: CommandChain,
    pub(crate) host: Option<Arc<Host>>,
    pub(crate) values: AppValues,
}

/// What a collection records for one of its items in a pass: the row flags the item folds into its state (step 12)
/// and its place in the set (step 16). The collection owns the record; a component reads it from its env.
#[derive(Clone, Debug)]
pub(crate) struct ItemFacts {
    pub(crate) item: Id,
    pub(crate) flags: crate::style::Flags,
    pub(crate) position: crate::runtime::access::SetPosition,
    /// The keys of the item's cells in display order, so a cell reports the position of its column.
    pub(crate) columns: Vec<craft_behavior::ColumnKey>,
}

impl Scoped {
    /// The facts the collection recorded for the item `id`.
    pub(crate) fn facts_of(&self, id: Id) -> Option<&ItemFacts> {
        self.item_facts.as_ref().filter(|facts| facts.item == id)
    }
}

/// Token-changing values for a container's body: the body and everything under it resolve their styles from
/// these tokens.
#[derive(Clone)]
pub struct ThemeScope {
    tokens: Arc<UiTokens>,
}

impl ThemeScope {
    /// A scope whose body resolves from `tokens`.
    pub fn new(tokens: UiTokens) -> Self {
        ThemeScope { tokens: Arc::new(tokens) }
    }

    pub(crate) fn tokens(&self) -> &Arc<UiTokens> {
        &self.tokens
    }
}

impl std::fmt::Debug for ThemeScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ThemeScope").field("key", &self.tokens.key()).finish()
    }
}

/// The inherited group of an env, captured to open a popup or viewport: the layer-local fields are reset.
#[derive(Clone)]
pub struct EnvCapture(Env);

impl EnvCapture {
    /// The env a new egui layer or viewport starts with.
    pub(crate) fn env(&self) -> Env {
        self.0.clone()
    }
}

impl Env {
    /// The word set `L` in the scope's language.
    pub fn words<L: WordSet + Send + Sync + 'static>(&self) -> Arc<L> {
        self.words.set::<L>()
    }

    /// How the scope writes numbers: locale digits, separators and grouping.
    pub fn format(&self) -> &Format {
        &self.format
    }

    /// How the scope writes dates: the host's catalog format, else the system region's.
    pub fn dates(&self) -> Arc<DateFormat> {
        Arc::clone(&self.dates)
    }

    /// Today's date as the host supplied it with `RootEnv::today`; `None` when it gave none.
    pub fn today(&self) -> Option<CalendarDate> {
        self.today
    }

    /// The scope's number format and unit words, to read and write numbers with their unit.
    pub fn units(&self) -> Units {
        Units::new(self.format.clone(), self.words::<UnitWords>(), self.words.clone(), self.values.get::<PointSize>())
    }

    /// How long ago `seconds` was, in the scope's language and digits: "Just now", "3 minutes ago", "Yesterday".
    pub fn relative_age(&self, seconds: u64, style: AgeStyle) -> String {
        craft_i18n::relative_age(&self.words, &self.format, seconds, style)
    }

    /// The root env of a frame: the app's values with the pixels per point read back from egui.
    pub(crate) fn from_root(ctx: &Context, root: &RootEnv, host: Option<Arc<Host>>, pixels_per_point: f32) -> Env {
        let tokens = root_tokens(ctx, root);
        let motion = MotionPolicy::new(motion_mode(ctx, root), tokens.tokens().motion);
        let os = windows::os_of(ctx);
        let scale = Scale(pixels_per_point);
        let line_ratio = crate::fonts::stack_facts(ctx).line_ratio;
        let shell = Arc::new(windows::shell_of(ctx, pass::current_viewport(ctx)));
        let active = shell.window.active;
        let keyboard = Arc::new(crate::runtime::keys::layout_of(ctx));
        Env {
            scoped: Arc::new(Scoped {
                key: EnvKey::new(tokens.key(), root.direction, scale, os, active, root.values.style_hash()).with_line(line_ratio),
                line_ratio,
                tokens,
                os,
                active,
                shell,
                keyboard,
                ink: Ink::default(),
                size: ControlSize::Md,
                enabled: true,
                read_only: false,
                direction: root.direction,
                motion,
                ids_stable: true,
                scale,
                arena: None,
                roving: None,
                transform: None,
                grid: None,
                content_column: false,
                scroll_owner: None,
                item: None,
                row: None,
                item_facts: None,
                commands: CommandChain::default(),
                words: host_words(ctx, host.as_ref()),
                format: host_format(ctx, host.as_ref()),
                dates: host_dates(ctx, host.as_ref()),
                today: root.today,
                tab_toggles_chrome: root.tab_toggles_chrome,
                mode: root.mode,
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
        if style::is_enabled(ui) { env } else { env.derive_local(|s| s.enabled = false) }
    }

    /// The running viewport's root env: what code with no `Ui` (the ghost layer) paints with.
    pub(crate) fn of_root(ctx: &Context) -> Env {
        let viewport = pass::current_viewport(ctx);
        let found = PerViewport::<ViewportEnvs>::with(ctx, viewport, |envs| envs.root.clone());
        found.unwrap_or_else(|| Env::from_root(ctx, &RootEnv::fallback(ctx), None, scale::pixels_per_point(ctx)))
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
            s.roving = None;
            s.transform = None;
            s.grid = None;
            s.content_column = false;
            s.scroll_owner = None;
            s.item = None;
            s.row = None;
            s.item_facts = None;
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
        scoped.key = EnvKey::new(scoped.tokens.key(), scoped.direction, scoped.scale, scoped.os, scoped.active, scoped.values.style_hash())
            .with_line(scoped.line_ratio);
        Env { scoped: Arc::new(scoped) }
    }

    /// A child entry that changes layer-local fields only (arena, item, row, roving, grid, scroll owner,
    /// transform, ink): the key carries over, because none of its inputs change.
    pub(crate) fn derive_local(&self, f: impl FnOnce(&mut Scoped)) -> Env {
        let mut scoped = (*self.scoped).clone();
        f(&mut scoped);
        Env { scoped: Arc::new(scoped) }
    }

    /// This env's container scope on top of `root`, the root env of the viewport the scope now runs in: the scale,
    /// active state, window facts and OS are the viewport's, everything else stays.
    pub(crate) fn rebase(&self, root: &Env) -> Env {
        self.derive(|s| {
            s.scale = root.scale;
            s.active = root.active;
            s.shell = root.shell.clone();
            s.os = root.os;
        })
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

    /// The root env of the running viewport, once its begin hook ran.
    pub(crate) fn root_of(ctx: &Context) -> Option<Env> {
        PerViewport::<ViewportEnvs>::with(ctx, ctx.viewport_id(), |envs| envs.root.clone())
    }

    /// The root env `viewport`'s last begin hook installed, for the input hook, which runs before the pass.
    pub(crate) fn root_for(ctx: &Context, viewport: ViewportId) -> Option<Env> {
        PerViewport::<ViewportEnvs>::with(ctx, viewport, |envs| envs.root.clone())
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
    entries: FastMap<Id, Env>,
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

/// The newest few root-token inputs are kept; an app swaps its sizes rarely.
const KEPT_ROOT_TOKENS: usize = 4;

#[derive(Default)]
struct RootTokenCache {
    entries: Vec<(TokenInputs, Arc<UiTokens>)>,
}

/// How many root-token entries the context holds.
#[cfg(feature = "testing")]
pub(crate) fn root_token_entries(ctx: &Context) -> usize {
    Service::<RootTokenCache>::run(ctx, |cache| cache.entries.len())
}

/// The tokens the root resolves to, cached by the inputs that determine them.
fn root_tokens(ctx: &Context, root: &RootEnv) -> Arc<UiTokens> {
    let inputs = TokenInputs {
        sizes: root.sizes.clone(),
        theme: root.theme.clone(),
        mode: root.mode,
        contrast: root.contrast,
        density: root.density,
        text_size: root.text_size,
    };
    let service = Service::<RootTokenCache>::of(ctx);
    if let Some(hit) = service.with(|cache| cache.entries.iter().find(|(key, _)| key.same(&inputs)).map(|(_, tokens)| tokens.clone())) {
        return hit;
    }
    let appearance = root.appearance();
    let themes = builtins();
    let theme: &ThemeDef =
        [root.theme.as_str(), SUITE_DEFAULT].into_iter().find_map(|id| themes.iter().find(|theme| theme.id.as_str() == id)).unwrap_or(&themes[0]);
    let tokens = Arc::new(UiTokens::new(Arc::new(resolve_sized(theme, appearance, None, &root.sizes).tokens), root.contrast, root.density));
    service.with(|cache| {
        if cache.entries.len() == KEPT_ROOT_TOKENS {
            cache.entries.remove(0);
        }
        cache.entries.push((inputs, tokens.clone()));
    });
    tokens
}

/// What the resolved root tokens depend on. The sizes are compared by identity, as `set_input` does.
struct TokenInputs {
    sizes: Arc<AppSizes>,
    theme: ThemeId,
    mode: Mode,
    contrast: Contrast,
    density: Size,
    text_size: TextSize,
}

impl TokenInputs {
    fn same(&self, other: &TokenInputs) -> bool {
        Arc::ptr_eq(&self.sizes, &other.sizes)
            && self.theme == other.theme
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
    /// The facts the collection recorded for the item `id`.
    pub(crate) fn facts_of(&self, id: Id) -> Option<&ItemFacts> {
        self.scoped.facts_of(id)
    }

    /// The innermost scrolling body around the component.
    pub(crate) fn scroll_owner(&self) -> Option<Id> {
        self.scoped.scroll_owner
    }

    /// The roving group of the innermost body with `ContainerProps.roving`.
    pub(crate) fn roving(&self) -> Option<Id> {
        self.scoped.roving
    }

    /// The layout direction.
    pub fn direction(&self) -> Direction {
        self.scoped.direction
    }

    /// The light or dark mode the scope's tokens are resolved in: the app's, or the one a scope with its own tokens
    /// set.
    pub fn mode(&self) -> Mode {
        self.scoped.mode
    }

    /// The motion mode in effect.
    pub fn motion(&self) -> MotionPolicy {
        self.scoped.motion
    }

    /// The pixels per point.
    pub fn scale(&self) -> Scale {
        self.scoped.scale
    }

    pub(crate) fn command_chain(&self) -> &CommandChain {
        &self.scoped.commands
    }

    /// The platform and the running viewport's window facts: the OS, the capabilities and the window's kind,
    /// chrome, state, insets and focus.
    pub fn shell(&self) -> &'a ShellInfo {
        &self.scoped.shell
    }

    /// The keyboard layout: which character each key position types, for text that names a key.
    pub fn keyboard_layout(&self) -> &'a craft_keymap::Layout {
        &self.scoped.keyboard
    }

    /// The app's `Host`, from `FrameInput.host`.
    pub fn host(&self) -> Option<&'a Arc<Host>> {
        self.scoped.host.as_ref()
    }

    /// A clone of the innermost app value, else `T::root()`.
    pub fn get<T: EnvValue>(&self) -> T {
        self.scoped.values.get::<T>()
    }

    /// The word set `L` in the scope's language, built once per `Words`.
    pub fn words<L: WordSet + Send + Sync + 'static>(&self) -> Arc<L> {
        self.scoped.words.set::<L>()
    }

    /// How the scope writes numbers: locale digits, separators and grouping.
    pub fn format(&self) -> &'a Format {
        &self.scoped.format
    }

    /// How the scope writes dates: the host's catalog format, else the system region's.
    pub fn dates(&self) -> Arc<DateFormat> {
        Arc::clone(&self.scoped.dates)
    }

    /// Today's date as the host supplied it with `RootEnv::today`; `None` when it gave none.
    pub fn today(&self) -> Option<CalendarDate> {
        self.scoped.today
    }

    /// The scope's number format and unit words, to read and write numbers with their unit.
    pub fn units(&self) -> Units {
        Units::new(self.scoped.format.clone(), self.words::<UnitWords>(), self.scoped.words.clone(), self.scoped.values.get::<PointSize>())
    }

    /// How long ago `seconds` was, in the scope's language and digits: "Just now", "3 minutes ago", "Yesterday".
    pub fn relative_age(&self, seconds: u64, style: AgeStyle) -> String {
        craft_i18n::relative_age(&self.scoped.words, &self.scoped.format, seconds, style)
    }

    /// `label` with a `Word` replaced by its text in the scope's language: what domain code that draws a `Label`
    /// itself shows.
    pub fn resolve(&self, label: &Label) -> Label {
        label.resolved(&self.scoped.words)
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
    /// The words of the scope and everything inside it.
    pub words: Option<WordsOverride>,
    /// App values set for the scope.
    pub values: AppValues,
}

/// How a scope changes the built-in text: the only override path for words.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum WordsOverride {
    /// These words stand in for the inherited ones.
    Replace(Arc<Words>),
    /// These rows win over the inherited words; every other word reads as before.
    Overlay(WordRows),
}

impl Overrides {
    fn apply(self, ctx: &Context, scoped: &mut Scoped) {
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
        match self.words {
            Some(WordsOverride::Replace(words)) => scoped.words = words,
            Some(WordsOverride::Overlay(rows)) => scoped.words = overlaid_words(ctx, &scoped.words, rows),
            None => {}
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
    let ctx = ui.ctx().clone();
    let inner = outer.derive(|scoped| overrides.apply(&ctx, scoped));
    let direction = (inner.direction != outer.direction).then(|| inner.direction.is_rtl());
    let disabled = !inner.enabled;
    child::scope(ui, disabled, direction, |child| {
        let _restore = Restore { ctx: &ctx, installed: Some(Env::install_for(child, inner)) };
        body(child)
    })
}

/// The words of the installed `Host`, kept until the `Host` changes so that the word sets built from them are
/// built once.
#[derive(Default)]
struct HostWords {
    host: Option<std::sync::Weak<Host>>,
    words: Option<Arc<Words>>,
}

/// The catalogs of `host` in its language, English without a host or a catalog.
fn host_words(ctx: &Context, host: Option<&Arc<Host>>) -> Arc<Words> {
    Service::<HostWords>::run(ctx, |cached| {
        let same = match (&cached.host, host) {
            (Some(kept), Some(host)) => std::sync::Weak::ptr_eq(kept, &Arc::downgrade(host)),
            (None, None) => true,
            _ => false,
        };
        if let (true, Some(words)) = (same, &cached.words) {
            return Arc::clone(words);
        }
        let words = Arc::new(match host.and_then(|host| host.i18n_shared().map(|i18n| (Arc::clone(i18n), host.lang()))) {
            Some((i18n, lang)) => Words::from_i18n(i18n, lang),
            None => Words::english(),
        });
        cached.host = host.map(Arc::downgrade);
        cached.words = Some(Arc::clone(&words));
        words
    })
}

/// The number format of `host`'s catalogs, else the system region's with the locale's digits, built once.
fn host_format(ctx: &Context, host: Option<&Arc<Host>>) -> Arc<Format> {
    if let Some(i18n) = host.and_then(|host| host.i18n_shared()) {
        return i18n.format();
    }
    #[derive(Default)]
    struct System(Option<Arc<Format>>);
    Service::<System>::of(ctx)
        .with(|system| Arc::clone(system.0.get_or_insert_with(|| Arc::new(Format::new(&craft_i18n::Region::system(), craft_i18n::Digits::Locale)))))
}

/// The date format of `host`'s catalogs, else the system region's, built once.
fn host_dates(ctx: &Context, host: Option<&Arc<Host>>) -> Arc<DateFormat> {
    if let Some(i18n) = host.and_then(|host| host.i18n_shared()) {
        return i18n.dates();
    }
    #[derive(Default)]
    struct System(Option<Arc<DateFormat>>);
    Service::<System>::of(ctx).with(|system| Arc::clone(system.0.get_or_insert_with(|| Arc::new(DateFormat::new(&craft_i18n::Region::system())))))
}

/// `words` with `rows` over them, built once per (inherited words, rows): a clone of a `Words` starts with an
/// empty set cache, so a fresh overlay each pass would rebuild every word set each frame.
fn overlaid_words(ctx: &Context, words: &Arc<Words>, rows: WordRows) -> Arc<Words> {
    /// Overlays kept before the oldest goes.
    const KEPT: usize = 16;
    struct Entry {
        inherited: std::sync::Weak<Words>,
        rows: WordRows,
        words: Arc<Words>,
    }
    #[derive(Default)]
    struct Overlays(Vec<Entry>);
    Service::<Overlays>::run(ctx, |cache| {
        cache.0.retain(|entry| entry.inherited.strong_count() > 0);
        let inherited = Arc::downgrade(words);
        if let Some(entry) = cache.0.iter().find(|entry| std::sync::Weak::ptr_eq(&entry.inherited, &inherited) && entry.rows == rows) {
            return Arc::clone(&entry.words);
        }
        let overlaid = Arc::new(words.overlay(&rows));
        if cache.0.len() == KEPT {
            cache.0.remove(0);
        }
        cache.0.push(Entry { inherited, rows, words: Arc::clone(&overlaid) });
        overlaid
    })
}
