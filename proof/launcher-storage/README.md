# Remembered game-directory storage

The launcher now uses the same exclusive, synced sibling replacement as interface preferences. No game assets are needed for this filesystem change.

`launcher-storage.typescript` records the three actual launcher `remember` regressions: occupied legacy file, occupied Unix symlink and its target, and publication failure followed by cleanup and retry. `shared-writer.typescript` records the shared helper collision limit and occupied file/directory/link checks, plus both existing interface preference handler checks.

`baseline.log` contains the three failures before the fix. Its production source is main `1257aed3dc108d8288964232b9c18c42d73a3f6e` with only the new tests in `baseline-tests.patch` applied. It is deliberately identified as an uncommitted test-only baseline, not a clean proof build. `baseline-executable.json` records its executable digest.

`check.log` records the required full workspace checks from the committed source. `test-executable.json` identifies the exact native test executable selected from Cargo's log, with its source tree, byte size and SHA256. No shared-target executable was used.

`runtime.typescript` records verification and `--help` from the isolated native app build. `runtime-receipt.json` records its committed source tree, Cargo command and program digest. Help is a native execution smoke check; the actual filesystem behavior is checked by the recorded launcher and handler tests.

The JSON path-string format, bounded reads, discovery order, simulation and saves are unchanged. These Linux fixtures do not verify retail data, native macOS chooser delivery, Retina, physical audio or original-game behavior. Known install paths and the remembered preference file are absent in this environment. This does not claim protection against an actor concurrently replacing entries in a writable containing directory.

Release upload previously returned HTTP 401 and was not retried. This branch-only proof directory is removed before squash merge; proof links pin this commit.
