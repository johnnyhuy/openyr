# Decimal integer arithmetic proof

Tested source 52b4659ac9659f8fa2c50b54f9af8dedda81e7c4, tree 53d6e52448157c69b1e0e9a082b1a4744e2b04d9.

terminal.typescript records seven authored CLI cases through verified baseline/final release programs. Five extreme damage tables are corrected; two ordinary outputs remain byte-identical after removing the proof tool's executable-path banner. results.json preserves full commands and output. The baseline CLI receipt was reverified against unchanged main 191287c's tree; it was built during #409. The final CLI was built in a fresh isolated target for this run.

math-debug.log and math-release.log cover integer construction (including const limits), representable wide products, saturation only after division, signed truncation and zero. workspace-tests.log records 730 passes, zero failures and one existing native audio-device ignore. Strict Clippy/fmt and three Python checks pass.

Simulation fixtures come from the integration test, not the CLI. The baseline test-only commit 22b5f39b765290d44fa55b5991e93df1814162c8 adds the same decimal_arithmetic.rs fixture on 191287c without changing production code; it was built in a fresh private target. The final test runs on the tested source. Four verses, six delivered shots each and save/load continuation yield 64 byte-identical saved snapshots: 73587 bytes, SHA256 edae703f981c186b82abf46b990d358ae602ae94c3ec134ac78483e6372f1b56. Final target HP values are 652, 514, 874 and 1000; final hashes are 61c7587551612770, 81cb152f159bb93c, ce930561999f9b58 and 5301a619e14f4ca5. Both snapshot streams and test logs are included.

The extreme saturation behavior is intentional native policy, not original-game calibration. Save encoding and ordinary arithmetic are unchanged. No retail, original runtime, physical Mac/Retina or audible-device checks ran. Data is absent; known blocked capture and release-upload paths were skipped.
