//! Tests for the update HTTP client.

use super::*;

const DAYS_30: Duration = Duration::from_secs(30 * 86_400);
const DAYS_7: Duration = Duration::from_secs(7 * 86_400);

#[test]
fn host_is_what_follows_the_scheme() {
    assert_eq!(host("https://api.github.com/repos/storytold/wordcraft"), "api.github.com");
    assert_eq!(host("http://127.0.0.1:8080"), "127.0.0.1:8080");
    assert_eq!(host("not a url"), "not a url");
    assert_eq!(host("http://objects.example.com?X-Amz-Signature=abc"), "objects.example.com");
    assert_eq!(host("http://user:secret@example.com/x"), "example.com");
}

#[test]
fn cleaning_keeps_current_partial_files_and_removes_leftovers() {
    let dir = tempfile::tempdir().unwrap();
    let old = std::time::SystemTime::now() - Duration::from_secs(3600);
    let make = |name: &str| {
        let p = dir.path().join(name);
        std::fs::write(&p, b"x").unwrap();
        std::fs::File::options().write(true).open(&p).unwrap().set_modified(old).unwrap();
        p
    };
    let keep = make("photocraft-0.5.0-macos-universal.dmg.150000.part");
    let legacy = make("photocraft-0.5.0-macos-universal.part");
    let finished = make("photocraft-0.5.0-macos-universal.dmg");
    let fresh = dir.path().join("x.part");
    std::fs::write(&fresh, b"x").unwrap();
    clean_downloads(dir.path(), DAYS_30, DAYS_7);
    assert!(keep.exists() && fresh.exists() && finished.exists());
    assert!(!legacy.exists());
    // A finished file goes sooner than a paused one.
    clean_downloads(dir.path(), DAYS_30, Duration::from_secs(60));
    assert!(keep.exists() && !finished.exists());
    clean_downloads(dir.path(), Duration::from_secs(60), DAYS_7);
    assert!(!keep.exists());
}

#[test]
fn a_kept_file_that_still_matches_is_not_fetched_again() {
    let (body, sha) = body();
    let len = body.len() as u64;
    let (url, requests) = serve(body.clone(), vec![Reply::Full, Reply::Full]);
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("file.bin");
    let http = Http::new(crate::TEST_AGENT).unwrap();
    http.download(&url, &dest, len, &sha, &Stop::default(), &mut |_| {}).unwrap();
    assert!(finished(&dest, len) && !finished(&dest, 0));
    // Old enough to be cleaned up, were it not about to be used.
    let old = std::time::SystemTime::now() - Duration::from_secs(40 * 86_400);
    std::fs::File::options().write(true).open(&dest).unwrap().set_modified(old).unwrap();
    let mut last = (0, 0);
    let again = http.download(&url, &dest, len, &sha, &Stop::default(), &mut |t| {
        if let Transfer::Bytes { done, total } = t {
            last = (done, total);
        }
    });
    assert_eq!(again, Ok(()));
    assert_eq!(last, (len, len));
    assert_eq!(requests.lock().unwrap().len(), 1);
    // A cleanup while the installer runs leaves the reused file alone.
    clean_downloads(dir.path(), DAYS_30, DAYS_7);
    assert_eq!(std::fs::read(&dest).unwrap(), body);
    discard(&dest, len);
    assert!(!finished(&dest, len));
}

#[test]
fn a_kept_file_that_no_longer_matches_is_fetched_again() {
    let (body, sha) = body();
    let len = body.len() as u64;
    let (url, requests) = serve(body.clone(), vec![Reply::Full, Reply::Full]);
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("file.bin");
    let http = Http::new(crate::TEST_AGENT).unwrap();
    http.download(&url, &dest, len, &sha, &Stop::default(), &mut |_| {}).unwrap();
    // The same size, so only the checksum can tell.
    std::fs::write(&dest, vec![b'x'; body.len()]).unwrap();
    http.download(&url, &dest, len, &sha, &Stop::default(), &mut |_| {}).unwrap();
    assert_eq!(std::fs::read(&dest).unwrap(), body);
    assert_eq!(requests.lock().unwrap().len(), 2);
}

