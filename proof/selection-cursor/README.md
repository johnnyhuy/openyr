# Shift-only selection cursor resolution

Issue #290, within #98/#97.
Source: cf06fe72dcd009b547d76f4af527c5a238cc5b21
Tree: 68f6cb76505f4b64724fec60d1f159962b51012f

`terminal.txt` records source/tree, the debug library-test executable SHA256, four focused resolver tests and the generated `resolver.json`. For ordinary Move, Attack, Harvest and AttackMove actions, selection-only mode returns Pointer with no commands. A 70-case matrix verifies owned Select targets and Pointer elsewhere, including stale/enemy/empty selections and invalid cells. Complete synthetic Sim save bytes (5,325 bytes) and hash 26aeb9068b9c7e1d remain unchanged.

The app passes its existing Shift-without-Control policy to the shared resolver. Ctrl+Shift's attack-move expression and the repair/sell early-return branch are unchanged. The fixture exercises resolver logic on a synthetic Sim, not actual modifier delivery or a complete Session. It does not verify retail repair/sell execution.

The isolated release receipt covers CLI `--help` only; the terminal recording uses the separately hashed debug test executable. `workspace-check.log` records 534 reported Rust passes, one existing ignored audio-device test, strict clippy, fmt and three Python provenance tests. Retail tests self-skip when data is absent; they are not retail passes.

Retail cursor sheets are absent, so no screenshot or rendered cursor comparison is claimed. Known permission-blocked original-game/macOS capture paths and release uploads were not retried. Physical Mac/Retina input, retail commands/save reconstruction, audio, original modifier behavior and cursor pixels/hotspots/cadence remain unverified. No original executable implementation, process memory or game assets were used. Simulation and save formats are unchanged.

Proof uses the authorized temporary branch fallback and is removed before squash merge. The final source tree must match the tested tree above.
