# CI bootstrap source proof

CI initially failed before tests because its mise action could not resolve
mise.jdx.dev for VERSION discovery. Pinning the bootstrap to 2026.11.3 avoids
that lookup; the subsequent macOS run installed tools and passed provenance
checks. Project tasks and Rust/Python choices are unchanged.

Final source 41bb2ee, tree 574b309d14937e0e1401b2d7a787ef06fd355ac5, was clean
before another isolated `mise run proof:build` and the recorded verified retail
headless command. Receipt and actual `script` terminal recording are included.
The executable SHA256 is identical to both live demo builds:
d2647c285a80153fbf1d6d50188f7a90baea0245bb2e9d66ad9df1cde1ff161e.
Thus the CI bootstrap change retains the captured native executable; the live
README-completion capture remains pinned separately with its own source label.

No physical Mac, Retina or audio-device checks ran. No original executable ran.
Remove proof files before squash merge; retain these pinned links for review.
