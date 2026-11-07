# Name-edit shortcut proof

Source 99968faf402fa251495a3759c10d3cff07e917cf, tree
1ae5e09b7ac938fa5b03cea48929732b5f6f871c. The clean isolated mise proof build
produced executable SHA256
7f222d088cc7f1547c95d548acb79b85201e791a37388e831d667d9190d61fdf.
The baseline is merged main f9b3636, verified against its existing isolated
build receipt immediately before capture. Both capture receipts retain full
build provenance. No shared-target executable was used for window evidence.

`capture.py` starts Xorg, the guarded native executable and capture processes
inside one task session. It loads an external retail save, opens Pause/Save,
types oldcheckpoint, sends Select All and Copy with Control or Super, types
newcheckpoint, then actually saves. Four cases cover 640x480 and 1280x800 with
both modifier families. Baseline saves are oldcheckpointacnewcheckpoint; all
fixed saves are newcheckpoint. Every file round-trips through the guarded native
loader and writer with identical bytes. Those save files are not published.
Cross-run hashes differ with live ticks; no cross-run timing/simulation parity
is inferred. The actual-handler retail test separately compares complete live
snapshot and saved bytes around the operation.

The same native script enters Skirmish and checks both player-name rows with
Super+A and replacement typing. Blue selection pixels are absent before and
present after. `comparison.png` enlarges selected/replacement editor crops at
2x nearest-neighbor scale; representative full frames and all 24 crops remain.
Videos show the actual 1280x800 Super-key Save sequence, not an image slideshow.
FFprobe metadata records their dimensions, frame rate and duration.

`regression-before.txt` is the actual pre-fix handler failure: Ctrl+A appends a.
`targeted-handlers.txt` and `targeted-compositor.txt` pass. `check.txt` is the final
`mise run check`: 469 Rust tests passed, 0 failed, 1 existing audio-device test
ignored; Clippy/fmt and three Python provenance tests passed. The unit/handler
checks additionally cover Unicode/input bounds, selected Backspace, unsupported
shortcuts, slot/page/navigation and focus/modifier cleanup.

Captures are native Linux/Xorg with synthetic XTest input. Super simulates the
Command modifier; physical Mac, Retina and original-game shortcut/timing parity
were not checked. Videos are silent; no audio-device check ran. Clipboard
copy/cut/paste behavior is not implemented, and unsupported shortcut letters
now leave names unchanged. Known blocked macOS capture/original-reference runs
were not retried; earlier HTTP401 release uploads were skipped for the authorized
pinned-proof fallback.

The proof commit alone retains this derived artwork. Remove every proof file
before squash merge and verify the final tree matches the tested source tree.
No game assets, executables or saves belong on main.
