# Bounded theater tile-index loading

Issue #292, within #96/#97. Follow-up defects #293/#294 remain open.
Source: e3d122af6bce842cdc7317b09a4af249f5f1cd89
Tree: bbc8f728aee0b64af99d5d04c5c7b952d02b8a94

`terminal.txt` records committed source/tree, three debug test executable hashes, eight focused parser/propagation/numbering tests, receipt verification, and the real `mix-dump render-map` command. The isolated release rejects synthetic TilesInSet=4294967295 metadata with a contextual error, exit code 1 and no PNG. `input/` contains only authored synthetic INI/map bytes, not game assets. No resource-exhaustion fixture was run against unfixed code.

The receipt covers the isolated release `mix-dump` executable and its render-map run. It does not cover an OpenYR native app release. Debug parser/terrain/radar tests use separately hashed executables. `workspace-check.log` records 540 reported Rust passes, one existing ignored audio-device test, strict clippy, formatting and three Python provenance tests. Tests requiring absent retail data self-skip; they are not retail passes.

Native budgets are 65,536 generated names and a conservative 16 MiB filename-byte/String-entry estimate, excluding allocator bookkeeping and already-parsed INI storage. Supplied malformed counts now return errors; a missing count remains zero. Names/numbering within bounds remain unchanged, and errors never return partial indexes. These are native safety policies, not original-engine limits. Simulation and save formats are unchanged.

No screenshot is claimed for this parser/loader slice. Retail scans/data, complete Sessions, native Mac/Retina, audio and original-game behavior remain unverified. Known permission-blocked original-game/macOS capture paths and release uploads were not retried. Proof uses the authorized temporary branch fallback and is removed before squash merge; the final source tree must match the tested tree above.
