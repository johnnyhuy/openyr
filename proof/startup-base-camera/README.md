# Owned-base startup proof

The before image is a native Linux Xorg window from main 2f9b83c before input.
It shows city scenery instead of the owned base. The after image and video use
source 07c6326a01c3302e207d170f954edc21c55ae7df through its isolated build receipt.
The corrected recording starts at the owned base without H, then uses synthetic
selection, move, attack-move, production and pause/resume input. The raw actions,
window captures and event timestamps are retained.

The video is silent: no audio device was available. This is native Linux window
capture, not physical input, Mac/Retina verification or original-game timing.
Known permission-blocked original/Mac capture paths were not retried. Xorg, the
app and FFmpeg ran within one task session with Unix socket access enabled.

Final checks passed 683 Rust tests with one existing audio-device test ignored,
strict Clippy, formatting and three Python provenance tests. The 16-archive
retail scan used the isolated unchanged parser build from main 2f9b83c.
Source SHA/tree and exact commands/results are in validation.json and receipts.
No original executables were extracted or run. The supplied data and saves stay
outside Git; only derived proof is present in this temporary branch commit.
