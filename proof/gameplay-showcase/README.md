# Battlefield gameplay showcase

These are reused verified native gameplay captures from earlier OpenYR builds, not a fresh recording of current main. Source releases and input/output SHA256 values, sizes, dimensions and animated frame counts are in conversion.json.

- combat.gif derives from the 36-frame allied-cannon.mp4 capture in PR #127, source 1c6403cd0f0708c82fe8b4327a2e7f6a118d29c7. It shows native 105mm/AP impacts at simulation ticks 4-39. The original clip is included; source documentation records controlled setup, composition and original-calibration limits. Conversion uses the full 2.4 seconds at 15 fps, 400x280 nearest-neighbor reduction and a 128-color GIF palette.
- movement.gif is the byte-identical EB1 infantry movement GIF from the infantry animation proof release, with its original tick caption intact. It is a sampled shared-renderer sequence, not real-time input delivery.
- base-operations.gif is the byte-identical repair/sale GIF from the structure controls proof release. Its controlled-damage and sampled-tick disclaimer is retained.

Downloaded input sizes match GitHub release metadata. Each GIF decodes into multiple distinct frames. No timing, physical Mac/Retina or current-build claims are inferred. These native captures use retail data, not original executable implementation or process memory.

A fresh current gameplay recording is blocked: retail MIX archives and an install/archive to restore are absent here. The known permission-blocked Mac capture paths and HTTP 401 release-upload path were not retried. The media is retained only in branch history, linked by commit, and removed from the source tree before squash merge. Depicted game content remains owned by its rights holders.
