# Structure repair and sale

The Allied `SIDEC01.MIX` contains `REPAIR.SHP` and `SELL.SHP`. Each is a
64x31 canvas with two frames. Frame 0 is the normal control; frame 1 lights
its symbol. Both use that archive's `SIDEBAR.PAL`, with index 0 transparent.
The observed 1024x768 original screenshot used for the sidebar work places
these sheets at panel offsets (20,166) and (84,166). The top controls do not
scale with the viewport height. See [sidebar evidence](sidebar.md).

The window draws the normal sheets through the shared sidebar renderer and
then overlays the selected sheet for the current mode. Hit testing uses the
normal sheet's opaque coverage. K selects repair, L selects sell, and a
second press, right click or Escape cancels the mode. Category/cameo/radar
input also leaves the mode. Left clicks resolve a structure by its full
footprint and dispatch an ordinary deterministic simulation command.

## Retail data and implemented policy

Merged `RULESMD.INI` and map overrides supply `Repairable`,
`ClickRepairable`, `Unsellable` and `Cost`. The retail comments document
`Repairable` as enabled by default. Explicit false repair flags and true
`Unsellable` exclude the corresponding action. Missing costs exclude both.
Only live player-owned buildings are eligible. Full health does not start
a repair job. The existing sandbox does not model capture or sale of units.

The retail general section gives `RepairPercent=15%`, `RepairStep=8`,
`RepairRate=.016` minutes and `RefundPercent=50%`. At the native session's
15 ticks/second, the repair interval rounds up to 15 ticks. Each interval
restores up to eight HP. The charged amount is the cumulative floor of
`Cost * RepairPercent * restored HP / maximum HP` within one repair job.
The fractional remainder carries between intervals. Insufficient credits
freeze both health and the fraction; the job retries on the next tick.
Canceling drops its fractional remainder. Multiple jobs spend in ascending
entity ID order after production and before harvesting. Repairs do not
require full power.

Sale immediately removes the building and credits the floor of
`Cost * RefundPercent`, capped at the snapshot credit limit. It frees the
footprint, clears attack targets and removes repair jobs. Factory/refinery
loss follows the existing production and harvesting behavior. A sale emits
`Sold` rather than `Died`, so it routes `EVA_StructureSold` without a unit-loss
announcement. An accepted repair start routes `EVA_Repairing`. The sample
names come from the installed `EVAMD.INI` sections, not replacement audio.

These are explicit native policies, not measured original-game parity.
The [RefundPercent documentation](https://modenc.renegadeprojects.com/RefundPercent)
and [RepairStep documentation](https://modenc.renegadeprojects.com/RepairStep)
support the data's meaning. No engine or third-party implementation code
was used. The original manual describes credit-funded wrench repair and
selling for a partial refund, with possible survivor infantry.

## State and verification

Repair recipes, countdowns and price remainders are authoritative state.
Snapshots validate target ownership/health, recipe bounds, countdowns and
fractions. Hashing visits jobs in stable ID order. Empty jobs are omitted
from saves and from the hash extension, preserving idle session snapshots
and baseline hashes. Active-repair saves require a build supporting this
extension; older readers reject their unknown field.

Unit checks cover credit starvation/resumption, repeated enable, cancellation,
exact cumulative charges, ownership, malformed jobs, competing jobs,
refunds and footprint release. The retail integration test compares every
opaque pixel of both control states with independently decoded SHP canvases.
It makes a controlled half-health GAPOWR snapshot, verifies retail recipes,
stalls at zero credits, repairs to full, resumes an active job from a save
with identical hashes, then sells the plant. Optional proof saves and UI
images are written outside the repository using `OPENYR_SERVICES_PROOF`.

## Fidelity still to measure

Original rounding, interval scaling by game speed, repair start delay,
insufficient-funds announcements, cancel/restart fractional behavior and
sale valuation at different health levels need original observations.
Repair cursor variants, the building wrench animation, loop sounds,
deconstruction animation and survivor infantry remain open under #86/#6.
The top controls and bottom command bar also remain incomplete. The Wine
launcher started `RA2MD.exe` and `gamemd.exe` during this run, but native app
inspection timed out with `timeoutReached`; no original-game capture or
interaction was obtained. This slice proves decoded assets and native
behavior, not full UI or animation parity.
