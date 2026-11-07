# Saved-file confirmation proof

Source 43b38d5, tree 4a1ed5b375c3d4b07fafa72bbd1ad3b56f95530d, was clean
before `mise run proof:build -- --output .tmp/runtime`. The isolated executable
SHA256 is 975a1e8a4e19aa7734dcdafc7000c2174472b8356bd888a6cadfe5e39cf09539.
Capture ran only through the provenance guard. The baseline is verified source
5473606; its engine files equal latest pre-fix main e401d00 (the intervening
commit changes only README/docs). Both phase receipts retain their own build.

`capture.py` starts Xorg, the guarded app and capture in one task session. At
640x480 and 1280x800 it uses actual keyboard events to open Pause Save overwrite,
Load and Delete confirmations, then Main/Single Player Load and Delete. Two
48-character names differ only in A/B; an external 201-character stem ends in Z.
Every operation is declined, including default No on front deletion, and all
four temporary save files retain identical bytes. The front heading guard
rejects an incorrect route and verifies No returned to the save list. Only the
corrected/guarded v2 captures are published. Saved files are not published.

The comparison enlarges no game pixels; each cell is an actual 640x480 native
window with a caption. Raw frames cover both sizes. `confirmation-flow.gif`
is a scaled 8 fps loop of the fixed 1280x800 live recording. Full recordings,
event timestamps, build receipts and GIF conversion metadata are retained.

The pre-fix compositor test fails because two Save targets yield identical
frames. Targeted checks pass: 4 library tests and 3 handler tests, including
both skins/fonts, Unicode, spaces, escaped controls, complete reachable overflow,
button navigation/defaults and actual retail save/load/delete byte invariance.
`mise run check` passes 475 Rust tests, 0 failures and 1 existing audio-device
ignore, strict Clippy/fmt and 3 Python provenance tests. This slice changes no
simulation or save fields and exposes no parent directories.

These are native Linux/Xorg captures with synthetic XTest input and silent
recordings. No original game process was run. Physical Mac/Retina/audio-device
and original dialog/timing calibration were not checked. Known blocked macOS
capture/original-reference paths and earlier HTTP401 release uploads were not
retried. The authorized pinned-proof fallback is used instead.

Remove every proof file before squash merge, and verify final tree equality
with the tested source. No artwork, saves or executables belong on main.
