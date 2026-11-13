# Alt/Option world-input proof

Before: native Linux Xorg on main b4a835d opens the quickload replacement
confirmation after Alt+F9. The reused isolated before-build receipt records
source 07c6326; its tree is identical to main b4a835d. The proof runner verified
that equality before this capture. The first Alt+F9 outcome is the before
comparison; subsequent baseline menu navigation is not a plain-F9 control.

After: source 1d2167aee222daedb3f71cc7e3978c07b15e9808 leaves the live match
visible after Alt+F9. The after recording then selects/moves units and verifies
that ordinary F9 still opens quickload. Capture metadata and scripts retain
actual event timestamps. These images establish routing, not original pixel
parity or playback cadence.

The recordings use synthetic XTest input and are silent because no audio device
is available. Physical Mac/Option, Retina and original runtime did not run.
Known permission-blocked Mac/original capture paths were not retried. Xorg,
the native app and FFmpeg run in one task session with Unix socket access.

Final source passes 685 retail-backed Rust tests with one existing audio-device
test ignored, strict Clippy/fmt, three Python proof tests and all 16 retail MIX
archives with zero parser errors. The final build reaches tick 6000 at hash
3082cb602467f8fc. Its fresh and restored saves are byte-identical to the merged
main baseline. No original executable, native binary, save or retail archive is
part of this proof; only derived media and verification records are retained.
