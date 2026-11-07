//! The theme under inspection, what the user picked in the panels, and the caches of the two sweeps.

use craft_theme::report::{ContrastReport, contrast_report};
use craft_theme::{AppId, AppSizes, Appearance, Explained, ThemeDef, TokenRef, explain_sized};
use std::sync::Arc;

/// The theme and appearance the panels explain, and the app whose pins apply.
#[derive(Clone)]
pub struct ThemeSubject<'a> {
    pub(crate) theme: &'a ThemeDef,
    pub(crate) appearance: Appearance,
    pub(crate) app: Option<&'a AppId>,
}

impl<'a> ThemeSubject<'a> {
    /// The subject `theme` shows in `appearance`, with no app pins.
    pub fn new(theme: &'a ThemeDef, appearance: Appearance) -> Self {
        ThemeSubject { theme, appearance, app: None }
    }

    /// The pins of `app` apply.
    #[must_use]
    pub fn app(mut self, app: &'a AppId) -> Self {
        self.app = Some(app);
        self
    }
}

/// A row the user picked in the browser, the Problems view or the detail: a colour or component token, or a size
/// by its path.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Pick {
    /// A colour or component token.
    Token(TokenRef),
    /// A size, by its path such as `size.control.md`.
    Size(String),
}

/// The inputs of the last explanation and the explanation.
type Explanation = (ThemeDef, Appearance, Option<AppId>, AppSizes, Arc<Explained>);

/// What the theme panels share: the app layer, the picked row, and the explanation and contrast report of the
/// subject, computed again only when the subject or the app layer changes.
#[derive(Default)]
pub struct ThemeWorkspace {
    explanation: Option<Explanation>,
    report: Option<(ThemeDef, Arc<ContrastReport>)>,
    pick: Option<Pick>,
    layer: AppSizes,
}

impl ThemeWorkspace {
    /// A workspace with an empty app layer and nothing picked.
    pub fn new() -> Self {
        Self::default()
    }

    /// Brings the explanation and the contrast report up to date with `subject` and the app layer.
    pub fn refresh(&mut self, subject: &ThemeSubject<'_>) {
        let current = self.explanation.as_ref().is_some_and(|(theme, appearance, app, layer, _)| {
            theme == subject.theme && *appearance == subject.appearance && app.as_ref() == subject.app && *layer == self.layer
        });
        if !current {
            let explained = Arc::new(explain_sized(subject.theme, subject.appearance, subject.app, &self.layer));
            self.explanation = Some((subject.theme.clone(), subject.appearance, subject.app.cloned(), self.layer.clone(), explained));
        }
        if self.report.as_ref().is_none_or(|(theme, _)| theme != subject.theme) {
            self.report = Some((subject.theme.clone(), Arc::new(contrast_report(subject.theme))));
        }
    }

    /// The explanation of the subject of the last `refresh`.
    pub fn explained(&self) -> Option<&Arc<Explained>> {
        self.explanation.as_ref().map(|(_, _, _, _, explained)| explained)
    }

    /// The contrast report of the theme of the last `refresh`.
    pub fn report(&self) -> Option<&Arc<ContrastReport>> {
        self.report.as_ref().map(|(_, report)| report)
    }

    /// The row picked last.
    pub fn pick(&self) -> Option<&Pick> {
        self.pick.as_ref()
    }

    /// Picks `pick`, when a panel reported one.
    pub fn apply_pick(&mut self, pick: Option<Pick>) {
        if pick.is_some() {
            self.pick = pick;
        }
    }

    /// The app's size overrides: the sizes the gallery resolves with.
    pub fn sizes(&self) -> &AppSizes {
        &self.layer
    }

    /// Replaces the app layer, when the editor reported a change.
    pub fn apply_sizes(&mut self, sizes: Option<AppSizes>) {
        if let Some(sizes) = sizes {
            self.layer = sizes;
        }
    }
}
