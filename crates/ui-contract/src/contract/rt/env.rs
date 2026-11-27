//! Env values as ui-rt cells (docs/contract-rt.md section 10, "Env"): theme tokens, density, flow
//! direction, motion mode and the layer's rounded frame. A window root provides one set of cells to
//! every node under it ([`craft_ui_rt::BuildCx::provide`]); a scope ([`EnvScope`], [`ThemeScope`])
//! gives its subtree cells of its own for what it overrides and shares the rest.
//!
//! A hook reads a value through the context it runs in ([`super::Cx::direction`], [`super::Paint::frame`], ...).
//! The run peeks the values before its hooks and registers a build read of each cell a hook asked for
//! after them ([`EnvSnapshot::register`]), so a write marks the nodes whose hooks used that value and
//! no others. Tokens are the exception: a run that does not keep its outputs reads them.

use std::cell::Cell;
use std::sync::Arc;

use craft_behavior::MotionMode;
use craft_i18n::Direction;
use craft_theme::{AppId, AppSizes, Appearance, Tokens};
use craft_ui_rt::{BuildCx, CornerRadii, Rect, State, Store, Writes};

use super::scope::MemoView;
use super::ui::Ui;
use crate::contract::{MemoKey, MemoProps};
use crate::runtime::env::ThemeScope;
use crate::style::UiTokens;

/// What the tokens of a window or a scope resolve from. A scope that changes the theme, mode,
/// contrast or density resolves a new base from its parent's.
#[derive(Clone)]
pub struct ThemeBase {
    /// The theme definition.
    pub def: Arc<craft_theme::ThemeDef>,
    /// Mode, contrast, density, text size.
    pub appearance: Appearance,
    /// The app whose pins apply over the theme.
    pub app: Option<AppId>,
    /// The app's size overrides.
    pub sizes: Arc<AppSizes>,
}

impl ThemeBase {
    /// The tokens the base resolves to.
    pub fn tokens(&self) -> Arc<Tokens> {
        UiTokens::of_theme(&self.def, self.appearance, self.app.as_ref(), &self.sizes).tokens().clone()
    }

    /// The base with `scope`'s settings applied.
    pub(crate) fn scoped(&self, scope: &ThemeScope) -> ThemeBase {
        let (def, mode, contrast, density) = scope.settings();
        let mut out = self.clone();
        if let Some(def) = def {
            out.def = def.clone();
        }
        if let Some(mode) = mode {
            out.appearance.mode = mode;
        }
        if let Some(contrast) = contrast {
            out.appearance.contrast = contrast;
        }
        if let Some(density) = density {
            out.appearance.size = density;
        }
        out
    }
}

impl PartialEq for ThemeBase {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.def, &other.def) && self.appearance == other.appearance && self.app == other.app && Arc::ptr_eq(&self.sizes, &other.sizes)
    }
}

/// The rounded frame of a layer, in window coordinates: surfaces painted in the layer that meet a
/// corner of `rect` take at least its radius there (a floating window's body keeps the square
/// surfaces of the panels inside it to its rounded corners).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayerFrame {
    /// The frame, in window coordinates.
    pub rect: Rect,
    /// Its corner radii.
    pub radii: CornerRadii,
}

/// How far a surface's edge may lie from the frame's and still meet it, in points: the rounding to
/// the pixel grid.
const FRAME_SLACK: f32 = 0.75;

impl LayerFrame {
    /// Rounds each corner of every rect in `items` (window coordinates, `origin` the node's
    /// position) that meets the same corner of the frame to at least the frame's radius there.
    pub fn fit(&self, origin: craft_ui_rt::Pos2, items: &mut [craft_ui_rt::Item]) {
        let frame = self.rect.translate(-origin.to_vec2());
        let near = |a: f32, b: f32| (a - b).abs() <= FRAME_SLACK;
        for item in items {
            let craft_ui_rt::Item::Rect { rect, radii, .. } = item else { continue };
            let (left, right) = (near(rect.left(), frame.left()), near(rect.right(), frame.right()));
            let (top, bottom) = (near(rect.top(), frame.top()), near(rect.bottom(), frame.bottom()));
            if top && left {
                radii.top_left = radii.top_left.max(self.radii.top_left);
            }
            if top && right {
                radii.top_right = radii.top_right.max(self.radii.top_right);
            }
            if bottom && left {
                radii.bottom_left = radii.bottom_left.max(self.radii.bottom_left);
            }
            if bottom && right {
                radii.bottom_right = radii.bottom_right.max(self.radii.bottom_right);
            }
        }
    }
}

/// The cells of one scope's env. Copy: a scope that overrides a value holds a cell of its own and
/// shares the rest with its parent.
#[derive(Clone, Copy, Debug)]
pub struct EnvCells {
    /// The resolved tokens.
    pub tokens: State<Arc<Tokens>>,
    /// What the tokens resolve from, when the host gave it; a [`ThemeScope`] needs it.
    pub theme: State<Option<Arc<ThemeBase>>>,
    /// The flow direction: layout order, arrow keys and roving.
    pub direction: State<Direction>,
    /// The density preset.
    pub density: State<craft_theme::Size>,
    /// The motion mode in effect; a scope can only lower it.
    pub motion: State<MotionMode>,
    /// The layer's rounded frame.
    pub frame: State<Option<LayerFrame>>,
}

