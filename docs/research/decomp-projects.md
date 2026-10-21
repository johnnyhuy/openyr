# Decompilation and RE-based game reimplementations: methods, verification, legal history (precedents for a native Rust Yuri's Revenge)

Research date: 2026-10-16. Sources are project repos (read via authenticated `gh`), court/news coverage and statute text. Dates are noted inline. Where a claim rests on one outlet or the project's own self-description, that is flagged.

## How were the precedent projects built? (approach, timeline to playable, team, licence, platform layer, legal history)

### Takeaway
Nearly every successful precedent on a 32-bit Windows binary used the same "hook-and-replace" pattern: inject a DLL into the original exe, replace functions one at a time, and keep calling the original for anything not yet done, until nothing of the original is left. Then they cut the exe dependency and ported to SDL/OpenGL-type platform layers. Time to a standalone build ran from roughly 2 years (re3, around 6 core people) to about 3.5 years (OpenRCT2, many contributors). The legally clean outcome came only when the rights holder released source (Generals, 2025) or tolerated the project. Take-Two sued re3.

### Cited Findings

**re3 / reVC (GTA III / Vice City)**
- Approach: aap began reversing GTA 3 casually in 2016. He then reversed collision and physics code "injecting this into the actual game for testing. This is how the re3 project began. aap replaced the functions of the original game until everything was replaced by the spring of 2018."[Eurogamer, 2021-02-17](https://www.eurogamer.net/articles/2021-02-17-how-a-small-group-of-gta-fanatics-reverse-engineered-gta-3-and-vice-city-without-so-far-getting-shut-down-by-take-two)
- Workflow quote: "So what we typically do is work with the output of the decompiler and massage it back into readable C++." Before that, the team had "been injecting our code into the games for many years for modding but of course that's sort of tedious."[Eurogamer](https://www.eurogamer.net/articles/2021-02-17-how-a-small-group-of-gta-fanatics-reverse-engineered-gta-3-and-vice-city-without-so-far-getting-shut-down-by-take-two)
- Timeline and team: pushed to GitHub in May 2019 with about 15,000 lines reversed. Work stalled in the second half of 2019 and resumed in early 2020. The first standalone executable came in April 2020. "The core team was just six people." They chose GTA 3 because it was the smallest game of its era ("less code to reverse"). [Eurogamer](https://www.eurogamer.net/articles/2021-02-17-how-a-small-group-of-gta-fanatics-reverse-engineered-gta-3-and-vice-city-without-so-far-getting-shut-down-by-take-two)
- Platform layer: rendering is either the original RenderWare (D3D8) or aap's reimplementation librw (D3D9, OpenGL 2.1+, GLES 2.0+). Audio is MSS (the original DLLs) or OpenAL. It ran on Windows, Linux, macOS and FreeBSD on x86, amd64, arm and arm64, with community Switch, Vita and Wii U ports. [re3 archived mirror README](https://github.com/Jai-JAP/re-GTA)
- Acknowledged fidelity gaps: "There are almost certainly still bugs in the re3 code that aren't original," aap admits. The To-Do list includes "compare CodeWarrior build with original binary for more accurate code (very tedious)". [Eurogamer](https://www.eurogamer.net/articles/2021-02-17-how-a-small-group-of-gta-fanatics-reverse-engineered-gta-3-and-vice-city-without-so-far-getting-shut-down-by-take-two); [re3 mirror README](https://github.com/Jai-JAP/re-GTA)
- Risk posture: the game required the user's own PC assets and was described as "educational, documentation and modding" use only. aap: "we were very much worried about that and tried to stay under the radar for as long as possible."[Eurogamer](https://www.eurogamer.net/articles/2021-02-17-how-a-small-group-of-gta-fanatics-reverse-engineered-gta-3-and-vice-city-without-so-far-getting-shut-down-by-take-two)
- Legal history:
  - Take-Two sent a DMCA takedown to GitHub in February 2021, [GitHub DMCA repo, 2021-02-19-take-two.md](https://github.com/github/dmca/blob/master/2021/02/2021-02-19-take-two.md)
  - Defendants filed sworn counter-notices between April and June 2021, which restored the repos, [TorrentFreak, 2021-09-03](https://torrentfreak.com/take-two-sues-enthusiasts-behind-gta-fan-projects-re3-revc-210903/)
  - In September 2021 Take-Two sued 14 defendants, 4 of them named (Angelo Papenhoff/aap, Theo Morra, Eray Orçunus, Adrian Graber). It alleged "willful and malicious" infringement through copying, adaptation and distribution of GTA source code, and also alleged bad-faith DMCA counter-notices, [TorrentFreak, 2023-04-05](https://torrentfreak.com/take-two-dismisses-claims-against-lead-defendants-in-gta-mods-lawsuit-230405/)
- Defence (November 2021 answer): the defendants pleaded fair use. Any copying was for interoperability and bug fixing and "was kept to the minimally required amount". The use was transformative, and the code was "useless in their own right" without owning the games, which Take-Two had stopped selling. [TorrentFreak, 2021-11-16](https://torrentfreak.com/gta-modders-to-court-our-game-fixes-enhancements-are-fair-use-211116/)
- Outcome: the case "settled in principle" in February 2023. On 2023-04-03 a joint stipulation dismissed all claims against the four named defendants **with prejudice**, with each side bearing its own costs and fees. The terms were not disclosed. [TorrentFreak, 2023-04-05](https://torrentfreak.com/take-two-dismisses-claims-against-lead-defendants-in-gta-mods-lawsuit-230405/); [NME](https://www.nme.com/news/take-two-and-grand-theft-auto-modders-will-settle-legal-battle-3394677)
  - Exputer reports the terms may be under NDA and that Take-Two was still pursuing the unnamed Does. This is a single source. [Exputer](https://exputer.com/news/industry/take-two-settle-lawsuit-modders/)
  - TorrentFreak notes the case initially looked set to continue against "Doe 1 (a/k/a Ash R. and ASH_735) plus Does 2 through 10". [TorrentFreak](https://torrentfreak.com/take-two-dismisses-claims-against-lead-defendants-in-gta-mods-lawsuit-230405/)
- Licence: the original GTAmodding/re3 repo is gone. Many mirrors persist on GitHub, for example Jai-JAP/re-GTA ("full archived mirror ... including wiki") and hottabxp/re3 with 492 stars, created 2021-10., gh search, [Jai-JAP/re-GTA](https://github.com/Jai-JAP/re-GTA)

**Devilution → DevilutionX (Diablo)**
- Sources used for the decomp:
  1. The Japanese PSX port's leaked debug symbols, `DIABPSX.SYM`, giving function names, line ranges and register variables.
  2. A debug PE build hidden in `DIABDAT.MPQ -> D1221A.MPQ -> DIABLO.EXE`, with assert strings.
  3. The PE Rich header, which identified the exact toolchain (VC++ 6.0 SP5 plus LINK 5.12).
  4. The recovered compiler flags (mostly `/O1`).
  5. The Beta and Alpha4 builds, which were not release-optimised, for cross-validation.

 , [DevilutionX docs/BACKGROUND.md](https://github.com/diasurgical/devilutionX/blob/master/docs/BACKGROUND.md); [Devilution README](https://github.com/diasurgical/devilution)
- Method: GalaXyHaXz "succeeded in the tremendous task of getting the decompiled source code of Diablo 1 compiling with the original toolchain". Progress was tracked by a "Binary identical functions" milestone "in tandem with an assembly diffing tool developed in Rust". Original bugs were preserved and annotated with `// BUGFIX:` comments. [BACKGROUND.md](https://github.com/diasurgical/devilutionX/blob/master/docs/BACKGROUND.md)
- Philosophy: Devilution "keeps everything as it was originally designed. This goes as far as bugs and badly written code", to serve as a base for ports. The README's credits joke that the work took "four months of my life". [Devilution README](https://github.com/diasurgical/devilution)
- Timeline: the Devilution repo was created 2018-04-02, with releases 0.3 on 2018-06-28 and 0.4 on 2018-09-16. The DevilutionX repo was created 2018-08-02 and has 9,869 stars and 100+ contributors as of 2026-10., GitHub API ([devilution](https://github.com/diasurgical/devilution), [devilutionX](https://github.com/diasurgical/devilutionX))
- Assets: the user must supply `DIABDAT.MPQ` from the CD, GOG or Battle.net. The shareware `spawn.mpq` is allowed for the shareware portion. Hellfire needs its own MPQs. [DevilutionX README](https://github.com/diasurgical/devilutionX)
- QoL vs vanilla: the health and XP bars are "off by default but can be enabled", and "Widescreen can also be disabled". [DevilutionX README](https://github.com/diasurgical/devilutionX)
- Licence: "Sustainable Use License", non-commercial only, with no affiliation or endorsement from Blizzard. [DevilutionX README](https://github.com/diasurgical/devilutionX)
- I found no record of a Blizzard takedown against Devilution or DevilutionX in my searches. See Gaps.

**OpenBW (StarCraft: Brood War)**
- Goal: a new engine "fully compatible with Brood War". Replay viewing is the first focus because "in order to play a replay, the entire game engine is required". The project states it contains no Blizzard IP except needing the three original MPQs. These are self-descriptions by the developer. [TeamLiquid OpenBW introduction (2017, via search snippet; page returned 403)](https://www.teamliquid.net/forum/brood-war/517127-openbw-introduction?view=all)
- Author: Vegard Mella (tscmoo), a Brood War bot author who later worked at FAIR. A 2026 article claims pixel-exact reproduction. That is a press claim, not an independent test. [Shazoo, 2026-07-25](https://shazoo.ru/2026/07/25/187772/issledovatel-ii-sobral-brauzernuiu-versiiu-starcraft-na-dvizke-openbw)
- Repo: OpenBW/openbw was created 2016-12-21, last pushed 2026-08-13, and has **no licence declared** on GitHub. A BWAPI 4.2.0 fork lets bots run on OpenBW. It needs Stardat/Broodat/Patch_rt MPQs from 1.16.1 or 1.18, and SDL2 for the optional UI. [OpenBW/openbw](https://github.com/OpenBW/openbw); [OpenBW/bwapi README](https://github.com/OpenBW/bwapi)
- Why replay compatibility is the acid test: Brood War replays store player commands, not game state, and are "literally replayed by the engine", so the engine logic must match exactly to stay in sync. [Liquipedia replays](https://liquipedia.net/starcraft/Portal:Beginners/Replays)
- Ecosystem: Titan Reactor is a 2.5D TypeScript StarCraft viewer (Show HN 2023-03) built on OpenBW-derived simulation. [HN / imbateam-gg/titan-reactor](https://github.com/imbateam-gg/titan-reactor). I found no Rust Brood War engine reimplementation on GitHub; the searches returned nothing.

**OpenRCT2 (RollerCoaster Tycoon 2)**
- RCT2 was originally written in x86 assembly by Chris Sawyer. OpenRCT2 began on 2014-04-02 (Ted "IntelOrca" John), with a first release on 2014-12-07 and 250+ contributors. Its latest release is v0.5.6 (2026-10-10). [Wikipedia: OpenRCT2](https://en.wikipedia.org/wiki/OpenRCT2); [OpenRCT2 README](https://github.com/OpenRCT2/OpenRCT2)
- Transition off the original exe (from PR history):
  - 2016-06 to 2016-09: "NO_RCT2 work" PRs added a build mode without rct2.exe ([#3887](https://github.com/OpenRCT2/OpenRCT2/pull/3887), [#4341 "Fix NO_RCT2 evaluation in hook.c"](https://github.com/OpenRCT2/OpenRCT2/pull/4341), [#4419](https://github.com/OpenRCT2/OpenRCT2/pull/4419)).
  - 2017-11: "Stub interop for NO_RCT2 builds" ([#6627](https://github.com/OpenRCT2/OpenRCT2/pull/6627)).
  - **2017-12-04: "Remove RCT2 interop"**, merged by IntelOrca, "Removes all `!NO_RCT2` blocks" ([#6772](https://github.com/OpenRCT2/OpenRCT2/pull/6772)).
  - This conflicts with the brief's "fully standalone in 2016". NO_RCT2 builds existed in 2016, but interop code was removed only in December 2017.
- Verification tooling: "Testpaint" (tests added 2016-11, [#4835](https://github.com/OpenRCT2/OpenRCT2/pull/4835)) still depended on RCT2 struct versions after interop removal. It was finally removed in 2022–2023 ([#17333](https://github.com/OpenRCT2/OpenRCT2/pull/17333), [#20489](https://github.com/OpenRCT2/OpenRCT2/pull/20489))., PR text, [#6772](https://github.com/OpenRCT2/OpenRCT2/pull/6772)
- Early-release changelog shows the hybrid period: DirectDraw, DirectInput, DirectPlay and DirectSound dependencies were dropped. The changelog also separates fixes marked "(original bug)" from new behaviour, for example "Fix: Guests will no longer ignore no entry signs ... (original bug)". [OpenRCT2 changelog.txt](https://github.com/OpenRCT2/OpenRCT2/blob/develop/distribution/changelog.txt)
- OpenRCT2 still requires the original RCT2 data files (for example, an install error "Could not find /Data/g1.dat"). [OpenRCT2 issue #23433](https://github.com/OpenRCT2/OpenRCT2/issues/23433)
- I found no record of legal action against OpenRCT2.

**Thyme (Generals: Zero Hour), Vinifera (Tiberian Sun), Phobos/Ares (Yuri's Revenge), and the 2025 Generals source release**
- Thyme: "a bottom-up rewrite of Generals: Zero Hour, using the original binary to provide functions that have not been implemented yet". It is GPL-2.0, and its repo was created 2017-01-09. [Thyme README](https://github.com/TheAssemblyArmada/Thyme)
- Thyme's hooker header provides:
  - `Make_Global<T>(addr)` to alias exe globals.
  - `Call_Function`/`Call_Method` to call not-yet-reimplemented originals.
  - DLL-injected replacement of functions.

  It is "Originally based on work by the Tiberian Technologies team to hook Renegade, rewritten based on work by LRFLEW for OpenMC2". [Thyme src/hooker/hooker.h](https://github.com/TheAssemblyArmada/Thyme/blob/develop/src/hooker/hooker.h)
- Thyme on native macOS/Linux: "because of how the project is developed, a fully playable native binary will not be possible for some time". A `-DSTANDALONE=TRUE` build exists but "has very limited functionality". Its last commits were on 2024-10-31, before EA's source release. [Thyme README](https://github.com/TheAssemblyArmada/Thyme); GitHub API commits
- The Assembly Armada also maintains Vanilla Conquer (TD/RA, built on EA's 2020 source drop) and lists sibling hook/RE projects such as OpenDUNE, OpenLoco, OpenMC2 and dethrace. [Thyme README](https://github.com/TheAssemblyArmada/Thyme); [Vanilla Conquer](https://github.com/TheAssemblyArmada/Vanilla-Conquer)
- EA release (late February 2025): GPL source for Tiberian Dawn, Red Alert, Renegade, and Generals plus Zero Hour, code only with no assets. **Tiberian Sun and Red Alert 2/YR were not released.** Speculation that the source is lost is unconfirmed. [Slashdot, 2025-02-27](https://games.slashdot.org/story/25/02/27/2144203/ea-releases-source-code-for-old-command-and-conquer-games)
- TheSuperHackers/GeneralsGameCode:
  - Repo created 2025-02-28, the day after the release; about 57 contributors; GPL-3.0.
  - It ported VS6/C++98 to VS2022/C++20, with the goal of fixing bugs "while maintaining compatibility with the original Generals version 1.08 and Zero Hour version 1.04. Once we can break retail compatibility, more fixes ... will be possible".

 , [GeneralsGameCode README](https://github.com/TheSuperHackers/GeneralsGameCode)
- Downstream (2026) native ports built on that GPL code:
  - Generals-Mac-iOS-iPad (via GeneralsX, DXVK/MoltenVK; 1,674 stars, created 2026-07), [ammaarreshi/Generals-Mac-iOS-iPad](https://github.com/ammaarreshi/Generals-Mac-iOS-iPad)
  - A direct D3D8→Metal shim attempt, [dvcdsys/GeneralsGameCode-macOS](https://github.com/dvcdsys/GeneralsGameCode-macOS)
  - An Android port via SDL3 plus DXVK, [molotovgit](https://github.com/molotovgit/CnC-Generals-ZeroHour-Android)
- Vinifera: an engine extension for Tiberian Sun, originally created by CCHyper and tomsons26 and now led by ZivDero and Rampastring. It is launched via `LaunchVinifera.exe` from the TS folder. Its stated purpose is "preservation, modding and compatibility"; it includes no original assets, and "EA has not endorsed and does not support this product". [Vinifera README](https://github.com/Vinifera-Developers/Vinifera)
- Phobos (directly relevant to YR): a "community engine extension project ... for Yuri's Revenge based on modified YRpp and SyringeEx to allow injecting code", complementary to Ares. It is GPL-3.0 and launched via `Syringe.exe "gamemd.exe"`. As of 2026-10 the README warns that it "is currently not maintained actively enough" and is seeking maintainers. [Phobos README](https://github.com/Phobos-developers/Phobos)

**Static recompilation (N64Recomp, Zelda64Recomp, XenonRecomp and successors)**
- N64Recomp takes the binary plus a symbol/metadata list, splits it into functions, and emits one C function per original function with very literal per-instruction translation (for example `ctx->r4 = ADD32(ctx->r4, 0X20);`). `jal` becomes a call, and `jr` with jump tables becomes `switch`. Its inspiration was IDO static recomp. A "single file output" mode plus linker symbol precedence lets hand-written patches override recompiled functions. [N64Recomp README](https://github.com/N64Recomp/N64Recomp)
- Zelda64Recomp (Majora's Mask; repo 2023-10, GPL-3.0) pairs N64Recomp with the RT64 renderer, adding high-FPS, widescreen and mods. The ROM is required. It has a **blanket ban on GenAI contributions**. [Zelda64Recomp README](https://github.com/Zelda64Recomp/Zelda64Recomp)
- XenonRecomp converts Xbox 360 PPC to C++. It provides "no runtime implementation ... Making the game work is your responsibility."
  - Function boundaries come from `.pdata` plus heuristics; the analyser "struggles with functions containing jump tables".
  - Virtual calls go through a runtime perfect-hash lookup table.
  - Exceptions are unsupported.
  - Weak-linked `PPC_FUNC` lets you override any function while still calling the original.

 , [XenonRecomp README](https://github.com/hedge-dev/XenonRecomp)
- In 2026, recomp ports multiplied:
  - The Simpsons Game, with Windows/Linux builds, [Generation Amiga, 2026-07-20](https://www.generationamiga.com/2026/07/20/the-simpsons-game-returns-with-a-new-windows-and-linux-recompilation-release/)
  - A GoldenEye XBLA build via the ReXGlue SDK, [tbreak](https://tbreak.com/goldeneye-007-xbox-360-pc-recompilation/)
  - DK64 Rekongpiled 1.0 on Windows/Linux/macOS, [HWBusters](https://hwbusters.com/news/dk64-rekongpiled-hits-1-0-running-donkey-kong-64-natively-on-pc-without-an-emulator/)
- General limitation: static translation "can be very difficult to do correctly, since not all code can be discovered by the translator", for example code reachable only through indirect branches. [Wikipedia: Binary translation](https://en.wikipedia.org/wiki/Static_recompilation)

**Matching decomps (SM64, OoT) and AI-assisted decomp 2025–2026**
- The SM64 decomp builds byte-identical ROMs for 5 regions, verified by SHA-1, for example us `9bef1128...`. The repo ships no assets; a baserom is needed for extraction., [n64decomp/sm64 README](https://github.com/n64decomp/sm64); OoT is similar, [zeldaret/oot README](https://github.com/zeldaret/oot)
- LLM benchmark (Macabeus, 2026-03-10):
  - Mizuchi pipeline: m2c → compile → objdiff → decomp-permuter, then a Claude Agent SDK loop (Sonnet 4.6, 12 attempts).
  - Result: 74% of 60 functions matched (Sonic Advance 3 GBA plus Animal Forest N64), with 88% reproducible outcomes.
  - Some functions need 20+ attempts, and one reportedly took 87.

 , [GambiConf: Can LLMs Really Do Matching Decompilation?](https://gambiconf.substack.com/p/can-llms-really-do-matching-decompilation)
- Whole-game application (2026-08-17): Klonoa: Empire of Dreams (GBA) reached 51% of code bytes decompiled using Claude Code.
  - Luvdis found 93 functions, Ghidra about 300 more, and Claude Code more again, totalling 663 after cleanup.
  - The repo is structured for agents with `INCLUDE_ASM` stubs and git worktrees for parallel work.

 , [GambiConf: Starting a Decompilation Project from Zero](https://gambiconf.substack.com/p/starting-a-decompilation-project)
- Academic work:
  - LLM4Decompile refines Ghidra output and decompiles end to end; limited to C on x86, [arXiv 2403.05286](https://arxiv.org/html/2403.05286v3)
  - D-LiFT improves 68.2% of functions vs the native decompiler, [arXiv 2506.10125](https://www.alphaxiv.org/abs/2506.10125.md)
  - A 2026 thesis flags hallucinated logic and misleading renaming, [Wooster](https://openworks.wooster.edu/independentstudy/13145)
- Tooling: LaurieWired/GhidraMCP (created 2025-03, about 11.3k stars), bethington/ghidra-mcp (200+ tools, headless, about 5.7k stars), pyghidra-mcp and ghidra-headless-mcp (2026-03)., gh search, [GhidraMCP](https://github.com/LaurieWired/GhidraMCP), [bethington/ghidra-mcp](https://github.com/bethington/ghidra-mcp)

### Inferences
- **Fit to YR:** YR (gamemd.exe, 32-bit MSVC C++) maps most closely to re3 and Thyme. YRpp/Syringe/Ares/Phobos already give the community a YRpp-style address map of gamemd.exe classes and globals, which is the same raw material Thyme's `Make_Global`/`Call_Method` hooker uses.
- **Why hook-and-replace won't port directly:** a Rust rewrite cannot use the hook path on macOS natively. It would have to run under Wine on x86 (32-bit Windows DLL injection), which conflicts with the user's goal, or use a staged "Rust DLL hooked into gamemd.exe under Wine for verification" phase before a standalone macOS build. That is the OpenRCT2 NO_RCT2 pattern.
- **Timeline expectation:** re3 took 2016→2020 to standalone with about 6 core devs, for a game smaller than its successors. OpenRCT2 took 2014→2017-12 to drop interop. A solo hobbyist should expect multi-year timelines unless AI-assisted decomp materially compresses them; the 2026 GBA evidence suggests it can for simpler functions.
- **Static recompilation for YR:** recomp for 32-bit x86 PE is technically plausible, since the instruction set is the host's on Intel Macs and translatable on ARM. But x86 C++ games have heavy vtable/indirect-call use, and Win32/DirectDraw/DirectSound APIs would need a full runtime shim. The recomp projects themselves say the runtime is the hard part ("Making the game work is your responsibility").

### Gaps
- No primary source found for a mature x86-32 PE → C/C++ static recompiler used on a commercial PC game; search returned only console (MIPS/PPC) recompilers. Feasibility for gamemd.exe is inference only.
- I could not retrieve OpenBW's own technical write-ups (the TeamLiquid thread returned 403; openbw.com shows a PHP fatal error as of 2026-10). Bit-exact replay methodology details are therefore unsourced beyond the developer's claims.
- No Blizzard public statement on OpenBW or Devilution(X) was found.
- No Rust reimplementation of StarCraft was found.
- Thyme's post-2025 status: no source states whether its developers formally moved to GeneralsGameCode. There is only the circumstantial evidence of last commits in 2024-10 and GGC's creation on 2025-02-28.
- Vinifera's licence SPDX and creation date did not render in the README fetch.

## Which verification techniques worked best?

### Takeaway
The strongest techniques were differential: compare the new code against the original binary, using byte-identical matching (Devilution, SM64), run-time A/B against original functions inside the original process (re3, Thyme, OpenRCT2 via hooking and testpaint), or deterministic replay of recorded inputs (OpenBW). Side-by-side playtesting was a backstop and was explicitly acknowledged to leave non-original bugs behind.

### Cited Findings
- re3 verified incrementally by injecting each reversed subsystem (collision and physics first) into the running original game, so every replaced function was exercised alongside original code until full replacement in spring 2018. [Eurogamer](https://www.eurogamer.net/articles/2021-02-17-how-a-small-group-of-gta-fanatics-reverse-engineered-gta-3-and-vice-city-without-so-far-getting-shut-down-by-take-two)
- re3's weakness: non-matching, "massaged" decompiler output leaves bugs "that aren't original". Matching against the CodeWarrior/PS2 build was listed as desirable but "very tedious". [Eurogamer](https://www.eurogamer.net/articles/2021-02-17-how-a-small-group-of-gta-fanatics-reverse-engineered-gta-3-and-vice-city-without-so-far-getting-shut-down-by-take-two); [re3 mirror README](https://github.com/Jai-JAP/re-GTA)
- Devilution used binary-identical function matching with the recovered original toolchain (VC++ 6 / LINK 5.12, `/O1`), a Rust assembly-diffing tool, and cross-validation against unoptimised Beta/Alpha builds. [BACKGROUND.md](https://github.com/diasurgical/devilutionX/blob/master/docs/BACKGROUND.md)
- SM64/OoT: whole-ROM SHA-1 equality is the end-to-end test. [sm64 README](https://github.com/n64decomp/sm64)
- Per-function matching loop: compile → objdiff → decomp-permuter, now automatable with LLM agents. That gives 74% match on a mixed-difficulty sample, and the objdiff score is the objective fitness signal. [GambiConf benchmark](https://gambiconf.substack.com/p/can-llms-really-do-matching-decompilation)
- OpenBW used replays as the full-engine regression oracle, since replays are command logs re-simulated by the engine. [TeamLiquid intro (snippet)](https://www.teamliquid.net/forum/brood-war/517127-openbw-introduction?view=all); [Liquipedia](https://liquipedia.net/starcraft/Portal:Beginners/Replays)
- OpenRCT2 kept a "Testpaint" harness tied to the original RCT2 structs to compare paint output, and maintained it beyond interop removal (2017) until removal in 2022–2023. [PR #6772](https://github.com/OpenRCT2/OpenRCT2/pull/6772); [PR #17333](https://github.com/OpenRCT2/OpenRCT2/pull/17333)
- OpenRCT2 and Devilution track deliberate deviations by labelling original bugs: "(original bug)" changelog tags, and `// BUGFIX:` comments that leave vanilla behaviour in place. [OpenRCT2 changelog](https://github.com/OpenRCT2/OpenRCT2/blob/develop/distribution/changelog.txt); [BACKGROUND.md](https://github.com/diasurgical/devilutionX/blob/master/docs/BACKGROUND.md)
- GeneralsGameCode treats retail compatibility (1.08/1.04) as a hard constraint before allowing behaviour changes. In practice that means multiplayer/replay interop with the unmodified game is the gate. [GeneralsGameCode README](https://github.com/TheSuperHackers/GeneralsGameCode)

### Inferences
- **Determinism makes YR a good fit for replay testing:** YR is a lockstep-networked deterministic RTS, the same class as Brood War and Generals. Its multiplayer sync and recorded-game mechanics make OpenBW-style replay/desync comparison the natural primary oracle for a Rust engine.
- **Proposed test stack:** capture input logs plus per-frame state hashes (CRC of object lists, RNG state) from gamemd.exe under Wine using a Syringe/Phobos-style hook DLL. Then require the Rust sim to reproduce the hash sequence.
- **Function-level oracle for Rust:** byte-matching is not possible for Rust (it is a different compiler), so Devilution/SM64-style matching is unavailable. The closest substitute is differential unit testing: call the original function in gamemd.exe (via hook DLL, or in a Unicorn/x86 emulator harness) and the Rust port with identical inputs and memory state, then compare outputs. This mirrors Thyme's `Call_Function` dual-path.
- **Lessons on what to keep:** testpaint-style render comparison and "(original bug)" ledgers are cheap disciplines worth copying. The OpenRCT2 history suggests oracle tests tied to original structs decay once the original is dropped, so persist golden outputs early.

### Gaps
- No quantitative desync rates or replay-pass percentages were found for OpenBW, re3 or Thyme.
- No source documents a project that used an x86 emulator (Unicorn etc.) as a per-function oracle against a PC game exe. This is a proposed technique, unsourced.

## What were the failure modes (abandonment, legal takedown, divergence)?

### Takeaway
The failure modes were legal takedown when the rights holder is aggressive (re3), stalls and maintainer loss in hybrid hook projects (Thyme went dormant after 2024-10; Phobos is seeking maintainers in 2026), and quiet behavioural divergence from non-matching decompiles (re3).

### Cited Findings
- Legal: GitHub DMCA takedown (2021-02), then a lawsuit (2021-09), then about 19 months of litigation before dismissal with prejudice. The lawsuit alleged "copying, adaption and distribution of GTA source code" and bad-faith counter-notices. [TorrentFreak 2021](https://torrentfreak.com/take-two-sues-enthusiasts-behind-gta-fan-projects-re3-revc-210903/); [TorrentFreak 2023](https://torrentfreak.com/take-two-dismisses-claims-against-lead-defendants-in-gta-mods-lawsuit-230405/)
- Abandonment risk is the stated motivation for Devilution: earlier Diablo reversers "rarely shared their work. Usually being a one-person job, they move on with their lives". [Devilution README](https://github.com/diasurgical/devilution)
- Earlier GTA 3 rewrite OpenRW "saw some active development but now appears dormant". re3 itself stalled in the second half of 2019 before the COVID-era revival. [Eurogamer](https://www.eurogamer.net/articles/2021-02-17-how-a-small-group-of-gta-fanatics-reverse-engineered-gta-3-and-vice-city-without-so-far-getting-shut-down-by-take-two)
- Hybrid lock-in: Thyme cannot produce a playable native binary "for some time" because of its hook-based design, and its last commit was 2024-10-31. [Thyme README](https://github.com/TheAssemblyArmada/Thyme)
- Phobos (YR) "is currently not maintained actively enough" (README, 2026). [Phobos README](https://github.com/Phobos-developers/Phobos)
- Divergence: re3 contains non-original bugs (aap). [Eurogamer](https://www.eurogamer.net/articles/2021-02-17-how-a-small-group-of-gta-fanatics-reverse-engineered-gta-3-and-vice-city-without-so-far-getting-shut-down-by-take-two)
- Static recompilation has its own limits: jump tables, indirect calls and exceptions stay unsolved or manual in XenonRecomp. [XenonRecomp README](https://github.com/hedge-dev/XenonRecomp)
- AI-assisted decomp risks: hallucinated logic and misleading renaming, [Wooster thesis 2026](https://openworks.wooster.edu/independentstudy/13145). Some flagship projects (Zelda64Recomp) ban GenAI contributions outright, [Zelda64Recomp README](https://github.com/Zelda64Recomp/Zelda64Recomp).

### Inferences
- The EA C&C precedent is mixed for YR. EA tolerated Vinifera, Ares and Phobos for years and GPL-released four titles, but not RA2/YR. The rights holder is permissive towards modding, but there is no source release to "graduate" onto, unlike Thyme's situation after 2025.
- A Rust rewrite avoids Thyme's hook lock-in only if it plans a decisive standalone cutover, as OpenRCT2 did with NO_RCT2 builds kept compiling in CI well before the interop code was deleted.
- If AI agents produce much of the translation, the provenance question ("is this a translation of EA's code?") gets sharper, not softer.

### Gaps
- No public EA statement on RA2/YR reimplementations was found.
- The re3 settlement terms are confidential, so it is unknown whether the developers agreed to cease distribution. The mirrors remain up.

## What does the legal framework and each precedent imply for a Ghidra-informed Rust port vs strict clean room?

### Takeaway
US and EU law both permit decompiling to understand functional, unprotectable elements and to achieve interoperability: Sega v. Accolade and Sony v. Connectix for intermediate copying, DMCA 1201(f), and EU Directive Art. 6. Neither protects shipping code that is a line-by-line translation of the decompiled expression. EU Art. 6(2)(c) explicitly bars using decompiled information to build a program "substantially similar in its expression". A Ghidra-informed Rust port that reproduces gamemd.exe's structure function by function sits close to the re3 risk profile. A two-team clean room (spec writers vs implementers), or at least spec-mediated development, is the established mitigation.

### Cited Findings
- **Sega v. Accolade (9th Cir. 1992):** disassembly to discover the functional TMSS lockout was fair use. Accolade copied only a header of about 20–25 bytes into games of 0.5–1.5 MB. The case "helped establish guidelines for permissible reverse engineering". [Wikipedia: Sega v. Accolade](https://en.wikipedia.org/wiki/Sega_v._Accolade)
- **Sony v. Connectix (9th Cir., decided 2000-02-10, 203 F.3d 596):** Connectix built a Mac PlayStation emulator by reverse engineering the BIOS, then wrote its own BIOS "using the original firmware as an aid for debugging". The court held that intermediate copies made during RE were fair use and that the product was "modestly transformative". Its reasoning was that penalising intermediate copies would push engineers to "inefficient engineering methods". [Wikipedia: Sony v. Connectix](https://en.wikipedia.org/wiki/Sony_Computer_Entertainment,_Inc._v._Connectix_Corp.)
- **Google v. Oracle (US Supreme Court, 2021-04-05, 6–2):** reimplementing the Java API declaring code was fair use. The court distinguished declaring code (an organisational function) from implementing code. Thomas's dissent criticised that distinction. [Wikipedia: Google v. Oracle](https://en.wikipedia.org/wiki/Google_LLC_v._Oracle_America,_Inc.)
- **DMCA 17 U.S.C. §1201(f):** permits circumvention to identify and analyse elements "necessary to achieve interoperability of an independently created computer program". Information may be shared "solely for the purpose of enabling interoperability", "to the extent that doing so does not constitute infringement". [17 U.S.C. §1201 (Cornell LII)](https://www.law.cornell.edu/uscode/text/17/1201)
- **EU Directive 2009/24/EC Art. 6:**
  - Decompilation is allowed without authorisation when "indispensable" to obtain information for "interoperability of an independently created computer program". The conditions are that the decompiler is a lawful user, the information is not readily available, and the work is confined to the necessary parts.
  - Information may not be used for other goals, given to others except as necessary, or "used for the development, production or marketing of a computer program substantially similar in its expression".

 , [Directive 2009/24/EC Art. 6](https://www.legislation.gov.uk/eudr/2009/24/article/6)
- **Clean-room design:**
  - Phoenix and AMI sold clean-room IBM-compatible BIOSes. Compaq and Columbia Data Products did clean-room BIOSes before Phoenix's July 1984 licensing announcement. Corona, Eagle and Handwell were sued by IBM and had to re-implement.
  - Clean room "is usually employed as best practice, but not strictly required by law". In NEC v. Intel (1990), early NEC microcode revisions infringed but later, sufficiently different ones did not, and a spec-only contractor's similar code showed some similarity was functionally dictated.

 , [Wikipedia: Clean-room design](https://en.wikipedia.org/wiki/Clean-room_design)
- **re3 shows how a rights holder frames decompiled ports:** Take-Two characterised re3 as "GTA source code" copying and adaptation, meaning a derivative work. Defendants relied on fair use and interoperability. The case was settled, so no ruling on decompiled-port legality exists. [TorrentFreak 2021-09](https://torrentfreak.com/take-two-sues-enthusiasts-behind-gta-fan-projects-re3-revc-210903/); [TorrentFreak 2021-11](https://torrentfreak.com/gta-modders-to-court-our-game-fixes-enhancements-are-fair-use-211116/); [TorrentFreak 2023](https://torrentfreak.com/take-two-dismisses-claims-against-lead-defendants-in-gta-mods-lawsuit-230405/)
- **Common mitigations across tolerated projects:** no assets shipped and the user supplies original files (DevilutionX MPQs, OpenBW MPQs, sm64 baserom, Vinifera/Phobos requiring an installed game); non-commercial or GPL licences; explicit non-endorsement disclaimers. [DevilutionX](https://github.com/diasurgical/devilutionX); [OpenBW/bwapi](https://github.com/OpenBW/bwapi); [sm64](https://github.com/n64decomp/sm64); [Vinifera](https://github.com/Vinifera-Developers/Vinifera)

### Inferences
- **The precedents split three ways:**
  - Matching decomps (Devilution, SM64) are the *highest* expression-copy risk, since by design they recreate the original's code. They survive largely through rights-holder tolerance.
  - Hook/extension projects (Vinifera, Phobos, Ares, Thyme) ship only new code plus address maps and run on the user's own exe. That is closer to Sega/Connectix-style interoperability and has been tolerated by EA for many years.
  - Spec-driven reimplementations (OpenBW's self-described "no Blizzard IP", Phoenix BIOS) are the lowest risk.
- **Rust translation vs clean room:**
  - A Rust port changes language, but translating Ghidra pseudo-C function by function into Rust is still plausibly an "adaptation" (derivative work) of the protected expression, which was exactly Take-Two's theory against re3.
  - EU Art. 6(2)(c) arguably excludes it outright, since a reimplementation of the game is not "interoperability with other programs" in the narrow sense, and the result would be "substantially similar in expression".
  - Connectix is the best US analogue for "use the original as a debugging oracle". Using gamemd.exe under Wine as a differential test oracle is defensible intermediate copying. Pasting decompiled logic is the riskier part.
- **Practical middle path for a hobbyist:**
  1. One phase/person documents behaviour from Ghidra in prose and data specs: formulas, state machines, INI semantics, file formats, network/replay formats.
  2. Implementation is written from those specs in idiomatic Rust, not transliterated.
  3. Fidelity is enforced by black-box replay/desync tests against the original exe.
  4. No assets ship; the user supplies the Origin/EA App/CD install, and the project carries a GPL or non-commercial licence plus a non-endorsement notice.

  A solo developer cannot form a true two-team Chinese wall. The NEC v. Intel note that clean room is "best practice, not strictly required" suggests documented separation and sufficiently different expression still help.
- **AI-assisted decomp blurs the line:** an agent that reads Ghidra output and emits Rust is a translation pipeline unless it is constrained to write specs first. If AI is used, use it on the spec side (behaviour extraction) and keep implementation spec-driven. Zelda64Recomp's GenAI ban shows some communities will also reject such contributions on provenance grounds.
- **Hook-DLL vs standalone for YR:** the safest-known shape for YR is Phobos/Vinifera: a GPL hook DLL plus the user's own gamemd.exe. Going standalone (re3/OpenRCT2 end state) moves the project from "mod" to "replacement engine", which is where Take-Two drew the line. EA has not drawn one publicly for C&C, but it has also not released RA2/YR source.

### Gaps
- I found no court decision ruling squarely on whether a decompiled, re-expressed reimplementation of a full game (re3-style) is infringing; re3 settled.
- I found no EFF or academic commentary specific to language-translation (C++→Rust) of decompiled code; the derivative-work analysis above is inference from statute and case text.
- The CJEU's SAS Institute v World Programming (2012, functionality not protected) is relevant to the EU picture, but I did not retrieve a primary source in this pass.
- No authoritative source on EA's position regarding the RA2/YR source code's existence.
