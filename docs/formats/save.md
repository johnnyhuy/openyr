# OpenYR match saves

This is OpenYR's own format, not a reader for the original game's saves.
The format captures every field of `yr-sim::Sim` without advancing the sim,
consuming RNG or draining events. `yr-sim` only converts bytes; the app
owns filesystem I/O and reconstructs the session and presentation.

## Envelope

All header integers are little-endian. A match file has this layout:

| Offset | Bytes | Value |
| --- | --- | --- |
| 0 | 8 | ASCII `OYRMATCH` |
| 8 | 4 | Match format version, currently 1 |
| 12 | 8 | Original setup seed |
| 20 | 8 | Setup compatibility fingerprint |
| 28 | 8 | Saved sim state hash |
| 36 | 1 | Outcome, 0 ongoing, 1 won, 2 lost |
| 37 | remainder | Simulation snapshot |

The simulation snapshot starts with `OYRSIM\0\0` (8 bytes), then a
little-endian u32 version (currently 2), then UTF-8 JSON of `Sim`.
Fixed-point values are raw i32 integers with 256 units per cell; rule
decimals are raw i64 millionths. No floating point enters the payload.
JSON's integer text preserves all bits of the u64 RNG and tick fields.

The payload includes unit paths, targets, attack-move goals, cooldowns,
harvest clocks and cargo; buildings and terrain edits; projectiles;
per-owner credits and paid queue progress; AI build choices, scout and
attack clocks; id allocation, tick, RNG and queued output events.
The terrain hash cache is omitted and recomputed. Events are preserved
but, as before, do not participate in the authoritative state hash.

The compatibility fingerprint is FNV-1a over the starting sim snapshot
and the serialized sidebar definitions, kind table and prerequisite
groups. The app rebuilds that setup using the saved seed and supplied
map/rules. A mismatch is rejected before replacing the current match.
This checks effective gameplay configuration, not the install's absolute
path or every art/audio byte. The state hash catches payload corruption;
these fingerprints are consistency checks, not cryptographic signatures.
Display names and cameo pixels are excluded so localized labels or new
sidebar art do not invalidate an otherwise compatible match (#64).

Simulation version 2 adds optional weapon/shot provenance and `Impact` output
events. The match wrapper remains version 1. Version-1 sim payloads are accepted
with unknown source metadata; their in-flight shots do not guess impact art.
New shots and flights retain provenance in version-2 snapshots. Setup keys use
the original version-1 projection without those visual fields, including unit,
queued-product, AI-build-profile and FreeUnit weapon metadata. This preserves
existing setup fingerprints while normal snapshots retain all new provenance. Visual metadata
does not enter the gameplay state hash.

Restored version-1 games use current corrected combat behavior: an already-fired
shell now survives shooter death (#119). A continuation involving that bug can
therefore differ from the older engine. The version-2 save marks the updated
schema; loading old payloads is supported, loading new saves in an old binary is
not.

## Loading and writing

Readers reject unsupported versions before parsing the payload. The app
reads at most 64 MiB plus the match header and one sentinel byte. JSON
vectors grow from actual elements, never header counts. After decoding,
checked terrain dimensions must equal the actual cell count and fit the
u16 coordinate domain. Entity ids must be ordered and unique with a valid
next id; footprints, health, weapon multipliers, coordinates, production
progress and payments are checked before the sim is exposed. Unknown
struct fields and malformed enum tags fail. Bump the format version when
the state schema or tick semantics change. The version-1 to version-2 path
above defaults only optional provenance; unsupported versions are rejected.

The app writes and syncs a sibling temporary file, then renames it over
the destination. An encoding/write failure leaves the old save intact.
Failed loads leave the running match intact. A successful live load
discards pending UI orders and rebuilds animation/audio state. Music
restarts; camera, selection and effect playback positions are not saved.

`--ticks` and `--frames` use absolute sim ticks, including after loading.
The headless demo queues units and power at tick 2 and a refinery at tick
1200, sends attacks every 450 ticks, and derives its placement ghost
phases from the absolute tick. Future commands therefore agree across a
split run. `--passive` disables demo orders. A capture may include the
loaded tick itself without stepping. Setup finishes at tick 1.

## Evidence

`eb1_saved_match_resumes_to_the_same_win` restores retail EB1 at ticks
100, 500, 900, 1350 and 3000, compares hashes at every intervening tick,
and reaches Won. It exercises active AI, harvesting, production, combat
and structure placement, and checks that invalid version, compatibility
and state hashes do not alter the existing match. Sim tests also cover
private path/attack/harvest state, RNG continuation, queued events,
hostile terrain dimensions and truncated headers.

Saves and proof captures contain derived retail data. Do not commit them.
