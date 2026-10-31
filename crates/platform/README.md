# craft-platform

Small platform lifecycle services for Craft applications. Application state, settings, palettes,
document policy and native command routing remain in their current owners. Defaults add no
third-party dependencies. Enable only the services a host uses.

| Feature | Shared responsibility |
| --- | --- |
| default | Typed dialog outcomes; no backend or toolkit. |
| `dialogs` | Reserve waiting workers before eager UI-thread native picker creation; serial requests, failure delivery and completion wakeups. The backend factory belongs to the app. |
| `theme` | `From<appearance::Reading> for craft_theme::SystemAppearance`, so a host hands the OS answer to `Base::resolve`. |
| `ui` | Combine with `dialogs` for the deferred-close guard or with `appearance` for native theme mapping. |
| `pointer` | Move the system pointer (`pointer::warp`), installed by the app as craft-ui's `Scrub::install_warp` service for a scrub's screen-edge wrap; macOS only, `false` elsewhere. |
| `appearance` | Linux/BSD XDG desktop preference observation (colour scheme, contrast, reduced motion, button layout); browser contrast and motion media queries; owned cancellable subscriptions, bounded reads/helper capture, legacy compatibility and reconnect. |
| `browser` | Wasm download link/object URL ownership, cleanup on setup failure or abandonment, and cleanup timer registration before clicking. |
| `clock` | `clock::Clock`: monotonic seconds (`web-time`, so also in the browser) for animation, autosave and deadlines, with a manual mode tests set and advance; `monotonic_ms`, `unix_ms`. |
| `pacing` | `FrameCap` (desktop frame-rate cap without vsync), `WindowState` (shown or hidden from the viewport info), `WorkBudget`/`WorkSlice` (background work per call, paused or sliced while hidden), `repaint_in`, `paint_while_hidden`. Adds `clock` and egui. |
| `logs` | The app log: `log` records to standard error and `<dir>/<app>.log` with `RUST_LOG`-style filters, rotation to `.1`/`.2`, per-instance logs while another instance runs, a size cap; `logs::crash`: the panic hook, crash log, last panic for the error window, and `catch`. |
| `session` | `session::Session` and `SessionKeeper`: the documents open at the last quit and where the user was in each, stored in the app's settings (serde) for a "Reopen Last Session" preference; `to_reopen` skips what is already open; a stored list longer than 64 documents is cut. No egui. |
| `usage` | `usage::sample`: the process's resident memory and the memory it may still use (`MemorySample`, `None` when unknown), for the status readout's Scratch and Efficiency items. Linux reads `/proc`; other platforms return `None`. |
| `open` | `open::FileIntake` with `IntakeSender` (the app's Apple Events and second-instance hand-over feed it): one queue for files the OS asks the app to open and files dropped on the window (`DroppedFile`, `DropTarget`: canvas, tab strip or window); `natural_cmp` and `same_file` for opening order and duplicates; `reveal` and `open_external`; the shared open-error texts. No egui. |
| `autosave` | `autosave::Autosave` (the timer: which documents are due, results by revision), `recovery_dir`, `adopt_legacy_recovery` and `RecoveryStore`: the recovery folder of copies with a JSON sidecar each (`RecoveryEntry`, the fields craft-ui's `RecoveryDialog` lists), `AUTOSAVE_FAILED`. The app serializes its documents. |
| `screen` | Screen capture for the screen colour picker (`craft_platform::screen`). |


## Native dialogs

`dialogs::wait_prepared(create, deliver)` reserves the worker first. Call it on the native UI thread.
`create` must call `pick_file`, `save_file`, or the equivalent backend method **before returning its
future**. Putting that call inside an async block delays native sheet creation until the worker
polls it. Worker reservation failure never opens the sheet. No live picker has a timeout.

`NativeDialogs<Request, Answer>` provides an optional serial queue. Pump it on the event-loop
thread before consuming answers and after code that can request a picker. Replies become visible
only after the pump releases transport ownership, so a continuation can enqueue the next picker.
Dropping a stale reply does not release the actual picker. A host-specific preflight is optional;
it runs off-thread and must be bounded. Unknown capability permits a picker. Proven absence may
request an application-owned fallback. Custom save-format sheets and portal choices remain intact.

`Outcome::Selected`, explicit `Cancelled`, `Failed` and `Indeterminate` have different evidence.
An rfd `None` is `Indeterminate`; it cannot prove user cancellation rather than backend failure.
Disconnected workers are failures. Apps own localization, document identity/revision checks, filters,
actual save receipts, chained continuations and the decision to close.

Retain the actual parent window through queued requests, preflight, the native picker AND any
unconsumed application continuation. `DeferredClose::update` cancels close during pending work and
returns a request to retry the application's normal close policy afterward. It never grants
permission to discard work. Run those guards before `craft_quit::on_close_request` in `App::logic`.

## Desktop appearance

`appearance::Watcher` caches the desktop preference and wakes only on changes. Subscription precedes
the first read. ReadOne/legacy Read, the standard namespace and GNOME fallback are handled together;
on GNOME the "default" (Light) setting and portal value 0 read as light; elsewhere 0 is no preference.
Malformed values retain the last valid answer, and explicit no-preference clears it.
`SystemReader::read()` gives colour scheme, contrast and reduced motion in one call on every target: the watcher on Linux/BSD, the browser media queries on the web (scheme from egui's system theme through `read_in(&ctx)`), no answers elsewhere. `SystemReader::for_egui()` repaints the context on a change, and `read_in(&ctx)` also runs `sync_native_window_theme`. `egui_service()` is the scheme-only callback over the same reader.

`Watcher::wait_first(timeout)` blocks until the first answer, a known absence, or the timeout (about 250 ms in an eframe creator), so the first frame uses the desktop theme. Disconnection
retains the last valid value until a fresh answer. A later service owner triggers a reread; session
reconnection has bounded backoff. Dropping cancels asynchronous work and waits at most 100 ms
for acknowledgement, joining only an already-finished worker. A synchronously blocked OS call or caller wake callback can outlive
the watcher: that exceptional worker is detached and retains its resources until it returns,
possibly until process exit. Wake callbacks must be nonblocking. No per-helper reader threads
are created.

App-selected Light/Dark/System, custom palettes, window decorations and unknown-theme fallback remain
application policy. `set_active(false)` suppresses wake notifications while continuing to cache.
Four existing custom-palette service hooks retain their current signatures and receive change-only
notifications even in manual mode; they do not change the chosen palette. Never infer Auto from the
egui theme preference when an app implements Auto by explicitly applying a Light/Dark palette.

`Watcher::button_layout()` is GNOME's `org.gnome.desktop.wm.preferences` `button-layout`, read and
followed on the same portal connection (ReadOne, then legacy Read; `gsettings` once when the portal
has no answer). It is the string as the desktop reports it, `None` until one arrives, and a change
runs the same wake callback. Malformed values keep the last one. `wait_first` does not wait for it.
`craft_window::policy::LayoutWatch` turns it into the caption layout; craft-window takes the string,
so it needs no D-Bus dependency.

`sync_native_window_theme(ctx, observed)` forwards the cached Linux/BSD desktop answer to native
decorations when egui's window policy is System, falling back to the current toolkit answer.
Its output hook runs after egui's automatic
window synchronization, including child-window output. Concrete Light/Dark preferences retain
precedence, absence of both answers releases the native pin, and `sync_window_theme = false` opts out.
It does not change styles or theme preference. Hosts whose custom palette is manual but whose
native window follows System keep observation active for the native window. Native decoration
rendering still depends on the window manager and requires visible acceptance testing.

### Contrast and reduced motion

`Watcher::contrast()` (`Contrast::Standard` or `High`) and `Watcher::reduced_motion()` follow the
portal's `org.freedesktop.appearance` `contrast` and `reduced-motion` keys. Without a
`reduced-motion` key, GNOME's `org.gnome.desktop.interface` `enable-animations` stands in. Both
are read beside the theme, so `wait_first` returns with them, and a change runs the same wake
callback. `None` means no answer yet or no such key; the `gsettings` fallback doesn't read them.
macOS and Windows have no watcher (their settings need platform bindings, and the crate forbids
`unsafe`). On the web, `browser_contrast()` and `browser_reduced_motion()` evaluate the
`prefers-contrast: more` and `prefers-reduced-motion: reduce` media queries on each call, with
no change notification.

## Browser downloads

`PreparedDownload::{bytes,blob}` owns a detached anchor and URL until click or drop. Call `click`
with the application's positive URL lifetime; setup/attachment/timer failure releases resources
without clicking. A successful return means a click was issued, not that the browser saved a file.
Applications own names, MIME types, pacing, capture hooks and user activation. File System Access
and print iframe lifetimes remain separate. This service has no native filesystem semantics.

## Integration

Enable only the services a host uses. Applications retain palettes, settings, document identity,
close policy, filters, names, MIME types and native command routing. Existing craft-settings,
craft-atomic, craft-print, craft-docpath and craft-quit remain canonical. `craft-window` owns
decoration selection and window-button layout; appearance observation and native color hints here
do not choose a frame or draw an application header.
