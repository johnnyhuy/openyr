# World feedback follows pointer ownership

These are actual shared-compositor frames from the native App handler test at
source commit 0bededaa976ea441fb65a13e11f9223351ff9d1f, source tree
a5cb025a47d4ef3d7f1657e2b9fa787d09b85a81. The Session is built through
public session::setup from authored in-memory SHP/TMP/INI/map fixtures. No
retail data, retail cursor art, original implementation or process memory was
used. Entity diamonds and the solid map canvas are synthetic fallback drawing.

| Frame | Native input/state | Cursor Kind receipt | Hover |
|---|---|---|---|
| 00 | Armed owned unit selected over enemy | Attack | Enemy brackets |
| 01 | ArrowRight pressed before redraw | Pointer | Suppressed |
| 02 | Left press/release while arrow remains held | Pointer | Suppressed, no intent |
| 03 | ArrowRight released | Attack | Enemy brackets return |
| 04 | Fresh Left press/release | Attack | One Attack intent queued |

All five state receipts retain simulation tick 3, hash 30dbf338b80849ca,
89747 encoded Session bytes and selected unit 6. The regression asserts the
complete encoded bytes are equal, beyond the receipt's byte count. Frame 04
queues Attack from unit 6 to enemy 7 without stepping the simulation. The GIF
shows the five exported states at one frame per second; this is presentation
ordering, not runtime timing measurement. JSON reports the native cursor Kind;
no retail cursor sheet is supplied and the PNGs do not verify cursor pixels.

The full native tests also cover 108 arrow/layout/action combinations, actual
Shift selection, Attack/Move/AttackMove/Harvest/Sell intent recovery, invalid
repair, a genuinely produced ready structure, opposing arrows, right press
before pan activation, active right pan, sidebar/menu routes and focus recovery.
Held gestures preserve queued intents, modes, complete Session bytes and the
simulation hash. Existing committed attack-target brackets remain visible.

The debug native test executable is identified separately in
native-test-receipt.json. Its capture command was:

```sh
OPENYR_WORLD_FEEDBACK_PROOF="$PWD/.tmp/world-feedback-proof" \
  .tmp/cargo-target/debug/deps/openyr-8539b05c03c1d83a \
  --exact hover_feedback::tests::ownership::world_feedback_ownership_matches_ignored_keyboard_pan_clicks \
  --nocapture
```

native-terminal.txt records this debug test through script. final-check.log
records `mise run check`, comprising:

```sh
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all --check
python3 -m unittest discover -s scripts -p 'test_*.py' -v
```

All passed: 561 Rust tests passed, one pre-existing ignored test, no failures;
three Python proof tests passed. Retail-dependent checks self-skip when the
install is absent and are not retail passes.

The separate isolated release receipt and CLI terminal capture establish the
clean committed release executable's provenance and help dispatch only. They
are not evidence that the release CLI exercised these native pointer handlers.

```sh
python3 scripts/proof_build.py build --output .tmp/world-feedback-runtime
python3 scripts/proof_build.py verify --receipt .tmp/world-feedback-runtime/receipt.json
python3 scripts/proof_build.py run --receipt .tmp/world-feedback-runtime/receipt.json -- --help
```

Retail reconstruction, original cursor mappings/hotspots/cadence, physical
macOS/Retina input and audio were not verified. Previously permission-blocked
Mac/original captures and HTTP401 release uploads were not retried. This uses
the authorized pinned branch-proof fallback; every proof file is removed from
the final branch tree before any squash merge.
