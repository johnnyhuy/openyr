World box-selection proof for #278 / #279

Source 031a7b6a0d4eacc5075a16dfae7ef775ab9be102, tree cd46ddc074c9937efb79a70bf68dff9b094e2822. Actual shared fallback compositor at 640x480: at 0s the drag ends over the sidebar but the rectangle stops at x471, leaving x472..639 untouched; at 1s it follows the pointer back into the world; at 2s release removes it. The canvas is synthetic, without a Session or retail artwork. Compositor tests check every sidebar pixel against the unselected baseline at three framebuffer sizes. The pre-change regression fails at sidebar row120.

The shared production picker is separately tested against a synthetic Sim with an owned visible unit, owned UI-hidden unit and visible enemy. Nine size/zoom cases, reversed and ordinary in-world rectangles pick only the visible owned ID. Full Sim snapshot bytes and state hash remain identical. sim-picks.json records the results. Geometry tests cover invalid, non-finite, empty, degenerate and fractional half-open viewport boundaries. This verifies the picker component, not a complete retail Session/save.

The terminal recording identifies the debug handler binary and records three passing tests. Full mise run check passes 520 reported Rust tests, one ignored, strict Clippy, formatting and three provenance tests. Local socket permissions were enabled for the existing UnixStream fixture. Retail-dependent cases self-skip. The separate isolated release receipt covers the --help smoke and executable, not the debug frames.

No retail reconstruction/archive scan, live game-window, physical Mac/Retina/audio or original-game selection calibration ran. Known blocked native Mac/original reference capture and HTTP401 release-upload paths were not retried. Remove all proof before squash merge.
