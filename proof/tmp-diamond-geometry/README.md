# TMP diamond geometry proof

The recorded terminal session runs isolated release binaries on authored MIX
fixtures. Before: main d32cbf2 (tree matching before receipt source a0a39b6)
accepts a 2x1 TMP, then render panics with exit 101. After source
6556ec9f9b8aba825bda6b994f124cc6ccc3e6f1: incompatible 2x1, 6x3 and 10x5
headers exit 1 without replacing protected output. These are expected rejection
results. The supplied fixtures contain only authored bytes and a red palette.

Valid 4x2, 8x4, TS 48x24, RA2 60x30 and 256x128 controls retain byte-identical
before/after PNGs. The independent Python PNG check validates chunk CRCs,
RGBA geometry and every red/transparent pixel, including exact packed area.
The Rust regressions check all 64 supported widths, invalid public dimensions,
CLI output preservation and filename-specific terrain-loader failure.

The terminal recording is terminal.typescript (also terminal.txt). Commands,
exit codes and output are retained in results.json and proof.py. Each child
has a 15-second timeout and a 512 MiB address-space limit. No original executable
or game process memory is used. The baseline and final receipts pin source
and executable hashes; no native binaries or saves are uploaded.

Final source passes 690 retail-backed Rust tests with one existing audio-device
test ignored, strict Clippy/fmt and three Python provenance checks. All 16 retail
MIX archives scan with zero errors. Fresh/restored tick-6000 save bytes match the
merged-main baseline, retaining state hash 3082cb602467f8fc.

This is nonvisual parser/CLI proof. Physical Mac/Retina, live audio and original
runtime checks did not run; known blocked capture/reference paths were not
retried. All temporary proof files are removed before squash merging.
