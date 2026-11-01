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

`verify` launches only the tool's own synthetic fixtures. The positive fixture
has a known dialog, heading, panel and private Edit value. Assertions check
exact geometry and omission of the Edit value. A second fixture exits zero
without creating a window; the observer must report failure with the actual
child exit code. An absent-target probe must also fail.

`observe` reads an already-running original. `launch` starts the exact original
image with `CreateProcessA`, then polls public windows every 200 milliseconds
until it observes the verified page, the child exits, or 60 seconds elapse.
The deadline starts after process creation. Wine startup and individual API
calls can add wall time. At a deadline the child is left running. Poll or
observe the existing process before deciding whether another launch is needed.
A successful snapshot does not prove the page remains open afterward.

All commands rebuild and stage only these generated helpers in
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
| 7 | Observation deadline, child still running |

Process status comes from
[WaitForSingleObject](https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-waitforsingleobject)
and [GetExitCodeProcess](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getexitcodeprocess).
Window ownership uses
[GetWindowThreadProcessId](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getwindowthreadprocessid)
and [QueryFullProcessImageNameA](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-queryfullprocessimagenamea).
Geometry uses
[GetClientRect](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getclientrect),
[GetWindowRect](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getwindowrect)
and [ClientToScreen](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-clienttoscreen).
These are logical UI coordinates, not calibrated macOS display pixels.

## Current reference failure

On 2026-11-01 the supported game launcher and a public child-process launcher
both failed to expose a usable front menu. The child launcher reported actual
Windows exit code zero. Read-only probes found no eligible game root. A
separate synthetic Win32 window could be created and destroyed successfully.
Wine service-start timeouts appeared, but their causal role is unknown.
The recovery work is [#148](https://github.com/johnnyhuy/openyr/issues/148).

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

Existing original renderer images remain valid static evidence. Their eleven
Skirmish preview-edge differences do not uniquely establish the entire client
rectangle. #144 remains open. No rectangle or individual-pixel patch follows
from this startup investigation. Normal warning cadence and movie/shell
synchronization also remain uncalibrated. Earlier irregular PrintScreen
exports cannot establish wall-clock timing.

Previously permission-blocked `screencapture -x` and `screencapture -V` paths
were skipped without retrying. This non-visual diagnostic uses terminal
recordings and synthetic public UI observations. It provides no new original
renderer pixel or animation-timing proof.
