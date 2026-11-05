# Match loading screen

The supported two-participant skirmish uses the retail multiplayer country artwork,
with the selected match's map thumbnail in its lower-right preview frame.
This is separate from the shell panel used while preparing Skirmish setup.
The shared compositor handles both front-menu starts and pause-menu restores.

## Retail assets

All country pairs resolve through `RA2MD.MIX/LOADMD.MIX` in the supplied install.
These names are confirmed against the MIX CRC IDs and are included in the
inspection tool's name database.

| File | MIX ID | Bytes | Observed contents |
| --- | --- | --- | --- |
| `LS800USTATES.SHP` | `6affc0dc` | 481232 | 800x600, one full-canvas frame, compression 2 |
| `MPLSU.PAL` | `7a0171a5` | 768 | 256 six-bit RGB triples for the American loading artwork |

The other selectable countries use these verified SHP/palette pairs from the
same archive. Every SHP is one 800x600 compression-2 frame, 481232 bytes; every
palette is 768 bytes. No guessed fallback palette substitutes for a country.

| Country | SHP | Palette |
| --- | --- | --- |
| Korea | LS800KOREA.SHP | MPLSK.PAL |
| France | LS800FRANCE.SHP | MPLSF.PAL |
| Germany | LS800GERMANY.SHP | MPLSG.PAL |
| Britain | LS800UKINGDOM.SHP | MPLSUK.PAL |
| Russia | LS800RUSSIA.SHP | MPLSR.PAL |
| Cuba | LS800CUBA.SHP | MPLSC.PAL |
| Libya | LS800LIBYA.SHP | MPLSL.PAL |
| Iraq | LS800IRAQ.SHP | MPLSI.PAL |

The loading-art cache is bounded to these nine known pairs. Country title and
brief labels use the selected country's CSF keys. Badges are 47x23 PCX files
with their own palettes and transparent magenta, scoped to the selected country. Caption colors use retail
[Colors] AlliedLoad=164,255,255 or SovietLoad=0,235,255, interpreted through the
same HSV conversion as house colors. Player labels and bars use the selected
names and color indices. Validated saved metadata selects these before
session reconstruction rather than showing the previous match's country.

The SHP has a 32-byte combined header and frame record. Its 600 rows each
contain a two-byte length of 802 followed by 800 indices. Index zero is drawn
over black. Palette expansion uses the existing six-bit conversion, then
the shared RGB565 presentation quantum. The campaign palette `LS800A01.PAL`
and generic `MPLS.PAL` are not substitutes for this country image.

Source SHA256 receipts:

```text
LS800USTATES.SHP d1f01f393ae5abc1840093899c2353e3aca4b221678c213004adc7d0a6eb5b6c
MPLSU.PAL a9e530e442a886cc8a82c978b0a97a58991508a16e9ecb979f25bb55e6cd9d47
```

The original untouched country-map receipt was `(450,32,316,313)`, 98908
pixels, RGBA FNV-1a `97061e52e7661e85` after RGB565 quantization. The country
name now occupies its observed lower-right position inside that panel. The
current source-art regression checks `(450,32,316,268)`, 84688 untouched pixels,
with independent raw-row/palette hash `12a2489dceca70f1` for America, at every
stage. All nine countries also compare every pixel in that untouched region
against their own raw rows. This checks source composition separately from
the original text measurements below.

## Map and loading state

The match preview is fitted into `(500,380,216,166)`, inside the stock lower
frame. It keeps both axes and uses black letterboxing. The existing preview
loader prefers the map's authored image and otherwise renders its terrain.
Malformed or absent optional preview data leaves the pane black and does not
prevent a valid match from loading. Missing country art uses a dark background
with the map and controls still visible. No asset dimensions control an
unbounded allocation: country decoding requires exactly one 800x600 frame.

Fresh starts can use the accepted Skirmish preview immediately. Saved-match
loads initially hide it because the saved map may differ. Once session
reconstruction resolves the installed map, the worker sends that map name
and its preview. Cancelled progress and images are discarded; a queued retry
captures its own setup. The old active map changes only when loading succeeds.

One local progress row represents this process preparing the match for both
participants. A computer opponent does not have an independent loading client.
Completed stages advance the bar without a measured percentage, remote status
or artificial delay. The row uses the selected human name, color and badge.
`GAME.FNT`, the selected specialty `Name:` key and full `LoadBrief:` value,
`GUI:LoadingEx` and the selected country's `Name:` key supply the retail text.
The specialty keys are `para`, `beagle`, `gtgcan`, `tnkd`, `snipe`, `dtruck`,
`deso`, `terror` and `ttnk` in retail country order. The Libyan briefing key
is actually spelled `LoadBrief:Lybia` in the supplied CSF. These are presentation
labels; country choices do not implement absent special gameplay abilities.

## Public original loading observation

