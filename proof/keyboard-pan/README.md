# Held-key camera pointer ownership

Source: ddd2531553a169d69fcb0bf02fe9edb18dc5adcd
Tree: 7d0833a6c1b4fe8d3719c6eae0edaf9913eadd4c
Issue #286, within #97.

`keyboard-pan.gif` assembles four unmodified 640x480 shared-compositor frames at one frame per second:

| GIF position | Native dispatch state |
| --- | --- |
| 0 seconds | ArrowRight held |
| 1 second | New Left press/motion ignored; no box armed |
| 2 seconds | Ignored Left release and ArrowRight key-up preserve selection |
| 3 seconds | Fresh Left press/motion draws a box |

Positions are fixture frame order, not measured runtime timestamps. First three images are identical; their JSON input receipts distinguish the states. No redraw handler runs during this sequence.

`terminal.txt` records source/tree, the debug test executable SHA256 and three focused tests, including 60 arrow/layout/mode cases and opposing-key/sidebar/menu routes. The isolated release receipt covers only CLI `--help`, not these debug test frames. `workspace-check.log` records 531 reported Rust passes, one existing ignored audio-device test, strict clippy, formatting and three Python provenance tests. Retail tests needing absent data self-skip, and are not retail passes.

The fixture has no Session, entities, retail art or original executable. Selection and queued-intent sentinels verify transient UI preservation, not actual retail commands or reconstructed saves. Camera timing/speed/bounds, simulation and save formats are unchanged. Physical Mac/Retina input, audio and original-game timing/pixel parity were not verified. Sidebar input routing is checked, not retail production execution.

Known permission-blocked original-game/macOS capture paths and release uploads were not retried. Proof uses the authorized temporary branch fallback and is removed before squash merge. Final source tree must match the tested tree above.
