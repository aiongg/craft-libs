#![allow(clippy::unwrap_used, reason = "tests")]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn run(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_craft-design-lint")).args(args).current_dir(dir).output().unwrap()
}

fn code(out: &Output) -> i32 {
    out.status.code().unwrap()
}

fn tree(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&root);
    for (rel, src) in files {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, src).unwrap();
    }
    root
}

#[test]
fn exit_codes() {
    let root = tree(
        "cli-exit",
        &[
            ("design-lint.toml", "[lint]\npaths = [\"src\"]\n"),
            ("src/lib.rs", "fn f() {}\n"),
            ("bad/design-lint.toml", "[lint]\npaths = [\"src\"]\n"),
            ("bad/src/lib.rs", "fn f(ui: &mut Ui) { ui.add_space(4.0); }\n"),
        ],
    );
    assert_eq!(code(&run(&root, &["check"])), 0);
    assert_eq!(code(&run(&root.join("bad"), &["check"])), 1);

    // Usage, configuration and I/O problems are 2.
    assert_eq!(code(&run(&root, &[])), 2);
    assert_eq!(code(&run(&root, &["bogus"])), 2);
    assert_eq!(code(&run(&root, &["check", "--bogus"])), 2);
    assert_eq!(code(&run(&root, &["check", "--config"])), 2);
    assert_eq!(code(&run(&root, &["check", "--baseline", "a", "--write-baseline", "b"])), 2);
    assert_eq!(code(&run(&root, &["check", "--config", "missing.toml"])), 2);
    std::fs::write(root.join("broken.toml"), "[lint\n").unwrap();
    assert_eq!(code(&run(&root, &["check", "--config", "broken.toml"])), 2);
    std::fs::write(root.join("unknown.toml"), "[lint]\npaths = [\"src\"]\nbogus = 1\n").unwrap();
    assert_eq!(code(&run(&root, &["check", "--config", "unknown.toml"])), 2);
    std::fs::write(root.join("nopath.toml"), "[lint]\npaths = [\"nowhere\"]\n").unwrap();
    assert_eq!(code(&run(&root, &["check", "--config", "nopath.toml"])), 2);
    assert_eq!(code(&run(&root, &["check-clippy", "--clippy-toml"])), 2);
    assert_eq!(code(&run(&root, &["clippy", "--bogus"])), 2);
}

#[test]
fn baseline_flags() {
    let root =
        tree("cli-baseline", &[("design-lint.toml", "[lint]\npaths = [\"src\"]\n"), ("src/lib.rs", "fn f(ui: &mut Ui) { ui.add_space(4.0); }\n")]);
    assert_eq!(code(&run(&root, &["check"])), 1);
    assert_eq!(code(&run(&root, &["check", "--write-baseline", "baseline.toml"])), 0);
    assert_eq!(code(&run(&root, &["check", "--baseline", "baseline.toml"])), 0);
    std::fs::write(root.join("src/lib.rs"), "fn f(ui: &mut Ui) { ui.add_space(4.0); ui.add_space(5.0); }\n").unwrap();
    assert_eq!(code(&run(&root, &["check", "--baseline", "baseline.toml"])), 1);
    assert_eq!(code(&run(&root, &["check", "--baseline", "missing.toml"])), 2);
    std::fs::write(root.join("src/lib.rs"), "fn f( {").unwrap();
    assert_eq!(code(&run(&root, &["check", "--write-baseline", "other.toml"])), 1);
    assert!(!root.join("other.toml").exists());
}

#[test]
fn summary_flag_prints_findings_then_the_table() {
    let root =
        tree("cli-summary", &[("design-lint.toml", "[lint]\npaths = [\"src\"]\n"), ("src/lib.rs", "fn f(ui: &mut Ui) { ui.add_space(4.0); }\n")]);
    let out = run(&root, &["check", "--summary"]);
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.find("src/lib.rs:1:").unwrap() < text.find("rule ").unwrap(), "{text}");
}
