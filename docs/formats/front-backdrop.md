# Setup and chooser presentation

The original YR Skirmish and Choose Map pages use different 632x568 canvases.
OpenYR previously drew the RA2 red world map on both pages and covered the
lower Skirmish body with a second large preview. It now loads each page's
retail canvas and palette, retaining the measured right-hand preview.

## Retail provenance

All four entries are in `RA2MD.MIX/NTRLMD.MIX`. Their filenames remain
unresolved. Matching index patterns first identified the image independently
of palette choice, then observed renderer colors identified its palette.
The app validates the one-frame 632x568 canvas before decoding it. A missing
optional image falls back to the existing shell art.

| Page | SHP entry | Palette entry |
| --- | --- | --- |
| Skirmish | `1fb68421` | `d833dd7e` |
| Choose Map | `2eec89db` | `78a45e31` |

SHA-256 values for this install:

```text
NTRLMD.MIX 51a4363245d3df14608679aee293450036bd8048a6c248322bc6e14a9ff829df
1fb68421 43a8a6216760e48d98f7c7daa057dee044a42dda19292f5e07b8d1391bf1c84a
d833dd7e 6f9e53f8ffa583f0f93d8f82497c86acaab9d1296a6258bc38677876001ef5c3
2eec89db 78ac028705fbd7697d7b056fb9e1ac73da0fc157b4e500a060050e9f59d98ae5
78a45e31 ae676a55490c6014382e21e4aab00b33fff4f3e52156d294614974d52073f124
```

The original Skirmish frame captured at 2026-11-02 03:50:31 UTC has source
PNG SHA-256 `39ff6f766c29fa9274d2ffc0ec971d1f156518bd046d8fb1ff3a851c8a76b620`.
Its safe lower crop `(0,400,632,168)` contains no player controls. All 106,176
RGBA samples match the actual native compositor, FNV-1a `63e66974d0db60a9`.

The original chooser frame captured at 2026-11-02 04:11:21 UTC has source
PNG SHA-256 `fd91f39945e1f1abb5fe6d74ebd790a1bf8dfe0a125c1379bcba346a7c1d6457`.
Its safe left crop `(0,0,632,568)` contains no player fields. The entire body
matches outside `(117,147,195,38)`, the two modes intentionally unavailable in
OpenYR: 351,566 RGBA samples, FNV-1a `d0f73ad2035e4de5`. This includes the
artwork under both lists, selection fill, list text, bevels and scrollbar.
The separate background-only check covers 203,050 samples, FNV-1a
`ff1d6ca05c26268d`, excluding all controls.

`retail_skirmish_backdrop_matches_original_unobstructed_lower_body` and
`retail_chooser_backdrop_matches_original_unobstructed_pixels` check these
values after actual page composition and the shared RGB565 presentation.
They skip only when the retail install is absent. Captures and retail assets
are external proof, never repository fixtures.

## Battle chooser

`MISSIONSMD.PKT [MultiMaps]` supplies active order. Exact comma-separated
`GameMode=standard` membership gives 91 installed Battle maps in this retail
install. Descriptions resolve through `RA2MD.CSF`. Unlisted sections and
missing assets are excluded. Filenames must pass the same bounded bare-map
reference validation used for saved matches. Packet scanning stops at 4096
entries and titles at 128 characters.

The chooser lists 18 rows at a 19-pixel pitch. List text begins at `(341,128)`;
mode text begins at `(119,128)`. List backing subtracts one saturated RGB565
quantum per channel. The original `(515,127,20,345)` scrollbar leaves the
track transparent. Its 299-pixel track begins at y=150; the measured thumb
is 41 pixels high. This size is calibrated for the fixed 800x600 page and
91-entry retail catalog, not claimed as a recovered general sizing rule.

The scrollbar's PCX entries in `LOCALMD.MIX` carry their own palettes:

| Part | Entry | Canvas |
| --- | --- | --- |
| Up | `eec2554b` | 18x22 |
| Down | `85ff1b15` | 18x22 |
| Thumb body | `145c99ef` | 18x30 |
| Thumb top | `3085cabc` | 18x2 |
| Thumb bottom | `e50a2b3a` | 18x2 |

Their filenames remain unresolved. The body repeats vertically between the
caps. Both arrow images match the original renderer exactly. Scroll arrows,
page clicks, dragging, wheel input and selection remain within the catalog.
Focus loss or leaving the page clears drag state.

Preview work runs outside simulation and uses one worker at a time. Rapid
selection replaces the pending target. A result must still match the current
page, epoch, index and asset. Cancel retains the accepted map and preview;
Use Map stays disabled until the current preview succeeds. Failure leaves
the accepted map intact and permits another selection. Accept updates the
map used by Start and retains it on return to the front menu. No simulation
or save encoding changes are introduced here.

The actual-handler test
`retail_battle_map_chooser_previews_cancels_and_commits_the_selected_map`
covers keyboard and mouse selection, pending cancellation, stale results,
preview failure/retry, focus cleanup, acceptance and Bering Strait match
entry. `retail_battle_catalog_previews_every_available_map` loads all 91
previews and requires at least two authored start coordinates in each.

## Preview and remaining gaps

The chooser preview control is `(643,37,146,114)`, one pixel larger than the
Skirmish control. It centers odd margins upward and uses direct horizontal
projection without the separately observed Skirmish bias. Five complete
original right-panel regions match: Alamo, City, Bering, Heck and Death.
Killer Instinct's second marker is one pixel right of the original (#164).
Its regression explicitly excludes only `(709,80,19,18)`; all other pixels
and markers are checked. No map-specific correction is used.

The chooser heading uses its observed `(634,3,164,18)` label rectangle and
matches the default original heading. The [heading observations](front-headings.md)
now cover refreshed Skirmish and chooser foregrounds (#153). Focus loss,
selection changes and modal return refresh the final glyph; leaving and
re-entering Skirmish restores its initial styling.

Free For All, Team Alliance and random map generation are disabled. The
Skirmish setup still uses the supported Allied sandbox profile. Complete
player/faction/options forms, transitions, timing and other menus remain
in #96. This slice makes no claim of complete form or animation parity.
