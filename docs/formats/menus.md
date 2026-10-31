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
keyboard focus underline, disabled delete control, save-slot list, confirmations,
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

Save writes through the existing atomic snapshot envelope. Named saves stay
beside the current quicksave path, with overwrite confirmation. Load requires
confirmation before discarding the current match. One worker prepares the
session and terrain while the old match remains paused. Failure leaves the
old match intact. Cancellation marks the result unwanted, so a late worker
cannot replace or resume it. Presentation is rebuilt after successful load;
queued commands and selection are discarded. The loaded slot becomes the
F5/F9 target. Restart rebuilds the same map with the same seed. Quit exits the
app; Main Menu discards the match through the explicit abort choice.

Controls persist actual mute and world zoom in
`~/Library/Application Support/OpenYR/interface.json`, through a sibling
file and rename. Reads are limited to 16 KiB with strict fields and zoom bounds.
Preference/save/load errors remain visible with a route back to the match.
Headless runs bypass preferences and retain their existing CLI behavior.

The retail flow test exercises the same modal handlers and compositor as the
window, including a simulated hour paused, one resumed tick, named save,
overwrite decline/accept, load decline/accept, corrupt-load recovery, cancelled
load, restart and result actions. `OPENYR_MENU_PROOF=<directory>` exports each
state outside the repository. Won/Lost menu captures set the outcome in a test
fixture; ordinary headless skirmishes still verify the actual match outcome.
