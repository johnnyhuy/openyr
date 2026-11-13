# Alt-modified menu confirmation proof

Before: main 6d7f808 accepts Alt+Enter on the selected quickload Yes control
and reaches the expected missing-file response. The reused isolated before
receipt records source 1d2167a with exactly the same tree as main 6d7f808;
the proof runner verified that tree before capture.

After: source a0a39b6a330860061fd6d1f8deaa851d0166e5f5 retains the selected
confirmation for Alt+Enter. Ordinary Enter then reaches the expected missing-file
response for the deliberately absent default quicksave. This demonstrates native
load dispatch; protected-file tests separately verify real overwrite/delete
behavior and the full retail suite exercises valid save reconstruction.

The recordings use native Linux Xorg and synthetic XTest input, with Xorg,
OpenYR and FFmpeg in one task session. They are silent because no audio device
is available. Physical Mac/Option, Retina and original runtime did not run;
known permission-blocked capture/reference paths were not retried.

Final source passes 687 retail-backed Rust tests with one existing audio-device
test ignored, strict Clippy/fmt and three Python proof checks. The isolated
reader scans all 16 retail MIX archives with zero errors. Tick-6000 fresh and
restored saves are byte-identical to the merged-main baseline. Actual native
handler tests cover 120 protected confirmation cases and nine menu pages,
with ordinary Enter and Unicode Option text recovery.

Only derived media and verification records appear in this temporary proof
commit. No executable, raw retail archive or save is included. All proof files
are removed before squash merging; final source equals the tested tree.
