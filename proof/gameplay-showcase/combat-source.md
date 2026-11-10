# Allied starting-force proof

PR #127 uses source `1c6403cd0f0708c82fe8b4327a2e7f6a118d29c7`.
The before capture and save use its unmodified parent
`4c3166276f682bb556f2a57e612be657bcc526cc`, exported with `git archive`.
The baseline's six workspace crate build products were cleaned before building,
so its capture cannot reuse the current app binary from the copied cache.

`starting-force-comparison.png` shows the same EB1 camera at tick 1 and seed 42.
Both sides use the actual window compositor. The montage crops (320,280) to
(760,600) and enlarges each panel 2x with nearest-neighbor sampling. The original
1280x800 images are `before-tick1.png` and `allied-tick1.png`.
The visual fixture disables AI without changing the starting units.
The new tick-1 JSON lists the actual player ids, kinds, roles, health and positions.

`setup-americans.png` comes from the real front flow after preview loading,
before Start. It names the supported country as Americans (Allied).
`allied-cannon.mp4` shows 36 completed simulation ticks, 4 through 39, at 15 fps.
It enlarges the raw 400x280 impact crop to 800x560 with nearest-neighbor sampling.
The selected MTNK uses the actual 105mm weapon, raw damage 65, source
`68d6f6764b01f705`, and AP impact `S_CLSN22`. Arrival occurs at tick 4.
Raw PNGs and JSON sidecars are in `proof-frames.zip`.

The release demo uses normal AI and the existing scripted commands. It captures
ticks 1, 300, 900, 1500 and 2400. It reaches Won at tick 2312 and finishes tick
2400 at hash `469c2a67da6ed8ee`. All four sidebar categories are included.

The debug workspace run passed 300 tests, failed none and ignored the existing
native audio-device test. Format, strict clippy, debug workspace build and
release app build passed. The front navigation, start, save, return, load,
cancel and error test also passed while exporting the setup image.
The recordings contain the exact commands and results.

`save-compatibility.json` records actual saves from both binaries. The old
mixed-force setup key is `cf884e2f5d1d8396`; the corrected Allied key is
`a6a6bd8d0e79dd60`. The current binary rejects the old save with its normal
recoverable setup error. A new save loads and re-encodes byte-for-byte.
Both simulation payloads remain schema version 3 and the match wrapper remains 1.

These are native clean-room captures using the user's retail install, not
captures of the original engine. Starting composition and movement cadence
still need controlled original observation. MCV/GI deployment and Chrono Miner
teleportation remain unimplemented. Projectile travel units remain in #122;
the cannon clip does not claim original timing parity.

Live `screencapture -x` images and `screencapture -V` recordings were already
permission-blocked in this session and were skipped without retrying.
Shared window-compositor and CLI headless captures supplied the proof.
No retail artwork or game archive is committed to git.
