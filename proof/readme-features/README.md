# README feature GIFs

These GIFs are re-encodings of existing verified native Linux window recordings,
not newly synthesized game scenes. `conversion.json` records source SHA256,
FFmpeg command, interval, dimensions and output sizes/hashes. All GIFs loop at
8 fps, with aspect-preserving downscaling and a 128-color palette.

Main Menu, loading and world-camera clips derive from source 11379ae:
https://github.com/johnnyhuy/openyr/tree/c7deba4c12ab133026c00570e4a2c78d7fa3fa65/proof/native-demo

Save-name editing derives from source 99968fa:
https://github.com/johnnyhuy/openyr/tree/346aeb37c82a332d87ab9755f3094ac5c6f8cff8/proof/name-shortcuts

The original archives retain isolated-build receipts, executable hashes, source
commits and capture scripts. Captures use real retail data and synthetic XTest
input. They establish neither physical Mac/Retina behavior nor original timing
parity. GIF palette reduction and downscaling are presentation choices.

These files contain derived game artwork. Retain them only in the pinned proof
commit, remove every proof file from the final branch tree and squash merge.
No game assets or executable binaries belong on main. Screenshots/GIFs do not
convey rights to the original game content.
