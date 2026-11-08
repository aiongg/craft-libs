//! The printer list, the job state and the print panel.

use craft_print::ui::{Discover, Feedback, Printer, State, shown_names};
use craft_ui::testing::{Fixture, trial};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::channel;
use std::time::Duration;

/// The longest a collision cascade of twenty thousand printers may take to resolve.
const CASCADE_LIMIT: Duration = Duration::from_secs(5);

fn queue(name: &str, default: bool) -> Printer {
    Printer::new(name).is_default(default)
}

fn labelled(name: &str, label: &str) -> Printer {
    Printer::new(name).label(label)
}

/// A discovery that finds `printers` at once.
fn finds(printers: Vec<Printer>) -> Discover {
    Box::new(move || {
        let (sender, receiver) = channel();
        sender.send(printers.clone()).expect("the receiver is alive");
        receiver
    })
}

/// The pass-by-pass resolution `shown_names` replaced, to check they agree.
fn shown_names_by_passes(printers: &[Printer]) -> Vec<String> {
    let base: Vec<&str> = printers.iter().map(|p| if p.label.trim().is_empty() { p.name.as_str() } else { p.label.as_str() }).collect();
    let mut shown: Vec<String> = printers
        .iter()
        .zip(&base)
        .map(|(printer, text)| {
            let shared = base.iter().filter(|other| *other == text).count() > 1;
            if shared && *text != printer.name { format!("{text} ({})", printer.name) } else { (*text).to_owned() }
        })
        .collect();
    loop {
        let shared: Vec<bool> = shown.iter().map(|text| shown.iter().filter(|other| *other == text).count() > 1).collect();
        let mut changed = false;
        for ((text, printer), shared) in shown.iter_mut().zip(printers).zip(shared) {
            if shared && *text != printer.name {
                text.clone_from(&printer.name);
                changed = true;
            }
        }
        if !changed {
            return shown;
        }
    }
}

/// Each printer's label is the name the one before it falls back to, so every pass of the old resolution freed exactly
/// one more printer.
fn cascade(n: usize) -> Vec<Printer> {
    let mut printers = vec![labelled("n0", "x"), labelled("n1", "x"), labelled("n2", "x (n0)")];
    printers.extend((3..n).map(|i| labelled(&format!("n{i}"), &format!("n{}", i - 1))));
    printers
}

/// The shown names resolve collisions like the pass-by-pass rule they replaced.
#[trial]
fn collisions_resolve_like_the_pass_by_pass_rule() {
    let names = ["a", "b", "c", "Office", "Office_1", "x"];
    let labels = ["", "a", "b", "Office", "Office (Office_1)", "x (a)", "a (b)", "c"];
    let mut seed = 7u32;
    for _ in 0..2000 {
        let printers: Vec<Printer> = (0..5)
            .map(|_| {
                seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                labelled(names[(seed >> 8) as usize % names.len()], labels[(seed >> 16) as usize % labels.len()])
            })
            .collect();
        assert_eq!(shown_names(&printers), shown_names_by_passes(&printers), "{printers:?}");
    }
    assert_eq!(shown_names(&cascade(40)), shown_names_by_passes(&cascade(40)));
}

/// A cascade where every pass of the old rule freed one more printer resolves in linear time.
#[trial]
fn a_long_collision_cascade_stays_fast() {
    let printers = cascade(20_000);
    let (sender, receiver) = channel();
    let work = printers.clone();
    std::thread::spawn(move || {
        let _ = sender.send(shown_names(&work));
    });
    let shown = receiver.recv_timeout(CASCADE_LIMIT).expect("the cascade resolves within the limit");
    assert_eq!(shown[0], "n0");
    assert_eq!(shown[1], "x (n1)");
    assert!(shown[2..].iter().zip(&printers[2..]).all(|(s, p)| *s == p.name));
}

/// The panel shows a printer's label, and choosing it still selects the queue: jobs go to the name, not the label.
#[trial]
fn the_dialog_shows_a_printers_label_and_keeps_its_queue_name() {
    // CUPS can't put dots in a queue name: the printer added as 192.0.2.15 is `_192_0_2_15`.
    let printers = [labelled("_192_0_2_15", "192.0.2.15"), queue("Plain_Queue", false), labelled("Blank", "  ")];
    assert_eq!(shown_names(&printers), ["192.0.2.15", "Plain_Queue", "Blank"], "the label, else the name");
    // Two printers described alike stay apart by their queue names; a unique label stays bare.
    let printers = [labelled("Office_1", "Office"), labelled("Office_2", "Office"), labelled("Lab", "Lab printer")];
    assert_eq!(shown_names(&printers), ["Office (Office_1)", "Office (Office_2)", "Lab printer"]);
    // A label that is another queue's name doesn't pass for that queue.
    assert_eq!(shown_names(&[queue("Office", false), labelled("Office_2", "Office")]), ["Office", "Office (Office_2)"]);
    // A label that is another printer's bracketed text doesn't make two entries read alike: the printers whose text is
    // still shared show their queue names.
    let printers = [labelled("Office_1", "Office"), labelled("Office_2", "Office"), labelled("Office_3", "Office (Office_1)")];
    assert_eq!(shown_names(&printers), ["Office_1", "Office (Office_2)", "Office_3"]);
    assert!(shown_names(&[]).is_empty());

    let discover = finds(vec![labelled("_192_0_2_15", "192.0.2.15").is_default(true)]);
    let mut state = State::default();
    assert!(state.refresh(&discover));
    state.poll(&egui::Context::default());
    assert_eq!(state.settings.printer, "_192_0_2_15");
    assert!(state.ready());
}

