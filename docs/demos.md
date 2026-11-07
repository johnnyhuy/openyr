# Game demonstrations

The README embeds screenshots and four looping GIFs in a feature table. GIFs
use existing verified native Linux recordings, with aspect-preserving downscaling,
8 fps sampling and a 128-color palette. They are real game-window sequences,
not generated illustrations. `conversion.json` in the pinned GIF proof records
the input file SHA256, FFmpeg command, start/duration, dimensions and output
hash/size. The [GIF conversion manifest](https://github.com/johnnyhuy/openyr/tree/1d3c9840cfa1293881ee093db9aec3e72a253235/proof/readme-features) is pinned separately from source. The original captures retain their own source/build receipts.

| GIF | Source recording | Interval |
| --- | --- | --- |
| Main menu | Native demo, source `11379ae` | 0-4 seconds |
| Loading | Native demo, source `11379ae` | 12-22 seconds |
| World camera | Native demo, source `11379ae` | 23-30 seconds |
| Save name | Name-shortcut fix, source `99968fa` | 0-5.5 seconds |

The original demo/capture receipt is retained in
[the native demo proof](https://github.com/johnnyhuy/openyr/tree/c7deba4c12ab133026c00570e4a2c78d7fa3fa65/proof/native-demo).
The Save sequence and its receipt are retained in
[the name-shortcut proof](https://github.com/johnnyhuy/openyr/tree/346aeb37c82a332d87ab9755f3094ac5c6f8cff8/proof/name-shortcuts).
The display was 1280x800 with synthetic XTest input. GIFs have no audio and do
not establish physical Mac/Retina behavior or original-game timing parity.
The README uses ordinary image URLs instead of MP4 playback links.

## Make a fresh demo

Install the project tools with `mise trust` and `mise install`. Commit the source
before building, then use a new proof directory:

```sh
mise run proof:build -- --output .tmp/demo-001
OPENYR_GAMEDIR=/path/to/RA2 mise run proof:run -- \
  --receipt .tmp/demo-001/receipt.json
```

Record the verified native window using the available desktop capture path.
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
