Repair/sell pointer tracking proof for #282 / #283

Source 4ad36b2bca06c1ad81420f71a55c3710d381d84f, tree 77dd774a0686e2d7b4a621370ac388cec1b79bac. Actual shared fallback-compositor sequence at 640x480: 0s has an active right pan; 1s K arms repair and clears pan tracking; 2s another K returns to ordinary input and a fresh left drag draws its box; 3s the old right release leaves that box visible. The first two canvas frames have no box; JSON receipts distinguish pan/mode state. These are native handler calls on a synthetic canvas without a Session or retail art.

The pre-change regression fails because the fresh box is invisible. Three passing tests cover the real pointer/key/composer sequence for K and L, 72 mode/gesture/layout combinations, inert stale releases, retained selection/queued intents, repeat/synthetic/focus/menu guards and fresh actions. The terminal recording identifies the debug handler executable.

Full mise run check passes 525 reported Rust tests, one ignored, strict Clippy, formatting and three provenance tests. Local socket permissions were enabled for the existing UnixStream fixture. Retail-dependent cases self-skip. The separate isolated release receipt covers its executable and --help smoke, not these debug frames.

No retail repair/sell commands, hover artwork, Session/save reconstruction, archive scan, live game-window, physical Mac/Retina/audio or original-game tracking calibration ran. Known blocked native Mac/original reference capture and HTTP401 release-upload paths were not retried. Remove all proof before squash merge.
