//! Renders gallery stories to PNG without a window.
//!
//! `capture <spec.json> <outdir>` draws every shot of the spec; `capture --list` prints the
//! components and their stories. See [`craft_ui_gallery::capture`] for the spec.

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.as_slice() {
        [flag] if flag == "--list" => {
            print!("{}", craft_ui_gallery::capture::list());
            Ok(())
        }
        [spec, out_dir] => craft_ui_gallery::capture::run(spec.as_ref(), out_dir.as_ref()),
        _ => Err("usage: capture <spec.json> <outdir> | capture --list".to_owned()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("capture: {message}");
            ExitCode::FAILURE
        }
    }
}
