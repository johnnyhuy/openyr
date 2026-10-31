# Rules data: rulesmd.ini / artmd.ini semantics

`crates/yr-rules` turns the merged INIs into typed technos, weapons, warheads
and projectiles (`Rules::load`). `mix-dump rules <gamedir> [ID...]` prints the
summary and issue report, or a fully resolved unit: display name, stats,
weapon chain with damage per armor type, art files with their archive, and
the raw INI lines.

## INI parsing rules

| Rule | Evidence |
|---|---|
| A section name runs to the first `]`; text after it is ignored | Retail `rulesmd.ini` line 27538: `[MirageWH]	// Supposed to be a heat ray.` Treating the header as invalid merged MirageWH's keys into `[LUNARWH]`, overwriting its `Verses`, and left the Mirage Tank's weapon without a warhead. Matches the loader behaviour in EA's GPL C&C source release. |
| `;` starts a comment anywhere on a line | retail headers like `[DISK];gs changed to K...` |
| Repeated sections merge | standard; maps rely on it |
| A repeated key: the **last** value wins | Ares added `+=` list syntax because reused list indices overwrite earlier entries. Not yet verified against the oracle. With the MirageWH fix, retail has one conflicting repeat: `[CRNeutronRifle] Report=` (empty, then `ChronoLegionAttack`). |
| Booleans are decided by the first character: `Y`/`T`/`1` true, `N`/`F`/`0` false | Westwood convention; `yes`, `true`, `1` all appear in retail |
| Enum values are case-insensitive | retail uses `Armor=Plate` beside `Armor=plate` |
| `none` means no reference | `Primary=none` |

## Type registration

- Technos come from `[InfantryTypes]`, `[VehicleTypes]`, `[AircraftTypes]` and
  `[BuildingTypes]`, in list order. The position is the engine's array index.
- A reused list index keeps its first position but takes the last value.
- An ID listed twice registers once. Retail lists `NAPSYA` twice, giving
  402 buildings from 403 entries.
- A listed type without a section exists with engine defaults. Retail has 6:
  `YDUM`, `APACHE`, `CASYDN01`, `CATIME`, `CALA02`, `CALOND02`.
- **Weapons have no list.** They exist when referenced (`Primary`,
  `Secondary`, `Elite*`, `Weapon1..WeaponCount`).
- **Warheads** come from `[Warheads]` plus any referenced by weapons. 5 retail
  warheads (`ApocAPE`, `GRIZAPE`, `RHINAPE`, `Shock`, `MagneShakeWH`) are only
  referenced. Projectiles exist when referenced.
- Art comes from the `artmd.ini` section named by `Image=` (default: the ID).
  `Voxel=yes` means `<image>.VXL`/`.HVA` plus `TUR`/`BARL` parts; otherwise
  `<image>.SHP`. Cameos come from `Cameo=`/`AltCameo=`.

## Values

