//! [`Case`]: the appearance inputs one conformance check runs for.

use craft_theme::{
    AppDefaults, AppId, Contrast, ContrastChoice, InterfaceFont, Mode, ModeChoice, Selection, Size, SystemAppearance, TextSize, ThemeDef, ThemeId,
    ThemePair, Variants, builtins,
};

use crate::direction::Direction;

/// What the user picked and what the OS and the app add to it: the inputs from which the
/// reference appearance and the app's own are both built.
#[derive(Clone, Debug)]
pub struct Case {
    selection: Selection,
    system: SystemAppearance,
    defaults: AppDefaults,
    app: Option<AppId>,
    font: InterfaceFont,
    dir: Direction,
    themes: Vec<ThemeDef>,
}

impl Case {
    /// A built-in theme by id (`"craft.pro"`) at the suite defaults: mode Light, standard
    /// contrast, compact, default text size, suite interface font, left to right.
    ///
    /// Panics on an unknown built-in id, which is a typo in the test.
    #[allow(clippy::panic)] // a test assertion
    pub fn new(theme: &str) -> Case {
        let Some(def) = builtins().iter().find(|t| t.id.as_str() == theme) else {
            panic!("conformance: `{theme}` is not a built-in theme");
        };
        let id = def.id.clone();
        Case {
            selection: Selection {
                theme: ThemePair::same(id),
                mode: ModeChoice::Light,
                contrast: ContrastChoice::Standard,
                size: Size::Compact,
                text_size: TextSize::Default,
            },
            system: SystemAppearance::default(),
            defaults: AppDefaults { theme: ThemePair::same(ThemeId::suite_default()), mode: ModeChoice::System, size: Size::Compact },
            app: None,
            font: InterfaceFont::Suite,
            dir: Direction::Ltr,
            themes: Vec::new(),
        }
    }

    /// A user theme, looked up by its id.
    pub fn theme_def(mut self, def: ThemeDef) -> Case {
        self.selection.theme = ThemePair::same(def.id.clone());
        self.themes.retain(|t| t.id != def.id);
        self.themes.push(def);
        self
    }

    /// Light, Dark, or System, which `system` answers.
    pub fn mode(mut self, mode: ModeChoice) -> Case {
        self.selection.mode = mode;
        self
    }

    pub fn system(mut self, system: SystemAppearance) -> Case {
        self.system = system;
        self
    }

    pub fn contrast(mut self, contrast: ContrastChoice) -> Case {
        self.selection.contrast = contrast;
        self
    }

    pub fn size(mut self, size: Size) -> Case {
        self.selection.size = size;
        self
    }

    pub fn text_size(mut self, text_size: TextSize) -> Case {
        self.selection.text_size = text_size;
        self
    }

    pub fn interface_font(mut self, font: InterfaceFont) -> Case {
        self.font = font;
        self
    }

    pub fn direction(mut self, dir: Direction) -> Case {
        self.dir = dir;
        self
    }

    /// The app whose per-app theme overrides apply.
    pub fn app(mut self, app: AppId) -> Case {
        self.app = Some(app);
        self
    }

    /// The app's own defaults, for the fields the selection leaves to the app.
    pub fn defaults(mut self, defaults: AppDefaults) -> Case {
        self.defaults = defaults;
        self
    }

    pub fn selection(&self) -> &Selection {
        &self.selection
    }

    pub fn system_appearance(&self) -> SystemAppearance {
        self.system
    }

    pub fn app_defaults(&self) -> &AppDefaults {
        &self.defaults
    }

    pub fn app_id(&self) -> Option<&AppId> {
        self.app.as_ref()
    }

    pub fn font(&self) -> InterfaceFont {
        self.font
    }

    pub fn dir(&self) -> Direction {
        self.dir
    }

