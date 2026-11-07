# Runtime proof provenance

The terminal recording reproduces #238 with two tiny clean Git worktrees. Their Rust packages print PROOF_A and PROOF_B. A shared target returned PROOF_A for B while Cargo reported fresh=true. The observation is bounded to this toolchain; tests also pass on versions that handle reuse correctly.

The new isolated builder compiles the correct fixture outputs despite an inherited shared target. Three tests cover that reproduction, stale source and replaced executable rejection, dirty input, identical source across different commits, output preservation, malformed/oversized receipts and failed compilation. Expected failure diagnostics are part of those passing negative tests.

The actual OpenYR app was built from the receipt's source tree through a fresh private target. The recording then verifies the copy, runs --help and a retail-backed EB1 headless tick, appends one byte and demonstrates that run refuses the changed executable, restores the original bytes and verifies again. The native run reached tick 1 Ongoing with hash c4740e7a21c92521. Program SHA256 is in receipt.json. The temporary Cargo target was removed; the executable and retail data are excluded from this proof bundle.

session.typescript is the actual script terminal recording; session.txt removes terminal escapes for reading. session.sh lists the commands. cargo.jsonl records Cargo's artifact events. Workspace tests passed 458 with one existing audio-device test ignored; strict Clippy and fmt passed. Rust/simulation/save behavior is unchanged. CI now runs the provenance fixtures, and AGENTS.md uses the guarded executable for headless proof.

This is nonvisual tooling proof. No physical Mac, Retina, audio-device or original-game runtime checks ran. Known permission-blocked macOS capture and original-reference paths were skipped. The earlier HTTP-401 release-upload route was not retried; this uses the authorized temporary branch-proof fallback. All proof files are removed from the final source tree before squash merge.
