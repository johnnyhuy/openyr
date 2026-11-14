# Projectile damage save proof

Tested source f8d19e6797bcb8c2bd00886b4c2d2e3db473bfaf; baseline 9026abf3e94883e68bcb40080c284cb0149f9ed8.

The authored public API caller resolves native rules, launches a physical projectile, saves it in flight and compares original/restored events, hashes and complete snapshots through impact. Six signed scaled-damage cases fail baseline save validation and pass with exact damage on the final source. Four valid controls preserve all 36 complete baseline/final snapshots byte for byte. Ten loader cases cover applied bounds, hostile integers and unchanged raw weapon/origin bounds. Four ordinary legacy payload versions load identically in both readers.

source-audit.json verifies every original archived file byte for byte. Only the two workspace manifests are augmented to register the identical authored caller; their expected changes are also verified. proof_build.py builds isolated committed snapshots and checks receipts before every run. No compiled programs are published.

Commands and output are in terminal.typescript and results JSON. Reproduce with make-probe.py for each recorded source commit, proof_build.py build --package projectile-damage-probe --bin projectile-damage-probe, then script -q -e -c 'python3 .tmp/probe.py' terminal.typescript. The probe uses the .tmp snapshot/output paths recorded in its source.

cargo test --workspace --locked: 754 passed, 0 failed, 2 existing ignored checks. cargo clippy --workspace --all-targets --locked -- -D warnings: passed. cargo fmt --all --check: passed. python3 -m unittest discover -s scripts -p 'test_*.py' -v: 3 passed. cargo test -p yr-sim --test projectile_damage --release --locked -- --nocapture: 3 passed.

No retail data is available in this executor. No retail, original-game runtime, native macOS/Retina or audible output verification is claimed. Known blocked release uploads were not retried. Proof is kept only in this historical PR commit, removed from the final branch tree, and requires squash merge.
