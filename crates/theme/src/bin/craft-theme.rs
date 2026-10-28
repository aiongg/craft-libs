//! `craft-theme check <file>... [--json]`: validates theme files and lists their contrast
//! warnings in every appearance. Exits 1 when any file has errors.

use std::process::ExitCode;

use craft_theme::AppId;
use craft_theme::format;
use craft_theme::{Appearance, Contrast, Mode, Size, TextSize, ThemeDef, Variants, resolve};
use serde_json::{Value, json};

const USAGE: &str = "usage: craft-theme check <file>... [--json]";

struct Warning {
    appearance: String,
    fg: String,
    bg: String,
    ratio: f64,
    minimum: f64,
}

struct Report {
    file: String,
    errors: Vec<(String, String)>,
    warnings: Vec<Warning>,
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() != Some("check") {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }
    let mut json_out = false;
    let mut files = Vec::new();
    for arg in args {
        match arg.as_str() {
            "--json" => json_out = true,
            a if a.starts_with("--") => {
                eprintln!("unknown option {a}\n{USAGE}");
                return ExitCode::from(2);
            }
            _ => files.push(arg),
        }
    }
    if files.is_empty() {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }

    let reports: Vec<Report> = files.iter().map(|f| check(f)).collect();
    if json_out {
        let objects: Vec<Value> = reports.iter().map(report_json).collect();
        println!("{}", serde_json::to_string_pretty(&objects).unwrap_or_default());
    } else {
        print_text(&reports);
    }
    if reports.iter().any(|r| !r.errors.is_empty()) { ExitCode::FAILURE } else { ExitCode::SUCCESS }
}

fn check(path: &str) -> Report {
    let mut report = Report { file: path.to_owned(), errors: Vec::new(), warnings: Vec::new() };
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            report.errors.push((String::new(), format!("cannot read: {e}")));
            return report;
        }
    };
    match format::parse(&bytes) {
        Ok(file) => report.warnings = warnings(&file.theme),
        Err(errors) => report.errors = errors.0.iter().map(|e| (e.path_string(), e.message())).collect(),
    }
    report
}

/// Every contrast warning of the theme in each mode it has, contrast and size, for the theme
/// itself and for each of its apps.
fn warnings(theme: &ThemeDef) -> Vec<Warning> {
    let modes: &[Mode] = match theme.variants {
        Variants::Both => &[Mode::Light, Mode::Dark],
        Variants::LightOnly => &[Mode::Light],
        Variants::DarkOnly => &[Mode::Dark],
    };
    let apps: Vec<Option<&AppId>> = std::iter::once(None).chain(theme.apps.keys().map(Some)).collect();
    let mut out = Vec::new();
    for app in apps {
        for &mode in modes {
            for contrast in [Contrast::Standard, Contrast::High] {
                for size in [Size::Compact, Size::Comfortable] {
                    let appearance = Appearance { mode, contrast, size, text_size: TextSize::Default, reduced_motion: false };
                    let label = format!(
                        "{}{}/{}/{}",
                        app.map_or(String::new(), |a| format!("{a}: ")),
                        if mode == Mode::Light { "light" } else { "dark" },
                        if contrast == Contrast::High { "high" } else { "standard" },
                        if size == Size::Compact { "compact" } else { "comfortable" },
                    );
                    for w in resolve(theme, appearance, app).warnings {
                        out.push(Warning { appearance: label.clone(), fg: w.fg.path(), bg: w.bg.path(), ratio: w.ratio, minimum: w.minimum });
                    }
                }
            }
        }
    }
    out
}

fn print_text(reports: &[Report]) {
    let (mut errors, mut warnings) = (0, 0);
    for r in reports {
        println!("{}", r.file);
        for (path, message) in &r.errors {
            if path.is_empty() { println!("  error: {message}") } else { println!("  error: {path}: {message}") }
        }
        for w in &r.warnings {
            println!("  warning: {}: {} on {}: {:.2}:1 (minimum {})", w.appearance, w.fg, w.bg, w.ratio, w.minimum);
        }
        errors += r.errors.len();
        warnings += r.warnings.len();
    }
    println!("{} file(s) checked: {errors} error(s), {warnings} warning(s)", reports.len());
}

fn report_json(r: &Report) -> Value {
    json!({
        "file": r.file,
        "errors": r.errors.iter().map(|(path, message)| json!({ "path": path, "message": message })).collect::<Vec<_>>(),
        "warnings": r.warnings.iter().map(|w| json!({
            "appearance": w.appearance, "fg": w.fg, "bg": w.bg, "ratio": w.ratio, "minimum": w.minimum,
        })).collect::<Vec<_>>(),
    })
}
