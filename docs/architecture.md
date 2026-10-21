# Architecture

## Goal

A native Rust engine that runs Red Alert 2: Yuri's Revenge assets on modern macOS,
structured so that the simulation is faithful and the presentation is native.
Design influences: the hook-and-replace precedents (re3, OpenRCT2) for fidelity
method, the clean-room reimplementations (OpenRCT2, OpenRA, VERA20k) for engine
structure, and the Rust RTS experiments (VERA20k, Iron Curtain, RA2NE) for
determinism discipline.

## Crate plan

```
crates/
  yr-formats    pure parsers: MIX, CSF, INI, then SHP, VXL, HVA, PAL, AUD/WSA/BIK metadata
  yr-data       typed game data: rulesmd.ini/artmd.ini -> TechTypes, Weapons, Warheads, ...
  yr-assets     VFS over the game install: MIX cascade, name database, caching, streaming
  yr-sim        deterministic simulation (fixed-point, no I/O, no floats, no HashMap iteration)
  yr-render     wgpu renderer (Metal backend): isometric tiles, SHP/VXL draw, palette effects
  yr-app        macOS app shell: window, input, audio (cpal/egui), asset path UI
tools/
  mix-dump      list/extract MIX archives (implemented)
  sim-bench     headless sim benchmark (later)
  parity-digest state hash / replay comparison against the Wine oracle (later)
```

Dependency direction (strict, enforced by cargo):

```
yr-app -> yr-render -> yr-sim -> yr-data -> yr-formats
yr-app -> yr-assets -> yr-formats
```

`yr-sim` never touches filesystem, GPU or wall-clock time.

## Key decisions

### Determinism first

Replay compatibility is the fidelity acid test (OpenBW precedent): RA2/YR replays
store commands, so a faithful engine must re-simulate identically. Rules adopted
from Iron Curtain / RA2NE / VERA20k:

- fixed-point maths only in `yr-sim` (`i32`/`i64` with a scale factor), never `f32`
- no `HashMap`/`HashSet` iteration order in sim paths; use `BTreeMap`/`IndexMap`
- fixed tick rate and explicit tick-phase order mirroring the original update order
- deterministic RNG with seed in game state; state hashing for parity checks

### Hand-rolled sim, not ECS

The only finished Rust reimplementation (doukutsu-rs) and the playable Rust RA2
engine (VERA20k) both use plain data-oriented structs in tick order. ECS system
scheduling is an extra determinism hazard without payoff at this scale. Revisit
only if entity counts demand it.

### Presentation behind traits

`yr-sim` exposes pure state + command interface; `yr-render` observes. This keeps
the Iron Curtain split ("UI never mutates authoritative sim state") and lets a
headless mode exist from day one.

### macOS specifics

- wgpu Metal backend (OpenGL is deprecated on macOS); integer-scaled pixel viewport
  for the 640x480-era UI, native resolution for cursor/UI chrome
- cursor confinement for edge-scrolling is a known pain point (winit `Confined` is
  unreliable on macOS), budget AppKit-level work here
- distribution: app bundle, Developer ID + hardened runtime + notarisation from CI
  (OpenRA's build scripts are the template); ad-hoc signing for local dev builds

### Clean-room discipline

File-format and behaviour knowledge comes from observation (binary inspection of
retail files, behaviour under Wine) and from published documentation, not from
copying YRpp/Phobos/XCC source into this repo. Format evidence lives in
`docs/formats/`. See `docs/legal.md`.

## Testing strategy

1. **Retail corpus tests**: parsers validated against the user's own archives
   (`langmd.mix`, `theme.mix` plaintext; `ra2md.mix`, `expandmd01.mix` encrypted).
   Known-answer tests: `RA2MD.CSF` hash must match its index ID, header sizes must
   match file sizes exactly, CSF counts must round-trip.
2. **Property/edge tests**: truncated files, empty archives, odd name lengths for
   the hash function.
3. **Parity tooling** (later), via `parity-digest` compares sim state hashes against
   the Wine install as oracle.