/// A rediscovery keeps the chosen queue while it exists, blocks the job while printers are being found, and drops a
/// queue that is gone.
#[trial]
fn discovery_preserves_choice_and_blocks_stale_or_removed_queues() {
    let ctx = egui::Context::default();
    let mut state = State::default();
    state.settings.printer = "Chosen".into();
    let discover = finds(vec![queue("Default", true), queue("Chosen", false)]);
    assert!(state.refresh(&discover));
    assert!(!state.ready());
    state.poll(&ctx);
    assert_eq!(state.settings.printer, "Chosen");
    assert!(state.ready());
    let empty = finds(Vec::new());
    assert!(state.refresh(&empty));
    assert!(!state.ready());
    state.poll(&ctx);
    assert!(!state.ready());
    assert_eq!(state.message, Some(Feedback::NoPrinters));
    assert!(state.settings.printer.is_empty());
    assert!(state.refresh(&discover));
    state.poll(&ctx);
    assert_eq!(state.settings.printer, "Default");
}

/// A discovery whose worker stops without a list leaves no queue to print to.
#[trial]
fn failed_discovery_cannot_reuse_a_previously_available_queue() {
    let mut state = State::default();
    state.printers.push(queue("Old", true));
    state.settings.printer = "Old".into();
    let discover: Discover = Box::new(|| channel().1);
    assert!(state.refresh(&discover));
    state.poll(&egui::Context::default());
    assert_eq!(state.message, Some(Feedback::DiscoveryFailed));
    assert!(!state.ready());
}

/// Only one print job can be active: a second `begin` fails, the receiver stays with the job, and submission, queue
/// refusal and a dropped worker report `Submitted`, `Failed` and `WorkerStopped` while returning to ready.
#[trial]
fn active_jobs_keep_their_receiver_and_report_receipt_refusal_or_worker_failure() {
    let ctx = egui::Context::default();
    let mut state = State::default();
    state.printers.push(queue("Queue", true));
    state.settings.printer = "Queue".into();
    let (sender, receiver) = channel();
    assert!(state.begin(receiver).is_ok());
    assert!(!state.ready());
    let (other_sender, other_receiver) = channel::<Result<String, String>>();
    assert!(state.begin(other_receiver).is_err());
    drop(other_sender);
    state.poll(&ctx);
    assert!(state.busy());
    sender.send(Ok("Queue-42".into())).expect("the job holds the receiver");
    state.poll(&ctx);
    assert_eq!(state.message, Some(Feedback::Submitted("Queue-42".into())));
    assert!(state.ready());
    let (sender, receiver) = channel();
    assert!(state.begin(receiver).is_ok());
    sender.send(Err("queue refused the job".into())).expect("the job holds the receiver");
    state.poll(&ctx);
    assert_eq!(state.message, Some(Feedback::Failed("queue refused the job".into())));
    let (sender, receiver) = channel::<Result<String, String>>();
    assert!(state.begin(receiver).is_ok());
    drop(sender);
    state.poll(&ctx);
    assert_eq!(state.message, Some(Feedback::WorkerStopped));
    assert!(state.ready());
}

/// Discovery and the shown panel fill the settings (printer, copies, duplex, grayscale) and make the state ready,
/// without starting or submitting a print job.
#[trial]
fn discovery_and_controls_do_not_submit_a_job() {
    let state = Rc::new(RefCell::new(State::default()));
    let discover = finds(vec![queue("Office", true)]);
    let shared = Rc::clone(&state);
    let mut fixture = Fixture::new(move |ui| {
        shared.borrow_mut().run(ui, &discover);
    });
    fixture.settle();
    let state = state.borrow();
    assert_eq!(state.settings.printer, "Office");
    assert!(state.ready());
    assert!(!state.busy());
    assert_eq!(state.settings.copies, 1);
    assert_eq!(state.settings.duplex, None);
    assert!(!state.settings.grayscale);
}
