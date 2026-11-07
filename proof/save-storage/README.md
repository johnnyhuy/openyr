# Save storage proof

Source commit `74e21a4cb7abebf9415af520d3e05afdbecf4e13`, tree
`95e6b421620cdda9e55e335434440a093112ba28`.

`terminal.typescript` and `terminal.timing` record an actual util-linux script
session from clean committed source. Replay with
`scriptreplay --log-out terminal.typescript --log-timing terminal.timing`.
`record.sh` holds the exact commands. `terminal.gif` and `terminal.png` render
recorded text; they are not gameplay or desktop screenshots. `before-fix.log`
contains the three ENAMETOOLONG failures from the previous storage algorithm.

Seven tests exercise the actual app-side storage function used by snapshot::save:
valid 255-byte ASCII and 254-byte Unicode basenames, non-UTF-8 Unix basenames,
exclusive candidate collisions/exhaustion, occupied symlinks, failed-rename
cleanup and destination-name exclusion. Files are synthetic test byte sequences,
not retail match saves. The tests verify exact replacement bytes and new-file
creation. Encoding/reconstruction and simulation were not modified.

`workspace-check.log` records the required checks passing: 486 reported Rust
passes, one existing ignored audio-device test, strict Clippy/fmt and three
Python provenance tests. Retail-dependent tests self-skip without game data.

`build-receipt.json` attests the isolated release app executable run with
`--help`. The terminal storage tests use Cargo's debug test executable; its
separate SHA256 is recorded in the transcript. The release receipt does not
attest that test executable or prove live menu/game delivery. No executable,
generated save or retail asset is uploaded.

No retail gameplay, original-game runtime, physical Mac/Retina or audible
playback checks ran. Previously blocked macOS capture paths were skipped.
Release asset uploads returned HTTP 401 in the preceding run in this same
runtime; the authorized branch-proof fallback is used without retrying them.
All proof files are removed from the final branch tree before squash merge.