#[test]
fn reads_content_ranges() {
    assert_eq!(parse_content_range("bytes 100-199/1000"), Some((100, Some(1000))));
    assert_eq!(parse_content_range("bytes 5-9/*"), Some((5, None)));
    assert_eq!(parse_content_range("items 1-2/3"), None);
    assert_eq!(parse_content_range("bytes x-9/10"), None);
}

use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

/// How the fake server answers one request.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Reply {
    /// The whole file.
    Full,
    /// Promise the whole file, send `n` bytes, hang up.
    DropAfter(usize),
    /// Honour `Range` with a 206.
    Ranged,
    /// The right length, wrong bytes.
    Corrupt,
    /// 404.
    Missing,
    /// The whole body, already a gzip stream, announced as `Content-Encoding: gzip`.
    Gzipped,
}

/// Serves `body` over plain HTTP following `script`, one reply per connection; records the
/// `Range` header each request carried.
pub(crate) fn serve(body: Vec<u8>, script: Vec<Reply>) -> (String, Arc<Mutex<Vec<Option<String>>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/file.bin", listener.local_addr().unwrap());
    let ranges = Arc::new(Mutex::new(Vec::new()));
    let seen = ranges.clone();
    std::thread::spawn(move || {
        for reply in script {
            let Ok((mut stream, _)) = listener.accept() else { return };
            let mut range = None;
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                    break;
                }
                if let Some(v) = line.to_ascii_lowercase().strip_prefix("range: bytes=") {
                    range = v.trim().trim_end_matches('-').parse::<usize>().ok();
                }
            }
            seen.lock().unwrap().push(range.map(|r| r.to_string()));
            let len = body.len();
            let _ = match reply {
                Reply::Full => {
                    write!(stream, "HTTP/1.1 200 OK\r\nContent-Length: {len}\r\nConnection: close\r\n\r\n").and_then(|()| stream.write_all(&body))
                }
                Reply::DropAfter(n) => write!(stream, "HTTP/1.1 200 OK\r\nContent-Length: {len}\r\nConnection: close\r\n\r\n")
                    .and_then(|()| stream.write_all(&body[..n])),
                Reply::Corrupt => write!(stream, "HTTP/1.1 200 OK\r\nContent-Length: {len}\r\nConnection: close\r\n\r\n")
                    .and_then(|()| stream.write_all(&vec![b'x'; len])),
                Reply::Missing => write!(stream, "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"),
                Reply::Gzipped => write!(stream, "HTTP/1.1 200 OK\r\nContent-Length: {len}\r\nContent-Encoding: gzip\r\nConnection: close\r\n\r\n")
                    .and_then(|()| stream.write_all(&body)),
                Reply::Ranged => {
                    let from = range.unwrap_or(0);
                    write!(
                        stream,
                        "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {from}-{}/{len}\r\nConnection: close\r\n\r\n",
                        len - from,
                        len - 1
                    )
                    .and_then(|()| stream.write_all(&body[from..]))
                }
            };
        }
    });
    (url, ranges)
}

pub(crate) fn body() -> (Vec<u8>, String) {
    let body: Vec<u8> = (0..700_000u32).map(|i| (i % 251) as u8).collect();
    let sha = hex(&Sha256::digest(&body));
    (body, sha)
}

fn run(script: Vec<Reply>) -> (Result<Vec<u8>, String>, Vec<Option<String>>) {
    let (body, sha) = body();
    let (url, ranges) = serve(body.clone(), script);
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("file.bin");
    let http = Http::new(crate::TEST_AGENT).unwrap();
    let mut last = (0, 0);
    let result = http.download(&url, &dest, body.len() as u64, &sha, &Stop::default(), &mut |t| {
        if let Transfer::Bytes { done, total } = t {
            last = (done, total);
        }
    });
    let result = result.map(|()| std::fs::read(&dest).unwrap());
    if result.is_ok() {
        assert_eq!(last, (body.len() as u64, body.len() as u64));
    }
    let seen = ranges.lock().unwrap().clone();
    (result, seen)
}

