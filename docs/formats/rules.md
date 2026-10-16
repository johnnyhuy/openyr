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

## Not yet modelled

- Ares-style `Inherit=`/`+=` (not vanilla).
- Engine defaults per key, `[General]`/`[CombatDamage]` globals, countries,
  superweapons, animations, terrain/overlay types, and map-specific sections.
