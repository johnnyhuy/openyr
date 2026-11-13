WAV export bounds proof for issues #390/#392 and PR #391

Tested source 8cf0dc674d82c61d153823989ad012fcbb835bea, tree 0e077838787ff1700ad24e6e93080e63133a2699. The CLI and runtime receipts bind fresh isolated release builds to that tree and their copied program hashes. Baseline receipts bind the preceding merged tree e9487759c80d748009a66a010786288b38151055. No binaries, saves or retail assets are included.

The actual recorded terminal sessions reproduce four corrupt baseline byte-rate headers and eight final error exits preserving both existing and absent output paths. Eleven valid controls are byte-identical, including mono/stereo PCM, both highest representable rates, fact-trimmed IMA, retail WAV, AUD and sound-bank entries. The complete 16-archive scan reports no errors and includes all 1,223 retail WAVs.

All 698 Rust tests pass, zero failures, one existing native audio-device ignore. Strict workspace Clippy, formatting and three Python provenance tests pass. Exact commands are in the PR and recordings. Test profiles omit debug symbols and strip symbols for disk limits; assertions remain enabled.

The shared native headless compositor produces identical tick-300 frames and cue logs. Its mixer produces identical stereo 44100 Hz 16-bit PCM for 299 advancing ticks (19.933333 seconds). The final native app reaches tick 6000 at hash 3082cb602467f8fc, and fresh/restored saves match prior merged main bytes. Retail-derived output stays local; the recordings carry hashes.

This is nonvisual encoder/CLI work. Terminal proof is used, with a headless frame/mixer compatibility control. Physical Mac/Retina, live audible device playback and original-game runtime did not run. Known blocked Mac/reference capture paths and HTTP 401 release uploads were not retried. All proof files are removed before squash merge.

Required CI 43410748672 exposed a preexisting map-extent test-fixture collision (#392). A wall-clock name could repeat, and a cleanup guard constructed before successful creation could delete another test owner's directory. The final source claims bounded directory candidates atomically and acquires cleanup ownership only after creation. The fixture terminal record executes all four map-extent tests with 16 test threads, including both original failures and a deterministic existing/parallel-owner regression. All four pass. The initial failed CI log is retained.

The final local Rust suite passed before a later Clippy invocation exhausted disk. Only this run's private build cache was removed; Clippy passed in a new smaller target. Both the disk error and successful recovered Clippy logs are retained. No product behavior changed during that repair.