#[test]
fn small_binary_files_are_read_whole_and_a_404_is_none() {
    let (body, _) = body();
    let http = Http::new(crate::TEST_AGENT).unwrap();
    let (url, _) = serve(body.clone(), vec![Reply::Full]);
    assert_eq!(http.get_bytes(&url, body.len()).unwrap(), Some(body.clone()), "a body of exactly the cap is fine");
    let (url, _) = serve(body, vec![Reply::Missing]);
    assert_eq!(http.get_bytes(&url, 1 << 20).unwrap(), None);
}

#[test]
fn a_file_over_the_cap_is_refused() {
    let (body, _) = body();
    let http = Http::new(crate::TEST_AGENT).unwrap();
    let (url, _) = serve(body.clone(), vec![Reply::Full]);
    let err = http.get_bytes(&url, body.len() - 1).unwrap_err();
    assert!(err.contains("more than"), "{err}");
}

#[test]
fn a_complete_body_of_exactly_the_published_size_succeeds() {
    // The bug this guards: a reader limit equal to the size failed on the final read.
    let (result, _) = run(vec![Reply::Full]);
    assert_eq!(result.unwrap(), body().0);
}

#[test]
fn a_dropped_connection_resumes_where_it_stopped() {
    let (result, ranges) = run(vec![Reply::DropAfter(300_000), Reply::Ranged]);
    assert_eq!(result.unwrap(), body().0);
    assert_eq!(ranges, [None, Some("300000".to_string())]);
}

#[test]
fn a_server_that_ignores_ranges_starts_over() {
    let (result, _) = run(vec![Reply::DropAfter(123_456), Reply::Full]);
    assert_eq!(result.unwrap(), body().0);
}

#[test]
fn corrupt_data_is_fetched_again_and_never_kept() {
    let (result, _) = run(vec![Reply::Corrupt, Reply::Full]);
    assert_eq!(result.unwrap(), body().0);
    let (result, _) = run(vec![Reply::Corrupt, Reply::Corrupt]);
    assert!(result.unwrap_err().contains("checksum"));
}

#[test]
fn pausing_keeps_the_partial_file_and_resuming_continues_it() {
    let (body, sha) = body();
    let len = body.len() as u64;
    let (url, ranges) = serve(body.clone(), vec![Reply::Full, Reply::Ranged]);
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("file.bin");
    let http = Http::new(crate::TEST_AGENT).unwrap();
    let stop = Stop::default();
    // Pause as soon as the first bytes arrive.
    let err = http
        .download(&url, &dest, len, &sha, &stop, &mut |t| {
            if matches!(t, Transfer::Bytes { done, .. } if done > 0) {
                stop.pause();
            }
        })
        .unwrap_err();
    assert_eq!(err, PAUSED);
    let kept = partial_bytes(&dest, len).unwrap();
    assert!(kept > 0 && kept < len, "{kept}");
    http.download(&url, &dest, len, &sha, &Stop::default(), &mut |_| {}).unwrap();
    assert_eq!(std::fs::read(&dest).unwrap(), body);
    assert_eq!(ranges.lock().unwrap()[1], Some(kept.to_string()));
}

#[test]
fn cancelling_throws_the_partial_file_away() {
    let (body, sha) = body();
    let len = body.len() as u64;
    let (url, _) = serve(body, vec![Reply::Full]);
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("file.bin");
    let stop = Stop::default();
    let err = Http::new(crate::TEST_AGENT)
        .unwrap()
        .download(&url, &dest, len, &sha, &stop, &mut |t| {
            if matches!(t, Transfer::Bytes { done, .. } if done > 0) {
                stop.cancel();
            }
        })
        .unwrap_err();
    assert_eq!(err, CANCELLED);
    assert_eq!(partial_bytes(&dest, len), None);
}

#[test]
fn a_missing_file_fails_at_once() {
    let (result, ranges) = run(vec![Reply::Missing, Reply::Full]);
    assert!(result.unwrap_err().contains("no longer"));
    assert_eq!(ranges.len(), 1);
}

