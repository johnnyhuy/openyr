# Animation clock cleanup

`yr-app::animation::UnitAnimations` observes movement and shot counters once per
completed simulation tick. These presentation clocks live outside `Sim` and its
save bytes. Their frame-selection policy remains documented in
`docs/formats/shp.md`; this change does not calibrate original playback.

Before #415, cleanup checked every retained ID with a fresh linear scan of the
live unit slice. Even with no deaths, N living units caused N(N+1)/2 ID checks
per update. This work grew quadratically before the ordinary BTreeMap lookups.

Each observed clock now records its last sampled tick. After sampling all live
units, one map traversal removes clocks whose mark differs from that tick.
Cleanup is O(retained clocks), and the whole update remains O(N log N) because
of the existing BTreeMap lookups. There is one additional u64 per clock, no
second membership collection, and no simulation or save-format field. Repeated
calls at the same tick return before sampling or cleanup, as before.

The army-churn regression removes and replaces 64 of 256 units for eight ticks.
It checks survivor movement and stationary phases, new IDs, removed clocks,
empty-army cleanup, repeat sampling and complete unchanged simulation save
bytes. Existing firing tests retain restart and expiry coverage.

Run the optional optimized probe with:

```sh
cargo test -p yr-app --release --lib animation_clock_scaling_probe -- --ignored --nocapture
```

It times only `UnitAnimations::update` over 48 distinct ticks for 512, 2048 and
8192 stationary unarmed units. Setup and simulation stepping are outside the
timed region. Timings describe this bookkeeping workload on the test machine,
not whole-frame throughput or a maximum supported army size. There is no timing
threshold in CI. The PR proof retains before/after terminal output, verification
commands, source hashes and an EB1 shared-compositor playback check.

## Shared building presentation sampling

Before #428, every building marker rescanned the building list for its owner's
power totals and first production host. N buildings therefore performed at
least N squared power contributions per redraw, even while idle.
`BuildingAnimations` now samples power and the first host of each role once
per redraw. Marker lookups search at most six roles in one owner's summary.
The owner key is a u8, bounding the owner map to 256 entries. Refinery unloading
still uses the simulation's authoritative docking query, once per owner with
a refinery. That query's internal scans are unchanged; this is not a claim
that all simulation or refinery work is linear.

Each redraw builds a fresh summary, including paused redraws. No cached tick,
new simulation field or save-format change can hide host destruction, health
changes, power shortages or completed production. The existing single-building
query remains available as an independent regression oracle. Authored checks
compare every animation field across all roles, duplicate hosts, four owners,
credit stalls, READY production, host destruction, empty bases and save/load.
The retail refinery test compares both paths throughout actual unloading.

Run the optional bookkeeping probe with:

```sh
cargo test -p yr-app --release --lib building_animation_scaling_probe -- --ignored --nocapture
```

It compares independent queries with shared sampling and all marker lookups
for 512, 2048 and 8192 idle buildings over eight redraws. Setup and simulation
stepping are outside the measurement. Timings are machine-specific and do not
measure complete rendering or establish original-game animation parity.
