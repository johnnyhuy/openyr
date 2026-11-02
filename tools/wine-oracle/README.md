# Original-game window observations

This diagnostic tool records public Win32 control metadata from the isolated
original-game reference install. It preserves the helpers used for menu
calibration and distinguishes a successful process exit from a verified menu.
OpenYR's application remains native Rust.

The tool matches the complete process image path
`C:\Westwood\RA2\gamemd.exe`, requires one visible root with a plausible client
size, and requires one visible 800x600 dialog belonging to that process.
It reports control IDs, classes, styles, enabled state, window rectangles and
client rectangles relative to that dialog. Window rectangle right/bottom
coordinates are exclusive. Client output is x, y, width, height.

Edit values, selected items, button captions and arbitrary window titles are
omitted. A Static control above row 40 can expose a `GUI:` heading key.
Full original Skirmish screenshots still contain private fields; retain safe
header/preview crops and source hashes when publishing pixel evidence.

## Run on macOS

Python 3, Apple clang, the active Rust toolchain's `lld-link`, and the existing
Wine prefix are required. No Windows SDK or original binary analysis is used.
The build generates its own kernel32 import library from public API names.
The required CI job compiles every helper using an empty temporary prefix;
the synthetic runtime verification requires the local Wine installation.

```sh
python3 tools/wine-oracle/oracle.py verify
python3 tools/wine-oracle/oracle.py observe
python3 tools/wine-oracle/oracle.py launch
```

`verify` launches only the tool's own synthetic fixtures. A bootstrap starts
the positive UI fixture, which has a known dialog, heading, panel and private
Edit value. Assertions check actual parent PID, exact geometry and omission
of the Edit value. A second fixture exits zero
without creating a window; the observer must report failure with the actual
child exit code. An absent-target probe must also fail.

`observe` reads an already-running original. `launch` starts the installed
`C:\Westwood\RA2\RA2MD.exe` with `CreateProcessA` and observes its
`C:\Westwood\RA2\gamemd.exe` child. Public process snapshots verify the
parent PID as well as the exact target image path. It polls every 200
milliseconds until it observes the verified page or 180 seconds elapse.
An intro window alone is not the menu page. A launcher that exits zero can
leave its game child running, so that exit does not end observation.
The deadline starts after process creation. Wine startup and individual API
calls can add wall time. At a deadline the child is left running. Poll or
observe the existing process before deciding whether another launch is needed.
A successful snapshot does not prove the page remains open afterward.

All commands rebuild and atomically stage the nine generated helpers in
`C:\openyr-oracle`. `build` stops after that step. Defaults use
`~/Games/RA2YR/prefix` and its bundled Wine. `--prefix`, `--wine` and `--out`
override those paths. The prefix must already exist. Files, stdout/stderr,
source/binary hashes, child return codes and monotonic elapsed times are
retained under `.tmp/wine-oracle` by default. Treat raw stderr as private until
reviewed. Live stdout/stderr files can be inspected while a command is still
running. File output avoids waiting for EOF from a pipe inherited by a child.
Each original observe/launch receipt has a unique UTC filename.

The helpers send no input and do not read process memory, inspect original
implementation code, modify game files, configure Wine, kill processes, or
capture graphics. Wine itself can update its normal prefix state. The
synthetic fixture closes its own window after five seconds.

## Interpret results

| Helper exit | Observation |
| --- | --- |
| 0 | Unique target page observed and control inventory completed |
| 2 | Required public API unavailable |
| 3 | Diagnostic stdout write failed |
| 4 | No verified page, existing target on launch, or child exited first |
| 5 | Observation or process-status API failed |
| 6 | Original process creation failed |
| 7 | Observation deadline; existing processes left alone |

