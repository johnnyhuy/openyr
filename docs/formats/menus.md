# In-match menus

The native pause flow uses art and strings from the user's retail install.
No decoded artwork, fonts or CSF strings are distributed in the repository.
This is the in-match part of #96, tracked by #108. It does not implement the
animated main menu or skirmish setup.

## Asset observations

These entries were decoded from the Allied `SIDEC01.MIX` in the local retail
install on 2026-10-31. The archive scope matters because other sides reuse
names and IDs.

| Entry | Canvas | Frames | Palette |
| --- | --- | --- | --- |
| `BKGDSM.SHP` | 472x448 | 1 | scoped `UIBKGD.PAL` |
| `BKGDMD.SHP` | 632x568 | 1 | scoped `UIBKGD.PAL` |
| `BKGDLG.SHP` | 856x736 | 1 | scoped `UIBKGD.PAL` |
| ID `2f43f08b`, unresolved filename | 125x25 | 3 | scoped `SIDEBAR.PAL` |
| ID `d508c1a4`, unresolved filename | 856x32 | 1 | scoped `SIDEBAR.PAL` |

The first three are the blue eagle plates for 640x480, 800x600 and 1024x768
menus. The remaining 168 pixels are the sidebar; the bottom 32 pixels hold
the status strip. The sidebar reuses `CREDITS`, `TOP`, `RADAR`, `SIDE1`, `SIDE2`,
`SIDE3` and the blank tab frames. Source indices use zero as transparency.
The dialog and control palettes differ. Applying `SIDEBAR.PAL` to the eagle
plate produces incorrect colors.

The Yuri `SIDEC02MD.MIX` in `RA2MD.MIX` holds seven entries, all now
hash-resolved in the name database. Its background plates follow the
Allied pattern with a `Y` suffix, via `BKGDSMY.SHP` (472x448),
`BKGDMDY.SHP` (632x568) and `BKGDLGY.SHP` (856x736). Its radar sheet is
`RADARY.SHP` (168x110, 33 frames), with palettes `UIBKGDY.PAL` and
`RADARYURI.PAL`, plus the shared `KEY.INI` key file. The contributed retail run
reported that every SHP frame decoded and both palettes parsed; the optional
retail corpus test retains those checks. Cloud checks without an install do
not repeat that decoding. Yuri gameplay remains unsupported, so this naming
does not wire a Yuri sidebar into a match.

`GAME.FNT` supplies glyphs; `RA2MD.CSF` supplies available labels and tooltips.
Native flow and error messages use short fallback text. Optional assets with
unexpected canvas/frame counts are rejected before allocating decoded canvases.
Missing optional art falls back to a usable plain panel.

The source backgrounds and button RGBA are checked against independently decoded
SHP indices and their scoped palettes. A rendered pause frame also checks
uncovered eagle pixels and the control bevel. This proves source-pixel fidelity,
not complete original screen parity.

