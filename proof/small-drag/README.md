# Small-step right-drag proof

Source commit `026d4f0cc648b7693be71d3cdb98c1750cd8beae`, tree
`df92dc56964654e0393210bfb597d3260cd7b866`.

The four raw frames and state receipts under frames/ come from the clean
committed source's native pointer handlers and App::compose. They show a
synthetic colored grid with no session, entities or retail assets. They are
not live-window screenshots or gameplay captures. The selection identifier
and queued Stop command are synthetic input-state sentinels, not a retail unit.

record.sh records the commands in terminal.typescript and terminal.timing.
Replay with scriptreplay --log-out terminal.typescript --log-timing terminal.timing.
The tests export a press, three one-pixel diagonal steps, 24 such steps, and
release. Camera positions are (1000,1000), (997,997), (976,976) and (976,976).
verify_frames.py checks the state receipts and 204288 exactly aligned world
pixels after a (24,24) screen translation. comparison.png and camera.gif place
those actual frames beside the press reference with explicit synthetic labels.

Three regressions cover 27 layout/zoom/direction combinations, press-relative
jitter, out-and-back drags, bounds and invalidated releases. before-handler.log
records all three failing against the old handler with the new fixture.
No pre-fix images are published. workspace-check.log records all required
checks passing: 489 reported Rust passes, one existing ignored audio-device
test, strict Clippy/fmt and three Python provenance tests. Retail-dependent
tests self-skip because retail data is absent.

build-receipt.json applies to the isolated release app executable run with
--help. The handler/compositor tests use Cargo's debug test executable, whose
separate SHA256 is in the transcript. The release receipt does not attest that
test executable or establish live game input delivery. No executable is uploaded.

No retail gameplay, original-game runtime, physical Mac/Retina or audible
playback checks ran. Previously blocked Mac capture paths were skipped.
Release uploads returned HTTP 401 earlier in this same runtime; the authorized
branch-proof fallback is used without repeating them. Drop all proof files
before squash merge. Simulation and save schemas are unchanged.
