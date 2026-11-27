//! `rt-runner check|pixels|bench|window`: see the crate docs and `--help`.

use std::process::ExitCode;

use craft_rt_runner::backend;
use craft_rt_runner::run::{self, Rule};
use craft_ui_rt::Renderer;
use craft_ui_test_kit::rt::alloc::CountingAlloc;

#[global_allocator]
static ALLOC: CountingAlloc = CountingAlloc;

const USAGE: &str = "usage: rt-runner <command> [--renderer epaint|skia|skia-gpu] [flags]
  check   [--reference DIR] [--observed DIR]     behaviour diff against the v1 reference (headless)
  pixels  [--reference DIR] [--out DIR]          render diff against the reference's PNGs (headless)
  bench   [--layers 20,600] [--scale 1] [--scenarios a,b] [--out FILE]   frame cost table (headless)
  window  [--layers N] [--exit-after SECS]       the panel in a window";

struct Args {
    command: String,
    renderer: String,
    flags: Vec<(String, String)>,
}

impl Args {
    fn get(&self, flag: &str) -> Option<String> {
        self.flags.iter().find(|(f, _)| f == flag).map(|(_, v)| v.clone())
    }

    fn list<T: std::str::FromStr>(&self, flag: &str, default: &[T]) -> Vec<T>
    where
        T: Clone,
    {
        self.get(flag).map_or_else(|| default.to_vec(), |v| v.split(',').filter_map(|x| x.trim().parse().ok()).collect())
    }
}

fn parse() -> Result<Args, String> {
    let mut it = std::env::args().skip(1);
    let command = it.next().ok_or("no command")?;
    let (mut renderer, mut flags) = ("epaint".to_owned(), Vec::new());
    while let Some(flag) = it.next() {
        let value = it.next().ok_or(format!("{flag} needs a value"))?;
        if flag == "--renderer" {
            renderer = value;
        } else {
            flags.push((flag, value));
        }
    }
    Ok(Args { command, renderer, flags })
}

fn headless<R: Renderer + 'static>(make: fn() -> R, name: &str, a: &Args) -> Result<bool, String> {
    let reference = run::reference_dir(a.get("--reference"));
    match a.command.as_str() {
        "check" => run::check(make(), &reference, a.get("--observed").as_deref().map(std::path::Path::new)),
        "pixels" => {
            let rule = if name == "epaint" { Rule::Identical } else { Rule::Tolerance };
            println!("{name} ({rule:?})");
            run::pixels(make, rule, &reference, a.get("--out").as_deref().map(std::path::Path::new))
        }
        "bench" => {
            let only: Option<Vec<String>> = a.get("--scenarios").map(|s| s.split(',').map(str::to_owned).collect());
            let rows = run::bench(make, &a.list("--layers", &[20, 600]), &a.list("--scale", &[1.0]), only.as_deref());
            print!("{}", run::bench_table(&rows));
            let path = a.get("--out").unwrap_or_else(|| format!("results/rt-runner-{name}.json"));
            if let Some(dir) = std::path::Path::new(&path).parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            std::fs::write(&path, run::bench_json(name, &rows)).map_err(|e| format!("{path}: {e}"))?;
            println!("wrote {path}");
            Ok(true)
        }
        "window" => {
            let layers = a.list("--layers", &[20]).first().copied().unwrap_or(20);
            let exit_after = a.get("--exit-after").and_then(|v| v.parse().ok());
            run::window(make, layers, exit_after).map(|()| true)
        }
        other => Err(format!("unknown command {other}")),
    }
}

fn main() -> ExitCode {
    let args = match parse() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    let result = match args.renderer.as_str() {
        "epaint" => headless(backend::epaint, "epaint", &args),
        #[cfg(feature = "skia")]
        "skia" => headless(backend::skia, "skia", &args),
        #[cfg(feature = "skia-gpu")]
        "skia-gpu" => headless(backend::skia_gpu, "skia-gpu", &args),
        other => Err(format!("unknown renderer {other} (skia and skia-gpu need the `skia` feature)")),
    };
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(2)
        }
    }
}
