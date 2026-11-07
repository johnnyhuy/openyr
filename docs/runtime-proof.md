# Runtime proof builds

Use `scripts/proof_build.py` before runtime captures or terminal proof. A shared
`CARGO_TARGET_DIR` can leave the root executable from another worktree even when
Cargo reports success. In the #238 fixture, two clean worktrees have the same
package name/version but print different strings. Both exist before building.
Building A, B and A through one target directory produced A, A and A on the
cloud toolchain; Cargo reported B as fresh. This bounds the observed failure
without claiming every Cargo version or platform behaves identically.

## Build and run

Commit source changes first. The tool refuses dirty tracked files and untracked
nonignored files. Use a new output directory outside the checkout or under an
ignored path such as `.tmp/`; existing output directories are never overwritten.
Python 3.9 or later, Git and Cargo are required. Normal Cargo/Rustup and platform
dependency configuration remain in effect; do not replace HOME.

```sh
python3 scripts/proof_build.py build --output .tmp/runtime-001
python3 scripts/proof_build.py verify --receipt .tmp/runtime-001/receipt.json
python3 scripts/proof_build.py run --receipt .tmp/runtime-001/receipt.json -- \
  /path/to/RA2 EB1.MMX --ticks 1
```

The default package/binary is `yr-app`/`openyr`. `--package` and `--bin` select
other Rust tools for small reproductions. `--repo <checkout>` before the
subcommand selects an explicit checkout; otherwise the script's own checkout
is used. Program arguments follow `--` for `run`.

Every build uses a newly created private target directory through Cargo's
explicit `--target-dir`, overriding an inherited shared target. The tool reads
the requested executable from Cargo's JSON artifact event, checks it resides
inside that target and copies it to `<output>/program`. It removes its temporary
target after copying; other worktrees and their build caches are untouched.
Compiler diagnostics and `cargo.jsonl` remain available if a build fails. Failed
builds and source changes during compilation produce no receipt.

`receipt.json` records the Git commit/tree, Cargo version, package/binary,
release build command, and the copied program's byte size and SHA256. `verify`
and `run` require a clean checkout whose tree matches the receipt, then check
the program's size and hash. `run` executes that verified copy and forwards
its exit status, never selecting `target/release/openyr`. The receipt's program
location is fixed beside the receipt. Receipt reads are capped at 64 KiB.

The tree comparison permits a later commit with identical source, including a
proof-removal commit after the temporary branch-proof fallback. It rejects
additional source changes even when their commit messages look similar. Record
the receipt and executable hash with the capture's commands and results; keep
the executable, generated saves and retail assets outside Git. A receipt detects
accidental stale/replaced artifacts. It does not fingerprint retail data,
ignored inputs, environment settings or compiler wrappers, and is not a signed
attestation. A correct artifact still needs the slice's behavioral checks.

## Verification

```sh
python3 -m unittest discover -s scripts -p 'test_*.py' -v
```

The dependency-free Cargo fixture uses two temporary Git worktrees and reports
actual shared-target outputs and Cargo freshness flags. It accepts Cargo
versions that already handle reuse correctly. It then verifies isolated A/B
executables despite an inherited shared target, rejects another tree's receipt,
replaces an executable and proves `run` refuses it, checks dirty source and
output preservation, and covers invalid receipts and failed Rust compilation.
These tests run in required CI. Native OpenYR proof additionally builds the
actual app through the isolated path and runs a retail-backed headless check;
physical macOS/Retina, audio-device and original-game checks remain separate.
