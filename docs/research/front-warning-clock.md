# Front-menu warning clock observations

Twenty original Main Menu captures on 2026-11-02 identify the movie and
warning source frames with zero pixel differences. They expose a timing gap
in the native provisional 15 fps warning clock (#174). They do not establish
the original scheduling rule or a replacement nominal frame rate.

The original was already running. Every request verified the complete game
image path, owned process, unique enabled 800x600 Main Menu, foreground window
and released modifier/PrintScreen keys. The public [SendInput](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput)
call records [GetTickCount](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-gettickcount)
immediately before and after the key pair. A host watcher pairs
one completed PNG with each request, checking dimensions and retaining source
SHA-256, file modification time and monotonic completion time. A missing,
ambiguous or refused capture stops the series without repeating input.
GetTickCount has finite timer resolution and wraps after about 49.7 days;
these recorded tick spans do not cross a wrap. Input delivery and completed
PNG times do not identify the exact presentation instant. No original
implementation or process memory was inspected.

The first series requests twelve captures five seconds apart. The second
requests eight captures thirteen seconds apart. All twenty full 632x568 movie
regions match decoded retail video after the existing color conversion and
RGB565 presentation. All twenty 112x68 warning regions, including the rim,
match actual native composition at their identified source phase. The
comparison covers 7,331,840 RGBA pixel samples without masks. Repeated source
phases have identical fingerprints.

## Measured progression

| Series | Captures | Input-tick span | Movie fit | Warning fit |
| --- | --- | --- | --- | --- |
| Five-second requests | 12 | 54.915 s | 14.533350 fps | 10.743586 fps |
| Thirteen-second requests | 8 | 91.495 s | 14.610056 fps | 10.789927 fps |

The first warning sequence advances 589 frames during its span; a fixed
15 fps clock would advance about 824. Its fitted warning residuals run from
-0.755 to 1.408 frames. The second fitted warning residuals run from -0.569
to 1.089 frames. These fits describe this reference run, including any
presentation or capture overhead. The movie header declares 15 fps, which is
also faster than its measured progression here.

A circular phase search tests constant warning rates from 1 through 60 fps
in 0.001 fps steps, allowing a separate phase offset for each run. This avoids
assuming a lap count from either sampling interval alone. The best joint rate
is 10.778 fps, with summed per-run mean squared circular error 1.127237. The
same statistic for 15 fps is 1261.009335. The per-run unwrapped fits use the
joint low-rate solution. Rates outside that search range and nonuniform
schedules are not excluded.

Reducing capture frequency does not remove the warning/movie gap. It does
not isolate the export cost, actual UI update cadence, renderer workload or
clock behavior across stalls. The runs also do not establish the initial
warning epoch when entering a page. A fitted 10.75 or 10.778 fps constant is
therefore not a demonstrated implementation rule. Native timing remains
provisional while #174 tracks the distinguishing observations and correction.

## Pixel coverage and retained evidence

The fresh captures add eighteen unique movie phases and fifteen unique
warning phases to the previous original comparisons. The retail regressions
now check 32 unique original movie phases and 26 warning phases. The existing
431-frame independent YUV/RGB decoding checks and warning-wrap checks remain.
These are source-pixel comparisons, not claims of matching live playback.

[The measurement data](front-warning-clock.json) records every source hash,
input tick, matched source phase, RGBA fingerprint, fit and search statistic.
External proof retains input/helper receipts, capture scripts, safe original
crops, actual native output and exact verification. No images or raw game
assets are committed. Previously permission-blocked `screencapture -x` and
`screencapture -V` paths were skipped. The proof uses guarded original
PrintScreen output and native shared composition, with no desktop recording
or normal-cadence claim.

The next experiment must distinguish an absolute timer from UI-update-driven
advancement and validate the initial epoch and a separate holdout sequence.
Do not close #174 or full menu issue #96 from these pixel checks alone.