[CodeWeavers' compatibility page](https://www.codeweavers.com/compatibility/crossover/command-and-conquer-red-alert-2-yuris-revenge)
provides a direct 800x600 original loading PNG, captioned as a CrossOver 13.1.0
capture on macOS 10.9. It uses Yuri, which is a static layout reference only;
Yuri remains absent from supported gameplay choices. The
[original image](https://media.codeweavers.com/pub/crossover/website/appdb/255644f70a7152708e96c9dd87e773e2.png)
has SHA256 `c803596685de291f2d50aa36ad3762140c3a49fbdb30b43a4dc2de5f3e8da768`.
No original implementation code or process memory was read. Public country
asset/key documentation helped locate candidates; every used file/key is
verified in the supplied archives and CSF.

| Element | Observed native-canvas position | Verification |
| --- | --- | --- |
| Specialty title | `(20,90)` | uppercase, black; 119 foreground pixels |
| Full briefing | `(20,158)` | 400-pixel word wrap, 17-pixel line pitch; 2854 foreground pixels |
| Loading label | `(20,300)` | 201 foreground pixels |
| Country label | right aligned to x=740, y=310 | 89 foreground pixels |
| Country badge | `(117,323)`, 47x23 | 716 non-magenta source pixels |
| Human name | `(174,326)` | bracketed; 306 foreground pixels |
| Progress border | `(21,329)`, 86x11 | observed geometry; native stage fill/style |

Five complete foreground masks contain 16638 samples and 3569 foreground
pixels. The regression reconstructs them with the shared detail/client helpers
and compares exact independent FNV-1a receipts from the reference. Both sides
are normalized to RGB565 high bits because the published PNG expands five/six
bit channels to eight bits. The source flag's 716 non-magenta pixels also match
the original exactly at the observed position. Optional exported captures use
the same helpers; no screenshot or source art is a repository fixture.

The screenshot foreground normalizes to `(248,20,16)`. Retail
`SovietLoad=0,235,255` evaluates exactly to `(255,20,20)` before RGB565. The
previous floating-point cancellation made the low components 19, changing the
green channel to 16. Bounded integer HSV ratios retain 20, and retain the exact
36-valued green component in `AlliedLoad=164,255,255`. The shared conversion also
serves Skirmish swatches and house remaps; no simulation state changes.

The five comparisons establish text shape and placement, not full-screen pixel
parity. Text backings, bar fill/gradient, the lower map preview and country-label
backing remain native choices. The reference map was not reliably identified
in the installed active catalog, so its preview bounds/sampling are not inferred.
A single static capture cannot establish progress cadence. Map/save names and
cancellation remain native additions. Actual original startup is still tracked
by #148; this public reference unblocks static measurements without an
unsandboxed original-game run.

The native cancel control occupies `(16,568,160,25)`. Its painted text and
mouse target use the same 800x600 transform as the background. Escape and
right-click keep the existing cancellation route. Front and pause loading use
the same coordinates; there is no invisible shell-sidebar hit target.

## Verification and remaining limits

Set `OPENYR_GAMEDIR` to a supplied install and `OPENYR_LOADING_PROOF` to an
external directory, then run `cargo test -p yr-app loading -- --test-threads=1`.
The retail test exports session, terrain and artwork-stage captures for EB1
and Alamo. Synthetic tests check invalid headers, aspect ratio, missing art,
small surfaces and scaled cancel hits. A controlled-worker test verifies that
a saved Alamo preview replaces an EB1 thumbnail without committing the map,
and that cancellation rejects later images. Front-flow tests additionally
exercise actual navigation, retry, start, save, restore and return handlers.

The artwork and preview frame are observed from the supplied retail data.
The measured static text and badge positions are listed above. Progress fill,
preview sampling and runtime timing remain uncalibrated. No full-screen
pixel-parity claim is made. A Linux Wine reference attempt did
not reach a usable original window. The first native Xorg capture crossed
isolated IPC namespaces and returned black with Mesa shared-memory errors.
Running Xorg and the native app in one task session resolved that capture
failure. Live screenshots and an eight-second, 60 fps X11 recording exercise
Main Menu, Single Player, Skirmish, loading and match entry through public
pointer input. Compositor exports retain each preparation stage separately.
That Linux session had no output device. Retail images and derived match data
stay outside git.

The macOS follow-up ran the full workspace test suite with the retail install,
strict clippy, formatting, all 19 observer tests and the recursive archive scan.
All passed. The native Main Menu process opened a 48000 Hz stereo F32 audio
device, and a release EB1 frame run reached Lost at tick 5767. Its tick-6000
hash was `752f3f10fc2821e5`, matching the Linux receipt. The actual front-flow
handler test exported cancel/retry, session/terrain/artwork preparation and
match-entry frames; separate source-art captures cover EB1 and Alamo.

The sandboxed window launch could not connect to macOS services. Launching
outside the sandbox reached Main Menu. Screenshot capture still failed with
`could not create image from display`, including outside the sandbox. The
recording attempt produced no usable file. Use the shared-compositor captures
for this run's proof. Live visual inspection, Retina input and audible playback
remain unverified; opening an audio device alone does not verify playback.

The Linux participant follow-up verifies all nine untouched country-map panels
against independent raw 802-byte row and palette traversal, along with selected
bar colors. Real menu handlers cover popup selection, dismissal over Start,
name entry, Short Game, speed, country loading, match entry and restart.
The latest sandboxed original-game probe used the supplied launcher in a
separate Wine prefix. It exited with signal 11, created no windows and yielded
no usable original capture. Automatic approval review rejected an unsandboxed
reference run because it would expose the wider filesystem and inherited
environment. That run was not retried. The later public capture supplies static text/badge measurements without
rerunning that executable. Exact progress style, preview placement and runtime
timing remain uncalibrated; source-pixel checks do not establish runtime parity. #148 tracks the reference startup failure.
