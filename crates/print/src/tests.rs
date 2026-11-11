use super::*;
use crate as spool;

mod windows_native;

/// Set on the child process when a test runs this test binary as a stand-in for `lp`.
const STAND_IN_LP: &str = "CRAFT_STAND_IN_LP";

/// Not a test of its own: [`stand_in_lp`] re-runs this binary with only this test selected, and it
/// then plays `lp`. It records its arguments and stdin in the folder named by [`STAND_IN_LP`], or,
/// when the folder is `refuse`, exits without reading stdin, the way `lp` refuses an unknown
/// printer.
#[test]
fn stand_in_for_lp() {
    let Some(record) = std::env::var_os(STAND_IN_LP) else { return };
    if record == "refuse" {
        eprintln!("lp: The printer or class does not exist.");
        std::process::exit(1);
    }
    let record = std::path::PathBuf::from(record);
    let args: Vec<String> = std::env::args().skip_while(|a| a != "--").skip(1).collect();
    let mut stdin = Vec::new();
    std::io::Read::read_to_end(&mut std::io::stdin(), &mut stdin).unwrap();
    std::fs::write(record.join("args"), args.join(" ")).unwrap();
    std::fs::write(record.join("stdin"), stdin).unwrap();
}

/// A spooler command that runs [`stand_in_for_lp`] with `record` (see there).
fn stand_in_lp(record: &std::ffi::OsStr) -> std::process::Command {
    let mut c = std::process::Command::new(std::env::current_exe().unwrap());
    c.args(["--exact", "tests::stand_in_for_lp", "--nocapture", "--"]).env(STAND_IN_LP, record);
    c
}

#[test]
fn print_jobs_reach_lp_on_stdin_never_through_a_shared_temp_folder() {
    // The job used to be written to `<temp>/craft-print-<pid>/job-<time>.pdf` and handed to `lp`
    // by name. In a shared /tmp another local user can predict that folder, create it first (or
    // plant a symlink there) and so read every printed document or swap the file before `lp`
    // reads it. Piping the job to `lp` leaves nothing on disk.
    static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let record =
        std::env::temp_dir().join(format!("craft-print-test-{}-{}", std::process::id(), N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)));
    std::fs::create_dir(&record).unwrap();
    // Bigger than any pipe buffer, so the spooler must read while the job is still being written.
    let pdf: Vec<u8> = b"%PDF-1.7 synthetic print job\n".iter().copied().cycle().take(3 << 20).collect();
    let job = Job { title: "memo.pdf".into(), ..Job::default() };
    let sent = spool::submit_via(stand_in_lp(record.as_os_str()), &pdf, &job);
    let args = std::fs::read_to_string(record.join("args"));
    let stdin = std::fs::read(record.join("stdin"));
    let _ = std::fs::remove_dir_all(&record);
    assert!(sent.is_ok(), "{sent:?}");
    assert_eq!(args.unwrap(), "-n 1 -t memo.pdf -o collate=true -o fit-to-page=false", "no file argument: lp reads the job from stdin");
    assert!(stdin.unwrap() == pdf, "lp receives the whole job on stdin");
    let predictable = std::env::temp_dir().join(format!("craft-print-{}", std::process::id()));
    assert!(!predictable.exists(), "nothing is created at {predictable:?}, a name other local users can predict");
}

/// A PPD shaped like a Fiery's: installable options, multi-line PostScript in the choices, Latin-1
/// labels, options the Print dialog sets itself.
const FIERY_PPD: &[u8] = b"*PPD-Adobe: \"4.3\"\n\
*LanguageEncoding: ISOLatin1\n\
*OpenGroup: InstallableOptions/Installable Options\n\
*OpenUI *EFFinisher/Finisher option: PickOne\n\
*DefaultEFFinisher: False\n\
*EFFinisher False/Not installed: \"\"\n\
*EFFinisher SingleStapler/Single stapler: \"\"\n\
*CloseUI: *EFFinisher\n\
*CloseGroup: InstallableOptions\n\
*OpenGroup: FPPaperSource/Media\n\
*OpenUI *InputSlot/Paper tray: PickOne\n\
*OrderDependency: 20.0 AnySetup *InputSlot\n\
*DefaultInputSlot: AutoSelect\n\
*InputSlot AutoSelect/Auto tray select: \"\n\
userdict /XJXEFIsetpageproperties known\n\
{ << /XJXsettrayselV2 [ 7 ] >> XJXEFIsetpageproperties } if\"\n\
*End\n\
*InputSlot ManualFeed/Bypass tray: \"\n\
{ pop 2 XJXsettrayselV2 } if\"\n\
*End\n\
*InputSlot Tray2/Tray 2: \"\"\n\
*fr.InputSlot Tray2/Bac 2: \"\"\n\
*CloseUI: *InputSlot\n\
*OpenUI *EFMediaType/Paper type: PickOne\n\
*DefaultEFMediaType: Plain\n\
*EFMediaType Plain/Plain: \"\"\n\
*EFMediaType Heavy1/Thick 1 (106\xad163 g/m\xb2): \"\"\n\
*CloseUI: *EFMediaType\n\
*CloseGroup: FPPaperSource\n\
*OpenUI *PageSize/Page size: PickOne\n\
*DefaultPageSize: A4\n\
*PageSize A4/A4: \"\"\n\
*CloseUI: *PageSize\n\
*OpenUI *EFRaster/Print queue action: PickOne\n\
*DefaultEFRaster: Bogus\n\
*EFRaster False/Print: \"\"\n\
*EFRaster Hold: \"\"\n\
*CloseUI: *EFRaster\n";

#[test]
fn printer_options_come_from_the_ppd() {
    let options = spool::parse_ppd(&spool::ppd_text(FIERY_PPD));
    let keys: Vec<&str> = options.iter().map(|o| o.key.as_str()).collect();
    assert_eq!(keys, ["InputSlot", "EFMediaType", "EFRaster"], "no installable options, no PageSize");
    let tray = &options[0];
    assert_eq!((tray.label.as_str(), tray.group.as_str(), tray.default.as_str()), ("Paper tray", "Media", "AutoSelect"));
    assert_eq!(
        tray.choices,
        [("AutoSelect".into(), "Auto tray select".into()), ("ManualFeed".into(), "Bypass tray".into()), ("Tray2".into(), "Tray 2".into())],
        "PostScript lines and translations (*fr.InputSlot) are not choices"
    );
    assert_eq!(options[1].choices[1].1, "Thick 1 (106\u{ad}163 g/m²)", "Latin-1 labels");
    // A default that isn't one of the choices falls back to the first; a choice without a label shows its keyword.
    assert_eq!((options[2].default.as_str(), options[2].choices[1].1.as_str()), ("False", "Hold"));
    assert!(spool::parse_ppd("*OpenUI *Broken\n*Broken A/B\n").is_empty(), "unclosed or malformed blocks are skipped");
}