#[test]
fn a_partial_file_from_an_earlier_run_is_continued() {
    let (body, sha) = body();
    let (url, ranges) = serve(body.clone(), vec![Reply::Ranged]);
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("file.bin");
    std::fs::write(part_path(&dest, body.len() as u64), &body[..500_000]).unwrap();
    assert_eq!(partial_bytes(&dest, body.len() as u64), Some(500_000));
    Http::new(crate::TEST_AGENT).unwrap().download(&url, &dest, body.len() as u64, &sha, &Stop::default(), &mut |_| {}).unwrap();
    assert_eq!(std::fs::read(&dest).unwrap(), body);
    assert_eq!(*ranges.lock().unwrap(), [Some("500000".to_string())]);
    assert!(!part_path(&dest, body.len() as u64).exists());
}

#[test]
fn an_oversized_partial_file_is_restarted_from_zero() {
    let (body, sha) = body();
    let (url, ranges) = serve(body.clone(), vec![Reply::Full]);
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("file.bin");
    let size = body.len() as u64;
    std::fs::write(part_path(&dest, size), vec![b'x'; body.len() + 1]).unwrap();
    Http::new(crate::TEST_AGENT).unwrap().download(&url, &dest, size, &sha, &Stop::default(), &mut |_| {}).unwrap();
    assert_eq!(std::fs::read(dest).unwrap(), body);
    assert_eq!(*ranges.lock().unwrap(), [None]);
}

#[test]
fn an_oversized_partial_file_that_cannot_be_removed_reports_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let part = part_path(&dir.path().join("file.bin"), 1);
    // A directory cannot be removed as a file on Unix. On Windows, hold a file open
    // without delete sharing: metadata remains readable, but removing it is refused.
    #[cfg(unix)]
    std::fs::create_dir(&part).unwrap();
    #[cfg(windows)]
    let _locked_partial = {
        use std::os::windows::fs::OpenOptionsExt;
        std::fs::write(&part, b"too large").unwrap();
        // FILE_SHARE_READ | FILE_SHARE_WRITE, without FILE_SHARE_DELETE.
        std::fs::File::options().read(true).share_mode(0x00000003).open(&part).unwrap()
    };
    assert!(std::fs::metadata(&part).unwrap().len() > 1);
    assert!(std::fs::remove_file(&part).is_err());
    let http = Http::new(crate::TEST_AGENT).unwrap();
    let stop = Stop::default();
    // Bound the regression: the old retry loop spins until Cancel, without a request.
    let (finished, done) = std::sync::mpsc::channel();
    let error = std::thread::scope(|scope| {
        let watchdog_stop = &stop;
        scope.spawn(move || {
            if done.recv_timeout(Duration::from_secs(10)).is_err() {
                watchdog_stop.cancel();
            }
        });
        let result = http.fetch_resuming("http://127.0.0.1:9/file.bin", &part, 1, &stop, &mut |_| {});
        let _ = finished.send(());
        result.unwrap_err()
    });
    assert!(error.contains("couldn't save the download"), "{error}");
    assert!(part.exists(), "a blocked partial file must be left alone");
}

#[test]
fn an_oversized_feed_answer_is_refused_before_parsing() {
    // More than MAX_SMALL (4 MiB) of decoded text: rejected outright, not truncated and parsed.
    let big = vec![b' '; (MAX_SMALL as usize) + 1024];
    let (url, _) = serve(big, vec![Reply::Full]);
    let http = Http::new(crate::TEST_AGENT).unwrap();
    let err = http.get_feed(&url).unwrap_err();
    assert!(err.why.contains("larger than"), "{}", err.why);
    // A small answer still comes back intact.
    let (url, _) = serve(b"{\"ok\":true}".to_vec(), vec![Reply::Full]);
    assert_eq!(http.get_feed(&url).unwrap(), "{\"ok\":true}");
}

#[test]
fn a_long_release_list_is_read_up_to_the_api_cap() {
    // SolidCraft's list of 100 releases was 6.5 MB, past MAX_SMALL (craft-launcher#1021).
    let long = vec![b' '; (MAX_SMALL as usize) + 1024];
    let (url, _) = serve(long.clone(), vec![Reply::Full]);
    let http = Http::new(crate::TEST_AGENT).unwrap();
    assert!(matches!(http.get_api(&url, None), Ok(Fetched::Body { text, .. }) if text.len() == long.len()));
    // Past MAX_API it is refused outright, not truncated and parsed.
    let (url, _) = serve(vec![b' '; (MAX_API as usize) + 1024], vec![Reply::Full]);
    let err = http.get_api(&url, None).unwrap_err();
    assert!(err.contains("larger than"), "{err}");
}

