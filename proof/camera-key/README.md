# Camera key-down gesture cancellation

PR: https://github.com/johnnyhuy/openyr/pull/285
Issue: #284, within #97.
Tested source: bf407b66b2109f8f8c2dd0ad96faf4f926276edb
Tested tree: 7322e926cf9d95a000afa3dac903fbc418bf7644

`camera-key.gif` assembles four unmodified 640x480 fallback compositor frames at one frame per second:

| GIF position | Native dispatch state |
| --- | --- |
| 0 seconds | Existing left-drag box |
| 1 second | ArrowRight pressed; tracking cleared before any redraw handler |
| 2 seconds | Old Left release leaves selection and queued intent intact |
| 3 seconds | Fresh box after ArrowRight key-up |

These are ordered fixture frames, not measured runtime timestamps. JSON receipts accompany each frame. The matrix covers all four arrows, three sizes and four gestures (48 cases). The box and right-click/pan are started through real handlers; radar tracking is seeded because the fixture has no Session.

`terminal.txt` records the committed source/tree, debug test executable hash and three focused tests. The isolated release receipt covers only `--help` CLI smoke, not debug test frames. `workspace-check.log` records 528 reported Rust passes, one existing ignored audio-device test, strict clippy, fmt and three Python provenance tests. Retail tests that require absent data self-skip and are not retail passes.

No retail assets or original executable were used. The fixture has no Session, units or simulation. It verifies transient selected-ID/queued-intent sentinels and event ordering, not retail orders, save reconstruction, camera runtime timing or original parity. Known blocked original-game/macOS capture routes and release uploads were not retried. Physical Mac, Retina and audio checks did not run.

All proof is removed from the final branch tree before squash merge. The final source tree must equal the tested tree above.