- No floats: `Decimal` stores value x 10^6 exactly (`Range=1.4`,
  `Verses=75%`, `CellSpread=.3`). Conversions are explicit:
  `to_leptons()` (x256, truncating; the engine's rounding is unverified) and
  `mul_int()` for damage x verses.
- `Verses=` has 11 entries in armor order `none, flak, plate, light, medium,
  heavy, wood, steel, concrete, special_1, special_2` (ModEnc "Verses",
  "Armor"). All 100 retail `Verses` lines have 11. An unspecified `Verses` is
  treated as 100% (to verify).
- Keys not given in the INI stay `None`, shown as `(default)`. Engine defaults
  are filled in only where documented and verified.

## Retail known answers (tests in `crates/yr-rules/tests/retail.rs`)

- Registries: 65 infantry, 80 vehicles, 12 aircraft, 402 buildings, plus 189
  weapons, 116 warheads and 37 projectiles reachable from technos. **Zero
  warnings**: every referenced section exists and every armor parses.
- `HTNK` (UI "Rhino Heavy Tank" from `RA2MD.CSF`): 400 HP, heavy armor,
  900 credits. `120mm`: damage 90, ROF 65, range 5.75 cells (1,472 leptons).
- `MirageWH` loads as its own warhead (wood 30%), and `LUNARWH` keeps its own
  `Verses`.

## Structures and placement (issue #46)

Keys read for buildings, with retail evidence from `rulesmd.ini`:

| Key | Meaning | Retail evidence |
|---|---|---|
| `Prerequisite=` | Type ids or generic tokens that must be owned | `GAREFN`: `POWER,GACNST`; `GAWEAP`: `PROC,GAPILE,GACNST` |
| `PrerequisitePower=` etc. in `[General]` | Buildings behind the tokens `POWER`, `FACTORY`, `BARRACKS`, `RADAR`, `TECH`, `PROC` (`PROC` also takes `PrerequisiteProcAlternate=`) | `PrerequisitePower=GAPOWR,NAPOWR,NANRCT,YAPOWR`; `PrerequisiteProcAlternate=SMIN` |
| `Adjacent=` | Most empty cells allowed between the new footprint and an owned base building | 2 on most structures, 4 on defences, 8 on walls, 12 on shipyards |
| `BaseNormal=` | `no` means the building does not extend the buildable area | defences (`GAPILL`, `ATESLA`, `NASAM`) and walls |
| `FreeUnit=` | Unit that rolls out once the building is placed | `GAREFN`: `CMIN`, `NAREFN`: `HARV` |
| `Refinery=` | Takes harvester deliveries | `GAREFN`, `NAREFN` |
| `ConstructionYard=`, `Factory=BuildingType` | Hosts the structure queue | `GACNST` (also `TechLevel=-1`: deployed from an MCV, never built) |
| `Wall=`, `WaterBound=` | Walls (overlay-like) and shipyards | `GAWALL`, `GAYARD` |
| `RequiredHouses=`, `ForbiddenHouses=` | Country gates | `AMRADR` Americans only, `GAAIRC` not Americans, `SNIPE` British only |

YR does not gate sides through `Owner=`: every retail structure lists all
ten playable countries there (`GAPOWR` includes `Russians` and
`YuriCountry`). The side gate is the construction yard in `Prerequisite=`
(`GAPOWR` needs `GACNST`, `NAPOWR` needs `NACNST`). The sidebar therefore
offers the structures whose `Prerequisite=` names the player's yard, with
`TechLevel=` from 1 to the skirmish level (10), filtered by the player's
country. `[Countries]` lists `0=Americans` first ("dialog returns 0 for the
first listing"), so the sandbox player is American. Walls, shipyards and
the yard are left out. On EB1 that gives 16 Allied structures after the
unit slots.

Placement rules in the sim (`Sim::can_place`), shared by the window ghost
and the `PlaceProduced` command:

- every footprint cell is on the grid, has no ramp and sits at one height
  level;
- every cell is land a tracked unit could enter (no water, cliff, wall or
  building) with no overlay (ore, gems, debris) and no ground unit on it;
- some footprint cell comes within `Adjacent=` empty cells (Chebyshev) of a
  building the same owner has with `BaseNormal=` not `no`. `Adjacent=0`
  therefore means touching, the ModEnc reading; the exact radius and the
  ore and slope rules are not yet verified against the Wine oracle.

A structure costs and builds like a unit (gradual payment, `BuildSpeed`
time, low-power slowdown), one at a time, then waits at the head of its
queue until placed; cancelling a finished one refunds the full `Cost=`.
Audio: `EVA_ConstructionComplete` when it finishes, and the sound named by
`[AudioVisual] BuildingSlam=` (`PlaceBuilding`, sample `uplace`) when it goes
down.

## Harvester docking

The retail `GAREFN` art foundation is 4x3. Testing adjacency to the
footprint centre leaves every accepted cell inside the blocked building,
so a full harvester never delivers (#68). The prototype now accepts the
exterior one-cell ring around the first owned refinery's whole footprint.
The movement system approaches its centre and substitutes a standable
cell outside it. Legacy refinery unit markers use a 1x1 footprint.
This is a prototype docking policy; exact docking cells, queue reservations
and Chrono Miner teleportation still need Wine validation.

The harvester must remain beside a live owned refinery throughout its
30-tick unloading timer. Destroying the host or moving away preserves the
cargo and restarts the approach; a replacement refinery allows delivery
to resume (#69). No credits arrive at a destroyed or enemy refinery.
These checks and the saved integer timer are part of the simulation.
`Sim::refinery_unloading` reads elapsed ticks for presentation without
advancing state or changing its hash. The snapshot layout is unchanged.

## Not yet modelled

- Ares-style `Inherit=`/`+=` (not vanilla).
- Engine defaults per key, `[General]`/`[CombatDamage]` globals, countries,
  superweapons, animations, terrain/overlay types, and map-specific sections.

## Radar presentation flags

`TechnoType` carries optional `Radar=` and `RadarInvisible=` booleans through
the existing INI first-character boolean parser. The sidebar reads them
from the merged rules. Radar providers are matched by owned building kind
and gated by the player's actual power balance, independently of enemy
power. These are presentation metadata and do not alter simulation hashes
or save compatibility. Retail providers, transition art, tests and remaining
comparisons are recorded in [radar presentation](radar.md).
