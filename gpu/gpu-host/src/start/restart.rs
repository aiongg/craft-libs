//! Starting the app again after its graphics failed while the window was starting up, and telling
//! a graphics failure from a bug.
//!
//! Nothing in a process can show a window once eframe has unwound out of a panic in wgpu, or its
//! UI thread is stuck in the driver, so the start goes on in a new process: the same program with
//! the same arguments, plus one [`RESTART_ARG`] that carries the rung, the adapters left out and
//! the rungs tried. An argument rather than an environment variable: removing a variable again
//! needs `unsafe`, and the processes the app starts (a browser for a link, `lpr`) would inherit it.

use std::ffi::OsString;
use std::sync::Once;
use std::sync::atomic::{AtomicBool, Ordering};

use super::ladder::Rung;

/// The argument a restarted process gets: `--craft-gpu-restart={"rung":…,"skip":[…],"tried":[…]}`.
pub const RESTART_ARG: &str = "--craft-gpu-restart=";

/// The longest skip or tried list taken from the argument.
const MAX_ITEMS: usize = 32;

/// Where a restarted start goes on: the rung, the adapters ([`crate::adapter::key`]s) left out,
/// and the rungs tried and failed before. Each restart leaves out one more adapter or moves to a
/// rung not tried, so the restarts end.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RestartState {
    /// The rung to start on.
    pub rung: Rung,
    /// The adapters left out.
    pub skip: Vec<String>,
    /// The rungs that failed before.
    pub tried: Vec<Rung>,
    /// Documents the OS handed the failed run outside its arguments (macOS Apple events), passed
    /// on as plain arguments: the OS doesn't send them twice.
    pub(crate) documents: Vec<OsString>,
}

impl RestartState {
    /// The state a restart passed to this process, taken out of `args` (with or without the
    /// program name), so the app's own parsing never sees it. Every [`RESTART_ARG`] is removed;
    /// the last one that reads is used.
    pub fn take_from_args(args: &mut Vec<OsString>) -> Option<Self> {
        let mut state = None;
        args.retain(|a| match a.to_str().and_then(|a| a.strip_prefix(RESTART_ARG)) {
            Some(value) => {
                state = Self::parse(value).or(state.take());
                false
            }
            None => true,
        });
        state
    }

    fn parse(value: &str) -> Option<Self> {
        let v: serde_json::Value = serde_json::from_str(value).ok()?;
        let texts = |key: &str| -> Vec<String> {
            v.get(key)
                .and_then(serde_json::Value::as_array)
                .map_or_else(Vec::new, |a| a.iter().filter_map(|s| s.as_str().map(str::to_string)).take(MAX_ITEMS).collect())
        };
        Some(Self {
            rung: Rung::parse(v.get("rung")?.as_str()?)?,
            skip: texts("skip"),
            tried: texts("tried").iter().filter_map(|r| Rung::parse(r)).collect(),
            documents: Vec::new(),
        })
    }

    /// The [`RESTART_ARG`] argument for this state.
    pub(crate) fn to_arg(&self) -> String {
        let tried: Vec<&str> = self.tried.iter().map(|r| r.name()).collect();
        format!("{RESTART_ARG}{}", serde_json::json!({ "rung": self.rung.name(), "skip": self.skip, "tried": tried }))
    }

    /// Start the app again in this state, with this process's arguments (an earlier restart's
    /// argument left out) and the documents it was handed. On Unix the new process replaces this
    /// one (the same process id, so an AppImage keeps its files mounted), and this returns only on
    /// failure; on Windows it starts beside this one, which then exits.
    pub fn exec(&self) -> std::io::Result<()> {
        log::logger().flush();
        let mut c = std::process::Command::new(std::env::current_exe()?);
        c.args(std::env::args_os().skip(1).filter(|a| a.to_str().is_none_or(|a| !a.starts_with(RESTART_ARG))));
        c.args(&self.documents);
        c.arg(self.to_arg());
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt as _;
            Err(c.exec())
        }
        #[cfg(not(unix))]
        c.spawn().map(|_| ())
    }
}

/// The last panic came from the graphics stack ([`watch_panics`]).
static GRAPHICS_PANIC: AtomicBool = AtomicBool::new(false);

/// Note whether each panic comes from the graphics stack, so a failing adapter is told from a bug
/// ([`graphics_panic`]). The hook installed before still reports the panic. Installed once,
/// however often it is called.
pub fn watch_panics() {
    static INSTALLED: Once = Once::new();
    INSTALLED.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            GRAPHICS_PANIC.store(info.location().is_some_and(|l| in_graphics_stack(l.file())), Ordering::Relaxed);
            previous(info);
        }));
    });
}

/// Whether the last panic came from wgpu, egui-wgpu or naga ([`watch_panics`]).
pub fn graphics_panic() -> bool {
    GRAPHICS_PANIC.load(Ordering::Relaxed)
}

/// A source file of wgpu (`wgpu-core-30.0.1/src/…`, `wgpu-hal-…`), egui-wgpu or naga.
pub(crate) fn in_graphics_stack(file: &str) -> bool {
    file.split(['/', '\\']).any(|dir| ["wgpu-", "egui-wgpu-", "naga-"].iter().any(|p| dir.starts_with(p)))
}
