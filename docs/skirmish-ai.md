# Basic skirmish opponent

The default two-start session now uses `yr-sim::ai::SkirmishAi` instead of the
sandbox's five free-spawn waves. Both players retain pre-built Allied bases;
the computer also starts with three guards and a barracks. This is a prototype
policy for one-map skirmishes, not a reconstruction of the original team AI.

`yr-app::session::setup` resolves Grizzly/GI choices, four support structures,
retail costs, `BuildSpeed`, weapons and prerequisite groups into typed specs.
All subsequent decisions run inside `Sim::step`, after external commands and
before production. Decisions occur every 30 ticks at the shared 15 Hz rate.
There is no wall clock or presentation dependency.

Each owner has separate credits and factory queues. Production visits owners
in sorted order and factories in `Factory::ALL` order. Buildings supply power
only to their owner. Missing factories and insufficient credits stall builds;
units must leave through a free standable exit. Harvesters choose an owned
refinery and deposit into their own account (#58).

The computer starts with the rules' skirmish money and earns $150 every 150
ticks while it owns a refinery. This economy-lite stipend is deliberate (#21).
It does not receive free harvesters when rebuilding a refinery. Original AI
harvesting, scouting under fog, tech progression and team scripts remain M6
fidelity work.

One vehicle and one infantry order can run concurrently. The controller stops
queuing combat units when live units plus pending unit orders reach 12. It
sends one infantry scout after tick 450. Starting at tick 1350, it sends up to
six waiting combat units when at least four are available, then waits 900
ticks before the next group. Idle attackers choose the nearest opponent entity,
with entity id breaking distance ties, and engage enemies along their route.
When the base falls, remaining guards join the attack. Decisions stop when the
opponent has no entities left.

A surviving construction yard can replace a missing power plant, refinery,
war factory or barracks. Replacements pay the retail cost and build time and
must meet their resolved prerequisites. Placement searches in building id order
through rings of up to 16 cells around the base, using `Sim::can_place`, exactly
like the player footprint ghost. A completed replacement waits if no legal
site exists. The AI cannot reconstruct a lost construction yard.

The state hash includes controller configuration and deadlines, attacker ids,
every owner's economy, queued specs, full movement paths and combat/harvest
state (#59). Terrain fingerprints include dimensions, origin and ramps, and
invalidate on mutable cell access. Event draining is presentation-only and
does not alter the hash.

`--frames <dir> <tick>...` still drives the produced-army and structure-placement
demo. Append `--passive` to give the player no orders and observe the computer's
attack. Both modes use the same session, AI and draw code as the native window;
the terminal capture prints AI credits, queue length, groups, scout status,
state hash and match outcome alongside each image. Proof assets contain retail
art and stay outside git.
