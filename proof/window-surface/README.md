# Window surface input proof

Changed source d7a91933ff2d49cfcb31499477303e28d89d6861; baseline source f7dae46. Binaries and retail source
assets are excluded. Binary SHA256 receipts identify the two local builds.
This directory is a PR-branch proof fallback and is removed from the final tree
before a squash merge. Release upload previously returned HTTP 401.

The capture script starts Xorg, the native app and capture in the same task
session. Public X11 input navigates Main Menu, Single Player, Skirmish and
Customize Battle, then resizes the actual client to 1600x1000 and 1920x1200.
At each size it moves into the physical top margin and sends one wheel step,
then moves inside and sends another. The baseline list changes in the margin;
the changed list stays identical. Valid in-frame scrolling works in both.
JSON receipts record actual list-region hashes. PNG compression is lossless,
with matching decoded RGBA receipts. Both live runs include 10-second videos.

The initial compact-window experiment revealed cropping, not letterboxing.
Its 640x480 image is supplied only for follow-up #224. The current fix does
not change the fixed framebuffer or its scaling policy. No physical macOS
Retina, native audio output or original-game runtime claim is made.

Full workspace tests, targeted pointer handlers, strict Clippy, formatting,
release build and 19 observation-helper tests pass. The retail test selects
an actual unit, rejects stale/outside gestures and retains complete save bytes.
See docs/window-input.md for behavior and limits.
