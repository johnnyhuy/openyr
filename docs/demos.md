# Game demonstrations

The README puts battlefield action before menus in Markdown feature tables.
The gameplay GIFs reuse earlier verified native shared-compositor captures.
They show engine simulation and retail rendering, with source captions retained;
they are sampled sequences rather than a fresh current-build window recording.
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
selection/health and starting-force revisions; they do not represent current
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

A fresh current-build gameplay recording remains in #339. The present Linux
environment has no retail install or archive to restore; only authored test MIX
fixtures were found. The README revision reuses captures and does not establish
a new engine run. Known blocked Mac capture paths and release uploads returning
HTTP 401 were not retried.

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
