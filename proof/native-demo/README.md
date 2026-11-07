# Native OpenYR demo

Captured from source 11379ae with `mise run proof:build -- --output .tmp/runtime-demo`.
`capture.json` contains the isolated-build receipt, source tree, executable SHA256,
video metadata and screenshot times. `manifest.json` records media bytes/SHA256.
The executable was d2647c285a80153fbf1d6d50188f7a90baea0245bb2e9d66ad9df1cde1ff161e.

The native Linux window runs at 1280x800 on Xorg with synthetic XTest input and
installed retail RA2/YR data. `capture.py` records the exact commands and UI
sequence. Xorg, app and FFmpeg ran in the same task session. Still images use
ImageMagick's window capture; video uses FFmpeg x11grab at 30 fps and H.264.
No original Windows executable ran. No physical Mac, Retina or audio check ran;
the video is silent. This is current OpenYR behavior, not original timing parity.

- `start-flow.mp4`: first 24 seconds, Main Menu through Skirmish and loading.
- `match-pause.mp4`: remaining 12 seconds, world zoom and pause/resume.
- `native-demo.mp4`: uninterrupted 36.17-second recording from which both clips derive.
- `overview.png`: world at three keyboard zoom-out steps after initial entry.

This directory is retained only in a pinned PR proof commit. Remove all proof
files from the final branch tree and squash merge. It contains derived game
artwork for demonstration, not assets required to build or run the engine.
