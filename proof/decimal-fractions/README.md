# Exact decimal overflow proof

Tested source 9db4b14 and tree 0fe8eaee3e9f8150a8680dc8ffbf876ac27e2667. Baseline b902ce2. Both release CLI executables were built in isolated targets with scripts/proof_build.py. Receipts record their complete source trees, commands, sizes and SHA256 hashes.

terminal.typescript records nine authored rules cases in both verified binaries. results.json preserves full commands, exit codes, stdout and stderr. Four out-of-range decimals/percentages wrap in the baseline and are absent in the final typed rules. Both signed limits print exact lepton values; three ordinary CLI outputs match byte for byte after excluding the proof tool's executable-path verification banner. All fixtures contain authored INI text, no game assets.

Debug cargo test --workspace --locked: 725 passed, zero failed, one existing audio-device ignore. cargo clippy --workspace --all-targets --locked -- -D warnings and cargo fmt --all --check pass. Three Python proof-tool checks pass. The signed parser boundary test also passes under cargo test -p yr-rules --lib --release --locked decimal_fraction_boundaries.

No current retail, original runtime, physical Mac/Retina or audible-device checks ran. Retail data is absent. Existing blocked capture/release-upload paths were skipped. from_int and mul_int arithmetic remains tracked in #408.
