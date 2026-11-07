# Final source verification

Source 8ebd3f8, tree 067493cf6043cde3fbee170fd85dfc2a9716d4ca, was clean before
the isolated mise proof build and this live native capture. Receipt and full
check output are included. Executable SHA256 matches the README demo build:
d2647c285a80153fbf1d6d50188f7a90baea0245bb2e9d66ad9df1cde1ff161e.

`capture.json` records screenshot timestamps, video metadata and the build
receipt. `native-demo.mp4` is the full live Linux/Xorg recording through Main,
Single Player, Skirmish, loading, match, three zoom-out steps and pause/resume.
The same capture script in the pinned README demo proof was used, with final
source repo/receipt and a fresh output directory as its three arguments.

Tests ran through `mise run check`: 464 Rust tests passed, one existing audio
device test ignored; fmt/Clippy passed; three proof-helper tests passed. Retail
data was present. `mise run run` and `mise run proof:run` both reached EB1 tick 1
with hash c4740e7a21c92521. Final source only changes README/demo documentation
from the tree where the same mise/code/config checks began.

Video is silent; input is XTest-generated. Physical Mac, Retina, audio device
and original-runtime timing were not checked. No original executable ran.

Remove every proof file before squash merge. No media, binaries or saves in main.