    /// The built-in themes and the case's own: what an app's theme lookup answers.
    pub fn lookup(&self) -> impl FnMut(&ThemeId) -> Option<ThemeDef> + '_ {
        |id| self.themes.iter().chain(builtins()).find(|t| t.id == *id).cloned()
    }

    /// A short unique name for messages: `pro-dark-high-compact-large-system-rtl`. Settings at
    /// their defaults (standard contrast, default text size, suite font, left to right) are left
    /// out.
    pub fn name(&self) -> String {
        let theme = self.selection.theme.light.as_str();
        let mut name = theme.strip_prefix("craft.").unwrap_or(theme).to_owned();
        name.push_str(match self.selection.mode {
            ModeChoice::Light => "-light",
            ModeChoice::Dark => "-dark",
            ModeChoice::System => "-system-mode",
        });
        match self.selection.contrast {
            ContrastChoice::Standard => {}
            ContrastChoice::High => name.push_str("-high"),
            ContrastChoice::System => name.push_str("-system-contrast"),
        }
        name.push_str(match self.selection.size {
            Size::Compact => "-compact",
            Size::Comfortable => "-comfortable",
        });
        if self.selection.text_size != TextSize::Default {
            name.push('-');
            name.push_str(&format!("{:?}", self.selection.text_size).to_lowercase());
        }
        if self.selection.mode == ModeChoice::System || self.selection.contrast == ContrastChoice::System {
            for (label, value) in [("os", self.system.mode.map(mode_text)), ("os-contrast", self.system.contrast.map(contrast_text))] {
                if let Some(value) = value {
                    name.push_str(&format!("-{label}-{value}"));
                }
            }
        }
        if self.font == InterfaceFont::System {
            name.push_str("-system");
        }
        if self.dir == Direction::Rtl {
            name.push_str("-rtl");
        }
        name
    }

    /// Every built-in theme in each mode it has, in standard and high contrast, compact and
    /// comfortable, at the default text size, suite font, left to right.
    pub fn matrix() -> Vec<Case> {
        let mut cases = Vec::new();
        for theme in builtins() {
            let modes: &[ModeChoice] = match theme.variants {
                Variants::Both => &[ModeChoice::Light, ModeChoice::Dark],
                Variants::LightOnly => &[ModeChoice::Light],
                Variants::DarkOnly => &[ModeChoice::Dark],
            };
            for &mode in modes {
                for contrast in [ContrastChoice::Standard, ContrastChoice::High] {
                    for size in [Size::Compact, Size::Comfortable] {
                        cases.push(Case::new(theme.id.as_str()).mode(mode).contrast(contrast).size(size));
                    }
                }
            }
        }
        cases
    }

    /// The default theme with each non-default text size, the system interface font, right to
    /// left, System mode and System contrast (each with the OS answering light and dark; the OS
    /// asks for high contrast in the System contrast cases), one axis at a time.
    pub fn axes() -> Vec<Case> {
        let base = || Case::new(ThemeId::suite_default().as_str());
        let mut cases: Vec<Case> = TextSize::ALL.into_iter().filter(|t| *t != TextSize::Default).map(|t| base().text_size(t)).collect();
        cases.push(base().interface_font(InterfaceFont::System));
        cases.push(base().direction(Direction::Rtl));
        for mode in [Mode::Light, Mode::Dark] {
            cases.push(base().mode(ModeChoice::System).system(SystemAppearance { mode: Some(mode), ..SystemAppearance::default() }));
        }
        for mode in [Mode::Light, Mode::Dark] {
            let system = SystemAppearance { mode: Some(mode), contrast: Some(Contrast::High), reduced_motion: None };
            cases.push(base().contrast(ContrastChoice::System).system(system));
        }
        cases
    }

    /// The mode the selection asks for, for the report header.
    pub(super) fn mode_name(&self) -> &'static str {
        match self.selection.mode {
            ModeChoice::Light => "light",
            ModeChoice::Dark => "dark",
            ModeChoice::System => "system",
        }
    }
}

/// `Mode` as lowercase text.
pub(super) fn mode_text(mode: Mode) -> &'static str {
    match mode {
        Mode::Light => "light",
        Mode::Dark => "dark",
    }
}

fn contrast_text(contrast: Contrast) -> &'static str {
    match contrast {
        Contrast::Standard => "standard",
        Contrast::High => "high",
    }
}
