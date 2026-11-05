# craft-timeline

Timelines for Craft apps, from FilmCraft (Premiere Pro), EffectCraft (After Effects) and SoundCraft (Pro Tools).

The core is egui-free:

- `time`: `Tick` (254,016,000,000 per second, exact for every broadcast frame rate and common sample rate), `TimeRange`, and `TimeView` (the time-to-pixel map with zoom and scroll).
- `rates`: `FrameRate` (rational), `SampleRate`, `PixelAspect` and their menu presets.
- `timecode`: SMPTE timecode (drop and non-drop), frames, feet + frames, samples and seconds, formatted and parsed the way Premiere's fields do.
- `clock`: `PlaybackClock` (play, stop, preroll, loop, stop point, an external audio clock) and `Follow` (the view follows the playhead while playing).
- `cache` and `frames`: a byte-budgeted `FrameCache` that spares the frames a playing preview reaches next, and a `FrameServer` that renders frames on worker threads by priority with playback prefetch (on wasm, `pump` runs them on the UI thread).
- `clips`: clip edges, trims and snapping rules; `markers`: timeline markers; `model`: the `TimelineModel` trait the app implements on its sequence.

With the `ui` feature, `craft_timeline::ui` has the `Timeline` panel (track headers, rows, playhead), `TimelineRuler`, `TimelineScrollBar`, `ClipLane`, `KeyframeLane` and `KeyframeNavigator`, `GraphEditor`, `TransportBar` and `MediaControls`, `TimecodeField` and `TimecodePanel`, `FramePreview`, `MarkerStrip` and `MarkerDialog`, `TimelineOverview` and `RenderQueueList`. Edits go out as events; the app applies them to its document and undo stack.

Stories (in `craft::story::all()` with the `timeline` and `ui` features): `patterns/timeline/panel`, `composites/timeline-ruler/ruler`, `composites/keyframe-lane/lane`, `patterns/graph-editor/editor`, `composites/transport-bar/bar`, `composites/media-controls/controls`, `controls/timecode-field/field`, `patterns/timecode-panel/panel`, `composites/frame-preview/preview`, `patterns/marker-dialog/dialog`, `composites/timeline-overview/overview` and `patterns/render-queue-list/list`. `ui::WORDS` lists the labels tables and `ui::COUNTS` the count words (`renderQueue.seconds`) for the facade's catalog extraction.
