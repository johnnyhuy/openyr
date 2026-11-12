# Game demonstrations

The README puts battlefield action before menus in Markdown feature tables.
The current gameplay table uses the retail-backed EB1 recording published with
[PR #372](https://github.com/johnnyhuy/openyr/pull/372). The recording was captured
on macOS through the native shared compositor with scripted orders. This README
update verifies and publishes those existing captures; it does not claim another
engine run in the Linux documentation environment.

## Current gameplay recording

[Full session with synchronized audio](https://raw.githubusercontent.com/johnnyhuy/openyr/de23c15a4b438357fb6db4a1291efaf8d74bf102/proof/retail-regressions/full-session.mp4)
retains all 2999 frames from ticks 2-3000 at 1280x800, 15 fps and 199.933 seconds.
Tick 1 is the initial state and is outside the MP4. Frame time is `(tick - 2) / 15`
seconds. This is scripted headless gameplay, not a desktop recording, physical
input test or original-game trace.

| README GIF | Recorded action | Retained interval |
| --- | --- | --- |
| Production and placement | GAPOWR and unit batches queued at tick 2; power plant placed at tick 560. | Ticks 500-619, 33.2-41.2 seconds, 120 frames at 640x400. |
| Battlefield assault | Attack-move every 450 ticks while enemy structures remain; vehicle production stays active. | Ticks 1400-1579, 93.2-105.2 seconds, 180 frames at 640x400. |

The full session also queues GAREFN at tick 1200 and places it at tick 2480
(165.2 seconds). GIFs use 15 fps, a 128-color palette and no audio; the full MP4
retains music, voices and combat audio. These frame times describe the native demo,
not calibrated original-game animation or wall-clock input timing.

[The frozen capture bundle](https://github.com/johnnyhuy/openyr/tree/de23c15a4b438357fb6db4a1291efaf8d74bf102/proof/retail-regressions)
retains the full MP4, both GIFs, ticked stills, conversion commands, event/audio
metadata, runtime receipts and a 50-payload size/SHA256 manifest. Its isolated
recording executable is from `089267c7a45a6d4effe3bbc45464afec216ee1cc`, tree
`0587a946fa381f05f71f59ee446ddb7f2cdceb20`. The final verification executable is
from `de0187ce95138d27eba61481dcd90b65bc20782e`, tree
`068003bdbf657cd9700b8c20861b583668d61c31`, which equals merged main `2adf44b`.
That intervening change updates tests and documentation; production code is
unchanged. All twenty final-build exports match the recording's sampled exports
byte for byte. The separate final run reaches Lost at tick 5738 through tick 6000;
that continuation is not included in the 199.933-second MP4.

This documentation pass verifies all 51 remote blob sizes and Git IDs, all 50
payload SHA256 digests, the README media responses and decoded animation/video
metadata. The source frames are gameplay output; neither their reuse nor the
rendered README checks establish a new retail run or physical Mac/Retina/audio
verification here. The source run's native window and audio smoke checks are
reported separately in its proof notes. Known Mac Screen Recording permission
blocks and release-upload HTTP 401 were not retried.

## Earlier captures

The former README gameplay previews remain available as historical examples.
They predate the current recording and retain their source captions and sampled
tick limits.
The [gameplay media and conversion manifest](https://github.com/johnnyhuy/openyr/tree/371bb427c434218bc571a819ff7797b3472725e7/proof/gameplay-showcase)
records input/output SHA256 values, byte sizes, dimensions, frame counts and the
FFmpeg conversion command. All inputs were downloaded again and their sizes
checked against GitHub release metadata.

| Gameplay GIF | Source | Presentation |
| --- | --- | --- |
| Tank combat | [Allied starting-force proof, PR #127](https://github.com/johnnyhuy/openyr/releases/tag/proof-allied-starting-force-1c6403c-20261101), source `1c6403cd0f0708c82fe8b4327a2e7f6a118d29c7` | All 36 frames from `allied-cannon.mp4`, ticks 4-39, 15 fps; nearest-neighbor reduction to 400x280 and a 128-color GIF palette. |
| Infantry movement | [Infantry animation proof, PR #55](https://github.com/johnnyhuy/openyr/releases/tag/proof-infantry-20261027-152706) | Byte-identical `eb1-walk.gif`, eight sampled frames at 1000x600, retaining its tick caption. |
| Base operations | [Structure-controls proof, PR #99](https://github.com/johnnyhuy/openyr/releases/tag/proof-structure-controls-20261031) | Byte-identical `repair-sale.gif`, nine frames at 850x390; its controlled-damage and sampled-tick disclaimer remains visible. |

Each GIF decodes to multiple distinct frames. The original combat clip and its
source notes are retained with the derived GIF. These examples predate later
selection/health, unit contrast/sampling and starting-force revisions; they do not represent current
visual parity, physical Mac/Retina delivery or original-game timing. They have
no audio. Game content retains its original ownership.

The menu/setup table retains earlier native Linux window captures. Their
aspect-preserving conversions use 8 fps and a 128-color palette.
The [menu GIF conversion manifest](https://github.com/johnnyhuy/openyr/tree/1d3c9840cfa1293881ee093db9aec3e72a253235/proof/readme-features)
records input hashes, commands, intervals and output hashes/sizes.

| Menu GIF | Source recording | Interval |
| --- | --- | --- |
| Main menu | Native demo, source `11379ae` | 0-4 seconds |
| Loading | Native demo, source `11379ae` | 12-22 seconds |
| Save name | Name-shortcut fix, source `99968fa` | 0-5.5 seconds |

The original demo/capture receipt is retained in
[the native demo proof](https://github.com/johnnyhuy/openyr/tree/c7deba4c12ab133026c00570e4a2c78d7fa3fa65/proof/native-demo).
The Save sequence and its receipt are retained in
[the name-shortcut proof](https://github.com/johnnyhuy/openyr/tree/346aeb37c82a332d87ab9755f3094ac5c6f8cff8/proof/name-shortcuts).
The display was 1280x800 with synthetic XTest input. The README uses image URLs
instead of MP4 playback links.

The fresh recording and README follow-up for #339 use the current recording
above. Older examples remain historical and are not relabelled as new captures.
The Linux documentation environment still has no retail install; the recording
came from the separate macOS run.

## Make a fresh demo

Install the project tools with `mise trust` and `mise install`. Commit the source
before building, then use a new proof directory:

```sh
mise run proof:build -- --output .tmp/demo-001
OPENYR_GAMEDIR=/path/to/RA2 mise run proof:run -- \
  --receipt .tmp/demo-001/receipt.json
```

Record the verified native window using the available desktop capture path.
Keep the gameplay sequence on the battlefield: select and move units, issue an
attack, queue/place a structure and show harvesting or repair. Retain the full
recording and event timestamps, then derive short GIFs from the action intervals.
On Linux, Xorg, OpenYR and the capture process must run in the same task session
so they share IPC. Xorg with the dummy driver, libXtst, ImageMagick and FFmpeg are
optional capture dependencies, not game dependencies; mise does not install
these system packages. The pinned `capture.py` documents the exact cloud command,
coordinates, delays and XTest sequence for this recording. Adapt display paths
and game data to your machine. It is an evidence script, not a portable runner.

FFmpeg's `x11grab` input records the desktop region; ImageMagick's `import -window`
records the app window. Check the native window size and origin before selecting
the video region. Keep
the full recording, event timestamps and FFprobe duration/dimensions, and inspect
representative stills before publishing. To embed animation in Markdown, convert
a short interval to a looping GIF and verify its frames, MIME type and public
image URL. Stills and GIFs are not evidence
of original-game runtime timing or physical Mac/Retina behavior.

## Publish without adding game assets to main

Follow [runtime proof](runtime-proof.md) and [AGENTS.md](../AGENTS.md). Retain the
build receipt, full recording and media checksums outside the source checkout.
Use release assets when uploads work; the authorized fallback is a temporary
`proof/` directory in a feature branch, removed before squash merge. Pin README
links to that proof commit, verify remote blob IDs/sizes, and retain source commit
labels even if only documentation changes afterward. Never merge media files,
programs, saves or raw assets into main. Demo media is derived artwork, not an
engine distribution or a replacement for user-owned game data.
