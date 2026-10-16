# Clean-room / behaviour-level open-source game engine reimplementations (lessons for a native Rust, macOS-first RA2:YR rewrite)

Research date: 2026-10-16. GitHub metadata (stars, created/pushed dates, licence as reported by the GitHub API, contributor counts) was pulled live via `gh api` on 2026-10-16; where GitHub reports `NOASSERTION` the licence is noted as such.

## Q1. Project profiles: how gameplay knowledge was obtained, architecture, assets, determinism, platform, modding, timeline, team, status, licence

### Takeaway
"Clean-room" covers a spectrum: OpenRA, OpenSAGE, doukutsu-rs and openage are genuinely from-scratch/behavioural; OpenTTD, DevilutionX, CSE2 and the 2026 RA2/TS projects (OpenTS, VERA20k) are decompilation/RE-derived. The from-scratch projects that shipped (OpenRA, doukutsu-rs, fheroes2) did so by accepting divergence or by targeting small, well-documented games; the RA2-generation C&C engine is the hardest case and, as of Oct 2026, the only playable RA2/YR reimplementations are RE/AI-driven and very young.

### Cited Findings

**OpenRA (C#, GPL-3.0)**
- Started 2007 by "3 of us in a basement" asking "how hard can it be to rebuild this"; first attempt was a Windows/D3D9 game that was shelved; later revived; early decision to move away from original RA1 map/rules formats to "custom and much more flexible formats" — [OpenRA Vice interview (Chris Forbes), 2018-07-27](https://www.openra.net/news/vice-interview/)
- GitHub repo created 2010-10-04; 17,574 stars; 454 contributors (2026-10-16) — `gh api repos/OpenRA/OpenRA`
- Knowledge source: developers state they are "(hobbyist) game developers, not reverse-engineers"; a faithful recreation "is impossible without having a detailed understanding of how they worked"; "I'm convinced that OpenRA would have died just like freecnc/freera/open red alert/etc did if we had gone down that path" — [CnCNet forums thread quoting OpenRA dev, 2014-12-29](https://forums.cncnet.org/topic/2536-what-do-you-think-about-openra/)
- Early OpenRA "did (try to) recreate the original balance... even used the original ini and map files"; INI was abandoned "because it did not feature nesting" — [same CnCNet thread](https://forums.cncnet.org/topic/2536-what-do-you-think-about-openra/)
- Critic's view: "OpenRA has neither reverse engineered anything, nor used original source code... It doesn't even use all original files; just the graphics and sounds" — [CnCNet forum, 2013](https://forums.cncnet.org/topic/1454-an-open-question-to-openra-folk/)
- After EA's 2020 GPL source release of TD/RA, OpenRA said it "cannot directly copy the source code, but will use it instead as a reference"; first fix was the original palette remap tables for civilian infantry — [OpenRA devblog 2020-06-29](https://www.openra.net/news/devblog-20200629/)
- Deliberate divergence: "OpenRA's goal is to envision what these games may instead have been were they developed again now - with features such as Attack Move, multiplayer-focused balance, and extensive modding" — [OpenRA devblog 2020-06-29](https://www.openra.net/news/devblog-20200629/)
- Divergence origin: many changes "were introduced in the early days of OpenRA to help balance the game and make it play well despite missing core gameplay features (... 5 infantry sharing the same cell or a proper implementation of the 'classic' engineer behaviour)", then "became entrenched"; the "Original Red Alert" vs "Original OpenRA" split "caused significant conflict" — [OpenRA devblog 2018-06-10](https://www.openra.net/news/devblog-20180610/)
- Lesson stated by OpenRA: releasing TS with disabled/broken features and rebalancing around them "is not a path we want to repeat after our experiences with Red Alert" — [OpenRA devblog 2018-06-10](https://www.openra.net/news/devblog-20180610/)
- Architecture: solution split into `OpenRA.Game`, `OpenRA.Mods.Common`, `OpenRA.Mods.Cnc`, `OpenRA.Platforms.Default`, `OpenRA.Server`, `OpenRA.Utility`, `OpenRA.Launcher` — `gh api repos/OpenRA/OpenRA/git/trees/bleed`
- Trait-based actors: published trait reference lists C&C-specific traits like `AttackTesla`, `Chronoshiftable`, `JumpjetLocomotor`, `Disguise`, `GpsPower`, `MadTank`, `IonCannonPower` — [docs.openra.net traits](https://docs.openra.net/en/release/traits/)
- Determinism: world positions are integer structs (`WPos` with `int X, Y, Z`); RNG is a `MersenneTwister` class — `OpenRA.Game/WPos.cs`, `OpenRA.Game/Support/MersenneTwister.cs` (bleed branch, via `gh api`)
- Desync detection: fields marked `[VerifySync]` on `ISync` objects are hashed via IL-generated functions (custom hashers for `WPos`, `CPos`, `WAngle`, `Actor`, `Player`, `Target`); clients send `SyncHash` + defeat state each net frame and `OutOfSync(frame)` fires on mismatch — `OpenRA.Game/Sync.cs`, `OpenRA.Game/Network/OrderManager.cs`
- Lockstep: orders are sent per net frame; a frame only advances when `pendingOrders.All(p => p.Value.Count > 0)`; net frames occur every `NetFrameInterval` local ticks; replays reuse the same path (`ReplayConnection`) — `OpenRA.Game/Network/OrderManager.cs`
- Releases: latest stable `release-20250330`; playtests `playtest-20260222` and `playtest-20261011` — `gh api repos/OpenRA/OpenRA/releases`; [openra.net download page](https://www.openra.net/download/)
- 2026 features: random map generators (playtest-20260222); save/load overhaul + QuickSave; TD HD mod using Remastered assets "feature-complete" with content manager to pick remastered or classic art — [openra.net front page, 2026-10-11](https://www.openra.net/)
- Assets: TS mod uses `FileSystem: ContentInstallerFileSystem` with `ContentPackages` pointing at original `.mix` files (cache.mix, conquer.mix, isosnow.mix, sidec01.mix...) in the user's support dir — `mods/ts/mod.yaml` (bleed)
- Asset readers in `OpenRA.Mods.Cnc`: `VxlReader`, `HvaReader`, `VoxelLoader`, `TmpTSLoader`, `ShpRemasteredLoader`, `Blowfish`/`BlowfishKeyProvider` (MIX header), `LZOCompression`, `XccGlobalDatabase`, `VqaVideo`, `RenderVoxels` — `gh api` tree listing
- OpenRA "supports voxels and TS/RA2 terrain with slopes... but doesn't support all features from those games yet"; SHP, PNG or voxel usable — [PPM forums, 2025-02-17](https://ppmforums.com/topic-72190/questions-about-openra-cnc-remastered-and-tiberian-sun/)

**OpenRA TS and RA2 mods**
- TS milestone "Tiberian Sun first 'full' (beta) release" — "An incomplete list of blockers", includes "Add support for voxel projectiles (TS/RA2)"; last updated 2025-12-29 — [GitHub milestone 30](https://github.com/OpenRA/OpenRA/milestone/30)
- TS in 2018: "broadly playable" but missing super weapons; subterranean units and cloak generators had game-breaking bugs — [OpenRA devblog 2018-06-10](https://www.openra.net/news/devblog-20180610/)
- `OpenRA/ra2` repo: created 2015-12-13, last push 2026-07-17, 1,137 stars, 39 contributors; recent commits are mostly CI bumps plus occasional fixes (e.g. 2025-11-28 IFV/engineer targeting) — `gh api repos/OpenRA/ra2`
- RA2 mod: "still in development"; requires "the original disk or a digital install (e.g. Origin/EA App)"; "the current ETA is never. Progress is really slow" — [OpenRA forum thread, 2023](https://forum.openra.net/viewtopic.php?t=21799)
- Community view (Sept 2026): "RA2 (and Tib Sun) are in a completely different engine to TD/RA/D2000. Any similar thing would need to be a new project" — [r/openra, 2026-09-09](https://www.reddit.com/r/openra/comments/1wbf9nv/red_alert_2_is_there_any_plans_to_support_it/) (Reddit user opinion, not official)

**OpenTTD (C++)**
- 2003: Ludvig Strigeus announced he would "reverse engineer Transport Tycoon Deluxe and convert the game to C"; v0.1 released 2004-03-06; early work restructured code for readability/extensibility and replicated TTDPatch features; multiplayer reprogrammed over IP; stable by late 2000s; stable 15.3 as of 2026 — [Wikipedia: OpenTTD](https://en.wikipedia.org/wiki/OpenTTD)
- GitHub repo (migration) created 2018; 8,375 stars; 269 contributors; GitHub licence field NOASSERTION — `gh api repos/OpenTTD/OpenTTD`
- "OpenTTD was also originally a reverse engineered engine like opendune and behaved almost exactly like the original" — [CnCNet forum post, 2014](https://forums.cncnet.org/topic/2536-what-do-you-think-about-openra/)

**OpenMW (C++, GPL-3.0+)**
- 0.1.0 released June 2008, initially on Ogre3D; switched to OpenSceneGraph at 0.37.0 due to concerns about Ogre3D's direction, bringing significant performance improvements; "Since 2016, all of the quests, classes, races... are fully playable"; stable 0.50.0 released 2025-11-07 — [Wikipedia: OpenMW](https://en.wikipedia.org/wiki/OpenMW)
- Leadership changed: original lead Nicolay Korslund left early, passed to Marc Zinnschlag; Bret Curtis (psi29a) took over in 2020 — [Wikipedia: OpenMW](https://en.wikipedia.org/wiki/OpenMW)
- 511 contributors; 6,633 stars; `openmw-51-rc1`/`rc2` tags exist — `gh api repos/OpenMW/openmw`

**VCMI (Heroes 3, C++, GPL-2.0)**
- "open-source recreation of Heroes of Might & Magic III engine"; plans major releases ~3x per year; latest 1.7.x line; saves across major versions usually not supported; installs for Windows/macOS/Linux/Android/iOS — [VCMI README](https://github.com/vcmi/vcmi)
- Repo created 2014-04 (project older); 5,947 stars; 242 contributors — `gh api repos/vcmi/vcmi`

**openage (Age of Empires II, C++/Python)**
- Repo created 2013-11-03; 14,509 stars; 228 contributors — `gh api repos/SFTtech/openage`
- Releases: v0.3.0-alpha (2019-06), v0.4.0 "4 years of progress" (2019-06), v0.4.1 "Last release before lockstep removal" (2020-08), v0.5.0 (2023-10), v0.6.0 (2024-11) — `gh api repos/SFTtech/openage/releases`
- 2020: new gamestate "replaces the lockstep implementation... with an event-driven system" using a "curve-based simulation model"; "the old gamestate implementation will no longer work... the gameplay features that were implemented along with it will no longer work"; they declined to keep parallel structures — [openage blog, 2020-08-15](https://blog.openage.dev/new-gamestate-2020.html)
- v0.5.0: "The gameplay found in this release is not much more than a tech demo"; new converter supports all Genie games; new threaded OpenGL renderer, nyan-backed game data, component-based architecture — [openage v0.5.0 release, 2023-10-03](https://github.com/SFTtech/openage/releases/tag/v0.5.0)
- Renderer stress test "barely managed handling 400 objects with 30 FPS" because of a per-frame call into the Python API for texture-cache lookups; after removal, >3,500 objects at 30 FPS — [openage blog, Aug 2023](https://blog.openage.dev/openage-development-news-august-2023.html)
- Converter "rivals the actual engine code in size" with files of "up to 10,000 lines"; must chase AoE2:DE data-format changes — [openage blog, 2025-07-14](https://blog.openage.dev/)
- v0.6.0: flow field pathfinder; node-graph "activity system" for unit behaviour configured via data files — [openage v0.6.0 release, 2024-11-26](https://github.com/SFTtech/openage/releases/tag/v0.6.0); [openage blog 2025-07](https://blog.openage.dev/)
- Asset conversion took up to ~960 s for AoE2:DE before multithreading, ~250 s after — [openage blog, nyan tag, 2024-02](https://blog.openage.dev/tag/nyan.html)

**OpenSAGE (C&C Generals, C#)**
- "This is a blackbox re-implementation project. The code... was written based on reading data files, and observing the game running... (for example refpack decompression) ... based on specs available on the Internet"; "No code that was acquired through reverse engineering executable binaries will be accepted" — [OpenSAGE README](https://github.com/OpenSAGE/OpenSAGE)
- "very early stages... a long way to go before there's anything playable"; "initial focus is on understanding the data formats"; platforms listed: Windows (OpenGL 4.3, D3D11), Mac (Metal 2, High Sierra+), Linux (OpenGL 4.3) — [OpenSAGE README](https://github.com/OpenSAGE/OpenSAGE)
- Repo created 2017-06-14; last tagged release v0.3.0 (2018-10); recent commits (Dec 2025) are Dependabot bumps; last push 2026-03-30; 35 contributors; 1,649 stars — `gh api repos/OpenSAGE/OpenSAGE`

**Julius / Augustus (Caesar III, C, AGPL-3.0)**
- Julius: "fully working... with the same logic as the original"; "saved games are 100% compatible with Caesar 3, and any gameplay bugs present in the original... will also be present"; enhancements limited to widescreen/windowed/QoL; gameplay changes live in the Augustus fork — [Julius README](https://github.com/bvschaik/julius)
- Julius repo 2017-08, latest v1.8.0 (2025-07-31), 59 contributors; Augustus repo 2020-01, v4.0.0 (2023-12) — `gh api`

**fheroes2 (HoMM2, C++, GPL-2.0)**
- "recreation of Heroes of Might and Magic II game engine"; advises purchasing on GOG/Ubisoft or using the free demo data — [fheroes2 README](https://github.com/ihhub/fheroes2)
- Repo 2019-08-30; 75 releases; latest 1.1.17 (2026-06-30); 194 contributors — `gh api repos/ihhub/fheroes2`

**DevilutionX (Diablo, C++)**
- "a port of Diablo and Hellfire" with engine improvements/QoL; needs `DIABDAT.MPQ` from CD/GOG (with a GOG-installer extraction guide) or shareware `spawn.mpq` — [DevilutionX README](https://github.com/diasurgical/devilutionX)
- The Diablo decompilation benefited from leaked debug info ("the same lucky-break the Diablo decompilation project had") — [CSE2 page quoting CSE2 readme](https://wiki.doukutsu.club/cse2)
- Repo 2018-08; 9,869 stars — `gh api`

**OpenRCT2 (contrast, C++, GPL-3.0)**
- Started 2014-04-02 by Ted "IntelOrca" John; ~250 other contributors — [Wikipedia: OpenRCT2](https://en.wikipedia.org/wiki/OpenRCT2)
- RCT2 "was originally written by Chris Sawyer in x86 assembly"; OpenRCT2 "requires original files of RollerCoaster Tycoon 2", adds increased limits and cooperative multiplayer — [OpenRCT2 README](https://github.com/OpenRCT2/OpenRCT2)
- Changelog notes "DirectDraw, DirectInput, DirectPlay and DirectSound dependencies are no longer used" and lists fixes for "(original bug)" items — OpenRCT2 changelog (indexed via `gh api`)

**doukutsu-rs (Cave Story, Rust, MIT)**
- "not a decompilation, as it's written from scratch and does not contain the source code of the original game (unlike CSE2...)"; "originally started as a Rust learning project and meme"; "Starting with version 0.99.0-beta1, released on January 16, 2022, the game is fully playable" — [doukutsu-rs FAQ](https://doukutsu-rs.gitbook.io/docs/faq)
- First commit 2020-08-18; 1.0.0 released 2026-06-03; drop-in replacement that mimics freeware, CS+, Switch, CSE2 and NXEngine behaviour depending on data present; Lua scripting existed early but was removed in 0.101.0-beta6 — [Cave Story Modding Wiki: doukutsu-rs](https://wiki.doukutsu.club/doukutsu-rs)
- 34 contributors; 1,313 stars — `gh api`

**2026 RA2/TS-generation projects (directly relevant)**
- OpenTS (C++): released 2026-08-27; "Building on EA's GPL-released code for the games around it and years of Tiberian Sun reverse engineering, we rebuilt the engine and matched it function by function against the original executable – all ~11,150 known functions, reaching roughly 98% instruction-level matching" — [r/commandandconquer announcement](https://www.reddit.com/r/commandandconquer/comments/1vzsssz/today_were_releasing_opents_a_reverseengineered/)
- OpenTS 0.1.0: full TS 2.03 Firestorm, campaigns/skirmish/save-load play-through tested, LAN with limited testing; bgfx renderer to 4K/ultrawide; Windows only; roadmap includes feature parity with Vinifera/ts-patches, "striving toward feature parity with Red Alert 2 and Yuri's Revenge", and incremental modernisation "toward an entity-component architecture" — [OpenTS repo](https://github.com/OpenTS-Developers/OpenTS); [TechSpot listing, 2026-09-02](https://www.techspot.com/downloads/7903-opents.html)
- OpenTS frozen source snapshot targets `GAME.EXE v2.03[EN]` (MD5 given); ~95% of functions byte-matched at drop; GPL-3.0+ — [OpenTS-Developers/TibSun](https://github.com/OpenTS-Developers/TibSun)
- VERA20k (Rust, GPL-3.0): repo 2026-03-29, 8 contributors, 98 stars; "Most of the code is written by AI coding agents. They use Ghidra to study the original engine, then port its behavior to Rust and check it with comparison tools and playtesting"; status "Mid development. Local skirmish is playable on Windows against a basic AI... Multiplayer, campaigns and the original AI are still missing"; "has been played on Windows, Linux and macOS" — [VERA20k README](https://github.com/YuriPlanet/vera20k)
- VERA20k AGENTS.md: targets `gamemd.exe` with retail rules/assets; uses `SimFixed` (the `fixed` crate) for sim math; executes native code under Unicorn as an oracle for kernels (lookup tables, fixed-point conversion, distance/angle math, RNG draws); "gamemd behavior is native code plus the INI keys it reads; port both"; native INI value readers rather than `str::parse` — [VERA20k AGENTS.md](https://github.com/YuriPlanet/vera20k/blob/main/AGENTS.md)
- Iron Curtain (Rust + Bevy, GPL-3.0-or-later "with modding exception"): early development, "no playable game build exists yet"; runnable Bevy "content lab" validating real `.mix/.meg/.shp/.pal/.aud/.vqa/.ini/.tmp` resources; sibling repos for clean-room format parsers (`cnc-formats`), `fixed-game-math`, `deterministic-rng`, and `cnc-content` acquisition — [Iron Curtain README](https://github.com/iron-curtain-engine/iron-curtain)
- RA2NE (Rust, 2026-09-30, no licence): fixed 30 TPS sim, integer coords, deterministic RNG, world-state hash, TCP lockstep with peer verification, INI frontend with UTF-8/Windows-1252/GBK decoding; "Overall feature coverage is estimated at approximately 25%"; "Original gameplay and ordinary MOD compatibility remain unverified" — [RA2NE README](https://github.com/18611429192/ra2ne)

### Inferences
- For RA2:YR specifically, there is no from-scratch, clean-room engine that has reached playable faithful status; OpenRA's RA2 mod has been in development ~11 years (2015-2026) without release, while RE-based OpenTS reached a complete TS in a single release and VERA20k reached local skirmish in ~6.5 months with AI agents + Ghidra.
- OpenTS's stated roadmap toward RA2/YR parity means a GPL C++ codebase of the same engine generation now exists; a Rust project should decide early whether it treats OpenTS/VERA20k as references (licence/legal implications: GPL, RE-derived) or stays strictly behavioural.

### Gaps
- No primary source found for VCMI's, Julius's or fheroes2's knowledge-acquisition method (disassembly vs observation). Julius's "same logic, bug-for-bug, save compatible" claim strongly implies binary study but I found no explicit statement.
- No primary source found confirming OpenRCT2 began as an incremental decompilation hooking into `rct2.exe` (widely reported; not verified here).
- Veloren is an original game, not a reimplementation; I did not research its stack beyond metadata (2018, Rust, GPL-3.0, 7,628 stars).
- Could not retrieve OpenRA's MiniYAML wiki page (moved/404); MiniYAML inheritance syntax details not captured here.

## Q2. Which projects reached "playable, faithful", how long, and what separated finishers from stalled ones?

### Takeaway
Finishers either (a) derived exact logic from binaries/source (OpenTTD in ~1 year, DevilutionX, OpenTS, CSE2), or (b) picked a small, bounded target and shipped incrementally (doukutsu-rs ~17 months to fully playable, Julius), or (c) accepted divergence and iterated with players (OpenRA). Stalled/slow projects (openage 13 years, OpenSAGE 9 years, OpenRA RA2 ~11 years) over-invested in generic engine architecture or format work and repeatedly rewrote the core before gameplay existed.

### Cited Findings
- OpenTTD: RE announced 2003, v0.1 on 2004-03-06 (~1 year), stable and popular by late 2000s — [Wikipedia: OpenTTD](https://en.wikipedia.org/wiki/OpenTTD)
- OpenMW: 0.1.0 June 2008 to "all of the quests... fully playable" in 2016 (~8 years); still pre-1.0 at 0.50.0 in Nov 2025; swapped render engine mid-project (Ogre3D -> OSG at 0.37) — [Wikipedia: OpenMW](https://en.wikipedia.org/wiki/OpenMW)
- doukutsu-rs: first commit 2020-08-18, fully playable 2022-01-16 (~17 months), 1.0.0 on 2026-06-03 — [doukutsu-rs FAQ](https://doukutsu-rs.gitbook.io/docs/faq); [Modding wiki](https://wiki.doukutsu.club/doukutsu-rs)
- CSE2 decompilation was fast because Cave Story was compiled "with no optimizations" and a 2011 Linux port leaked "the names of every C++ source file, as well as the variables, functions, and structs" — [CSE2 wiki](https://wiki.doukutsu.club/cse2); it received Nicalis DMCA takedowns on 2020-11-19 and 2020-12-08 — [same](https://wiki.doukutsu.club/cse2)
- OpenRA: 2007 start; shelved early prototype; gained a boost from TotalBiscuit's 2014 review — [Vice interview](https://www.openra.net/news/vice-interview/); OpenRA dev believes a disassembly-first approach would have killed it like freecnc/freera — [CnCNet 2014](https://forums.cncnet.org/topic/2536-what-do-you-think-about-openra/)
- OpenRA early hard-coding: "the engine still had lots of elements that were not very flexible or even completely hard-coded with the C&C/RA mods in mind, but over time we've gradually improved or rewritten those parts" — [Vice interview](https://www.openra.net/news/vice-interview/)
- openage: 2013 repo; lockstep removed 2020 and old gameplay intentionally broken; 2023 v0.5.0 "not much more than a tech demo"; 2024 v0.6.0 adds pathfinding/behaviour graphs; 2025 blog notes next interactions like "building construction" and "proper handling of unit deaths" still pending — [openage blog 2020](https://blog.openage.dev/new-gamestate-2020.html); [v0.5.0](https://github.com/SFTtech/openage/releases/tag/v0.5.0); [blog 2025-07](https://blog.openage.dev/)
- openage maintainer: devlogs slowed because "when the choice is between writing devlogs and coding, coding is still much more fun" — [openage blog 2025-07](https://blog.openage.dev/)
- OpenSAGE: 2017 start, "initial focus is on understanding the data formats", still "nowhere near playable"; no release since 2018 — [OpenSAGE README](https://github.com/OpenSAGE/OpenSAGE); `gh api` releases
- Rust RTS "oxidator" (Ruddle): Rust + WebGPU, TA/Spring-inspired, demo with 35,000 units and map/unit editors; last commit 2020-03-03, i.e. stalled ~6.5 years — [Ruddle/oxidator README](https://github.com/Ruddle/oxidator), `gh api`
- VERA20k: repo 2026-03-29 to playable local skirmish (Windows) by Oct 2026 using AI agents + Ghidra — [VERA20k README](https://github.com/YuriPlanet/vera20k)

### Inferences
- The strongest predictor of finishing is a cheap, authoritative oracle for "correct" behaviour (original binary, leaked symbols, released source, or a small observable game). Without one, projects either diverge (OpenRA) or stall in architecture (openage, OpenSAGE).
- openage's pattern (generic multi-game engine, new data language nyan, curve-based event sim, Python/C++ split, multi-game converter) is a cautionary tale for a solo/small Rust project: generality and "engine-first" design delayed gameplay for a decade. A YR rewrite should get a vertical slice (one map, one unit type moving/shooting via retail INI) before generalising.
- A late renderer/engine swap (OpenMW at 0.37, openage renderer rewrite) is survivable but costly; choosing a long-lived graphics API early (wgpu/Metal) reduces this risk.

### Gaps
- No postmortem with hours/person-years for any project; "team size" is approximated via GitHub contributor counts, which include drive-by contributors.
- No source found on VCMI's or fheroes2's time to "playable faithful".

## Q3. How did clean-room projects measure faithfulness without the binary?

### Takeaway
Behavioural projects relied on observation, published specs, player feedback and later released source; RE projects use byte/instruction matching or native-code oracles; the most practical shared tool is deterministic replays + state hashes for regression, plus save-game compatibility where formats are known.

### Cited Findings
- OpenSAGE: code written "based on reading data files, and observing the game running" and public specs — [OpenSAGE README](https://github.com/OpenSAGE/OpenSAGE)
- OpenRA: a lot of game feel "comes from the handling of random values... projectile inaccuracy, and speeds... harvesting speed, build speeds", which they lacked; gameplay "evolved based on player and developer feedback" — [CnCNet 2014](https://forums.cncnet.org/topic/2536-what-do-you-think-about-openra/)
- OpenRA used EA's 2020 GPL source "as a reference to learn what made the original games tick" (palette remap tables) — [OpenRA devblog 2020-06-29](https://www.openra.net/news/devblog-20200629/); D2K got "new weapon damage calculations that are more faithful to the original game" in playtest-20201213 — [OpenRA playtest-20201213](https://www.openra.net/news/playtest-20201213/)
- Julius uses save-file compatibility and bug-for-bug parity as its definition of fidelity — [Julius README](https://github.com/bvschaik/julius)
- doukutsu-rs FAQ: "Work is currently underway to further improve the engine and fix some discrepancies with the original game" — [doukutsu-rs FAQ](https://doukutsu-rs.gitbook.io/docs/faq)
- OpenTS: function-by-function matching against the original executable (~98% instruction-level), plus full campaign play-throughs — [Reddit announcement](https://www.reddit.com/r/commandandconquer/comments/1vzsssz/today_were_releasing_opents_a_reverseengineered/)
- VERA20k: Unicorn-emulated native code as an oracle for numeric kernels and RNG; "play it next to the original and tell us what feels wrong" — [VERA20k AGENTS.md](https://github.com/YuriPlanet/vera20k/blob/main/AGENTS.md); [README](https://github.com/YuriPlanet/vera20k)
- RA2NE: replay determinism smoke check comparing a reference log vs lockstep input and file round trip; experimental INI bridge applies unit/weapon properties "with explicit movement and timing calibration" — [RA2NE README](https://github.com/18611429192/ra2ne)
- OpenRA collects a "sysinfo database" of player hardware used to justify dropping legacy GL (29 of 45,302 entries) — [OpenRA PR #21143](https://github.com/OpenRA/OpenRA/pull/21143)

### Inferences
- For a behaviour-level YR rewrite, the community-documented INI semantics (ModEnc, Ares/Phobos docs, credited by VERA20k) plus side-by-side capture (video/frame timing of the original under Wine) are the realistic oracle; formulas that matter for "feel" (RNG use, ROF/speed timing, armour Verses) should be pinned with recorded tests.
- Telemetry like OpenRA's sysinfo is a cheap way to make platform-support decisions later.

### Gaps
- No public methodology document found from OpenRA or OpenMW describing systematic fidelity test suites (e.g. recorded behavioural regression tests against original footage).

## Q4. Asset handling: user-owned originals, installers/extractors, content-download flows

### Takeaway
Every successful project ships zero original assets and requires a legal copy; the best UX auto-detects common install locations (Steam/GOG/Origin/EA app/CD) and copies or mounts the needed archives into a support directory, with free demo/shareware/trial data as a fallback where it exists.

### Cited Findings
- OpenRA TS mod mounts original `.mix` archives from `~^SupportDir|Content/ts` via `ContentInstallerFileSystem` — `mods/ts/mod.yaml`
- OpenRA RA2 mod needs "the original disk or a digital install (e.g. Origin/EA App)" — [OpenRA forum](https://forum.openra.net/viewtopic.php?t=21799)
- OpenRA TD HD mod adds "a new content manager that allows you to select between remastered or classic artwork, audio, and music" — [openra.net, 2026-10-11](https://www.openra.net/)
- openage: CLI startup flow; "Common installation folders for games can be automatically searched"; if no installation, "the converter is now able to download the AoC Trial Version as a fallback" — [openage blog Aug 2023](https://blog.openage.dev/openage-development-news-august-2023.html)
- openage converts assets into its own modpack formats (cost: up to 16 min for AoE2:DE before multithreading) — [openage blog 2024-02](https://blog.openage.dev/tag/nyan.html)
- doukutsu-rs: drop-in next to `Doukutsu.exe`; on first start it "will automatically extract the additional resources that are embedded in the vanilla game into the data directory"; drop-in replacement for `CaveStory+.exe` with no file modification — [doukutsu-rs README](https://github.com/doukutsu-rs/doukutsu-rs)
- DevilutionX: copy `DIABDAT.MPQ` or extract from GOG installer; shareware `spawn.mpq` fallback — [DevilutionX README](https://github.com/diasurgical/devilutionX)
- fheroes2: GOG/Ubisoft purchase or free demo data — [fheroes2 README](https://github.com/ihhub/fheroes2)
- Julius: GOG, Steam or CD-ROM required — [Julius README](https://github.com/bvschaik/julius)
- OpenTS: "drop Game.exe into your Tiberian Sun install from The Ultimate Collection"; "OpenTS supplies the engine, not the game data" — [Reddit](https://www.reddit.com/r/commandandconquer/comments/1vzsssz/today_were_releasing_opents_a_reverseengineered/); [OpenTS repo](https://github.com/OpenTS-Developers/OpenTS)
- VERA20k: user sets `ra2_dir` in `config.toml` to the game folder — [VERA20k README](https://github.com/YuriPlanet/vera20k)
- Iron Curtain: separate `cnc-content` repo for "C&C content acquisition — download, verify, and manage"; content lab probes host-native default roots — [Iron Curtain README](https://github.com/iron-curtain-engine/iron-curtain); `gh search repos`
- RA2NE: `ra2ne-inspect audit` inspects user resource files "without modifying them"; VFS mounts replace whole files while the rules loader merges individual INI keys — [RA2NE README](https://github.com/18611429192/ra2ne)

### Inferences
- On macOS the RA2/YR retail install will typically come from the EA app/Steam on Windows or a Wine/CrossOver bottle; a macOS-first rewrite needs a "point to folder / scan Wine prefixes / copy .mix files into Application Support" flow rather than relying on registry keys.
- Keep originals read-only and mount them in a VFS (RA2NE/OpenRA pattern); layer mod files with per-key INI merge semantics matching the original's `rulesmd.ini` override behaviour.

### Gaps
- Could not retrieve OpenRA's RA/TS installer YAML (source definitions with registry keys / Steam app IDs / disc IDFiles) — files were not at the paths queried; exact detection mechanism not captured.

## Q5. Modding model: adding new units (data-driven traits, scripting, new asset formats)

### Takeaway
Two successful models: (1) OpenRA-style composable traits configured in a nested data format (MiniYAML) with Lua for missions and C# for new traits; (2) "faithful core + fork/extension" (Julius vs Augustus; OpenTS planning Ares/Phobos-like capabilities). Rust newcomers converge on tiered data -> script -> sandboxed WASM.

### Cited Findings
- OpenRA moved from original INI to a custom format because INI "did not feature nesting" — [CnCNet 2014](https://forums.cncnet.org/topic/2536-what-do-you-think-about-openra/); docs have separate references for Lua, sprite sequences, traits and weapons — [docs.openra.net](https://docs.openra.net/en/release/traits/)
- OpenRA dev: anyone wanting original behaviour "could easily write their own traits that implemented the original behaviour and package them up with their own game rules in a classic-recreation mod" — [CnCNet 2014](https://forums.cncnet.org/topic/2536-what-do-you-think-about-openra/)
- OpenRA can use SHP, PNG or voxel art — [PPM forums 2025](https://ppmforums.com/topic-72190/questions-about-openra-cnc-remastered-and-tiberian-sun/); Remastered SHP loader exists (`ShpRemasteredLoader`) — `gh api` tree
- Julius keeps vanilla logic; Augustus fork "is implementing many long-wanted gameplay changes" — [Julius README](https://github.com/bvschaik/julius)
- openage: entity behaviour as node graphs configurable "via the game data files, so almost everything can be configured without changing the code" but "no failsafes for checking if a graph is 'correct'" — [openage blog 2025-07](https://blog.openage.dev/)
- doukutsu-rs removed its Lua scripting in 0.101.0-beta6 — [Modding wiki](https://wiki.doukutsu.club/doukutsu-rs)
- Iron Curtain invariant: "Modding is Tiered (YAML -> Lua -> WASM)": "YAML for data (80% of mods), Lua for scripting (missions, abilities), WASM for total conversions"; WASM sandbox capability-based and fog-filtered; `ic-script` uses `mlua` and `wasmtime`; MiniYAML supported; roadmap includes "OpenRA export" — [Iron Curtain AGENTS.md](https://github.com/iron-curtain-engine/iron-curtain/blob/main/AGENTS.md)
- OpenTS roadmap: "growing engine capabilities that match or exceed the popular Yuri's Revenge engine extensions" — [TechSpot/OpenTS](https://www.techspot.com/downloads/7903-opents.html)
- VERA20k credits YRpp, Ares, Phobos, ModEnc — [VERA20k README](https://github.com/YuriPlanet/vera20k)

### Inferences
- For YR, keeping retail `rulesmd.ini`/`artmd.ini` as the native data format (with Ares/Phobos-style extension keys) preserves the huge existing mod ecosystem, whereas OpenRA's format switch is a big reason RA2 mods can't be reused directly. New units can be added as new INI sections + new keys, with PNG/glTF loaders added beside SHP/VXL.
- A scripting tier should be optional and deterministic (doukutsu-rs dropped Lua; openage shows unvalidated behaviour graphs can crash the engine).

### Gaps
- No source captured on OpenRA's actual MiniYAML inheritance/override syntax (`Inherits`, `^Abstract`, `-Trait:` removal) — documentation fetch failed.

## Q6. Platform layer lessons for macOS / Apple Silicon (Metal, notarisation, bundles, retina, input)

### Takeaway
OpenRA ships a notarised universal (x86_64+arm64) .app via a custom native launcher and still renders through SDL2+OpenGL profiles (ANGLE / GL 3.2 core / GLES); Apple deprecated OpenGL in 10.14, so a new Rust engine should target Metal via wgpu (as VERA20k does) and plan for code signing + notarisation from the start; doukutsu-rs shows the cost of skipping notarisation (users must right-click > Open).

### Cited Findings
- Apple: "OpenGL was deprecated in macOS 10.14... use the Metal framework instead"; Macs support OpenGL up to 3.2 — [Apple developer: About OpenGL for OS X](https://developers.apple.com/opengl/)
- OpenRA renderer selection tries `GLProfile.ANGLE`, then `Modern` (GL 3.2 core), then `Embedded`; sets `SDL_WINDOW_ALLOW_HIGHDPI`, computes window scale from `SDL_GL_GetDrawableSize`, sets `SDL_HINT_MAC_OPENGL_ASYNC_DISPATCH` because "SDL expects OpenGL Context to be on the main thread on OSX"; supports `OPENRA_DISPLAY_SCALE` override; pixel-doubles cursor "on non-OSX" — `OpenRA.Platforms.Default/Sdl2PlatformWindow.cs` (bleed)
- OpenRA on Windows switched to "DirectX 11 (via the ANGLE library)" in playtest-20201213; legacy GL 2.1 disabled by default then; main macOS release required 10.13+ with a separate compat build — [OpenRA playtest-20201213](https://www.openra.net/news/playtest-20201213/)
- Legacy OpenGL removed entirely (PR #21143) because only 29/45,302 (0.06%) sysinfo entries used it; profile option retained for GL/GLES/ANGLE switching — [OpenRA PR #21143](https://github.com/OpenRA/OpenRA/pull/21143)
- Native arm64 support (PR #20559, Dec 2022): .NET 6 lacked proper universal binaries, so OpenRA ships both runtimes and a custom minimal .NET host selected by an arm64+x86_64 `Launcher`; "macOS windowing system... requires executables to live in the parent MacOS directory for the dock icon / app name associations to work"; "Open using Rosetta" works — [OpenRA PR #20559](https://github.com/OpenRA/OpenRA/pull/20559)
- OpenRA build script: `codesign --timestamp --options runtime --entitlements entitlements.plist`; DMG submitted with `xcrun notarytool submit --wait`; apphosts compiled with `-target arm64-apple-macos10.15` / `x86_64-apple-macos10.15` — `packaging/macos/buildpackage.sh` (bleed)
- OpenRA macOS download ~150 MB vs ~42-46 MB Linux AppImage per mod — [openra.net download](https://www.openra.net/download/)
- macOS hardware bug example: "My game crashed when MacBook is plugged into dock with Displayport Connection" (open since 2021) — [OpenRA issue #19328](https://github.com/OpenRA/OpenRA/issues/19328)
- doukutsu-rs: default backend SDL2 + OpenGL (`glow`), optional `winit`+`glutin` backend, `cpal` audio, imgui; macOS bundle via `cargo-bundle` metadata (min 10.15, @2x icons); separate Intel and Apple M1 builds; README: "If you get a 'doukutsu-rs can't be opened' message, right-click doukutsu-rs.app and click open" — [doukutsu-rs Cargo.toml / README](https://github.com/doukutsu-rs/doukutsu-rs)
- VERA20k: `wgpu` 27 + `winit` 0.30 (ApplicationHandler, deferred window creation), `pollster` to bridge wgpu async init; requires "a GPU with Vulkan, DirectX 12 or Metal"; Win32 `ClipCursor` used because winit's `CursorGrabMode::Confined` mis-computes the clip rect on frameless windows — [VERA20k Cargo.toml](https://github.com/YuriPlanet/vera20k/blob/main/Cargo.toml)
- OpenSAGE lists Metal 2 (High Sierra) as its Mac backend — [OpenSAGE README](https://github.com/OpenSAGE/OpenSAGE)
- OpenTS is Windows-only (bgfx renderer); "Wine is expected to work but is not officially supported" — [OpenTS repo](https://github.com/OpenTS-Developers/OpenTS)

### Inferences
- For a Rust macOS-first engine: wgpu's Metal backend avoids the OpenGL deprecation trap and the ANGLE dependency; budget early for Developer ID signing, hardened runtime entitlements and notarytool in CI (OpenRA's script is a concrete template); handle Retina by rendering at drawable size and scaling the pixel-art viewport by integer factors.
- Cursor confinement/edge-scrolling is a known pain point (VERA20k needed Win32-specific workaround); on macOS this will need equivalent AppKit/CGAssociateMouseAndMouseCursorPosition-style handling (not researched).

### Gaps
- Could not confirm whether OpenRA's macOS package bundles ANGLE (i.e. renders via Metal) or uses Apple's deprecated GL 3.2 core; the macOS build script grep showed no ANGLE/libEGL references, suggesting native GL, but this is unverified.
- No information found on OpenMW's or VCMI's macOS/Apple Silicon packaging specifics.

## Q7. Rust-specific reimplementations: wgpu vs SDL vs Metal, ECS vs hand-rolled, determinism

### Takeaway
The only finished Rust reimplementation (doukutsu-rs) used SDL2 + OpenGL with a hand-rolled game loop (no ECS). The 2026 Rust C&C attempts split: VERA20k (playable) uses wgpu/winit with a hand-rolled sim mirroring the original engine's structure, while Iron Curtain (not playable) commits to Bevy ECS plus strict determinism invariants. Determinism practice converges on fixed-point integers, ordered collections, fixed tick rate, and state hashing.

### Cited Findings
- doukutsu-rs: SDL2 backend, OpenGL via `glow`, optional glutin/winit, `cpal`, `imgui`; forks of winit/glutin/cpal pinned by git rev; optional `netplay` feature — [doukutsu-rs Cargo.toml](https://github.com/doukutsu-rs/doukutsu-rs)
- VERA20k: `wgpu` 27, `winit` 0.30, `glam`, `fixed` crate (`SimFixed`), `serde`/`bincode`/`ron`/`toml`, `nom`; optional egui diagnostic overlay ("Player UI uses the retail UI owners"); binaries `parity-digest`, `sim-bench`, `mix-browser` — [VERA20k Cargo.toml](https://github.com/YuriPlanet/vera20k/blob/main/Cargo.toml)
- VERA20k: "All math must preserve simulation determinism across supported platforms and CPU architectures for identical state, inputs and RNG. Prefer SimFixed"; floats only where native behaviour requires, documented — [VERA20k AGENTS.md](https://github.com/YuriPlanet/vera20k/blob/main/AGENTS.md)
- Iron Curtain: "Fixed-point math only — i32/i64 with scale factor 1024... Never f32/f64 in sim-facing code"; "No HashMap/HashSet — non-deterministic iteration order breaks lockstep. Use BTreeMap/BTreeSet/IndexMap"; network model pluggable via trait so "Swapping lockstep for rollback touches zero sim code" (`RelayLockstepNetwork`, `LocalNetwork`, `ReplayPlayback`); "UI Never Mutates Authoritative Sim State"; `ic-sim` pure (no I/O, no floats), `ic-render` Bevy isometric renderer, `ic-game` Bevy ECS orchestration — [Iron Curtain AGENTS.md](https://github.com/iron-curtain-engine/iron-curtain/blob/main/AGENTS.md)
- RA2NE: fixed 30 TPS, integer coordinates, deterministic RNG, reproducible world-state hash, deterministic spatial broad phase, 10,000-unit pure-sim benchmark — [RA2NE README](https://github.com/18611429192/ra2ne)
- OpenRA's equivalent: 1024-unit world coordinates in `WPos` ints and per-net-frame sync hashes — `OpenRA.Game/WPos.cs`, `Sync.cs`
- Other Rust RTS engines: Ruddle/oxidator (Rust + WebGPU, 35,000-unit demo, stalled since 2020-03); `foomsim` ("lockstep RTS engine supporting 100k units", 4 stars); `trilateral` ("deterministic competitive 1v1 RTS... fixed-point, lockstep"); `mpe-ra` (Rust+WASM OpenRA-style) — `gh search repos` results 2026-10-16
- Iron Curtain stated as "GPL-3.0 with modding exception", built on Bevy — `gh search repos` description

### Inferences
- Evidence favours a hand-rolled, data-oriented sim (struct-of-arrays or simple entity tables in tick-phase order mirroring the original's update order) over a general ECS for a faithful reimplementation: ECS system scheduling and archetype iteration order are extra determinism hazards, and the only playable Rust RA2 engine did not adopt one. Bevy remains reasonable for the presentation layer if the sim is a pure crate behind it (Iron Curtain's split), but no Bevy-based C&C engine is yet playable.
- Separating `sim` (pure, fixed-point, no floats/HashMap) from `render`/`app` crates and building a headless sim benchmark + replay hash checker early (RA2NE, VERA20k `sim-bench`/`parity-digest`) is the common pattern worth copying.

### Gaps
- No evidence found of any Rust project using hecs or legion for a classic-game reimplementation; "renew" and "orion" Rust RTS engines named in the brief could not be located on GitHub.
- No postmortems from Rust projects comparing wgpu vs SDL2 on macOS (e.g. frame pacing, ProMotion, fullscreen behaviour).
