# craft-automation

Drive a running Craft app from tests, agents and the CLI. Native only (no wasm target).

| Module | What it gives the app |
| --- | --- |
| `args` | `ControlOptions`: the `--control <PORT>`, `--control-token`, `--control-token-file` values and their `<APP>_CONTROL_*` environment fallbacks, checked the way the apps' command lines report them (usage errors exit 2). |
| `security` | Tokens (`generate_token`, `validate_token`, `server_token`, `client_token`), the `auth` handshake, bounded line reads, connection and request limits. |
| `protocol` | The JSON-lines wire format: one `{"id", "method", "params"}` request per line, one `{"id", "ok", "result" | "error"}` reply per line; `Params` validators that name the bad parameter. |
| `control` | `ControlRequest`, `Outcome`, `Dispatch` and `ControlChannel`: the UI-thread side that drains requests between frames, replies after synthetic input or a background job settles, and admits screenshots. The app supplies the method table. |
| `synthetic` | `SyntheticInput`: `ui.click`, `ui.move`, `ui.key` and `ui.type` as egui events injected one step per pass. |
| `server` (`tcp`) | `start`: the loopback TCP listener that authenticates each connection and forwards requests to the channel. |
| `client` (`tcp`) | `ControlClient`: connect, authenticate, call, screenshot; for integration tests and CLIs. |
| `screenshot` | `Captures`: the bounded, cancellable screenshot queue over `craft_ui::capture::CaptureSlot`, with settling, geometry epochs, retries and the PNG reply. |
| `profiler` | `FrameStages` and `SlowFrames`: where a frame's time went, the slow ones kept for the control channel's `perf` output and one log line a second. |
| `trace` | `FrameTrace`: every frame of a gesture (a brush stroke, a drag) as a row, a percentile summary per gesture, optional CSV. |

The app owns its methods (`engine.execute`, `ui.inspect`, `ui.set`, …): `ControlChannel::next_request` hands it each request and `ControlChannel::settle` takes the `Outcome`. The library owns everything a method table does not: deadlines, dispatch accounting, waiters, the screenshot budget, synthetic input and the transport.
