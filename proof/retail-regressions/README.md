# Retail regression proof

This slice corrects three stale retail-only test expectations. Production
rendering, simulation, snapshot decoding and game assets are unchanged.

The full-session MP4 is an unbroken shared-compositor export from main
`089267c`, built through `scripts/proof_build.py` in an isolated target. It
contains every tick from 2 through 3000 at 15 fps, with synchronized retail
music, voices and combat audio. Tick 1 is the initial state, with no preceding
audio transition; it is retained locally with the raw frame export. The MP4
has 2999 frames at 1280x800 and lasts 199.933 seconds. This is scripted
headless playback, not a desktop screen recording or an original-game trace.

The demo queues GAPOWR and unit batches at tick 2, issues attack-move every
450 ticks while enemy structures remain, places GAPOWR at tick 560, queues
GAREFN at tick 1200 and places it at tick 2480. `placement.gif` retains ticks
500-619 (8 seconds); `assault.gif` retains ticks 1400-1579 (12 seconds), showing
combat and the active vehicle queue. GIFs are 640x400, 15 fps and 128 colors.
Frame timestamps in the full MP4 are `(tick - 2) / 15` seconds. These are
native demo timing choices, not calibrated original-game animation timing.

The fixed and old-overlay depth stills show the same actual retail shot at
tick 9. The old-overlay images reconstruct the superseded paint-after-world
policy; they are not screenshots of a separate old executable. JSON receipts
identify the state hashes. Trail stills at tick 41 retain the separate 78-pixel
building-occlusion check. The only mask adjustment excludes every nonzero
live-unit coverage value from building-only assertions.

`state-diff.json` compares legacy-profile seed-42 EB1 construction at
`76bdc0d` and `089267c` using the same local retail input. Diagnostic test
hooks wrote the two simulation JSON files outside Git; they are not published.
Exactly 44 scalar fields differ: 41 structure maximum HP values, one zero-map-
health structure's current HP, and two unit headings. Reverting only those
initialization assumptions reproduces the old golden hash exactly. The old
source build uses a separate Cargo target. Retained historical v1/v2 saves
stay outside Git; current tests check setup rejection, exact raw simulation
snapshot bytes and source-file preservation.

`baseline-test-failure.log` records the initial two app failures. Subsequent
workspace validation exposed the third failure in the retail infantry death
color expectation. Fixed verification logs and terminal recordings distinguish
those failures from the final passing suite. The death check uses sixteen
literal strong/weak channel samples, preserving all source-body, shadow,
facing, phase and expiry assertions without calling the remapper as its oracle.

The native macOS window entered a retail session, stayed live for 15 seconds
and initialized 48 kHz stereo CoreAudio. The smoke process was then terminated
for cleanup. Screen capture and video capture were skipped because their
Screen Recording permission blocks are documented. Known release-upload HTTP
401 was not retried; this is the authorized temporary proof-branch fallback.
All proof payloads are removed from the branch tip before squash merge. Links
pin this proof commit. No executable, save file or raw game asset is published.

The final isolated runtime receipt points to source `de0187c`. Its EB1 run
continues through tick 6000, reaches Lost at tick 5738 and ends with state hash
`3082cb602467f8fc`. The twenty final exports (five ticks, four tabs) are byte-
identical to the original `089267c` full-session export. Final native startup
also stays live for 15 seconds with 48 kHz stereo audio. One buffer underrun
notification occurs under concurrent validation load; the stream remains active.
This smoke check does not establish glitch-free device playback.

The required check sequence passes 670 Rust tests, zero failures and one
existing ignored native audio-device test. Clippy with warnings denied,
formatting and all three proof-helper tests pass. The separate final full
workspace run uses `--no-fail-fast` and the retained historical-save fixtures.
