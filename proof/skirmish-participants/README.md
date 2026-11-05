# Skirmish participant proof

Source commit 118d7e013f37f82ae60da91b52f182b270ba9182. These files are a
PR-branch proof fallback because the release upload endpoint returned HTTP 401
in this session. They are removed from the final branch tree before a squash
merge, so no retail-derived image or video enters main.

The live Xorg capture drives the release window through public pointer input.
The 10-second video is 1280x800 at 60 fps (599 frames). Country exports use the
shared loading compositor. PNG recompression is lossless; pixel-receipts.json
records matching decoded RGBA hashes before/after compression. Source SHP/PAL
files, game executables, saves and binaries are excluded.

All nine retail country-map panels pass independent raw-row/palette checks.
The native EB1 run reaches Lost and tick 6000 hash 752f3f10fc2821e5.
No audio device is available in this Linux session. Actual macOS Retina and
audible playback are unverified. Original-game startup still fails (#148),
so these captures establish native behavior and source-art selection, not exact
original text/bar geometry or timing. Country special abilities and Yuri remain
unsupported. See docs/formats/front-menu.md and loading-screen.md in the source.
