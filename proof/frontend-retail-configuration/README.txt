Source commit: 5b1549e607b233d4ae5442d590e404629d7a0bc7
Source tree: 0ea240a9bdb26ae7fd7d78828a079c7cebfcf884
Retail archive SHA256: 9cbb15a9fcbe0581765de43aa4422c58f788f4fe4ed2a1ff1783bdfe4b9824b4

Eleven existing retail frontend checks now run against OPENYR_GAMEDIR. The assertion hashes, compared regions and masks were not modified. The #164 chooser marker exclusion remains. Shared-compositor proof frames are source phase 0, not original-game captures. The chooser test state intentionally lacks catalog rows outside the compared region. Seven complete Skirmish preview comparisons retain their prior observation hashes.

missing-path.txt is an intentionally failing test probe. checks.sh asserts that exactly one test ran and produced the configured-directory assertion, then continues. The unconfigured missing-home path preserves the existing skip behavior for asset-free CI. Full workspace checks run with actual retail data. validation.typescript is a real terminal recording.

The first development filter mistakenly selected the binary target and ran zero tests. The corrected library target ran all eleven; no zero-test result counts as retail verification. Both development logs are included for clarity.

The isolated release runtime receipt checks source tree and executable SHA256. Its supporting EB1 frame does not establish menu fidelity. No original executable, process memory, original timing, physical Mac/Retina or physical input/audio checks ran. Known permission-blocked original/Mac and release-upload paths were skipped. Historical proof is dropped from the final tree and squash merge is required.
