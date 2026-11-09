# Base-navigation proof

Source commit e989266d52c02a16c2e29af2c0b04ddc1ca0d8e5, tested tree 52b7d4d71d2f93ff6456b607efde6e1619279354. Remove every
proof file and return to this tree before squash merge. Neither executable is
uploaded; hashes identify retained local copies.

The six PNGs use the real App compositor and native keyboard/pointer handlers
with public setup and authored Session/INI/map/SHP/TMP fixtures. Gray diamonds
are fallback entities, not retail artwork. Before frames precede the tested
right-pan gesture; after frames show H centering the construction yard and
cancelling that gesture. Selection and pending Stop remain unchanged.

The GIF preserves the 640x480 frames pixel for pixel at simulation tick 3.
Its 1400 ms duration is presentation timing, not game timing. PNGs also cover
800x600 and 1280x800. JSON sidecars record selected IDs, pending commands, tick,
simulation hash and encoded Session size. No retail cursor or artwork is used.

## Validation

```
source /workspace/openyr-env.sh
export CARGO_TARGET_DIR=$PWD/.tmp/cargo-target CARGO_BUILD_JOBS=2
mise run check
```

Passed `cargo test --workspace --locked` (588 Rust passes, 0 failures, 1 existing
native audio-device test ignored), `cargo clippy --workspace --all-targets
--locked -- -D warnings`, `cargo fmt --all --check` and three Python provenance
tests. Local socket access was enabled for CLI regressions. Retail-labelled
tests return early without data; their reported passes are not retail checks.

```
CARGO_TARGET_DIR=$PWD/.tmp/cargo-target CARGO_BUILD_JOBS=2 cargo test -p yr-app --bin openyr --locked --no-run --message-format=json
OPENYR_WORLD_FEEDBACK_PROOF=$PWD/.tmp/base-proof-final <recorded executable> hover_feedback::tests::ownership::base_navigation:: --nocapture
CARGO_BUILD_JOBS=2 python3 scripts/proof_build.py build --output .tmp/base-runtime
python3 scripts/proof_build.py verify --receipt .tmp/base-runtime/receipt.json
python3 scripts/proof_build.py run --receipt .tmp/base-runtime/receipt.json -- --help
```

Four targeted tests pass: nine elevated-ground layout/zoom cases; bounded and
oversized viewports; multiple, destroyed and unavailable owned yards; 15
mode/layout cases with old-pointer cleanup; repeat/focus/synthetic/modifier/
outcome/missing-data guards; Save text; and actual Ctrl+H Harvest commands.
Complete encoded Session bytes, selected IDs, valid pending orders and zoom
follow their existing policies. H preserves armed command modes; Ctrl+H retains
the old Harvest cleanup and command behavior.

`test-executable.json` records the private-target debug executable used for the
compositor proof. `receipt.json` records the isolated release OpenYR executable,
verified through `--help` in `runtime.typescript`; the release smoke does not
execute the visual fixture.

## Source and limits

The [public RA2 manual, Basic Keyboard Functions, printed page 2](https://static.wikia.nocookie.net/cnc_gamepedia_en/images/3/3b/CCUC-RA2-pcMANOLeUS.pdf/revision/latest)
maps H to Center tactical map on base. This intentionally moves the earlier
native Harvest binding to Ctrl+H. Construction-yard choice, no alternate
building fallback/cycling, notice timing/wording, bounds and Ctrl+H remain
native policies pending original comparison under #97.

Retail data is absent at the cloud and standard Mac paths. No retail archive
scan, original-game runtime/timing, retail sprite/cursor pixels, physical
Mac/Retina or audio-device check ran. Known permission-blocked original/Mac
captures and HTTP 401 release uploads were not retried. This uses the authorized
temporary proof-branch fallback. #97/#98 remain open for their broader criteria.
