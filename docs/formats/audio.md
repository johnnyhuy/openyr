# Audio: AUD, AUDIO.BAG/IDX, IMA WAV

All retail audio is IMA ADPCM, 22,050 Hz, decoded to signed 16-bit PCM by
`crates/yr-formats/src/audio.rs`. `mix-dump wav <archive>:<path> out.wav`
exports any of the three containers. Bank entries are addressed as
`<archive>:<path>/AUDIO.BAG#<name>`.

## Containers

| Container | Where | Used for |
|---|---|---|
| `.AUD` (Westwood) | `ra2.mix/LOCAL.MIX`, `SIDENC0x.MIX` | score/menu loops, `INTRO.AUD`, side speeches |
| `AUDIO.BAG` + `AUDIO.IDX` | `language.mix/AUDIO.MIX`, `langmd.mix/AUDIOMD.MIX` | unit voices and sound effects (`ichratta`, ...) |
| RIFF WAV, format `0x11` | same `AUDIO.MIX` archives | EVA lines (`CEVA060.WAV`), 1,223 files |

The `AUDIO.MIX` archives have **classic (flagless) headers but CRC32 entry
IDs**. `MixArchive::get` therefore tries the CRC hash first for every archive.

### AUD

```
u16 sample_rate      22050
u32 size             file size - 12
u32 output_size      decoded bytes
u8  flags            1 stereo, 2 16-bit (retail: always 2)
u8  codec            99 = IMA ADPCM (retail: always 99)
chunks:
  u16 in_size, u16 out_size, u32 0x0000DEAF, in_size bytes of nibbles
```

Nibbles are low-first, and the IMA predictor/index state **carries across
chunks** (no per-chunk header). A chunk yields `out_size / 2` samples. Full
chunks are 512 bytes in, 2,048 bytes out. **Retail final chunks may claim
one more sample than their nibbles hold** (`out = in*4 + 2`, 7 of 23 files,
e.g. `INTRO.AUD`, `EFFICIEN.AUD`). Decoding stops when the nibbles run out.

### AUDIO.IDX / AUDIO.BAG

```
IDX: char[4] "GABA", u32 version = 2, u32 count,
     count x { char[16] name, u32 offset, u32 size, u32 sample_rate,
               u32 flags, u32 chunk_size }
BAG: raw sample data at [offset, offset + size)
```

Flags: `1` stereo, `2` PCM, `4` 16-bit, `8` IMA ADPCM. Retail combinations:

| Flags | Meaning | Entries (both banks) |
|---|---|---|
| 12 | 16-bit mono IMA, 512-byte blocks | 3,318 |
| 13 | 16-bit stereo IMA, 1,024-byte blocks | 7 |
| 6 | 16-bit mono PCM | 105 |
| 7 | 16-bit stereo PCM | 8 |

IMA entries are Microsoft IMA blocks with `chunk_size` as the block size
(below). Entries tile the bag exactly (`language.mix` bank: 1,153 entries,
17,643,176 bytes).

### Microsoft IMA (WAV `0x11`, bag IMA entries)

Each block restarts the decoder from a per-channel header
`i16 predictor, u8 index, u8 0`. The predictor is the first output sample.
The header is followed by 4-byte groups alternating between channels, each
holding 8 low-first nibbles. Mono 512-byte blocks hold 1,017 samples. The
WAV `fact` chunk gives the true frame count, and the final block is padded.

## Two IMA roundings

The two packings use different nibble expansions, and each matches ffmpeg
bit-exactly only with its own:

- **AUD** uses the multiplicative form `diff = ((2*d + 1) * step) >> 3`
  (ffmpeg `adpcm_ima_ws`).
- **MS IMA** (WAV, bag) uses the reference bitwise form
  `step>>3 + (d&4 ? step) + (d&2 ? step>>1) + (d&1 ? step>>2)`
  (ffmpeg `adpcm_ima_wav`, 4-bit path).

## Verification (retail corpus)

- `mix-dump scan` decodes every file: 23 AUD, 1,223 WAV (1,197 mono,
  26 stereo), 3,438 bank entries, zero errors.
- Bit-exact against ffmpeg 7: all 23 AUD files (`-f wsaud`), 60 WAVs, and 27
  bank entries (both stereo IMA entries plus 25 mono), the latter by wrapping
  each entry in a `0x11` WAV header. ffmpeg emits the padded tail of the
  final WAV block; we truncate to the `fact` count.

## Runtime lookup and playback (retail EB1, 2026-10-23)

`yr-app::audio` keeps audio outside the deterministic simulation. The window
opens the default output through [CPAL](https://docs.rs/cpal/0.18.2/cpal/),
which uses CoreAudio on macOS. Missing files or an unavailable device leave
the match running silently. M toggles mute without pausing playback.

Observed in the retail INIs and archives:

- `rulesmd.ini` assigns `VoiceSelect`, `VoiceMove` and `VoiceAttack` to
  sound definitions. `E1` uses `GISelect`, `GIMove` and `GIAttackCommand`.
  Its `Primary=M60` resolves to `Report=GIAttack` when a `Fired` event arrives.
  `DieSound` is resolved from the last known type after a `Died` event.
- `soundmd.ini` has whitespace-separated sample stems in `Sounds`, sometimes
  prefixed with `$` for voices. `GISelect` lists `$igisea` through `$igisef`.
  `Volume=85` becomes 0.85 gain. The application cycles sample variants using
  its own counter, leaving the simulation RNG untouched.
- `evamd.ini` maps event names directly to side-specific samples. The current
  sandbox uses `Allied`: `EVA_UnitReady` maps to `ceva062`, training to
  `ceva066`, victory to `ceva022`, defeat to `ceva023`. CSF sound references
  are a fallback when an EVA event is supplied as a `VOX:*` label.
- Both sound banks must stay paired with their own index. In this install,
  `AUDIO.IDX` resolves to `LANGMD.MIX/AUDIOMD.MIX` (2,285 entries), while
  `LANGUAGE.MIX/AUDIO.MIX` has 1,153 more. The VFS winner is searched first,
  then the YR and RA2 archive banks. Loose WAV/AUD samples take precedence.
- `thememd.ini` lists `BrainFreeze` as the first normal theme, with
  `Sound=BrainFre`. Its stereo IMA WAV is in `THEMEMD.MIX`. The app loops
  the first normal theme whose PCM decodes, skipping menu/loading themes.

The mixer converts mono/stereo PCM to the output rate using linear
interpolation. Music runs at 18 percent, voices/EVA at 80 percent and
weapon/death effects at 50 percent, multiplied by the definition's volume.
Responses replace earlier responses; EVA lines play serially with at most
eight waiting. At most 32 voices and 64 device commands are retained.
Repeated EVA notifications have cooldowns to avoid a line for every casualty.
The callback receives decoded PCM only; archive reads and decoding stay on
the app thread. This is a first playback pass, without spatial attenuation,
pitch variation (`FShift`), original sound priorities, or playlist controls.

`openyr <gamedir> EB1.MMX --frames <dir> 300 900 1500 1800` now writes
`audio.wav` (44,100 Hz stereo PCM) and `audio-cues.tsv` beside the frames.
The cue log records simulation ticks at which sounds are submitted; EVA
playback can start later because speech is serialized. The WAV uses the
same mixer as the device callback and lasts one fifteenth of a second per
tick. It is an offline mix, not a microphone or device-loopback recording.
These outputs contain retail audio/art and must remain outside git.