impl EnvCells {
    /// Cells in `store`, not owned by any node: a host that writes them (a language change, a
    /// theme switch) keeps them and removes them with the window. `tokens` is the window's token
    /// cell.
    pub fn in_store(store: &mut Store, tokens: State<Arc<Tokens>>) -> EnvCells {
        EnvCells {
            tokens,
            theme: store.create(None),
            direction: store.create(Direction::Ltr),
            density: store.create(craft_theme::Size::Compact),
            motion: store.create(MotionMode::Full),
            frame: store.create(None),
        }
    }

    /// Cells owned by `cx`'s node: defaults for a window whose host sets none.
    pub fn owned(cx: &mut BuildCx<'_>, tokens: State<Arc<Tokens>>) -> EnvCells {
        EnvCells {
            tokens,
            theme: cx.state(None),
            direction: cx.state(Direction::Ltr),
            density: cx.state(craft_theme::Size::Compact),
            motion: cx.state(MotionMode::Full),
            frame: cx.state(None),
        }
    }

    /// The cells the node `cx` builds sees, or `None` outside a window root.
    pub fn of(cx: &BuildCx<'_>) -> Option<EnvCells> {
        cx.provided::<EnvCells>()
    }
}

/// The tokens a run of `cx`'s node reads: its scope's cell, else the window's. The read is recorded,
/// so a change marks the node.
pub(crate) fn tokens(cx: &mut BuildCx<'_>) -> Arc<Tokens> {
    match EnvCells::of(cx) {
        Some(cells) => cells.tokens.get(cx),
        None => cx.tokens(),
    }
}

/// Which values a run's hooks asked for.
#[derive(Default)]
struct EnvUse {
    direction: Cell<bool>,
    density: Cell<bool>,
    motion: Cell<bool>,
    frame: Cell<bool>,
}

/// The env values a run's hooks can read, peeked before they run, and the record of which they read.
pub struct EnvSnapshot {
    cells: Option<EnvCells>,
    direction: Direction,
    density: craft_theme::Size,
    motion: MotionMode,
    frame: Option<LayerFrame>,
    used: EnvUse,
}

impl Default for EnvSnapshot {
    /// The values of a context with no node behind it (a measuring run, a kept node's finish): the
    /// defaults, and nothing to register.
    fn default() -> Self {
        EnvSnapshot {
            cells: None,
            direction: Direction::Ltr,
            density: craft_theme::Size::Compact,
            motion: MotionMode::Full,
            frame: None,
            used: EnvUse::default(),
        }
    }
}

impl EnvSnapshot {
    /// The values `cx`'s node sees now, read without recording a read.
    pub(crate) fn peek(cx: &mut BuildCx<'_>) -> EnvSnapshot {
        let Some(cells) = EnvCells::of(cx) else { return EnvSnapshot::default() };
        // Through the store, which records no reader and brings a derived cell up to date.
        let store = cx.store_mut();
        EnvSnapshot {
            cells: Some(cells),
            direction: cells.direction.get(store),
            density: cells.density.get(store),
            motion: cells.motion.get(store),
            frame: cells.frame.get(store),
            used: EnvUse::default(),
        }
    }

    pub(crate) fn direction(&self) -> Direction {
        self.used.direction.set(true);
        self.direction
    }

    /// The direction without recording a read: what a `Draw` starts from; it records the reads it
    /// makes itself.
    pub(crate) fn peek_direction(&self) -> Direction {
        self.direction
    }

    pub(crate) fn mark_direction(&self) {
        self.used.direction.set(true);
    }

    pub(crate) fn density(&self) -> craft_theme::Size {
        self.used.density.set(true);
        self.density
    }

    pub(crate) fn motion(&self) -> MotionMode {
        self.used.motion.set(true);
        self.motion
    }

    pub(crate) fn frame(&self) -> Option<LayerFrame> {
        self.used.frame.set(true);
        self.frame
    }

    /// Records a build read of each cell a hook asked for, for `cx`'s node.
    pub(crate) fn register(&self, cx: &mut BuildCx<'_>) {
        let Some(cells) = self.cells else { return };
        if self.used.direction.take() {
            cells.direction.get(cx);
        }
        if self.used.density.take() {
            cells.density.get(cx);
        }
        if self.used.motion.take() {
            cells.motion.get(cx);
        }
        if self.used.frame.take() {
            cells.frame.get(cx);
        }
    }
}

/// The cells a [`ThemeScope`] derives from its parent's: the scoped base, the tokens resolved from it and its density.
/// They are derived, so a write to the parent's base marks the nodes that read them at the start of the pass.
#[derive(Clone, Copy)]
struct ThemeCells {
    base: State<Option<Arc<ThemeBase>>>,
    tokens: State<Arc<Tokens>>,
    density: State<craft_theme::Size>,
}

