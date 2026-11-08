# Selected-save identity proof (#265)

Source: `b7ef55a16e28ce16508d4576c4965307a4cd13f8`.
Source tree: `30431ec0617ebe64f285f09a63f028f7c5275943`.

`frames/` contains four actual 640x480 fallback menu compositions and accompanying JSON state from the final debug handler tests. The GIF sequences those captures: selection, exact long-basename overwrite confirmation, decline, deliberate replacement-name confirmation. No original artwork or retail assets are used.

`terminal.txt` and `terminal.time` record the commands in `run-proof.sh` (replay with `scriptreplay --log-timing terminal.time --log-out terminal.txt`). The five debug tests use actual temporary files and protected siblings. They establish the requested/accepted Commit target, unchanged sibling bytes during requests/decline, and safe recovery on stale selections. No Session is created, so they do not perform the real save backend overwrite or reconstruct a retail save.

`release-receipt.json` covers a clean isolated release build and its CLI help smoke only. It does not cover the separate debug test executable; that executable's hash is in the terminal recording. `full-check.log` records 497 Rust passes and one ignored audio smoke test, strict workspace Clippy, formatting and three proof tooling tests. Tests requiring retail data self-skip internally when data is absent, so reported pass counts are not retail coverage.

Retail data, a native Mac session, physical/Retina input, audio playback and live-window checks were unavailable and were not claimed. The captured fallback compositions are shared-compositor handler proof, not live screenshots or original-game geometry/timing parity evidence.

These proof files are temporary branch-only material. They are removed from the final PR tree; squash merge only. `manifest.json` records file byte counts, SHA-256 and Git blob identities for remote verification.
