//! [`ActionsFile`]: the `actions.json` form, compatible with PhotoCraft's.

use super::model::Action;

/// The actions file (PhotoCraft's `actions.json` in its preset store): `{"version": 1,
/// "actions": [...]}`, plus `sets` with the names of empty sets, left out when there are none
/// and optional on load, so PhotoCraft's files load and save unchanged. Unknown fields are
/// ignored.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ActionsFile {
    /// The file version, [`ActionsFile::VERSION`].
    pub version: u32,
    /// The actions in list order.
    pub actions: Vec<Action>,
    /// Sets that hold no action.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sets: Vec<String>,
}

/// Why an actions file did not load.
#[derive(Debug)]
#[non_exhaustive]
pub enum ActionsFileError {
    /// The file's version is not [`ActionsFile::VERSION`]; a newer file is refused so it is never
    /// loaded and then overwritten.
    Version(u32),
    /// The text is not a valid actions file.
    Json(serde_json::Error),
}

impl std::fmt::Display for ActionsFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActionsFileError::Version(version) => write!(f, "unsupported actions file version {version}"),
            ActionsFileError::Json(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for ActionsFileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ActionsFileError::Version(_) => None,
            ActionsFileError::Json(error) => Some(error),
        }
    }
}

impl From<serde_json::Error> for ActionsFileError {
    fn from(error: serde_json::Error) -> Self {
        ActionsFileError::Json(error)
    }
}

impl ActionsFile {
    /// The version this crate writes (PhotoCraft's).
    // pc: crates/engine/src/preset_store.rs:1112.
    pub const VERSION: u32 = 1;

    /// Reads a file's JSON text.
    pub fn from_json(text: &str) -> Result<ActionsFile, ActionsFileError> {
        let file: ActionsFile = serde_json::from_str(text)?;
        if file.version != Self::VERSION {
            return Err(ActionsFileError::Version(file.version));
        }
        Ok(file)
    }

    /// The file as pretty JSON text.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}
