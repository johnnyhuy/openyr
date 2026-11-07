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
| SDBTNBKGD.SHP | 168x42 | 1 | SHELL2.PAL |
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
scale it down uniformly. The sidebar starts at x=632; button art starts at
x=644. Control rows start at y=199 and advance 42 pixels. Exit/Back/Main Menu
occupy y=535. Blank row backgrounds extend through that row. The bottom cap
starts at y=577 and clips at the canvas edge. Status starts
at y=568, overlapping the movie's lower two border pixels. The same transform
handles drawing, pointer hits and saved-slot clicks. World zoom has no effect.

The observed normal control uses frame 2. Native hover/pressed phases still use
frames 3/4 pending controlled comparison. Empty rows use SDBTNBKGD alone.
The warning sheet is anchored at `(670,48)` and currently advances at 15 fps. Entry/exit transitions,
warning cadence, keyboard focus decoration and movie/shell synchronization
remain uncalibrated. The source movie advances on its declared rational clock.

## Local original renderer observations

On 2026-11-01 the installed retail game reported version 1.001. Its DDRAW
renderer exported 800x600 PNGs through the configured PrintScreen hotkey.
The Wine host window reported 1024x769, while its child menu dialog and
controls retained the 800x600 coordinates. Scaling input by the host size
missed the controls. Enumerating the game's child controls and activating the
observed Single Player and Skirmish buttons reached those pages. These were
ordinary documented Win32 UI calls; no original implementation was read or
copied. The helpers are diagnostic tools, not part of the native application.

The main capture at 08:19:45 has SHA256
`c89c6963b4abe4a5911181cf60b85285a9ae365b2a71abe8360d4e8dcc63d4a8`.
The Single Player capture at 08:24:18 has SHA256
`95f10c857a5fee120f46bfb0b43932468084f3f61a11a019c681dd40300d07f1`.
Raw images and the helper source belong to the external proof bundle for
#133. They contain retail art and are not repository fixtures.

Static header rims, blank rows, status borders and normal buttons match
RGB565 output: red and blue retain five high bits, green retains six.
The compositor applies this truncation after drawing the front shell, movie
and text. Asset decoding and world rendering retain their existing precision.
Button labels match with a two-pixel left inset relative to the art's center
and rounded-up horizontal and vertical centering. The tall header title begins
at y=9; the compact Skirmish map header uses y=3. Yellow button
text presents as `(248,252,0)`. All five enabled button images compared across
Main and Single Player match the captured original pixels exactly.

The retail regression hashes 13 RGBA rectangles with FNV-1a. Main rectangles
are `(632,0,168,8)`, `(632,140,168,59)`, `(0,570,632,8)`,
`(632,409,168,126)`, `(632,577,168,4)`, `(644,199,156,12)`,
`(644,230,156,11)`, `(644,199,156,42)`, `(644,367,156,42)` and
`(644,535,156,42)`. Single Player rectangles are `(632,325,168,210)`,
`(644,283,156,42)` and `(644,535,156,42)`. Each tuple is x, y, width, height.
No dynamic movie or warning region is included. The test also keeps the
independent palette/SHP decode checks.

The initial front page no longer paints a keyboard underline or idle help
text. Explicit native keyboard navigation retains its provisional focus cue;
pointer press and page changes hide it. A captured original Tab input did not
establish a focus change, so it does not calibrate that cue.

