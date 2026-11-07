//! The registry of command definitions: the sets `#[derive(CommandSet)]` registers, loaded at install.

use craft_keymap::{BindingError, CommandDef};
use std::collections::HashMap;

/// One registered command set.
#[doc(hidden)]
#[derive(Clone, Copy)]
#[non_exhaustive]
pub struct SetEntry {
    /// Returns the set's type name.
    pub name: fn() -> &'static str,
    /// The set's definitions.
    pub defs: fn() -> Result<Vec<CommandDef>, CommandDefError>,
}

impl SetEntry {
    /// The entry of the set named `name`.
    pub const fn new(name: fn() -> &'static str, defs: fn() -> Result<Vec<CommandDef>, CommandDefError>) -> Self {
        SetEntry { name, defs }
    }
}

/// Every command set linked into the running binary.
#[doc(hidden)]
#[linkme::distributed_slice]
pub static COMMAND_SETS: [SetEntry];

/// Registers the command set `$ty`, so `install` loads it.
#[doc(hidden)]
#[macro_export]
macro_rules! __register_command_set {
    ($ty:ty) => {
        const _: () = {
            #[$crate::__private::linkme::distributed_slice($crate::runtime::commands::COMMAND_SETS)]
            #[linkme(crate = $crate::__private::linkme)]
            static REGISTRATION: $crate::runtime::commands::SetEntry =
                $crate::runtime::commands::SetEntry::new(::std::any::type_name::<$ty>, <$ty as $crate::runtime::commands::CommandSet>::defs);
        };
    };
}

/// Why a command definition is unusable.
#[derive(Debug)]
#[non_exhaustive]
pub enum CommandDefError {
    /// A default binding's text does not parse.
    #[non_exhaustive]
    Binding {
        /// The command's id.
        command: &'static str,
        /// What is wrong with the text.
        error: BindingError,
    },
    /// The set's scope name is not a valid scope name.
    #[non_exhaustive]
    Scope {
        /// The command's id.
        command: &'static str,
    },
    /// Two commands have one id.
    #[non_exhaustive]
    Duplicate {
        /// The id.
        command: String,
    },
}

impl CommandDefError {
    #[doc(hidden)]
    pub fn binding(command: &'static str, error: BindingError) -> Self {
        CommandDefError::Binding { command, error }
    }

    #[doc(hidden)]
    pub fn scope(command: &'static str) -> Self {
        CommandDefError::Scope { command }
    }
}

impl std::fmt::Display for CommandDefError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandDefError::Binding { command, error } => write!(f, "the default binding of {command}: {error}"),
            CommandDefError::Scope { command } => write!(f, "the scope of {command} is not a valid scope name"),
            CommandDefError::Duplicate { command } => write!(f, "two commands have the id {command}"),
        }
    }
}

impl std::error::Error for CommandDefError {}

/// The loaded definitions, by id.
#[derive(Default)]
pub(crate) struct Registry {
    defs: Vec<CommandDef>,
    index: HashMap<String, usize>,
}

impl Registry {
    /// Loads every linked set.
    pub(crate) fn load() -> Result<Registry, CommandDefError> {
        let mut registry = Registry::default();
        for entry in COMMAND_SETS {
            for def in (entry.defs)()? {
                registry.add(def)?;
            }
        }
        Ok(registry)
    }

    fn add(&mut self, def: CommandDef) -> Result<(), CommandDefError> {
        let id = def.command.id.clone();
        if self.index.insert(id.clone(), self.defs.len()).is_some() {
            return Err(CommandDefError::Duplicate { command: id });
        }
        self.defs.push(def);
        Ok(())
    }

    pub(crate) fn get(&self, id: &str) -> Option<&CommandDef> {
        self.index.get(id).map(|&i| &self.defs[i])
    }

    pub(crate) fn defs(&self) -> &[CommandDef] {
        &self.defs
    }
}
