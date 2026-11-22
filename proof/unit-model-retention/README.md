# Unit model retention proof

Tested source a5e3e342c48f9dc2a8f5688601df023ff48e38fb, tree 3401375453cc61cd01f4d3a1369069ec4fb34c3e. No retail assets are included.

validation.typescript records cargo test --workspace --locked (768 passed, zero failures, four existing ignores), cargo clippy --workspace --all-targets --locked -- -D warnings, cargo fmt --all --check and three passing Python proof tests. The commands use a private target directory. model-proof.typescript records two authored model tests in that same isolated target before the full checks, from a clean source tree.

parent-test.patch applies only authored fixtures and a 128-record assertion to parent 3d99db6f5497214772b9e2b3e0b3d9c30c98df59. Run cargo test -p yr-render --lib unit_model_parent_history_regression --locked -- --nocapture there. parent-regression.log records the expected exit 101 at 129 retained models. Use a separate target directory for this parent test.

initial-shared-cache-failure.typescript retains the first workspace attempt: unit/integration groups passed, but a concurrent parent build in the same target replaced dependencies before rustdoc ran (E0463). The complete private-target rerun passed. This is an execution-cache correction, not a suppressed source test failure.

The authored tests churn 300 valid body/turret sources and 300 malformed/missing sources; 128 full RGBA/anchor comparisons cover byte eviction, oversized requests, eight facings, two remaps and four clocks. Retail tests return early without installation data and do not establish retail parity. Live-window, original-runtime, physical Mac/Retina and audible-device checks did not run. The payload/count budget is not a process-RSS cap; see docs/formats/vxl.md.

Proof is retained only in a historical PR-branch commit. All proof files are removed before squash merge and the final source tree must equal the tested tree. Existing blocked macOS/original capture and release-upload routes were not retried.
