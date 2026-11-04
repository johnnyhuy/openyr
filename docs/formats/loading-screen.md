# Match loading screen

The supported American sandbox uses the retail multiplayer country artwork,
with the selected match's map thumbnail in its lower-right preview frame.
This is separate from the shell panel used while preparing Skirmish setup.
The shared compositor handles both front-menu starts and pause-menu restores.

## Retail assets

Both files resolve through `RA2MD.MIX/LOADMD.MIX` in the supplied install.
These names are confirmed against the MIX CRC IDs and are included in the
inspection tool's name database.

| File | MIX ID | Bytes | Observed contents |
| --- | --- | --- | --- |
| `LS800USTATES.SHP` | `6affc0dc` | 481232 | 800x600, one full-canvas frame, compression 2 |
| `MPLSU.PAL` | `7a0171a5` | 768 | 256 six-bit RGB triples for the American loading artwork |

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

The untouched country-map panel at `(450,32,316,313)` contains 98908 pixels.
An independent row and palette traversal gives RGBA FNV-1a
`97061e52e7661e85` after RGB565 quantization. The retail compositor regression
checks that whole region at each loading stage. This verifies source-art
selection and composition, not original runtime parity.

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

The two preparation bars represent the supported local player and computer.
They advance at completed session and terrain stages while the worker prepares
the sprite bank. They do not claim a measured percentage or remote peer status.
`GAME.FNT`, `Name:Americans`, `LoadBriefShort:USA`, `GUI:LoadingEx` and
`GUI:Battle` supply the retail text. The current player labels follow the fixed
Skirmish profile. There are no new faction choices or gameplay abilities.

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
Text placement, player-bar geometry, preview sampling and progress timing
are native choices until a usable original loading-screen capture is obtained.
No full-screen pixel-parity claim is made. A Linux Wine reference attempt did
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