[DeeZeL's pause-menu asset guide](https://generals.projectperfectmod.com/newpausemenu/)
identifies the three background names and resolution families. A
[Cupshadow mod recording thumbnail](https://odysee.com/@Cupshadow:f/red-alert-2-flipped-missions-allied-10:4)
shows the retained pause layout: eagle over the map area, vertical controls
below the radar, resume near the bottom and status text below the eagle.
That mod image is a qualitative layout reference, not an unmodified oracle.
Only asset facts and observed layout informed this implementation.

## Native presentation policy

The renderer picks the largest authored canvas that fits, centers it, and
scales down only if the surface is smaller than 640x480. At the current
1280x800 framebuffer, the 1024x768 menu starts at (128,16). World zoom does
not scale menu art. Rendering and hit testing share the same transform.

The right controls begin at local (world width + 23,228), with 25-pixel rows.
Resume and Back occupy the row 65 pixels above the canvas bottom. The native
keyboard focus underline, save-slot list, deletion page, confirmations,
controls, loading and result screens are provisional. Control frames currently
use 0 for normal, 1 for pressed and 2 for hover. Exact frame semantics, tracking,
widescreen layout, non-Allied skins and timing still need an original oracle.

The noncombat shell and `ra2ts_l.bik` main-menu movie now have their own
[front-menu flow](front-menu.md). The pause eagle is not used as a substitute.

## Match flow

Escape cancels repair, sell, placement or attack-move first. Otherwise it opens
Pause. Opening a modal clears held keys, modifiers and pointer gestures while
preserving selection. Map orders, camera scrolling and world hotkeys do not
receive modal input. Left press/release must hit the same enabled control;
focus loss clears the pending press. Tab selects controls. On Load, arrows
select slots and scroll the list. Save accepts a bounded filename component.
Right-click and Escape return, decline a confirmation or resume as appropriate.
Won/Lost screens require restart or quit.

Visible menus stop sim ticks. Every transition clears elapsed-time accumulation;
resuming starts a fresh clock without replaying paused wall time. Drawing does
not mutate the simulation. Music may continue while the match is paused.

F5 uses the same input cleanup as normal menu entry when either its save result
or an error opens a message (#237). Placement, repair/sell and attack-move modes,
held modifiers, pointer gestures and transient team-recall feedback are cancelled.
Selection, assigned teams and already-queued valid commands remain intact. A
failed save retains its previous quicksave target. Dismissing the result cannot
reactivate a mode or turn an earlier button release into a new order. This is
native modal-isolation behavior, not a claim about original-game shortcuts.

The Save name editor keeps the end of the name and its caret visible (#243).
When the complete name fits the measured field width, its display is unchanged.
Otherwise a two-dot prefix marks hidden leading characters and the visible suffix
ends with the existing caret. The display window moves at Unicode character
boundaries and does not alter the stored name, input limit or output filename.
Read-only save-slot labels retain their existing prefix fitting. This is native
editing behavior; original caret styling/timing and physical Mac/Retina input
remain uncalibrated. Fallback/retail compositor checks at three sizes and actual
append/backspace/save handlers verify visibility and unchanged complete save bytes.

Save and Skirmish player-name fields support Ctrl+A and Command+A to select the
whole name (#248). Unmodified typing replaces that selection; Backspace clears
it. The Save field uses the same blue selection fill as a selected Skirmish
name. Other Control/Command text shortcuts cannot insert their shortcut letters
or erase the selection. Clipboard copy/cut/paste are not implemented here.
Ordinary Unicode text, filename filtering and existing length limits remain.
Tab, page/slot changes, mouse interaction and focus loss clear Save selection;
focus loss and menu/match transitions also clear the tracked Command modifier.
Skirmish navigation clears name selection before moving to another field.

Handler checks cover both modifier families and both player rows, replacement,
Backspace, unsupported shortcuts, bounds and stale focus. Retail/fallback Save
composition at three sizes confines selection changes to the editor; a real
retail save retains complete match bytes and uses the replacement filename.
These are native editing choices, not original shortcut calibration or physical
Mac/Retina evidence.

Save writes through the existing atomic snapshot envelope. Named saves stay
beside the current quicksave path, with overwrite confirmation. Load requires
confirmation before discarding the current match. One worker prepares the
session and terrain while the old match remains paused. Failure leaves the
old match intact. Cancellation marks the result unwanted, so a late worker
cannot replace or resume it. Presentation is rebuilt after successful load;
queued commands and selection are discarded. The loaded slot becomes the
F5/F9 target. Restart rebuilds the same map with the same seed. Quit exits the
app; Main Menu discards the match through the explicit abort choice.

A successful fresh front-menu Start chooses `quicksave.oyrsave` in the current
save folder (#306), rather than inheriting an earlier named match's basename.
The target changes only when the new prepared match commits. Returning to Main,
preview success, load failure and cancellation keep the previous target and do
not write a save. Saved-load commits still select the supplied named slot;
in-match restart retains its current target. Existing quicksave replacement,
named-save confirmation, explicit CLI startup and save schemas are unchanged.

Asset-independent native Session checks drive normal Main Menu entry, controlled
prepared-result channels and actual F5/F9 handlers. They verify protected named
bytes, exact fresh quicksave bytes, confirmation paths, three layouts and the
load/restart/preview/failure/cancel policies. They do not execute the retail asset
loader or reconstruct a saved match. `OPENYR_NEW_MATCH_SAVE_PROOF=<directory>`
exports actual synthetic compositor frames and state receipts without retail art;
physical Mac/Retina and original-game comparisons remain unverified.

A committed saved-match load from the front or pause flow briefly shows
`Match loaded (tick N): <save name>` in the existing world feedback strip (#97).
Only the filename stem is shown: controls are removed, outer whitespace is
trimmed and the name is capped at 48 characters. The restored tick precedes the
name so compact layouts retain it when measured fitting clips a long label.
The notice begins after the prepared match and presentation are committed;
loading progress, failures, cancelled workers, restart and fresh setup do not
report a saved load. Explicit CLI startup still bypasses menu feedback.

That load notice has no marker and shares the existing two-second monotonic
notice lifetime and cleanup/replacement rules. It adds no modal or simulation
pause. This text and timing are native policy, not original-game calibration.
Complete restored save bytes remain unchanged. `OPENYR_LOAD_SUCCESS_PROOF`
exports matched notice/cleared compositions at three sizes from the real
retail-backed asynchronous loader regression.

Delete Game opens a separate saved-match list from Pause or the Load page.
Mouse selection, arrows and wheel scrolling use the same slot geometry as Load.
Delete stays unavailable until a slot is selected. Confirmation names the save,
asks whether to delete it permanently and defaults to No. Escape, right-click
and No return to the list without removing the file. Confirmation and result
pages use the current front or in-match skin; this is a native layout policy,
not an observed original deletion-dialog layout (#171, #96).

Only listed regular `.oyrsave` files in the active save folder can be deleted.
The app checks the path, extension and file type again when confirming, without
following a replacement symlink. It refreshes slots after success or failure,
clamps selection and scrolling, and shows a result with a route back. Missing
files and directory/symlink replacements produce recoverable errors. Deleting
the last slot shows the empty list with Delete disabled. The running match and
its save target stay in memory; saving again can recreate a deleted active slot.
No simulation state or save schema changes. The handler tests use temporary
files and compare complete encoded retail match bytes before and after deletion.
`OPENYR_DELETE_PROOF=<directory>` exports front/pause selection, confirmation,
success, stale-file error and empty-list frames through the shared compositor.

## Saved-file confirmation targets

Save overwrite, Load and Delete confirmations show the quoted target basename,
including `.oyrsave` (#253). They derive it from the pending intent's actual path,
not the truncated slot-list label. Long external save names keep their complete
suffix. Parent directories stay hidden; quotes, backslashes and nonprinting
characters are escaped for display without changing the actual file path.

A shared renderer measures character advances and wraps only at Unicode
character boundaries, preserving spaces. Pause text starts at `(40,72)` in its
existing body region. Front text starts at `(32,112)` below the question, within
a 568x432 region. Both skins use their own font for drawing and scrolling.
Overflow uses the wheel, Page Up/Down and Home/End; the footer reports its visible
line range. Page changes reset scrolling and resized views clamp it.

Arrows and Tab retain button navigation. Enter still requires a deliberate
press, and Delete still defaults to No. Escape, right-click and No retain their
existing return paths. Merely displaying or scrolling a confirmation neither
changes slots nor reads/writes/removes a save. Save/load/delete validation, atomic
writes, simulation and save schemas are unchanged. This is native confirmation
safety, not original-game dialog calibration or physical Mac/Retina evidence.

Fallback/retail compositor checks cover both skins at 640x480, 800x600 and
1280x800, including names with identical prefixes, long suffixes, Unicode,
repeated spaces and escaped controls. Actual-handler checks retain keyboard
choices and compare complete retail match/file bytes through overwrite decline
and accept, load decline and accept, and delete decline and accept.

Pause recovery messages (#234) now share the front menu's measured word wrapping.
The text rectangle begins at local `(40,72)`, ends 40 pixels before the sidebar,
and stops 48 pixels above the authored canvas bottom. Rows use the greater of
font line height, glyph canvas height and 20 pixels, plus four pixels of spacing.
This is native layout policy, not original-game calibration. Explicit paragraph
breaks and complete long tokens remain reachable; tokens split only at Unicode
character boundaries. The prior 50-character rows and 400-character cutoff no
longer apply to recovery diagnostics.

Up/Down, Page Up/Down, Home/End and the wheel scroll overflowing messages.
Input and drawing use the same active pause-canvas dimensions and font metrics;
resize clamps the visible offset. The footer reports the visible line range.
Back/Escape keep the existing recovery destination and menus keep the match
paused. Named-save targets, simulation snapshots and save formats are unchanged.
`OPENYR_RECOVERY_PROOF=<directory>` exports first/last pages at three sizes from
the real-handler regression, using a controlled overflowing diagnostic after a
real save failure. Front-menu message geometry and existing pixel checks remain
unchanged. Physical Mac/Retina checks remain in #97.

Controls persist actual mute, world zoom and edge scrolling in
`~/Library/Application Support/OpenYR/interface.json`, through an exclusively
created sibling file and rename (#299). The writer tries at most 32 process/serial
candidate names using `create_new`; occupied files, directories and symlinks are
skipped without modification. It writes and syncs the owned file before closing
and renaming it, and removes only that candidate if publication fails. Existing
predictable `interface.<pid>.tmp` files are left untouched. This prevents a stale
or planted temporary symlink from redirecting the write to an unrelated file.
It does not claim safety against concurrent replacement by an actor who can
modify the containing directory.

Reads retain their 16 KiB limit, strict fields and zoom bounds; JSON and game save
formats are unchanged. Actual native EdgeScroll handler tests use owned scratch
files to verify Unix symlink-target preservation, reopening, failed publication,
Controls recovery and retry. The launcher shares this writer for its remembered JSON game-directory path
(#362; `docs/macos-app.md`). Separate helper tests cover occupied candidates,
32-collision exhaustion and owned cleanup. No retail assets or physical desktop
are needed for these filesystem checks; physical Mac delivery remains unverified.
Preference/save/load errors remain visible with a route back to the match.
Headless runs bypass preferences and retain their existing CLI behavior.

Live preference-write failures enter recovery through normal modal entry (#303).
The M shortcut therefore cancels held keys/modifiers, gestures and command modes
before showing the error, matching Pause and F5 recovery. Controls remains the
return destination. Selection, assigned teams, queued valid orders and encoded
Session bytes survive; after resuming, a fresh ground click uses ordinary Move
instead of a pre-dialog attack-move or service mode. A successful live mute
write stays nonmodal and preserves its existing input behavior.

Authored native Sessions cover attack-move/repair/sell recovery at three sizes,
protected destination bytes, stale release rejection, team recall and actual
fresh Move commands. `OPENYR_PREFERENCES_MODAL_PROOF=<directory>` exports fallback
compositor frames and state receipts. No retail art or cursor sheet is supplied;
these checks do not establish original-game or physical Mac/Retina behavior.

The retail flow test exercises the same modal handlers and compositor as the
window, including a simulated hour paused, one resumed tick, named save,
overwrite decline/accept, load decline/accept, corrupt-load recovery, cancelled
load, restart and result actions. `OPENYR_MENU_PROOF=<directory>` exports each
state outside the repository. Won/Lost menu captures set the outcome in a test
fixture; ordinary headless skirmishes still verify the actual match outcome.

## Selected Save-file identity

Choosing an existing Save slot retains its actual listed path, rather than
rebuilding a filename from the 48-character display label (#265). This also
preserves leading/trailing spaces, names disallowed for newly created slots,
and supported filesystem byte names. The unchanged selected target enables
Save and supplies the exact path for overwrite confirmation and commit intent.
Declining an overwrite preserves that target for another deliberate attempt.

An accepted character edit or Backspace clears selected identity and the slot
highlight; the editor then follows the existing bounded new-name rules. Filtered
characters, unsupported shortcuts, selection-only Ctrl/Command+A and input
ignored at the length limit retain the target. Unrelated page/list transitions
clear the selected target and its display-derived editor name, so a later Save
cannot accidentally reconstruct a shorter sibling from a stale label. Typed
new names retain their existing page-navigation behavior.

Before requesting an overwrite, the app rechecks that selected identity still
matches the current listed slot, the active save folder, the `.oyrsave` extension
and no-follow regular-file metadata. A different regular file at the same
path is still that selected path; this is not inode/content identity tracking.
Missing, unlisted, outside or directory/symlink-replaced selected targets report recovery and clear the editor instead of falling back
to a label-derived path. Fresh typed-name behavior, backend atomic writes,
simulation and save encoding are unchanged.

Asset-independent handler tests use actual temporary long, spaced and hidden
filenames plus protected siblings. They check exact confirmation/Commit intent,
decline, accepted/ignored edits, stale target recovery and byte preservation
while requesting/declining confirmations. Non-UTF-8 filename creation is checked
on Linux only. `OPENYR_SAVE_IDENTITY_PROOF=<directory>` exports fallback menu
compositions and state receipts without a Session or retail art. These do not
prove an actual retail overwrite, save reconstruction or physical Mac/Retina
input delivery; those remain separate checks.

## Saved-list keyboard navigation

Save, Load and Delete retain every regular `.oyrsave` found within the existing
scan of at most 4,096 directory entries (#308). The earlier 64-slot truncation
could hide a successfully written named save that sorted later. Entries still
sort by exact filesystem path, display names remain capped at 48 characters,
and symlinks, directories and unrelated files are excluded. The scan bound
counts all directory entries, including unrelated ones; this is not an unlimited
directory catalogue. File writing, confirmation and save formats are unchanged.

Authored native Session tests write a real 65th named save through keyboard
dispatch, then verify Load/Delete endpoint/page navigation and exact confirmation
paths in both skins at three sizes. Save wheel/pointer selection reaches that
same file. Declining confirmations preserves every file and encoded Session.
A separate metadata fixture checks the scan bound with 4,097 regular entries
and no-follow filtering. `OPENYR_SAVE_CATALOG_PROOF=<directory>` exports fallback
compositor frames and state receipts. These checks do not reconstruct retail
saves or establish original-game or physical Mac/Retina behavior.

Load and Delete lists support Page Up/Page Down by ten slots, matching their
visible row count, and Home/End for the first/last slot (#270, #96, #97).
The selection remains visible through the same scrolling helper as Up/Down.
With no selection, the first arrow/page key selects the first slot; Home/End
select their respective endpoints. Empty lists remain unselected and disabled.
Native footer hints name the page/endpoint keys. Tab still navigates action
buttons; Enter uses the selected file's exact path
through the existing confirmation flow. Save name editing is unchanged.

The endpoint/page arithmetic saturates before clamping to the slot count.
Native keyboard dispatch tests cover empty/short/long lists, focus/synthetic-key
guards, front/pause flows at three sizes and exact temporary-file confirmation
targets. Cancellation preserves all listed file bytes. Optional
`OPENYR_LIST_KEYS_PROOF=<directory>` captures fallback composer frames and state
receipts. The fixtures contain arbitrary bytes, not reconstructed retail saves;
these checks do not establish original shortcut parity or physical Mac delivery.

## Held keys at modal boundaries

Focused physical key presses are tracked separately from active world keys
(#314). Entering or leaving a menu page or Skirmish dropdown suppresses repeats
from keys already held. The same cleanup applies on focus loss, native folder
selection and prepared-load completion. A release or fresh non-repeat press
restores that key. Ordinary repeats within the same input surface remain
available for name editing, list navigation, camera movement and zoom.

Clearing the world-key set alone was insufficient: the next OS repeat could
insert a held letter into a new Save name, revive camera movement after Resume,
or alter a country just selected by pointer from a dropdown. The native
keyboard and pointer dispatchers now compare menu/popup context around their
handlers. Match/preview/error results also suppress held keys before presenting
their destination. Existing synthetic/unfocused key guards remain in place.

Authored Session checks cover Save editing, 24 keyboard/pointer resume cases,
country-dropdown closure, focus return without synthetic events and controlled
prepared-match commits at three layouts. Fresh presses and normal same-editor
repeats still work. Selection, pending valid orders and saved/encoded Session
bytes follow their existing transition policies. This does not reconstruct
retail saves or establish original-game timing or physical Mac/Retina delivery.
