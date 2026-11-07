//! `Env::folder`: the text folding of the scope's language, for search matching.

use super::{Env, EnvView, Scoped};
use craft_i18n::Folder;

impl Scoped {
    /// The folder of the host's interface language, English without a host or a catalog.
    fn language_folder(&self) -> Folder {
        let code = self.host.as_ref().and_then(|host| host.i18n_shared().map(|i18n| i18n.code(host.lang())));
        Folder::new(code.unwrap_or("en"))
    }
}

impl Env {
    /// Folds text for search in the scope's language: case and accents ignored, dotted and dotless i by locale.
    pub fn folder(&self) -> Folder {
        self.language_folder()
    }
}

impl EnvView<'_> {
    /// [`Env::folder`] for the env a component's context reads.
    pub fn folder(&self) -> Folder {
        self.scoped.language_folder()
    }
}
