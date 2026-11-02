# Skirmish setup controls

The native form now exposes Credits and Unit Count at the original positions.
Both values reach actual match creation, save/load and restart. The player
rows describe the current two-owner Allied sandbox; additional rows are closed.
Country, color, start and team controls remain fixed. Game Speed and the five
game-rule checkboxes remain unavailable until their behavior is implemented.
This is progress toward #96, not complete form or simulation parity.

## Rules and input

The preview loads RULESMD.INI and map overrides together. The form derives
MinMoney, Money, MaxMoney, MoneyIncrement, MinUnitCount, UnitCount and
MaxUnitCount from MultiplayerDialogSettings. This install gives credits
5000 through 10000 in steps of 100, and unit count 0 through 10.
The app bounds malformed limits and values before starting any allocation.
Starting a match captures the displayed values, including clamped defaults.

Tab and Shift+Tab navigate the two sliders and enabled buttons. Arrows adjust
one step; Home/End select endpoints. Mouse presses select a track value or
begin a thumb drag. Grabbing a thumb retains its value. Focus loss, leaving a
menu and mouse release clear drag state. Menu input does not reach the world.

Original observations use guarded public Win32 controls and renderer PrintScreen
output. No original implementation or process memory was read. The public
[trackbar messages](https://learn.microsoft.com/en-us/windows/win32/controls/trackbar-control-reference)
and [trackbar notifications](https://learn.microsoft.com/en-us/windows/win32/controls/trackbar-controls)
document the observation interface, not the original control's internal code.
The original returned the current scalar values but zero for its standard
range queries. Home left Credits unchanged. Guarded clicks verified the
endpoints and the 7400-credit/5-unit midpoint, then restored 10000 and 10.

## Renderer measurements

| Element | Observed position |
| --- | --- |
| Credits label | x=302, y=314 |
| Unit Count label | x=302, y=341 |
| Credits track | x=404, y=314, 22px high |
| Unit Count track | x=404, y=340, 22px high |
| Thumb travel | 66px |
| Each value-well interior | x=485, 48x22 |

The retail thumb is LOCALMD.MIX PCX entry `5c26a605`, 12x22, using its embedded
palette. Its filename is unresolved. Initial thumbs run from x=404 to 470;
after interaction they run from x=405 to 471. That presentation state is
separate from the selected value. The original midpoint puts the credit thumb
at (436,314) and the unit thumb at (438,340). Values use GAME.FNT at the
observed baseline and center. Bevels and track backing use the measured RGB565
composition already verified for the chooser.

Four safe original crops at `(300,310,236,54)` cover both controls. Their UTC
receipts and source PNG hashes are in the proof bundle:

| State | Credits | Units | Receipt finished UTC on 2026-11-02 |
| --- | --- | --- | --- |
| Both maximum | 10000 | 10 | 06:18:11 |
| Minimum credits | 5000 | 10 | 06:17:05 |
| Both minimum | 5000 | 0 | 06:17:34 |
| Midpoint | 7400 | 5 | 06:55:07 |

`retail_setup_slider_tracks_and_digits_match_original_captures` checks 10632
RGBA pixels per state outside the two value-well interiors: 42528 samples
with zero differences. It separately checks the yellow digit/label mask over
all 12744 pixels per state. All four masks match, including the value digits.
The regression skips when the retail install is absent; no images are fixtures.

The value-well texture still differs (#168), with 1898 through 1973 different
pixels per complete crop. The provisional native fill is LOCALMD.MIX PCX entry
`b1a77caf`. Failed direct, scaled and cropped candidates remain in the research
receipts. The two interiors are explicit exclusions from the RGBA comparison,
not a claim of complete control parity. The full side-by-side/difference images
retain those gaps.

## Session behavior

Starting-unit count now applies to both generated owners (#166). Zero yields
no generated starting units; six yields six for each owner. The same credits
reach both economies. The composition is still the documented Allied sandbox
with prebuilt bases, not calibrated original MCV deployment or army selection.
Original AI behavior, factions and full setup rules remain incomplete.

A live computer harvester now supplies real resource income instead of also
receiving economy-lite refinery income (#167). Without a live owned harvester,
the existing fallback remains. A controlled tick-150 test first reproduced an
extra 150 credits, then verifies suppression, ownership, save/reload and fallback.
No simulation field or schema was added for this condition.

The new app envelope records the shared-force profile and explicit overrides.
Versions 1/2 reconstruct their old three-guard setup; details and limits are in
[save.md](save.md). The actual-handler test
`retail_skirmish_settings_start_save_restore_and_restart_both_participants`
checks 5000 credits and six units per owner through Start, saved loading,
malformed-option recovery, restart and front-menu return. Tick-1 hash
`9a31b9137b75e27b` is retained through the custom match flow. Prior-release
EB1 saves retain seed-42 tick-1 hash `57dac47c915b074a`.