#[test]
fn a_gzip_bomb_answer_is_refused_before_parsing() {
    // A few hundred bytes on the wire that decode to more than MAX_SMALL. ureq's own limit
    // sits below the transparent gzip decoder, so only a cap on the decoded stream (CWE-409)
    // refuses this; a wire-byte limit would hand the parser truncated JSON.
    let big = vec![b'x'; (MAX_SMALL as usize) + 1024];
    let gz = gzip(&big);
    assert!(gz.len() * 4 < MAX_SMALL as usize, "no longer a bomb: {} compressed bytes", gz.len());
    let (url, _) = serve(gz, vec![Reply::Gzipped]);
    let http = Http::new(crate::TEST_AGENT).unwrap();
    let err = http.get_text(&url).unwrap_err();
    assert!(err.contains("larger than"), "{err}");
    // A small gzip answer still decodes intact.
    let (url, _) = serve(gzip(b"{\"ok\":true}"), vec![Reply::Gzipped]);
    assert_eq!(http.get_text(&url).unwrap().unwrap(), "{\"ok\":true}");
}

/// `data` as a real gzip stream (what `Reply::Gzipped` serves).
fn gzip(data: &[u8]) -> Vec<u8> {
    let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    enc.write_all(data).unwrap();
    enc.finish().unwrap()
}

/// Live: a real Craft App release asset (`cargo test -p artcraft-launcher-native -- --ignored`).
#[test]
#[ignore = "needs network access"]
fn downloads_a_real_release_asset() {
    let http = Http::new(crate::TEST_AGENT).unwrap();
    let Fetched::Body { text, .. } = http.get_api("https://api.github.com/repos/storytold/wordcraft/releases/latest", None).unwrap() else {
        panic!("no release")
    };
    let release = crate::release::parse_github_release(&text, "storytold/wordcraft").unwrap();
    let sums_url = &release.sums_asset().unwrap().url;
    let sums = crate::release::parse_sha256sums(&http.get_text(sums_url).unwrap().unwrap());
    let asset = release.assets.iter().find(|a| a.name.ends_with("freebsd-x86_64.tar.gz")).unwrap();
    let sha = asset.sha256.clone().or_else(|| sums.iter().find(|(_, n)| *n == asset.name).map(|(h, _)| h.clone())).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join(&asset.name);
    http.download(&asset.url, &dest, asset.size, &sha, &Stop::default(), &mut |_| {}).unwrap();
    assert_eq!(std::fs::metadata(&dest).unwrap().len(), asset.size);
}

/// Answers one request with `head` (status line and headers) and an empty body.
fn answer_once(head: String) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/repos/storytold/photocraft/releases/latest", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        let Ok((mut stream, _)) = listener.accept() else { return };
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                break;
            }
        }
        let _ = write!(stream, "{head}Content-Length: 0\r\nConnection: close\r\n\r\n");
    });
    url
}

#[test]
fn a_secondary_limit_waits_for_retry_after_not_the_hourly_reset() {
    // GitHub's secondary limit: quota left, an hourly reset 50 minutes away, and "try again
    // in a minute". That's a minute, not 50 minutes of "the hourly limit was reached".
    let now = crate::now();
    let url =
        answer_once(format!("HTTP/1.1 403 Forbidden\r\nx-ratelimit-remaining: 41\r\nx-ratelimit-reset: {}\r\nretry-after: 60\r\n", now + 50 * 60));
    let Fetched::RateLimited { reset: Some(until) } = Http::new(crate::TEST_AGENT).unwrap().get_api(&url, None).unwrap() else {
        panic!("not rate limited")
    };
    assert!((now + 60..now + 120).contains(&until), "limited for {} s", until - now);
}

#[test]
fn an_empty_hourly_quota_waits_for_the_reset() {
    let now = crate::now();
    let reset = now + 31 * 60;
    let url = answer_once(format!("HTTP/1.1 403 Forbidden\r\nx-ratelimit-remaining: 0\r\nx-ratelimit-reset: {reset}\r\n"));
    assert_eq!(Http::new(crate::TEST_AGENT).unwrap().get_api(&url, None).unwrap(), Fetched::RateLimited { reset: Some(reset) });
}