The heading correction in #142/#143 matches Main Menu, Single Player and
Skirmish Game, including the observed final glyph color. The native regression
adds three complete heading regions to the thirteen static rectangles above.
The [heading observations](front-headings.md) record the masks, source labels,
coordinates and limits. Public control measurements and original raster
comparisons now calibrate the Skirmish preview background, image bounds and
independent-axis sampling (#144, #151). The [preview observations](front-preview.md)
record exact landscape/portrait checks and numbered markers (#152). Heading
lifecycle checks cover focus and chooser return (#153). Unsupported actions
remain visibly disabled and
the version strip identifies OpenYR. Broader setup, animation timing and
physical macOS interaction remain in #96 and #97.

The original capture path used
[SendInput](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput)
for PrintScreen and
[BM_CLICK](https://learn.microsoft.com/en-us/windows/win32/controls/bm-click)
on uniquely identified game controls. Queued PrintScreen window messages did
not export an image. Native helpers worked when staged inside the prefix's
C drive; Unix-path invocation failed before producing an entry marker.
No Z drive mapping or game configuration change was needed. The previously
permission-blocked macOS screenshot and recording paths were skipped.

The [animation capture observations](front-animation-oracle.md) add eleven
warning phases and fourteen visible movie phases from the original renderer.

## Match entry and recovery

Main opens Single Player, actual controls or an exit confirmation. Internet,
network, campaign and movie/credits actions are visibly disabled. Single
Player opens saved matches or the supported skirmish setup. Setup shows the
retail map's authored preview and two participants. Names, nine Allied/Soviet
countries in retail registry order, eight colors, authored start indices, opposing teams, Short Game,
three native speeds, credits and unit count feed actual setup and saved metadata.
The form retains two participants; extra map rows stay closed. Country menus
exclude Yuri because the slave economy and its starting setup are unsupported.
Free For All, Team Alliance, difficulty and random maps remain unavailable.
Its form layout is provisional. The authored thumbnail is preferred; a map
without one falls back to the shared terrain renderer.

Each country uses its side's base, production catalog, starting force, AI
choices and SIDEC archive. Allied bases use GACNST, GAPOWR, GAREFN, GAWEAP,
GAPILE, and their ten-unit force uses AMCV, CMIN, four MTNKs and four E1s.
Soviet equivalents are NACNST, NAPOWR, NAREFN, NAWEAP, NAHAND and SMCV,
HARV, four HTNKs and four E2s. Required/forbidden-country restrictions still
filter production, including British snipers. Setup validates required types
and body art before committing the match. Map-authored objects retain their
types and remain separate from this generated force.

Names support up to 32 characters and 96 UTF-8 bytes, with no controls or
blank-only values. Duplicate colors, starts or matching nonzero teams are
unavailable. Teams identify opposing participants in this 1v1 setup; this is
not cooperative AI. Start selection retains authored waypoint indices even
when they are sparse. Auto chooses a different point from an explicit opponent.
Changing maps clears positions absent from the accepted map. Drop-downs support
mouse and arrow/Enter selection, Escape dismissal and Tab navigation. Clicking
outside dismisses the popup and consumes the click, including over Start.
Name editing supports selection replacement, Ctrl+A and Unicode backspace;
Enter moves focus without starting a match. Invalid names disable Start.

Selected colors come from retail [Colors] HSV values evaluated with bounded
integer ratios, avoiding float cancellation at exact component boundaries. They reach sprite remaps,
radar, fallback markers and loading bars. Short Game counts surviving structures
for victory; otherwise surviving units also count. Slow/Normal/Fast pace the
window at 10/15/30 steps per second without changing simulation tick arithmetic.
These are native speed presets, not measured original speed-slider timings.
Unsupported MCV Repacks, Crates, Superweapons and Build Off Ally remain disabled.
Soviet source controls differ from Allied dimensions: repair/sell are 52x32,
power pips 16x2 and tabs 32x28. Decode bounds and pointer hits use those sizes;
exact side-specific placement still needs original observation.

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
replacement, and ignores cancelled results. Its worker prepares the session,
terrain and sprite bank before committing the replacement. Preview loading
shows the selected map and current stage. Match loading uses the retail
selected country screen, the selected map thumbnail and preparation bars;
see [loading-screen.md](loading-screen.md) for asset evidence and calibration
limits. The bars represent completed stages, not elapsed time or network peers.
Back/Escape requests cancellation; the
worker checks that request before each stage. An immediate retry captures its
map and setup and waits for that worker to stop. Cancelling again removes the
queued retry. Stale progress and results cannot replace the active screen.
Starting a match drops the movie,
preview and front art, and clears held modifiers (#115). Return-to-menu drops
the match, queued input and old
presentation. Abort requires an explicit discard choice. Won/Lost screens
also offer Main Menu. Front loading failures keep the front context and a
usable route back. A movie failure can be dismissed while the other controls
remain usable.

The first Tab on Main Menu selects Single Player, and the first Shift+Tab
selects Exit. Further navigation skips disabled actions. Single Player uses
the same first-focus rule. These are native keyboard navigation choices.

Front-flow tests accept `OPENYR_GAMEDIR` for a supplied retail install, falling
back to the standard macOS folder. Set `OPENYR_FRONT_PROOF` to an external
directory to capture the menu, preview, loading stages and resulting match.
The cancel/retry regression drives the shared mouse and keyboard handlers,
checks the requested credits after start, and verifies that the sprite bank
is ready when loading ends. Synthetic worker tests hold completion so queued
cancellation, captured setup, stale progress and recovery routes are checked
without timing assumptions. Physical window input and live audio output are
not covered by these compositor captures.

Native startup and
load center the world viewport using its actual zoom, excluding the sidebar
(#114).

Player-name fields accept Ctrl+A or Command+A selection and ignore unsupported
modified text shortcuts (#248), consistently with the Save name editor.
Typing replaces the selected name and Backspace clears it; field navigation and
focus loss clear selection. Command is tracked separately from Control, so world
command/control-group routing is unchanged. See [name editing](menus.md).

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


## Saved map selection

Native front and pause loads follow the installed map reference in new match
saves (#159). A save created on Alamo can be loaded from the default EB1 front
menu. The asynchronous job rebuilds that map's session and terrain before
replacing the map name, sprites and audio state. Failed or cancelled jobs retain
the prior map and match. Version-1 files still use the current map because their
envelope has no filename; explicit CLI loads retain their supplied-map check.
The actual-handler regression checks front loading, pause loading across maps,
cancellation, malformed files, absent map assets and mismatched setup hashes.
The broader original chooser and setup layout remain in #96.

## Recovery message layout

Front recovery messages use a 568-pixel text width at `(32,72)` inside the
632-pixel body (#161). Lines use the loaded GAME.FNT glyph widths and tracking,
or the eight-pixel fallback advance. Newlines, blank paragraphs and CRLF input
retain their line breaks. Wrapping prefers whitespace; an unbroken filename
or token splits at a Unicode character boundary. Layout borrows the original
string and never truncates the diagnostic to a character limit.

The text viewport ends at y=544. Line spacing is at least 24 pixels and grows
to cover taller optional fonts. Overflow scrolls with Up/Down, Page Up/Down,
Home/End or the wheel. The footer shows the visible line range. Opening another
page or message resets its scroll position. Back, Enter on Back and Escape
retain their recovery routes; scrolling never reaches match input.

`retail_recovery_messages_use_font_width_and_stay_inside_the_panel` checks
proportional widths, Unicode token reconstruction, all reachable lines,
composition bounds and scaled Back hit boxes. The actual-handler test
`retail_front_recovery_messages_wrap_scroll_and_return_through_actual_handlers`
exercises short, multiline, overflowing and unbroken diagnostics. Set
`OPENYR_FRONT_PROOF=<external-dir>` while running that test to export shared
native compositor frames at presentation phase 0, including both ends of
overflowing messages. These frames contain retail art and stay outside git.
This is a native recovery policy; original dialog geometry and behavior remain
under #96. In-match message layout is unchanged by this slice.
