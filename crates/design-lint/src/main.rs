use std::path::{Path, PathBuf};
use std::process::ExitCode;

use craft_design_lint::{CLIPPY_TEMPLATE, Config, Error, check, check_clippy, write_clippy};

const USAGE: &str = "usage:
  craft-design-lint check [--config <path>] [--summary] [--deny-unused]
                                                              report literal sizes and named colours (default config: design-lint.toml)
  craft-design-lint clippy [--write] [--clippy-toml <path>]   print the template, or merge it into clippy.toml
  craft-design-lint check-clippy [--clippy-toml <path>]       fail unless clippy.toml carries the template";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(code) => code,
        Err(Error::ClippyDrift(drift)) => {
            for d in &drift {
                eprintln!("{d}");
            }
            eprintln!("design-lint: {} template entries missing or changed in clippy.toml; run `craft-design-lint clippy --write`", drift.len());
            ExitCode::from(1)
        }
        Err(e) => {
            eprintln!("design-lint: {e}");
            ExitCode::from(2)
        }
    }
}

fn run(args: &[String]) -> Result<ExitCode, Error> {
    let Some((command, rest)) = args.split_first() else {
        return Ok(usage());
    };
    if command == "check" {
        return run_check(rest);
    }
    let mut write = false;
    let mut clippy_toml = PathBuf::from("clippy.toml");
    let mut rest = rest.iter();
    while let Some(arg) = rest.next() {
        match (command.as_str(), arg.as_str()) {
            ("clippy", "--write") => write = true,
            ("clippy" | "check-clippy", "--clippy-toml") => match rest.next() {
                Some(path) => clippy_toml = PathBuf::from(path),
                None => return Ok(usage()),
            },
            _ => return Ok(usage()),
        }
    }
    match command.as_str() {
        "clippy" if write => {
            let changed = write_clippy(&clippy_toml)?;
            println!("design-lint: {} {}", clippy_toml.display(), if changed { "updated" } else { "already current" });
        }
        "clippy" => print!("{CLIPPY_TEMPLATE}"),
        "check-clippy" => {
            check_clippy(&clippy_toml)?;
            println!("design-lint: {} carries the template", clippy_toml.display());
        }
        _ => return Ok(usage()),
    }
    Ok(ExitCode::SUCCESS)
}

fn usage() -> ExitCode {
    eprintln!("{USAGE}");
    ExitCode::from(2)
}

fn run_check(args: &[String]) -> Result<ExitCode, Error> {
    let mut config_path = PathBuf::from("design-lint.toml");
    let mut summary = false;
    let mut deny_unused = false;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--summary" => summary = true,
            "--deny-unused" => deny_unused = true,
            "--config" => match args.next() {
                Some(path) => config_path = PathBuf::from(path),
                None => return Ok(usage()),
            },
            _ => return Ok(usage()),
        }
    }
    let mut config = Config::load(&config_path)?;
    config.deny_unused |= deny_unused;
    let root = config_path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let report = check(&config, root)?;
    if summary {
        print!("{}", report.summary_table());
        println!("{}", report.summary_line());
    } else {
        print!("{}", report.render());
    }
    Ok(ExitCode::from(report.exit_code()))
}
