#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! The control channel's command-line values and their environment fallbacks.
//!
//! An app's command line has `--control <PORT>`, `--control-token <64-HEX>` and
//! `--control-token-file <PATH>`; each has a variable (`<APP>_CONTROL_PORT`,
//! `<APP>_CONTROL_TOKEN`, `<APP>_CONTROL_TOKEN_FILE`) that stands in for it when the flag is
//! absent. A flag wins over its variable; a blank variable counts as unset; a bad variable value
//! is a usage error that names the variable, never the token's value. Parsing the command line
//! itself (clap in PhotoCraft) stays in the app: this module checks the values and reports them
//! in the apps' wording, so the exit code is 2 and the message is the one the apps' integration
//! tests look for.
//!
//! What the user sees: `photocraft --control 0 --control-token abcd --control-token-file t`
//! prints `error: the argument '--control-token' cannot be used with '--control-token-file'`
//! and exits 2; with the token from `PHOTOCRAFT_CONTROL_TOKEN` the message names the variable
//! instead of the flag.

use std::path::PathBuf;

use crate::security::AutomationError;

/// The exit status of a bad control configuration (usage errors, clap's convention).
pub const USAGE_EXIT_CODE: i32 = 2;

/// The environment variable names an app reads for its control flags: `<PREFIX>_CONTROL_PORT`,
/// `<PREFIX>_CONTROL_TOKEN`, `<PREFIX>_CONTROL_TOKEN_FILE`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnvNames {
    pub port: String,
    pub token: String,
    pub token_file: String,
}

impl EnvNames {
    /// Names with `prefix` (`"PHOTOCRAFT"` gives `PHOTOCRAFT_CONTROL_PORT` and friends).
    pub fn for_app(prefix: &str) -> Self {
        // new: PhotoCraft hard-codes the names in `clap` attributes (crates/automation/src/args.rs:13)
        todo!()
    }
}

/// Where a control value came from, for error messages: the flag's own spelling, or the variable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Flag,
    Env,
}

/// The control values after the command line and the environment were read.
///
/// `port` on means the control server starts; `None` means no `--control` and no port variable.
/// The pair `token` / `token_file` is either-or (checked by [`Self::check`]); neither set means
/// a token is generated and printed to stderr at start.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ControlOptions {
    pub port: Option<u16>,
    pub token: Option<String>,
    pub token_file: Option<PathBuf>,
    /// Where each set value came from, so an error names the flag or the variable.
    pub sources: Sources,
}

/// The [`Source`] of each set field of [`ControlOptions`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sources {
    pub port: Option<Source>,
    pub token: Option<Source>,
    pub token_file: Option<Source>,
}

/// A bad control configuration, worded like clap's usage errors; printed by the app with its
/// `error: ` prefix and a pointer at `--help`, then exit [`USAGE_EXIT_CODE`].
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum OptionsError {
    /// `invalid value 'nope' for 'PHOTOCRAFT_CONTROL_PORT'` (or `'--control <PORT>'`).
    #[error("invalid value '{value}' for '{shown}': expected a port, 0-65535")]
    NotAPort { value: String, shown: String },
    /// `the argument '--control-token' cannot be used with 'PHOTOCRAFT_CONTROL_TOKEN_FILE'`.
    /// Never contains the token.
    #[error("the argument '{a}' cannot be used with '{b}'")]
    Conflict { a: String, b: String },
}

impl ControlOptions {
    /// The values from the environment alone: a blank variable is unset; a bad port is
    /// [`OptionsError::NotAPort`] naming the variable.
    pub fn from_env(names: &EnvNames) -> Result<Self, OptionsError> {
        // pc: crates/automation/src/args.rs:65 (blank_env_is_unset), apps/photocraft/src/launch_args.rs:55 (PHOTOCRAFT_CONTROL_PORT), crates/automation/src/args.rs:24, :28 (the token variables)
        todo!()
    }

    /// `flags` (what the command line gave, each with `Source::Flag`) over `env`: a flag wins
    /// over its variable, and a variable is never read when its flag is present.
    pub fn merged(flags: Self, env: Self) -> Self {
        // pc: apps/photocraft/tests/it/command_line_usage.rs:137 (variables are the flags' fallbacks)
        todo!()
    }

    /// Rejects `token` together with `token_file` ([`OptionsError::Conflict`], naming the flag or
    /// variable each came from). Only checked when `port` is set: without `--control` the token
    /// pair is ignored, as in PhotoCraft.
    pub fn check(&self, flags: &FlagNames, names: &EnvNames) -> Result<(), OptionsError> {
        // pc: crates/automation/src/args.rs:91 (conflict), tests/it/control_exit_code.rs:59
        todo!()
    }

    /// The server token: the supplied one (validated), the file's content, or a fresh one.
    /// `Err` is "cannot configure control authentication", exit 2.
    pub fn server_token(&self) -> Result<String, AutomationError> {
        // pc: crates/automation/src/security.rs:185 (server_token)
        todo!()
    }

    /// The lines the app prints to stderr about the token: the file path, `control token: <t>`
    /// for a generated one, `using supplied control token` otherwise. The token is printed only
    /// when the app generated it.
    pub fn token_notice(&self, token: &str) -> String {
        // pc: apps/photocraft/src/main.rs:302
        todo!()
    }
}

/// The flags' spellings in usage errors (`--control <PORT>`, `--control-token`,
/// `--control-token-file`), so an app with other names reports its own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlagNames {
    pub port: String,
    pub token: String,
    pub token_file: String,
}

impl Default for FlagNames {
    /// PhotoCraft's spellings: `--control`, `--control-token`, `--control-token-file`.
    fn default() -> Self {
        // pc: apps/photocraft/src/launch_args.rs:55, crates/automation/src/args.rs:24, :28
        todo!()
    }
}

/// `"0"`–`"65535"` with surrounding spaces to a port.
pub fn control_port(s: &str) -> Result<u16, NotAPort> {
    // pc: crates/automation/src/args.rs:54
    todo!()
}

/// Not a port number.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("expected a port, 0-65535")]
pub struct NotAPort;

#[cfg(test)]
#[path = "args_tests.rs"]
mod args_tests;
