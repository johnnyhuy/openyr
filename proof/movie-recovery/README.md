# Menu movie failure recovery

Issue #288, within #96/#97.
Source: a10401463a8a1de2e1ab31f1c0d9a5d868320ef1
Tree: 10f155ba58f900f4480d0d501594e11bfb6f3005

`movie-recovery.gif` assembles four unmodified 640x480 fallback compositor frames at one frame per second:

| GIF position | Native state |
| --- | --- |
| 0 seconds | Synthetic preview load in progress |
| 1 second | Actual movie decoder error; normal modal entry cancels the load |
| 2 seconds | Late successful preview/progress ignored; same error remains |
| 3 seconds | Deliberate Enter returns to Main |

Positions are fixture frame order, not measured runtime timestamps. JSON receipts distinguish worker cancellation, preview ownership and page state. The error comes from the real Movie worker decoding the existing 56-byte synthetic malformed BIKi fixture. Loader completion/progress use controlled channels, without retail assets or a Session.

`terminal.txt` records committed source/tree, debug test executable SHA256 and both focused tests. They cover late preview success, late load error and progress at three sizes, plus queued retry cancellation and input cleanup. The isolated release receipt covers only CLI `--help`, not these debug test frames. `workspace-check.log` records 533 reported Rust passes, one existing ignored audio-device test, strict clippy, formatting and three Python provenance tests. Tests requiring absent retail data self-skip; these are not retail passes.

This does not verify successful retail movie playback, actual match reconstruction, audio, physical Mac/Retina delivery, original error UI parity or timing. No original executable or process memory was used. The change retains the existing return-to-Main policy and leaves movie decoding/scheduling, simulation and save formats unchanged.

Known permission-blocked original-game/macOS capture paths and release uploads were not retried. Proof uses the authorized temporary branch fallback and is removed before squash merge. The final source tree must match the tested tree above.
