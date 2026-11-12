# Rejected selected installs and bounded MIX headers

`selected-install.typescript` records four native launcher tests using actual scratch directories and authored MIX archives. Ambiguous loose-file creation orders and a required symlink-only archive reject before replacing the previous preference. Empty, malformed, truncated and entryless required archives retain the previous location; repaired mixed-case archives can be selected. Source bytes and symlink identity remain intact.

`flagged-header.typescript` records 70 truncated prefixes across zero-flag and checksum-flag archives rejecting without panic. Complete payloads and the existing checksum round-trip still pass.

`baseline.log` records the unchanged production code with only the tests from `baseline-tests.patch` applied: one existing selection test passes, two invalid-install rejection assertions fail, and the six-byte flagged header panics in the parser. `baseline-executable.json` identifies that uncommitted test-only baseline and its native executable digest. It is not presented as a clean committed proof build.

`check.log` contains the full workspace checks. `test-executables.json` records the source commit/tree and the exact native Cargo test artifacts, with sizes and SHA256 hashes. Private targets were used, and the binaries were retained before removing this run's build cache.

`runtime.typescript` records verification and help from the isolated native app; `runtime-receipt.json` records the build command, source tree, program size and SHA256. Help is only an execution smoke check. The actual loading and parsing behavior is verified by the native filesystem and parser tests above.

The initial interactive chooser and menu Data Folder action share the validation helper. Discovery order and existing write-error policies stay unchanged, as do preference JSON and simulation/save formats. This checks archive/container eligibility, not every asset needed to play. Retail data, physical macOS chooser delivery, Retina, audio-device and original-game behavior were not exercised. Known data paths, the remembered preference and retail-sized workspace candidates are absent. Known blocked Mac captures and release upload HTTP 401 were not retried.

This proof directory is pinned on the PR branch and removed from its final tree before squash merge. No executable, game asset or save is uploaded.