Process status comes from
[WaitForSingleObject](https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-waitforsingleobject)
and [GetExitCodeProcess](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getexitcodeprocess).
Window ownership uses
[GetWindowThreadProcessId](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getwindowthreadprocessid)
and [QueryFullProcessImageNameA](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-queryfullprocessimagenamea).
Launch lineage uses
[Process32First](https://learn.microsoft.com/en-us/windows/win32/api/tlhelp32/nf-tlhelp32-process32first)
and the public process snapshot's parent PID field.
Geometry uses
[GetClientRect](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getclientrect),
[GetWindowRect](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getwindowrect)
and [ClientToScreen](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-clienttoscreen).
These are logical UI coordinates, not calibrated macOS display pixels.

## Original launcher observations

On 2026-11-01 direct gamemd.exe launches exited zero without a usable menu.
Starting the installed RA2MD.exe instead produced a live gamemd.exe, followed
by Main Menu after the intro. Its public parent PID matched the launcher.
Single Player, Skirmish and the map chooser were reached through their
observed controls. The internal reason for the different startup outcomes
was not inspected or inferred. That session supplied fresh preview evidence.

A later stopped-state run of this tool started the launcher and game but
reached its 180-second observation deadline (exit 7). A separate observation
of that same live process found zero windows (exit 4). Wine reported that
its explorer window driver could not load. Both original configuration files
still had their original SHA256 values. The processes were not restarted
because of the deadline. Repeatable startup remains open in #148; the
successful earlier session does not establish that every startup works.

The installed third-party display wrapper accepted a per-process
`CNC_DDRAW_CONFIG_FILE` path. A copied configuration selecting GDI/windowed
output did not restore the game window. The normal `ddraw.ini` remained
unchanged. A separate 800x600 game Video experiment also failed; `ra2md.ini`
was restored byte for byte with identical SHA256 receipts. These experiments
do not establish the cause and are not automatic steps in this tool.

The wrapper's public configuration override is documented by its
[configuration loader](https://github.com/FunkyFr3sh/cnc-ddraw/blob/master/src/config.c).
Its [setup guide](https://github.com/FunkyFr3sh/cnc-ddraw) requires the game to
use fullscreen while the wrapper controls windowed output. An earlier `-win`
experiment also failed; it is not a recommended recovery step.

Fresh public UI measurements established Skirmish's 145x113 preview client
at `(644,37)`. Original renderer crops then calibrated its black background,
landscape/portrait fitting and sampling. See
[front-preview.md](../../docs/formats/front-preview.md). Those measurements
resolve #144 and #151; the earlier eleven pixels alone were insufficient.
Numbered markers and modal heading colors remain in #152/#153. Normal warning
cadence and movie/shell synchronization also remain uncalibrated. Earlier
irregular PrintScreen exports cannot establish wall-clock timing.

Previously permission-blocked `screencapture -x` and `screencapture -V` paths
were skipped without retrying. This non-visual diagnostic uses terminal
recordings and synthetic public UI observations. Separate safe original
renderer crops establish preview pixels, not animation timing.

## Record renderer exports

`record_exports.py` observes files produced by a separately authorized capture
command. It keeps collecting after that command exits, through a quiet period
bounded by a settling deadline. The command's stdout/stderr, exact exit status,
PID, request metadata and each observed complete PNG revision are retained.
Identical PNGs with different source names or revisions remain separate records.

```sh
python3 tools/wine-oracle/record_exports.py \
  --directory "$HOME/Games/RA2YR/prefix/drive_c/Westwood/RA2/Screenshots" \
  --out .tmp/renderer-exports \
  --quiet-seconds 2 --settle-limit 10 --run-limit 90 \
  -- /path/to/guarded-capture-producer
```

Choose a new output directory for each recording. The producer owns input
permission and guards; this recorder only supervises the command and observes
files. Original-game producers must verify the intended process, current page,
foreground and released modifiers before each input. The existing nine
observer/launcher helpers remain read-only diagnostics; a capture producer is
supplied separately. The original holdout proof includes its public-API source
and build hashes.

The recorder checks PNG signature, bounded chunk lengths, CRCs, expected image
dimensions and a complete IEND before copying a stable source snapshot. It does
not decompress pixels. Byte and export limits produce explicit incomplete
results. The output preserves SHA-256, source modification time, host observation
times and copied bytes. Raw captures and producer stderr stay private; publish
safe crops and metadata, and never commit game artwork or player-name fields.

A producer can report one line after each guarded key pair:

```text
capture_index=0 capture_tick_before=1234 after=1234
```

Indices must start at zero and increase without gaps. Ticks are unsigned Win32
GetTickCount values; a short wrap is accepted. The recorder preserves request
multiplicity but marks every export's association unresolved. All reported
requests remain candidates. File-observation order does not prove input-event
ownership, a presentation timestamp or an original animation rate.

`receipt.json` distinguishes producer exit from collection completion. A producer
deadline returns its still-running PID and logs without killing or restarting
it. Inspect that same process before further input. A settling deadline,
unfinished PNG, invalid metadata or export limit remains explicit. The CLI
succeeds only with a successful producer, settled complete file accounting,
valid request metadata and at least one retained export. The bounded observation
window cannot establish that no file appears later or that no intermediate file
revision occurred between polls.

Run the synthetic regressions without Wine or game data:

```sh
python3 -m unittest discover -s tools/wine-oracle -p 'test_*.py' -v
```

They cover late duplicates after producer exit, partial/corrupt PNGs, overwritten
files, ambiguous request ownership, split/hostile metadata, failed/live producers,
CLI status and bounds. Quiet/deadline, partial-write and validation-cost tests use
a controlled clock and ordered filesystem events. Actual subprocess tests cover
exports written after exit and producers held alive by explicit release files.
These synthetic times are test inputs, not original-game measurements (#178).
The tests run in the required CI `test` job. An original
holdout retained all twelve exports for twelve guarded requests, including one
observed after producer exit. Independent source/copy SHA-256 reconciliation
matched every file; the collector observed for another 2.182 seconds after exit.
The earlier 24-export run demonstrates why multiplicity is not a fixed constant.
No timing-rate correction is inferred from this repair (#174).
