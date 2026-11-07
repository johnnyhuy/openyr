# Pause recovery proof

This slice fixes #234 within #96/#97. The existing pause compositor failed the new narrow-font regression: 200 glyphs fit a measured row, but main 02e5f51 split them at 50 characters. The baseline regression log records that expected failure. The final renderer preserves complete diagnostics and shares wrapping with the front menu.

The shared compositor fixture first produces a real failed atomic save with a path longer than 400 characters, then appends a controlled overflowing diagnostic to exercise every page. First/last captures at 640x480, 800x600 and 1280x800 show the recovered suffix. Handler tests verify arrows, page keys, Home/End, wheel, Escape, frozen ticks and complete saved-match byte equality. The generated saves are excluded.

The live Linux fixture creates a temporary retail-backed save through the normal headless CLI, loads it in the native window and replaces only that generated save destination with a directory. F5 then produces a real atomic-rename error containing a long synthetic path. End reveals its final reason, Home reproduces the first frame, a later frame remains identical while paused, and Escape returns to Save. Xorg, the app, XTest input and ImageMagick capture share one task session. Receipts include the binary hash and capture assertions.

These are native error layouts and controlled filesystem failures, not original-game captures or proof of original menu parity. No physical Mac, Retina, audio-device or original-runtime checks ran. Previously permission-blocked macOS screencapture paths and rejected original-runtime launches were skipped. Earlier release-upload HTTP 401 prevented that route; this uses the authorized temporary branch-proof fallback, removed before squash merge.
