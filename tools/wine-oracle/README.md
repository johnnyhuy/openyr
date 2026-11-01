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
