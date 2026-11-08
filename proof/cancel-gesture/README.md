# Pointer cancellation proof

Source commit `213d5b10328765f4e28b35bf81885882260ca77f`, tree
`b273010f19ca74c7ad8adcfb21bd3ca1199c7bf3`.

The four raw frames/state receipts under frames/ come from the clean committed
source's real pointer handlers and App::compose. The canvas has no Session,
entities or retail art. Selection id 17 and its queued Stop are synthetic state
sentinels, not a retail unit or a command validated against a live match.

record.sh records exact commands through util-linux script. Replay with
scriptreplay --log-out terminal.typescript --log-timing terminal.timing.
Three tests cover 15 size/right-route cases, world click/pan ownership, both
release orders, invalidation and fresh-input recovery. before-handler.log
records all three failing with the old handler; no pre-fix images are published.
workspace-check.log records the required checks passing: 492 reported Rust
passes, one existing ignored audio-device test, strict Clippy/fmt and three
Python provenance tests. Retail-dependent tests self-skip without data.

The capture shows an unfinished box, right press, right release and stale left
release. verify_frames.py checks that the band disappears at right press and
cannot reappear, while queued commands remain identical. Selection is preserved
at press and cleared at release by the existing normal right-click policy.
comparison.png and cancel.gif juxtapose these actual frames with the initial
box state and explicit synthetic labels. They are not native-window screenshots.

build-receipt.json attests the isolated release app executable run with --help.
The handler/compositor tests run in Cargo's debug test executable, separately
hashed in the transcript. The release receipt does not attest that test binary
or establish live game input delivery. No executable is uploaded.

No retail gameplay/save reconstruction, original-game runtime, physical Mac/
Retina or audible playback checks ran. Previously blocked Mac capture paths
were skipped. Known release-upload HTTP 401 is not retried; the authorized
branch-proof fallback drops all proof files before squash merge. Simulation
and save schemas are unchanged.
