# Native front menu

No-argument and Finder launches enter the native front menu. Explicit game,
map, save/load and headless commands keep their prior launch behavior. The
front menu holds no session until Start or a saved-match load succeeds.
The in-match flow is in [menus.md](menus.md); #110 adds the front flow for #96.

## Observed shell art

These retail filenames resolve from RA2's NEUTRAL and CACHE archives through
AssetFs. They are read from the user's install. No source or decoded art is
shipped in git.

| Asset | Canvas | Frames | Palette |
| --- | --- | --- | --- |
| SDTP.SHP | 168x199 | 2 | SHELL.PAL |
| SDWRNANM.SHP | 92x53 | 91 | SHELL2.PAL |
| SDBTNANM.SHP | 156x42 | 17 | SDBTNANM.PAL |
| SDBTNBKGD.SHP | 168x42 | 1 | SDBTNANM.PAL |
| SDBTM.SHP | 168x65 | 1 | SHELL.PAL |
| LWSCRNL.SHP | 632x32 | 1 | SHELL.PAL |
| MNSCRNL.SHP | 632x568 | 1 | SHELL.PAL |

The shell guide by [ArgCmdr](https://www.yrargentina.com/old/index.php?page=tutorials%2Fgui1)
helped identify filenames. Dimensions, palette selection and source pixels
were checked against the local assets. In particular, SHELL.PAL is wrong for
SDBTNANM.SHP: it turns silver edges orange. Runtime CSF labels and GAME.FNT
supply text. Malformed optional sheets fall back to plain controls.

A [Dedoimedo original-game screenshot](https://www.dedoimedo.com/games/steam-proton-ra2-yuri-revenge.html)
shows the main layout under Proton: movie at left, warning display above
right-side controls, and Exit above the version strip. It is a qualitative
layout reference; its movie phase and display scaling were not controlled.
The movie's decoded planes have a separate exact comparison in [bink.md](bink.md).

The logical shell is 800x600, centered without upscaling. Smaller framebuffers
scale it down uniformly. The sidebar starts at x=632; control rows start at
y=199 and advance 42 pixels. Exit/Back/Main Menu occupy y=535. Status starts
at y=568, overlapping the movie's lower two border pixels. The same transform
handles drawing, pointer hits and saved-slot clicks. World zoom has no effect.

Native stable control phases use frames 2/3/4 for normal/hover/pressed; empty
rows use frame 0. The warning sheet currently advances at 15 fps. Entry/exit
transitions, warning cadence, keyboard focus decoration, text tracking and
color still need a controlled original oracle. The source movie advances on
its own declared rational clock.

## Match entry and recovery

Main opens Single Player, actual controls or an exit confirmation. Internet,
network, campaign and movie/credits actions are visibly disabled. Single
Player opens saved matches or the supported skirmish setup. Setup shows the
retail map's authored preview, fixed Allied player/computer profile and actual
starting credits. Broader maps/factions/modes/difficulty remain unavailable.
Its form layout is provisional. The authored thumbnail is preferred; a map
without one falls back to the shared terrain renderer.

The supported country is `Americans`. Generated bases use GACNST, GAPOWR,
GAREFN, GAWEAP and GAPILE. The retail `UnitCount=10` force is one AMCV,
one CMIN, four MTNKs and four E1s (#124). The computer guards use MTNK too.
Setup checks `Owner=`, `RequiredHouses=` and `ForbiddenHouses=` for these
types, and decodes their required body art before entering the match.
Missing types, unavailable art or an ineligible country return through the
existing loading error page. Map-authored objects retain their types and
remain separate from this generated force.

Smaller unit counts take a prefix of that force; larger counts retain one
MCV and miner, then repeat groups of four tanks and four infantry. Negative
counts, counts larger than the map rectangle, or insufficient nearby space
return a setup error instead of silently reducing the force. This is the
native sandbox composition, pending original starting-force observations.
MCV deployment, Chrono Miner teleportation, GI deployment and country
special abilities remain unimplemented. CMIN uses the shared harvesting
and unloading loop; its presence does not imply those abilities work.

One loader handles preview and replacement-match jobs. It keeps the current
page's return route, shows loading, retains the old match until successful
replacement, and ignores cancelled results. Starting a match drops the movie,
preview and front art, and clears held modifiers (#115). Return-to-menu drops
the match, queued input and old
presentation. Abort requires an explicit discard choice. Won/Lost screens
also offer Main Menu. Front loading failures keep the front context and a
usable route back. A movie failure can be dismissed while the other controls
remain usable.

Native startup and
load center the world viewport using its actual zoom, excluding the sidebar
(#114).

Front options use actual mute and world zoom preferences. Game Data Folder
opens the existing macOS picker, restores the system pointer for the dialog,
validates both root MIX archives before remembering the directory, then reloads
front art, pause-menu art and audio. The pause-menu font, labels and backgrounds
are replaced from the selected install as well (#118). Invalid/cancelled
selections retain a recovery route.
The chooser's script and acceptance helper are tested separately; physical
desktop/Retina/resizing exercise remains open in #97 because the live binding
was unavailable.

The music router selects THEMEMD.INI's INTRO sound, even though Normal=no
excludes it from the match playlist. The existing CoreAudio worker owns its
playback and mute state. This is separate from the silent Bink movie.

The retail flow test drives actual handlers and composition through initial
menu, disabled modes, single player, preview, start, save, abort/return, options,
front load, result/return, cancellation, load error, movie error and exit
confirmation. A separate test compares every used shell sheet pixel and rendered
button bevels to the scoped palette decode. Proof remains outside git.
