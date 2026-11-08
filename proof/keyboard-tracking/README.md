Menu keyboard interruption proof for #274 / #275

Source 7475d87ee6d2c35b005571b5bd6d41b551bcd78e, tree 476068e70650095730749dd09ddbc7e1e0ecff38. At 0s Confirm is held; at 1s Tab moves focus to Keep current and clears the pending press; at 2s the stale release leaves the confirmation and temporary file intact. These are actual fallback-compositor outputs from debug keyboard/pointer dispatch tests, without a Session or retail art. The JSON receipts record file existence, pending press and focus. The tests also read the original file bytes and verify a fresh deliberate click can still delete the fixture.

The terminal recording identifies the debug handler binary and records three passing regressions. Tests cover temporary-file preservation in both shells and three sizes, interrupted/fresh credit and unit sliders, map-thumb dragging, popup keyboard navigation and synthetic/repeat guards. The pre-change regression fails because the stale release actually deletes the temporary file. It contains arbitrary fixture bytes, not a retail snapshot.

Full mise run check passes 513 reported Rust tests, one ignored, strict Clippy, formatting and three provenance tests. Local socket access was enabled for the existing UnixStream fixture. Retail-dependent cases self-skip. The isolated release receipt covers the separate --help smoke and executable, not these debug frames.

No live game-window, retail reconstruction/archive scan, physical Mac/Retina/audio or original-game tracking calibration ran. Known blocked native Mac/original reference capture and HTTP401 release-upload paths were not retried. All proof must be removed before squash merge.
