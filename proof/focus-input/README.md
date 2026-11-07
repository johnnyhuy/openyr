# Focus-input guard proof

Source commit `3e5be5a14f25477b6fb71ea2ae722a5d3e72c3fe`, tree
`34fa42ff94fa15b4e8fac76d6fe2c95fd5f8b4a2`.

`terminal.typescript` and `terminal.timing` are an actual util-linux script
recording from this clean committed source. Replay with
`scriptreplay --log-out terminal.typescript --log-timing terminal.timing`.
`record.sh` contains the recorded commands. `terminal.gif` and `terminal.png`
render the captured text; they are not screenshots of gameplay or a desktop.
The GIF also renders the earlier failed regression output in `before-guard.log`.

The four tests call the same keyboard dispatcher used by WindowEvent routing.
They exercise a listed temporary save and actual deletion backend, synthetic
and unfocused presses, held-key repeats, menus and release cleanup. Deliberate
Enter deletes the test file; synthetic Enter must preserve its exact bytes.
`workspace-check.log` records all required checks passing. The earlier guard-free
regression failed all three initial tests, including a missing file after
synthetic Enter. The fourth repeat regression was added with the guard.

`build-receipt.json` applies to the isolated release app executable, whose
verified copy was run with `--help`. The terminal tests use Cargo's debug test
executable; its SHA256 is recorded separately in the transcript. The release
receipt does not attest that test executable or establish live input delivery.
No executable is uploaded.

No retail data is available in this fresh Linux environment. Retail-dependent
tests self-skip and the existing native audio-device test remains ignored.
No retail gameplay, live-window, original-game, physical macOS/Retina or audible
playback verification ran. Previously blocked Mac capture paths were skipped.
Simulation and save schemas are unchanged.
