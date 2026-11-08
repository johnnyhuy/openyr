Synthetic menu wheel proof for #268 / #269

Source 0276ce104ee226670afd9b0f94de0a729b979c99, tree ba30811a487ad0e348aa3d89b77529fa28732e8a. Four fallback compositor frames: 0s starts at row4; 1s after four -10 pixel events remains row4; 2s the fifth event moves to row5; 3s a -3.5 line event moves to row8 with a half-row remainder. These are actual debug handler/compositor outputs with no Session or retail data. The terminal recording identifies the debug test binary hash. The isolated release receipt covers a separate --help CLI smoke, not these debug frames.

Validation: cargo test --workspace --locked reports 502 passed and one ignored with local socket access enabled. The initial mise run check failed only because its existing Unix-socket fixture received PermissionDenied under the default sandbox; formatting, strict Clippy and three provenance tests passed in that run. The successful workspace rerun is recorded separately. Retail-dependent tests self-skip. Three menu-wheel regressions pass; the prior code fails the first pixel event. Physical Mac/Retina/audio, live game-window, retail reconstruction and original-game wheel calibration did not run. Known blocked native Mac/original reference capture and release upload paths were not retried.

No original art is present in these fallback frames. This proof directory must be removed before squash merge.
