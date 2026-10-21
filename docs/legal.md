# Legal posture

This project is a **from-scratch engine** for a game whose source code has never
been released (EA's February 2025 source release covered Generals/Zero Hour,
Renegade, Tiberian Dawn and Red Alert 1, **not** Tiberian Sun or Red Alert 2/YR).
Everything here is built by observation and documentation.

## Rules

1. **No EA assets in this repository.** No art, audio, video, maps, INI files, or
   strings from the game. Users supply their own legally owned install at runtime.
2. **No copied code.** Nothing from the original binary, YRpp, Phobos, Ares, XCC
   Utilities, or any other project is transplanted into this repo. Behaviour and
   file-format knowledge is fine; expression is not. When we consult such projects,
   it is to confirm observable behaviour, and the work here is written fresh.
3. **Documentation of facts is fine.** File formats, algorithms and constants
   (e.g. the Westwood MIX public key, which is a number) are recorded in
   `docs/formats/` with evidence from our own binary inspection.
4. **New content must be original.** RA3-inspired units are original creations in
   the *style* of the genre, no traced or extracted EA art, no EA voice lines, no
   trademarked logos or names in shipped assets.

## Precedent summary (detail in `docs/research/`)

| Project | Rights-holder response | Outcome |
|---|---|---|
| re3 / reVC (GTA) | Take-Two DMCA + lawsuit (2021) | Settled 2023, terms undisclosed; repo mirrors persist |
| DevilutionX (Diablo) | No recorded action | Alive; user-supplied MPQs, non-commercial licence |
| OpenRCT2 | No recorded action | Alive; requires original data files |
| OpenRA | Tolerated, C&C community engagement | Alive |
| Thyme (Generals) | No recorded action | Pre-2025 source release; GPL-2.0 |
| GeneralsGameCode | EA released source 2025 | GPL-3.0 official continuation |

Lessons: require user-owned data, keep the engine separate from assets, avoid
distributing anything derived from the binary, and prefer behaviour-level
compatibility claims over "this is the original code". EA has engaged positively
with the C&C community (OpenRA, CnCNet) since ~2018, but there is no explicit
blessing for RA2/YR reimplementations.

## Publishing

This repo is currently **local and private**. Publishing is a deliberate decision
to take later: keep it private until M3+ shows the shape of the work, then publish
engine code only (it already excludes assets). Not legal advice, just the
precautionary posture adopted from the precedents.

## Licence

GPL-3.0 (see `LICENSE`), matching the C&C reimplementation norm
(GeneralsGameCode GPL-3.0, Thyme GPL-2.0). Revisit if we ever need to combine
code with GPL-2.0-only references.
