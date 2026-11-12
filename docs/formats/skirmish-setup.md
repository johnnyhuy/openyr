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

Five safe original crops at `(300,310,236,54)` cover both controls. Their UTC
receipts and source PNG hashes are in the proof bundle:

| State | Credits | Units | Receipt finished UTC on 2026-11-02 |
| --- | --- | --- | --- |
| Initial untouched | 10000 | 10 | 05:39:16 |
| Both maximum | 10000 | 10 | 06:18:11 |
| Minimum credits | 5000 | 10 | 06:17:05 |
| Both minimum | 5000 | 0 | 06:17:34 |
| Midpoint | 7400 | 5 | 06:55:07 |

`retail_setup_sliders_match_complete_original_captures` checks all 12744
RGBA pixels per state, including both value wells: 63720 samples with zero
differences. Initial untouched thumbs, interacted endpoints and midpoint all
match. The regression skips when the retail install is absent; no images
are fixtures.

The numeric well uses three classic PCX pieces from LOCAL.MIX, nested in
RA2.MIX as entry `a8548fd9`. The corresponding replacement skin in LOCALMD.MIX
does not match these controls. Each PCX uses its embedded palette:

| Piece | Entry ID | Dimensions | Raw PCX SHA-256 |
| --- | --- | --- | --- |
| Left cap | `4c8719b4` | 8x24 | `73e1505ad0cabef8f61aa1bf50c1383ac46c75ac0c2f7c4d94760397f481d446` |
| Center | `802d192a` | 114x24 | `38736874bee0c1461519d243ae1159ffe8ef4b9c6416a782b2e61319baf4f78c` |
| Right cap | `25b63aad` | 10x24 | `289788a3bafa7b56373875ae52ccc890d5fe84075ef7769cb46e02172fd66f59` |

The measured control is 50x24. Place the caps at x=0 and x=40; center the
114-pixel strip across the full control at x=-32, clipping it between the
caps. This exposes center columns 40 through 71. Do not scale the strip.
Copy only the interior `(1,1,48,22)` so the already verified bevel remains.
Filenames are unresolved; the loader scopes these IDs to LOCAL.MIX and
requires the measured dimensions.

The independent search compared every possible contiguous 32-pixel center
crop and nearest, bilinear, bicubic and Lanczos resizing against the exposed
background pixels in eight original Credits/Unit Count wells. Only source
column 40 matched every sample. The untouched Game Speed well was a separate
holdout: 1030 background pixels matched in each of four captures, with only
the yellow digit excluded from that texture-only check. Game Speed remains
unavailable in the native form. This evidence establishes this 50x24 skin
composition; other control sizes and game-speed behavior remain uncalibrated.
Failed models and the original pre-fix differences remain in the proof bundle.

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
`b3b7b060c10d0b7d` is retained through the custom match flow. A freshly
constructed legacy-force-profile EB1 match with seed 42 now has tick-1 hash
`4601c68d2468f7dc` (#370). The test retains exact complete save bytes across
loading that profile, in addition to its participant and state-hash checks.

The earlier fresh-state hash `57dac47c915b074a` predates the map structure
health and map unit heading corrections (#348, #353). Comparing the same
retail input at `76bdc0d` and `089267c` finds exactly 44 changed scalar fields:
41 structure maximum-health values, one zero-fraction structure's current
health (3 becomes 1), and two map unit headings (128 becomes 0 or 160).
Restoring those old initialization assumptions reproduces the earlier hash
exactly. This is a fresh-construction baseline change, not a save migration.
Affected old app setup keys remain rejected; raw simulation snapshots retain
their saved authoritative values as described in [save.md](save.md).
