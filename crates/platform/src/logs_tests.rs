//! Behaviour tests of `logs`.
use super::*;
use std::time::Duration;
use web_time::UNIX_EPOCH;

const NAMES: LogNames = LogNames::new("photocraft");

#[cfg(not(target_arch = "wasm32"))]
fn temp_dir(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    let d = std::env::temp_dir().join(format!("craft-logs-{tag}-{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
    let _ = std::fs::remove_dir_all(&d);
    d
}

#[cfg(not(target_arch = "wasm32"))]
fn read(path: &Path) -> String {
    std::fs::read_to_string(path).expect("read log")
}

// The default filter shows the app's info and other crates' warnings; directives follow env_logger, the most specific wins, hostile specs never panic.
#[test]
fn filter_directives_follow_env_logger() {
    assert_eq!(NAMES.default_filter(), "warn,photocraft*=info");
    let f = Filter::parse(&NAMES.default_filter());
    assert_eq!(f.level_for("photocraft"), LevelFilter::Info);
    assert_eq!(f.level_for("photocraft_gpu::health"), LevelFilter::Info);
    assert_eq!(f.level_for("wgpu_core::device"), LevelFilter::Warn);
    assert_eq!(f.max(), LevelFilter::Info);

    let f = Filter::parse("info,wgpu_core=error,photocraft_gpu=trace,photocraft_gpu::health=off");
    assert_eq!(f.level_for("eframe"), LevelFilter::Info);
    assert_eq!(f.level_for("wgpu_core::instance"), LevelFilter::Error);
    assert_eq!(f.level_for("photocraft_gpu::canvas"), LevelFilter::Trace);
    assert_eq!(f.level_for("photocraft_gpu::health"), LevelFilter::Off);
    assert_eq!(f.max(), LevelFilter::Trace);
    assert_eq!(f.level_for("wgpu_core_extra"), LevelFilter::Info, "a name matches at `::` boundaries");
    assert_eq!(Filter::parse("naga").level_for("naga::front"), LevelFilter::Trace);
    assert_eq!(Filter::parse("naga").level_for("eframe"), LevelFilter::Error);

    for spec in ["", ",,,", "=", "=debug", "photocraft=", "nonsense=loud", "🦀=info", "*", "a=b=c", "warn,,photocraft=DEBUG"] {
        let f = Filter::parse(spec);
        let _ = (f.level_for("photocraft"), f.max());
    }
    assert_eq!(Filter::parse("warn,,photocraft=DEBUG").level_for("photocraft"), LevelFilter::Debug);
    assert_eq!(Filter::parse("nonsense=loud").level_for("nonsense"), LevelFilter::Error);
}

// Timestamps are UTC with milliseconds, the epoch before 1970, and full years past 9999.
#[test]
fn timestamps_are_utc_with_milliseconds() {
    assert_eq!(timestamp(UNIX_EPOCH), "1970-01-01T00:00:00.000Z");
    assert_eq!(timestamp(UNIX_EPOCH + Duration::from_millis(1_791_446_357_728)), "2026-10-08T07:59:17.728Z");
    assert_eq!(timestamp(UNIX_EPOCH + Duration::from_secs(1_709_164_800)), "2024-02-29T00:00:00.000Z");
    assert_eq!(timestamp(UNIX_EPOCH - Duration::from_secs(5)), "1970-01-01T00:00:00.000Z");
    assert_eq!(timestamp(UNIX_EPOCH + Duration::from_secs(253_402_300_800)), "10000-01-01T00:00:00.000Z");
    if let Some(far) = UNIX_EPOCH.checked_add(Duration::from_secs(10_000_000_000_000)) {
        assert_eq!(timestamp(far), "318857-05-20T17:46:40.000Z");
    }
    assert_eq!(format_line("t", log::Level::Warn, "main", "app::gpu", "lost\n"), "t WARN  [main] app::gpu: lost\n");
}

// Each start shifts app.log to app.1.log and app.2.log; while another instance holds the owner lock, a start writes app-<pid>.log and leaves the chain alone.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn rotation_keeps_previous_logs_and_a_running_instance_log() {
    let dir = temp_dir("rotate");
    for run in 1..=4 {
        let LogFile { path, .. } = rotate(&dir, NAMES).expect("rotate");
        assert_eq!(path, dir.join("photocraft.log"));
        assert_eq!(read(&path), "");
        std::fs::write(&path, format!("run {run}")).expect("write");
    }
    assert_eq!(read(&dir.join("photocraft.log")), "run 4");
    assert_eq!(read(&dir.join("photocraft.1.log")), "run 3");
    assert_eq!(read(&dir.join("photocraft.2.log")), "run 2");
    assert!(!dir.join("photocraft.3.log").exists(), "only {KEEP} old logs are kept");

    // A start while the first instance runs leaves its log in place.
    let mut first = rotate(&dir, NAMES).expect("first start");
    first.file.write_all(b"before\n").expect("write");
    for _ in 0..=KEEP {
        let second = rotate(&dir, NAMES).expect("second start");
        assert_eq!(second.path, dir.join(format!("photocraft-{}.log", std::process::id())));
    }
    first.file.write_all(b"after\n").expect("write");
    assert_eq!(read(&dir.join("photocraft.log")), "before\nafter\n");
    assert_eq!(read(&dir.join("photocraft.1.log")), "run 4", "the chain did not move");

    // Once it has exited, the next start rotates again.
    drop(first);
    let third = rotate(&dir, NAMES).expect("third start");
    assert_eq!(third.path, dir.join("photocraft.log"));
    assert_eq!(read(&dir.join("photocraft.1.log")), "before\nafter\n");

    assert_eq!(NAMES.instance_pid("photocraft-123.log"), Some(123));
    assert_eq!(NAMES.instance_pid("photocraft-12a.log"), None);
    assert_eq!(NAMES.instance_pid("photocraft.1.log"), None);
    let _ = std::fs::remove_dir_all(&dir);

    // A file where the directory should be is an error.
    let blocked = temp_dir("blocked");
    std::fs::write(&blocked, "a file").expect("block");
    assert!(rotate(&blocked, NAMES).is_err());
    let _ = std::fs::remove_file(&blocked);
}

// Lines before the file opens are written on attach, at most MAX_PENDING with a dropped-count line; the file stops at its cap with one note.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn the_sink_keeps_early_lines_and_stops_at_its_cap() {
    let dir = temp_dir("sink");
    let LogFile { path, file, .. } = rotate(&dir, NAMES).expect("rotate");
    let mut sink = Sink::new(MAX_FILE_BYTES);
    for i in 0..MAX_PENDING + 5 {
        sink.write(&format!("line {i}\n"));
    }
    sink.attach(file);
    sink.write("late\n");
    let text = read(&path);
    assert_eq!(text.lines().filter(|l| l.starts_with("line ")).count(), MAX_PENDING);
    assert!(text.contains("5 earlier log lines were dropped"), "{text}");
    assert!(text.contains("line 0\n") && !text.contains(&format!("line {MAX_PENDING}\n")) && text.ends_with("late\n"));

    let LogFile { path, file, .. } = rotate(&dir, NAMES).expect("rotate");
    let mut sink = Sink::new(100);
    sink.attach(file);
    for i in 0..50 {
        sink.write(&format!("line {i:02} with some padding\n"));
    }
    let text = read(&path);
    assert!(text.lines().filter(|l| l.starts_with("line ")).count() < 5, "{text}");
    assert_eq!(text.matches("log file reached 100 bytes").count(), 1, "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

// The logger writes only records its filter passes, as whole lines from many threads, and keeps logging to stderr when the directory is unusable.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn the_logger_filters_and_writes_whole_lines() {
    use log::Log;
    let dir = temp_dir("logger");
    let logger = AppLogger::new(NAMES, Filter::parse(&NAMES.default_filter()), false);
    let record = |level, target: &str, msg: &str| {
        logger.log(&log::Record::builder().level(level).target(target).args(format_args!("{msg}")).build());
    };
    record(log::Level::Info, "photocraft::dirs", "early");
    record(log::Level::Info, "wgpu_core::device", "too chatty");
    let path = logger.attach_dir(&dir).expect("attach");
    assert_eq!(path, dir.join("photocraft.log"));
    record(log::Level::Warn, "wgpu_hal::vulkan", "a real warning");
    record(log::Level::Debug, "photocraft", "below info");
    std::thread::scope(|s| {
        for t in 0..8 {
            let record = &record;
            s.spawn(move || {
                for i in 0..50 {
                    record(log::Level::Info, "photocraft::work", &format!("worker {t} line {i}"));
                }
            });
        }
    });
    let text = read(&path);
    assert!(text.contains(" INFO  [") && text.contains("photocraft::dirs: early\n"), "{text}");
    assert!(text.contains("wgpu_hal::vulkan: a real warning\n"), "{text}");
    assert!(!text.contains("too chatty") && !text.contains("below info"), "{text}");
    let work: Vec<&str> = text.lines().filter(|l| l.contains("photocraft::work: worker ")).collect();
    assert_eq!(work.len(), 8 * 50);
    assert!(work.iter().all(|l| l.ends_with(|c: char| c.is_ascii_digit()) && l.contains(" INFO  [")), "interleaved lines");

    let blocked = temp_dir("unusable");
    std::fs::write(&blocked, "not a directory").expect("block");
    let logger = AppLogger::new(NAMES, Filter::parse("info"), false);
    record_to(&logger, "kept until close");
    let err = logger.attach_dir(&blocked).expect_err("unusable directory");
    assert!(err.starts_with(&blocked.display().to_string()), "{err}");
    record_to(&logger, "still fine");
    let _ = std::fs::remove_file(&blocked);
    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(not(target_arch = "wasm32"))]
fn record_to(logger: &AppLogger, msg: &str) {
    use log::Log;
    logger.log(&log::Record::builder().level(log::Level::Error).target("x").args(format_args!("{msg}")).build());
}
