use super::smi::{Ambiguous, bounded_output, parse_nvidia, pci_address, pick};
use super::*;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

const fn adapter(vendor: u32, device: u32) -> Adapter {
    Adapter { vendor, device, bus: None }
}

/// Wait until `flag` is set (a fake query that answers when the test says so), at most 10 s.
fn wait_for(flag: &AtomicBool) {
    let start = Instant::now();
    while !flag.load(Ordering::SeqCst) && start.elapsed() < Duration::from_secs(10) {
        std::thread::sleep(Duration::from_millis(1));
    }
}

/// Wait until `cache` has no background query for `a` running, at most 10 s.
fn settled(cache: &Cache, a: &Adapter) {
    let start = Instant::now();
    while cache.asking(a) && start.elapsed() < Duration::from_secs(10) {
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(!cache.asking(a), "the background query finished");
}

/// No render waits for `nvidia-smi` once an adapter has an answer: a stale one comes back at once
/// while one background query asks again, and its answer is kept.
#[test]
fn a_stale_answer_is_returned_while_a_new_one_is_asked_for() {
    static CACHE: Cache = Cache::new(Duration::from_secs(30));
    static ASKED: AtomicU64 = AtomicU64::new(0);
    static ANSWER: AtomicBool = AtomicBool::new(true);
    fn blocked(_: &Adapter) -> Option<u64> {
        wait_for(&ANSWER);
        Some(ASKED.fetch_add(1, Ordering::SeqCst) + 1)
    }
    let a = adapter(NVIDIA, 1);
    assert_eq!(CACHE.get(&a, blocked), Some(1), "the first question waits for the answer");
    assert!(CACHE.expire(&a));
    ANSWER.store(false, Ordering::SeqCst);
    let t = Instant::now();
    assert_eq!(CACHE.get(&a, blocked), Some(1));
    assert_eq!(CACHE.get(&a, blocked), Some(1));
    assert!(t.elapsed() < Duration::from_secs(5), "waited {:?} for the query", t.elapsed());
    assert!(CACHE.asking(&a));
    ANSWER.store(true, Ordering::SeqCst);
    settled(&CACHE, &a);
    assert_eq!(CACHE.get(&a, blocked), Some(2), "the background answer is kept");
    assert_eq!(ASKED.load(Ordering::SeqCst), 2, "one query in the background, not one per question");
    assert_eq!(CACHE.get(&adapter(AMD, 2), blocked), Some(3), "another adapter is asked for directly");
}

/// A card is found by its device id among several; with several GPUs and no match there is no
/// answer; a single GPU is taken even when its id is reported differently; other output is no
/// card.
#[test]
fn nvidia_output_is_matched_by_address_then_device_id() {
    let two = "00000000:01:00.0, 0x179B10DE, 3990\n00000000:02:00.0, 0x1F0810DE, 5120\n";
    let cards = parse_nvidia(two);
    assert_eq!(pick(&cards, &adapter(NVIDIA, 0x1f08), Ambiguous::Unknown), Some(5120 << 20));
    assert_eq!(pick(&cards, &adapter(NVIDIA, 0x179b), Ambiguous::Unknown), Some(3990 << 20));
    assert_eq!(pick(&cards, &adapter(NVIDIA, 0x2204), Ambiguous::Smallest), None);
    let one = parse_nvidia("00000000:01:00.0, 0x179B10DE, 3990\n");
    assert_eq!(pick(&one, &adapter(NVIDIA, 0x1234), Ambiguous::Unknown), Some(3990 << 20));
    assert!(parse_nvidia("No devices were found\n").is_empty());
    assert_eq!(pick(&parse_nvidia(""), &adapter(NVIDIA, 0x179b), Ambiguous::Unknown), None);
}

/// Two cards of one model are told apart by PCI address; without one, free memory has no answer
/// (the caller keeps its fixed budget) and an address no card has is no card.
#[test]
fn two_cards_of_one_model_are_told_apart_by_address_or_not_at_all() {
    let twins = parse_nvidia("00000000:01:00.0, 0x1F0810DE, 1000\n00000000:02:00.0, 0x1F0810DE, 6000\n");
    let at = |bus| Adapter { bus: pci_address(bus), ..adapter(NVIDIA, 0x1f08) };
    assert_eq!(pick(&twins, &at("0000:02:00.0"), Ambiguous::Unknown), Some(6000 << 20));
    assert_eq!(pick(&twins, &at("0000:01:00.0"), Ambiguous::Unknown), Some(1000 << 20));
    assert_eq!(pick(&twins, &adapter(NVIDIA, 0x1f08), Ambiguous::Unknown), None);
    assert_eq!(pick(&twins, &at(""), Ambiguous::Unknown), None);
    let mixed = parse_nvidia("00000000:01:00.0, 0x1F0810DE, 1000\n00000000:02:00.0, 0x220410DE, 6000\n");
    assert_eq!(pick(&mixed, &at("0000:03:00.0"), Ambiguous::Smallest), None);
}

/// A card's total memory, when several cards share its ids and no address tells them apart, is the
/// smallest (a budget planned for it fits either); a zero size is no answer.
#[test]
fn nvidia_smi_cards_sharing_an_id_report_the_smaller_memory() {
    let total = |out: &str| {
        let mut cards = parse_nvidia(out);
        cards.retain(|c| c.bytes > 0);
        pick(&cards, &adapter(NVIDIA, 0x2c05), Ambiguous::Smallest)
    };
    assert_eq!(total(", 0x2C0510DE, 16303\n, 0x2C0510DE, 8192\n"), Some(8192 << 20));
    assert_eq!(total(", 0x2C0510DE, 8192\n, 0x2C0510DE, 16303\n"), Some(8192 << 20));
    assert_eq!(total(", 0x2C0510DE, lots\n, 0x2C0510DE, 16303\n"), Some(16303 << 20), "a bad line is skipped");
    assert_eq!(total(", 0x2C0510DE, 0\n, 0x2C0510DE, 16303\n"), Some(16303 << 20), "a zero size is no answer");
}

/// amdgpu's VRAM size is read from the card whose ids match; a card without the file, with garbage
/// ids or a malformed size, a missing card and a missing folder all give nothing.
#[test]
fn drm_video_memory_reads_the_matching_cards_vram() {
    let drm = std::env::temp_dir().join(format!("craft-gpu-host-drm-{}", std::process::id()));
    let card = |name: &str, vendor: &str, device: &str, vram: Option<&str>| {
        let dev = drm.join(name).join("device");
        std::fs::create_dir_all(&dev).unwrap();
        std::fs::write(dev.join("vendor"), vendor).unwrap();
        std::fs::write(dev.join("device"), device).unwrap();
        if let Some(v) = vram {
            std::fs::write(dev.join("mem_info_vram_total"), v).unwrap();
        }
    };
    card("card0", "0x8086\n", "0x9a49\n", None);
    card("card1", "0x1002\n", "0x73bf\n", Some("17163091968\n"));
    card("card2", "not hex\n", "0x73bf\n", Some("1\n"));
    card("card3", "0x10de\n", "0x2684\n", Some("lots\n"));
    assert_eq!(drm_total(&drm, &adapter(0x1002, 0x73bf)), Some(17_163_091_968));
    assert_eq!(drm_total(&drm, &adapter(0x8086, 0x9a49)), None, "no amdgpu VRAM file");
    assert_eq!(drm_total(&drm, &adapter(0x10de, 0x2684)), None, "a malformed size");
    assert_eq!(drm_total(&drm, &adapter(0xdead, 0xbeef)), None, "no such card");
    assert_eq!(drm_total(&drm.join("missing"), &adapter(0x1002, 0x73bf)), None, "no drm folder");
    std::fs::remove_dir_all(&drm).unwrap();
}

/// The first question for an adapter runs on the caller's thread, often while the window opens:
/// a query the driver never answers gives up at the deadline instead of keeping the app from
/// starting. A quick one answers; a failed or missing one gives nothing.
#[cfg(any(unix, windows))]
#[test]
fn a_child_that_never_answers_gives_up_at_the_deadline() {
    let command = |unix: &[&str], windows: &[&str]| {
        let args = if cfg!(windows) { windows } else { unix };
        let mut c = std::process::Command::new(args[0]);
        c.args(&args[1..]);
        c
    };
    let start = Instant::now();
    assert_eq!(bounded_output(command(&["sleep", "10"], &["ping", "-n", "11", "127.0.0.1"]), Duration::from_millis(200)), None);
    assert!(start.elapsed() < Duration::from_secs(2), "took {:?}", start.elapsed());

    let out = bounded_output(
        command(&["printf", "00000000:01:00.0, 0x2C0510DE, 16303\\n"], &["cmd", "/C", "echo 00000000:01:00.0, 0x2C0510DE, 16303"]),
        Duration::from_secs(10),
    )
    .expect("a quick command answers");
    assert_eq!(parse_nvidia(&String::from_utf8_lossy(&out)).first().map(|c| c.bytes), Some(16303 << 20));

    assert_eq!(bounded_output(command(&["false"], &["cmd", "/C", "exit 3"]), Duration::from_secs(10)), None, "a failed run");
    assert_eq!(bounded_output(std::process::Command::new("craft-no-such-tool"), Duration::from_secs(1)), None, "not installed");
}