impl ThemeCells {
    fn derive(cx: &mut BuildCx<'_>, parent: EnvCells, settings: State<ThemeScope>) -> ThemeCells {
        let base = cx.derived(move |c| {
            let scope = settings.get(c);
            parent.theme.get(c).map(|base| Arc::new(base.scoped(&scope)))
        });
        let tokens = cx.derived(move |c| match base.get(c) {
            Some(base) => base.tokens(),
            None => parent.tokens.get(c),
        });
        let density = cx.derived(move |c| match base.get(c) {
            Some(base) => base.appearance.size,
            None => parent.density.get(c),
        });
        ThemeCells { base: base.state(), tokens: tokens.state(), density: density.state() }
    }
}

/// A scope that overrides env values for its subtree. A value left unset is inherited.
#[derive(Clone, Default)]
pub struct EnvScope {
    theme: Option<ThemeScope>,
    direction: Option<Direction>,
    motion: Option<MotionMode>,
    frame: Option<Option<LayerFrame>>,
}

impl EnvScope {
    /// A scope that changes nothing.
    pub fn new() -> EnvScope {
        EnvScope::default()
    }

    /// Resolves tokens (and the density) under `theme`.
    #[must_use]
    pub fn theme(mut self, theme: ThemeScope) -> EnvScope {
        self.theme = Some(theme);
        self
    }

    /// Lays out and mirrors in `direction`.
    #[must_use]
    pub fn direction(mut self, direction: Direction) -> EnvScope {
        self.direction = Some(direction);
        self
    }

    /// Lowers the motion mode to `mode` (it never rises).
    #[must_use]
    pub fn motion(mut self, mode: MotionMode) -> EnvScope {
        self.motion = Some(mode);
        self
    }

    /// Keeps the surfaces of the subtree to `frame` (`None`: to no frame).
    #[must_use]
    pub fn frame(mut self, frame: Option<LayerFrame>) -> EnvScope {
        self.frame = Some(frame);
        self
    }

    /// Shows `content` under the scope. Like a [`MemoView`], its closure runs only when the scope
    /// moved, a value it read changed or a node under it was marked; nodes under it that read an
    /// overridden value are marked when the scope writes it.
    pub fn show<R: Clone + 'static>(self, ui: &mut Ui<'_, '_>, mut content: impl FnMut(&mut Ui<'_, '_>) -> R) -> R {
        MemoView::new(self).show(ui, |ui, scope| {
            scope.enter(ui);
            content(ui)
        })
    }

    /// Gives `ui`'s node cells for what the scope overrides and the flow its tokens. In a measuring
    /// run no node is reached: the flow gets the tokens the scope resolves from the peeked base.
    fn enter(&self, ui: &mut Ui<'_, '_>) {
        let measuring = ui.flow.measuring.is_some();
        let Some(parent) = EnvCells::of(ui.cx) else { return };
        if measuring {
            if let (Some(scope), Some(base)) = (&self.theme, parent.theme.get(ui.cx.store_mut())) {
                ui.flow.tokens = base.scoped(scope).tokens();
            }
            return;
        }
        let mut cells = parent;
        if let Some(scope) = &self.theme {
            let settings = ui.cx.slot("env-settings", || scope.clone());
            settings.set_if_changed(ui.cx, scope.clone());
            let kept = ui.cx.slot("env-derived", || None::<ThemeCells>);
            let own = match *ui.cx.peek_state(kept) {
                Some(own) => own,
                None => {
                    let own = ThemeCells::derive(ui.cx, parent, settings);
                    kept.set(ui.cx, Some(own));
                    own
                }
            };
            (cells.theme, cells.tokens, cells.density) = (own.base, own.tokens, own.density);
            ui.flow.tokens = cells.tokens.get(ui.cx.store_mut());
        }
        if let Some(direction) = self.direction {
            cells.direction = ui.cx.slot("env-direction", || direction);
            cells.direction.set_if_changed(ui.cx, direction);
        }
        if let Some(mode) = self.motion {
            let lowered = parent.motion.get(ui.cx).lowered_by(mode);
            cells.motion = ui.cx.slot("env-motion", || lowered);
            cells.motion.set_if_changed(ui.cx, lowered);
        }
        if let Some(frame) = self.frame {
            cells.frame = ui.cx.slot("env-frame", || frame);
            cells.frame.set_if_changed(ui.cx, frame);
        }
        ui.cx.provide(cells);
        ui.refresh_env();
    }
}

impl MemoProps for EnvScope {
    fn key(&self, key: &mut MemoKey) {
        if let Some(theme) = &self.theme {
            key.feed_hash(&true);
            theme.key(key);
        }
        key.feed_hash(&(self.direction, self.motion.map(|m| m as u8)));
        key.feed_hash(&self.frame.map(|f| f.map(|f| [f.rect.min.x, f.rect.min.y, f.rect.max.x, f.rect.max.y].map(f32::to_bits))));
    }
}

impl ThemeScope {
    /// Shows `content` with this scope's tokens: [`EnvScope::theme`] alone.
    pub fn show<R: Clone + 'static>(self, ui: &mut Ui<'_, '_>, content: impl FnMut(&mut Ui<'_, '_>) -> R) -> R {
        EnvScope::new().theme(self).show(ui, content)
    }
}
