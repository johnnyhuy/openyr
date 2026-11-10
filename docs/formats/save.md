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
| 8 | 4 | Match format version, 1, 2, 3 or 4 |
| 12 | 8 | Original setup seed |
| 20 | 8 | Setup compatibility fingerprint |
| 28 | 8 | Saved sim state hash |
| 36 | 1 | Outcome, 0 ongoing, 1 won, 2 lost |
| 37 | remainder | Version 1: simulation snapshot |
| 37 | 2 | Version 2: map-name byte length N, 1 through 255 |
| 39 | N | Version 2: installed map filename |
| 39+N | remainder | Version 2: simulation snapshot |

New sessions write version 3. It records the shared starting-force profile
(#166), a bounded optional map filename, and explicit credits/unit overrides.
After the common 37-byte header:

| Offset | Bytes | Version-3 value |
| --- | --- | --- |
| 37 | 2 | Map-name byte length N, 0 through 255 |
| 39 | N | Installed map filename, absent when N is zero |
| 39+N | 1 | Option flags: bit 0 credits, bit 1 unit count |
| 40+N | 8 | Starting credits, signed integer |
| 48+N | 2 | Starting unit count, unsigned integer |
| 50+N | remainder | Simulation snapshot |

Zero flags select the rules defaults. An absent option must have a zero value
in its slot. Reserved flags, truncated fields, credits outside 0 through
1,000,000,000 and unit counts above 4096 are rejected before reconstruction.
The fixed metadata adds 11 bytes; the simulation payload limit remains 64 MiB.
The simulation schema is unchanged.

Version 4 adds participant configuration after the version-3 options: a
little-endian u16 JSON byte length, followed by that many UTF-8 bytes, then the
unchanged simulation snapshot. The length must be 1 through 4096 and fit the
actual input before decoding. The object contains exactly two participants
(name, country enum, color index, team and optional authored waypoint index),
Short Game and the native speed enum. Unknown fields/enums, malformed JSON,
out-of-range values and conflicting choices fail before reconstruction.
Names are bounded by 32 characters and 96 bytes, with no controls or blank-only
values. See [front-menu.md](front-menu.md) for supported choices and limitations.

Only non-default participant configuration uses version 4. Default and legacy
profiles retain their prior envelope versions and setup fingerprints. Menu
loads report validated map/configuration before expensive reconstruction, so
the loading screen uses the saved country's artwork and player labels.
Save/load and restart retain all settings. No additional simulation fields,
floating-point state or presentation clocks enter the snapshot.

The map reference is a bounded ASCII filename with letters, digits, spaces,
underscores, hyphens and dots. Leading dots, path separators, drive prefixes,
control bytes and invalid encoding are rejected before loading. A present
reference must resolve in the selected install. Version 2 requires a nonempty
name; version 3 uses zero length for direct setup or external-path sessions.
A map reference is loading metadata and does not alter the sim payload or hash.

Native front and pause menus rebuild the referenced map with the saved profile,
seed and overrides, then commit its name, terrain, sprites and session together
after the complete load succeeds. Failed or cancelled jobs retain the old match.
Explicit CLI loads continue to check their supplied map against the saved setup.
Restart retains the loaded profile and overrides. A new match from the front
menu uses the shared-force profile and the displayed controls.

Versions 1 and 2 remain readable. They reconstruct the prior Allied sandbox
with three computer guards, retaining its starting fingerprint. Version 1 uses
the supplied map; version 2 follows its recorded installed map in menus. Saving
an ongoing legacy session keeps its profile and envelope version. This preserves
the most recent three-guard profile; older incompatible engine configurations
still fail their setup fingerprint checks as described below.

The actual-handler regression checks 5000-credit, six-unit settings for both
owners through Start, save/load, malformed metadata, restart and return to setup.
A retained prior-release version-2 save and equivalent version-1 payload also
load at EB1 tick 1, seed 42, hash `57dac47c915b074a`. Their producing command,
binary hash and save hashes are recorded in external proof. Save files are not
repository fixtures or distributed proof assets.

The simulation snapshot starts with `OYRSIM\0\0` (8 bytes), then a
little-endian u32 version (currently 3), then UTF-8 JSON of `Sim`.
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

Initial map structure health (#346) now preserves the full type maximum
instead of reducing current and maximum HP together. Full-health structures
with Strength in 1..=i32::MAX produce the same initial simulation bytes and
setup key as before. Damaged, zero/rounded-down or overfull health, and
nonpositive or oversized Strength, can intentionally change the initial state
and its setup key. Existing match saves whose setup key changes fail the normal compatibility check
before replacing the current session. Continue those matches with the older
engine; no migration silently repairs their maximum HP. Raw simulation saves
remain readable and retain their encoded current/maximum HP exactly. Match
and simulation schema versions are unchanged. Authored tests verify both
unaffected acceptance and affected rejection without mutating the session.

Initial map unit health (#349) now applies the previously ignored fraction for
vehicles, infantry and aircraft. Full-health supported map units and generated
starting units retain their existing setup state. Maps whose unit HP changes
intentionally get a different setup key; old match files for those maps fail
before replacing a live session. Continue them with a matching older engine.
Valid raw old simulation snapshots still retain their encoded HP exactly, with
no migration or schema change. Authored tests verify unaffected old-match
acceptance, affected rejection without mutation, and unchanged raw snapshots.

Initial map unit facing (#350) now supplies the normalized authored byte rather
than the previous unconditional 128. A setup whose units still start at 128
retains its old key. Other headings, including missing/malformed fields now
using zero, intentionally change the setup key. Affected old match saves reject
before replacing the current session; continue them with the matching older
engine. Valid old raw simulation snapshots retain their encoded heading, with
no schema change or migration. Authored checks verify unaffected acceptance,
affected rejection without mutation and exact new-session save/load.

Map-unit live composition (#351) adds only a reconstructed id/kind-to-house
presentation table to Session. It is absent from the match envelope, simulation
payload and setup fingerprint. Movement and restored state still refer to the
same stable ids; kind mismatches cannot inherit another source object's color.
Changing or clearing this table leaves exact match bytes and setup keys intact.
This presentation fix changes no schema or save compatibility policy.

The corrected American starting force (#124) changes effective gameplay
configuration from SMCV/HARV/HTNK to AMCV/CMIN/MTNK. Its kinds, health and
weapons produce a different setup key. Old mixed-force match saves are
rejected without changing the current match; continue them with a matching
older engine. Newly created Allied matches round-trip normally. The match
wrapper and simulation schema versions are unchanged by this setup change.

The raw-lepton projectile conversion (#122) changes stored weapon velocities
and the effective setup key. Old matches created with the extra 256 factor
are rejected by the match wrapper before replacing the session. The sim
schema remains 3: raw positions and velocities already carry their units,
and the tick-phase algorithm is unchanged. Standalone old sim snapshots
remain readable and retain their encoded velocities; they are not silently
rescaled. Newly created physical flights round-trip and continue with the
same positions, events and hash. Invisible projectiles now use the explicit
zero-speed instant path. Presentation observes the saved flight directly,
so rebuilding its sprite cache cannot advance or restart it.

Simulation version 2 adds optional weapon/shot provenance and `Impact` output
events. That simulation change did not change the match wrapper. Version-1 sim payloads are accepted
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

Simulation version 3 distinguishes absent map coordinates with the `Absent`
ground tag (#105). Normal grid access, movement and placement treat those
slots as missing cells. They retain no overlays, resources or building state.
The support pattern participates in both terrain hashes and setup fingerprints.
Old match saves built from the permissive sparse rectangle fail the setup
compatibility check without replacing the current session. Use a matching
older engine to continue those matches; their old grid is not silently upgraded.
Version-1/2 simulation payloads remain readable, and unchanged fully populated
setups retain their existing compatibility key. New saves require an engine
that reads version 3.

## Loading and writing

Readers reject unsupported versions before parsing the payload. The app
reads at most 64 MiB plus 4403 bytes of bounded match metadata and one sentinel byte. JSON
vectors grow from actual elements, never header counts. After decoding,
checked terrain dimensions must equal the actual cell count and fit the
u16 coordinate domain. Entity ids must be ordered and unique with a valid
next id; footprints, health, weapon multipliers, coordinates, production
progress and payments are checked before the sim is exposed. Unknown
struct fields and malformed enum tags fail. Bump the format version when
the state schema or tick semantics change. Versions 1 through 3 are readable;
the older provenance fields default as above. Sparse-grid setup changes are
checked separately by the match wrapper. Unsupported versions are rejected.

The app writes and syncs an exclusively created sibling temporary file, then
renames it over the destination. Its short ASCII basename is
`.openyr-save.<pid>.<process counter>.pending`, independent of the target's
basename (#257). Valid long ASCII/Unicode names and non-UTF-8 Linux names do not
expand during temporary-file creation. The process counter is app-side I/O
bookkeeping, not simulation state or RNG.

Up to 32 candidate names are tried per write. Occupied files, directories and
symlinks are left untouched; the destination's own basename is also skipped.
Other creation errors fail immediately. Write/sync/rename failures attempt to
remove only the newly created temporary path. Existing pending files are never
reused or cleaned up as a side effect of saving. This retains the existing
same-directory rename policy and does not add directory fsync or change the
on-disk match format. Asset-independent storage tests exercise actual long-name
creation/replacement, collision exhaustion, symlinks and rename-failure cleanup.
The non-UTF-8 filename case runs on Linux: macOS CI rejected the fixture's
initial file creation with EILSEQ, before the storage function ran. That is a
filesystem limitation, not a bypass of the ASCII/Unicode replacement checks.
An encoding/write failure leaves the old save intact.
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

## Held production

Simulation version 4 adds optional `held: true` on a suspended build order.
False flags are omitted, preserving the legacy setup fingerprint bytes.
Versions 1-3 load absent flags as false. The flag is a JSON boolean;
non-boolean values and held completed orders fail validation. The match
wrapper version and setup compatibility key are unchanged.

A held order retains paid credits and fixed-point progress. Saving/loading
cannot resume it; explicit resume or cancellation is required. Hashing adds
a tagged owner/queue-index suffix only when a held order exists, preserving
historical hashes for matches that never hold production. Simulation tests
compare uninterrupted and reloaded ticks, events, refunds and rollout.
