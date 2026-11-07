# Save storage proof

Source commit `6f6d5b84390271f57315fccb3f80b6951f5554aa`, tree
`4aba73f5349ad44316f1075605f4b6b2f9dd8418`.

`terminal.typescript` and `terminal.timing` record an actual util-linux script
session from clean committed source. Replay with
`scriptreplay --log-out terminal.typescript --log-timing terminal.timing`.
`record.sh` holds the exact commands. `terminal.gif` and `terminal.png` render
recorded text; they are not gameplay or desktop screenshots. `before-fix.log`
contains the three ENAMETOOLONG failures from the previous storage algorithm.

Seven tests exercise the actual app-side storage function used by snapshot::save:
valid 255-byte ASCII and 254-byte Unicode basenames, non-UTF-8 Linux basenames,
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

`macos-fixture-rejection.txt` records why the non-UTF-8 case is Linux-only.
The initial macOS CI fixture failed on its own fs::write before calling app
storage: that filesystem rejects the name with EILSEQ. ASCII/Unicode and other
storage cases retain their actual macOS CI execution; the Linux byte-name test
is not represented as Mac support.
