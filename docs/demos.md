# Game demonstrations

The README links native screenshots and two H.264 MP4 clips. They are a live
Linux Xorg capture of the Main Menu, Single Player, Skirmish, country/map loading,
EB1 sandbox, keyboard zoom and pause/resume. The full uninterrupted video is
linked beside them. Screenshot timestamps, source tree, executable SHA256 and
FFprobe metadata are in the pinned capture receipt; asset hashes and byte counts
are in its manifest. The display is 1280x800 and the recording requests 30 fps.
Input was delivered through XTest, not a physical mouse/keyboard. The clips are
silent because the cloud session has no audio device.

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
the video region. Use H.264, yuv420p and MP4 faststart for browser playback. Keep
the full recording, event timestamps and FFprobe duration/dimensions, and inspect
representative stills before publishing. Stills and split clips are not evidence
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
