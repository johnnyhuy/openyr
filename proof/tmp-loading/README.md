# Terrain TMP loader recovery

PR: https://github.com/johnnyhuy/openyr/pull/296
Issue #293, within #96/#97. Coordinate bounds remain in #294.
Source: 2761389c82023b25dc90535356f2944852a8fc27
Tree: c1d6342a6e20a0b204a6e5256240ecb16335b55e

`terminal.txt` records source/tree, a private-target debug test executable hash, four direct `build_terrain` API tests and the receipt-verified CLI control. The unfixed missing-template regression panicked at `TMP readable`; fixed missing/malformed/truncated files return filename-specific errors. Valid/repeated/clear-sentinel metadata and unknown/empty subtile sparsity preserve the expected complete synthetic Sim save bytes (2,839 bytes), hash 5ae3413acffebcda and roundtrip.

`valid-control.png` is an authored synthetic single-diamond map-art render, not a retail screenshot or a picture of the changed recovery path. The isolated `mix-dump render-map` release/receipt verifies this separate renderer control only. `build_terrain` is exercised directly by the debug tests, whose executable SHA256 is recorded separately. No production CLI was added solely for proof.

`make-input.py` recreates six synthetic input files. No game assets or original implementation were used. `workspace-check.log` records 544 reported Rust passes, one existing ignored audio-device test, strict clippy, formatting and three Python provenance tests. Required Rust checks used a private worktree target and two build jobs. Retail tests requiring absent data self-skip and are not retail passes.

No complete Session or retail save reconstruction, retail scan, physical Mac/Retina, audio or original-game behavior was verified. Known Mac/original-game capture permission blocks and release-upload HTTP401 paths were not retried. The authorized temporary branch-proof fallback is used; every proof file is removed before squash merge and the final source tree must equal the tested tree above.
