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