#[test]
fn github_refusals_are_read_the_way_github_documents_them() {
    const NOW: i64 = 1_791_500_000;
    // Out of quota: until the reset, or Retry-After if that's later.
    assert_eq!(refused_until(403, Some("0"), Some(NOW + 1800), None, NOW), Some(Some(NOW + 1800)));
    assert_eq!(refused_until(403, Some("0"), Some(NOW + 30), Some(90), NOW), Some(Some(NOW + 90)));
    assert_eq!(refused_until(403, Some("0"), None, None, NOW), Some(None));
    // A secondary limit: Retry-After, never the hourly reset.
    assert_eq!(refused_until(429, Some("12"), Some(NOW + 1800), Some(30), NOW), Some(Some(NOW + 30)));
    // A 429 that doesn't say: a minute.
    assert_eq!(refused_until(429, Some("12"), Some(NOW + 1800), None, NOW), Some(Some(NOW + 60)));
    // A plain 403 (blocked, private, …) isn't a rate limit.
    assert_eq!(refused_until(403, Some("12"), Some(NOW + 1800), None, NOW), None);
    assert_eq!(refused_until(403, None, None, None, NOW), None);
}

#[test]
fn partial_files_are_named_after_the_file_and_size() {
    assert_eq!(part_path(Path::new("/d/photocraft.dmg"), 9), Path::new("/d/photocraft.dmg.9.part"));
}

#[test]
fn the_remote_agents_refuse_plain_http_before_connecting() {
    // ureq checks every hop, redirects included, before it connects; `.invalid` never
    // resolves, so an agent that tried would fail on DNS instead.
    let http = Http::build(crate::TEST_AGENT).unwrap();
    let refused = |r: Result<_, String>| r.is_err_and(|e| e.contains("refused plain HTTP to example.invalid") && !e.contains("secret"));
    assert!(refused(http.get_api("http://example.invalid/repos/x/y/releases?secret", None).map(|_| ())));
    assert!(refused(http.get_text("http://example.invalid/SHA256SUMS.txt?secret").map(|_| ())));
    // The scheme is case-insensitive, and an upper-case one isn't a loopback link either.
    assert!(refused(http.get_text("HTTP://example.invalid/SHA256SUMS.txt").map(|_| ())));
    assert!(refused(http.get_bytes("http://example.invalid/icon.png", 100).map(|_| ())));
    assert!(refused(http.redirect_target("http://example.invalid/releases/latest").map(|_| ())));
    assert!(http.get_feed("http://example.invalid/feed.json").is_err_and(|e| e.why.contains("refused plain HTTP")));
    // A download fails at once rather than retrying until it pauses.
    let dir = tempfile::tempdir().unwrap();
    let (_, sha) = body();
    let result = http.download("http://example.invalid/file.bin?secret", &dir.path().join("file.bin"), 10, &sha, &Stop::default(), &mut |_| {});
    assert_eq!(result.unwrap_err(), "the download led to plain HTTP at example.invalid, so it wasn't fetched");
}

#[test]
fn https_only_still_reaches_a_loopback_link() {
    let (body, _) = body();
    let (url, _) = serve(body.clone(), vec![Reply::Full]);
    let http = Http::build(crate::TEST_AGENT).unwrap();
    assert_eq!(http.get_bytes(&url, body.len()).unwrap(), Some(body));
}

#[test]
fn get_bytes_rejects_oversized_payload() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        let Ok((mut stream, _)) = listener.accept() else { return };
        let mut line = String::new();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        while reader.read_line(&mut line).unwrap_or(0) > 0 && line != "\r\n" {
            line.clear();
        }
        let body = vec![b'A'; 200];
        let _ = write!(stream, "HTTP/1.1 200 OK\r\nContent-Length: 200\r\nConnection: close\r\n\r\n");
        let _ = stream.write_all(&body);
    });
    let http = Http::new(crate::TEST_AGENT).unwrap();
    let result = http.get_bytes(&format!("http://127.0.0.1:{port}/test.bin"), 100);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("more than 100 bytes"));
}
