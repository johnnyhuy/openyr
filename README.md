# OpenYR

A native, reverse-engineered rewrite of **Command & Conquer: Red Alert 2 — Yuri's Revenge** for modern macOS (Apple Silicon first). Long-term goal: a faithful, moddable engine that runs the original game assets natively — no Wine — so we can ship patches, quality-of-life fixes and new units.

> Working name. This is a from-scratch engine in Rust that reads the original Westwood data files (MIX/CSF/INI/SHP/VXL/…). It ships **no EA assets**; you supply your own legally owned game install.

## Why

Wine runs YR on macOS, but it is a compatibility layer: resolution, input, colour depth and modding are all capped by a 2001-era Windows binary. A native engine gives:

- modern macOS: Retina/integer scaling, real fullscreen, Metal, gamepads, no Rosetta
- a moddable codebase: data-driven units (including new ones), scripted QoL patches
- a foundation for content additions (e.g. RA3-style units with new art and audio)

## Status

**Pre-alpha — asset layer in progress.** See [docs/roadmap.md](docs/roadmap.md).

| Area | State |
|---|---|
| MIX archive reader (incl. encrypted headers) | done |
| CSF string tables | done |
| INI rules parser | done |
| PAL colour tables | done |
| SHP / VXL / HVA / TMP readers | done (whole retail corpus decodes) |
| MIX name recovery (~88% of entries) | done |
| `mix-dump` inspection CLI (`scan`, nested `extract`) | done |
| PCX images | done |
| Audio (AUD, sound bank, IMA WAV; bit-exact vs ffmpeg) | done |
| `yr-assets` VFS | next |
| Simulation | not started |
| Renderer / macOS app | not started |

## Layout

```
crates/yr-formats/   pure parsers for Westwood file formats (no I/O side effects)
tools/mix-dump/      CLI to list/extract MIX archives from a game install
docs/                architecture, roadmap, format specs, research notes
```

## Building

```sh
cargo build --workspace
cargo test --workspace
```

Tests read the real game archives from `~/Games/RA2YR/prefix/drive_c/Westwood/RA2`
when present (skipped otherwise), so they double as a parity check against retail data.

## Using your own game data

Point the tools at your own install:

```sh
cargo run -p mix-dump -- list "/path/to/RA2/langmd.mix"
cargo run -p mix-dump -- extract "/path/to/RA2/ra2md.mix" RULESMD.INI out/
cargo run -p mix-dump -- extract "/path/to/RA2/ra2.mix" LOCAL.MIX/HTNK.VXL out/
cargo run --release -p mix-dump -- scan /path/to/RA2/*.mix   # census + names
cargo run --release -p mix-dump -- render \
  /path/to/RA2/ra2.mix:SNOW.MIX/GACNST.SHP /path/to/RA2/ra2.mix:CACHE.MIX/UNITSNO.PAL gacnst.png
```

## Legal

See [docs/legal.md](docs/legal.md). Short version: clean-room method, no EA code or
assets in this repository, user-supplied assets only. Precedent (re3, DevilutionX,
OpenRCT2, Thyme) is summarised in `docs/research/`.
