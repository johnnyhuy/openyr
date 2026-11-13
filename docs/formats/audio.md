# Audio: AUD, AUDIO.BAG/IDX, IMA WAV

All retail audio is IMA ADPCM, 22,050 Hz, decoded to signed 16-bit PCM by
`crates/yr-formats/src/audio.rs`. `mix-dump wav <archive>:<path> out.wav`
exports any of the three containers. Bank entries are addressed as
`<archive>:<path>/AUDIO.BAG#<name>`.

Canonical 16-bit WAV export is fallible (#390). `encode_wav` requires complete
mono/stereo frames and a nonzero rate. It checks block alignment, byte rate,
sample byte count, RIFF size and platform allocation limits before reserving
the output buffer. A valid 8-bit source at rate 4,294,967,295 cannot be exported
as 16-bit PCM because doubling its byte rate exceeds the RIFF field. The CLI
reports the error before creating or replacing the destination; supported
exports retain their exact bytes. The native recorder propagates encoder
errors before opening its output and retains its existing streaming size cap.
Simulation and save formats are unchanged.

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

### WAV container validation

The decoder bounds chunk reads by the declared RIFF size (#387). It rejects
an incomplete chunk header, a chunk payload beyond that boundary, or missing
odd-length chunk padding. Bytes after the RIFF container are ignored and
cannot supply a missing format or data chunk. Unknown complete chunks and
data-before-format ordering remain supported.

Supported WAV output is mono or stereo with a nonzero sample rate. PCM 8-bit
and 16-bit headers must declare exactly `channels * bytes_per_sample` block
alignment, the corresponding checked byte rate, and complete frames. IMA
block decoding and `fact` trimming retain their existing policies. This does
not add new codecs or change ADPCM predictor math.

Authored parser controls cover both channel counts and PCM depths, padded
odd chunks, container boundaries and IMA fact trimming. CLI checks retain an
existing export on malformed input and report each invalid WAV while scanning
valid neighbors. Native sound lookup falls back from a malformed loose WAV to
its paired bank entry, caches that result and keeps unrelated samples usable.
The retail census still decodes all 1,223 WAVs without parser errors.

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
which uses CoreAudio on macOS. Missing files leave the match running silently.
An unavailable device is retried by an audio worker. M toggles mute without
pausing playback.

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

## Device interruptions and recovery (#60)

The first output implementation discarded every CPAL callback error and
dropped the whole audio scene on the next simulation tick. A regression
test reproduced permanent silence from `ErrorKind::Xrun`. In CPAL 0.18.2,
the CoreAudio processor-overload listener reports that kind for an underrun;
the stream remains active. `DeviceChanged` also leaves a default-device
stream active after CoreAudio reroutes it. These backend behaviours are
documented in CPAL's `src/error.rs` and `src/host/coreaudio/macos/mod.rs`.
The earlier live-session logs did not retain the error kind, so their
precise trigger cannot be established retrospectively.

The callback now submits the original error through bounded channels.
Active-stream notices (`Xrun`, `DeviceChanged`, `RealtimeDenied`) keep the
stream and log at most once every five seconds. A separate fault slot
ensures repeated underruns cannot hide device loss. Device/host absence,
busy devices, invalidated streams, resource exhaustion and unclassified
backend errors close the old stream and retry the current default device.
The delay doubles from 500 ms to a maximum of eight seconds, and resets
after a successful open. Permission, unsupported configuration/operation,
invalid-input and unknown error kinds disable playback with their diagnostic;
they require correction and a session restart.

All device discovery, open, close, retry and diagnostic output happen on a
separate audio worker. The app only sends decoded playback commands with
`try_send`, or changes an atomic mute flag. The callback never logs or waits
to deliver its error. Dropping the audio handle disconnects the worker
without joining it on the app thread. Initial device absence follows the
same retry path, retaining the sound bank and scene for later playback.

Each replacement stream starts a fresh mixer at its newly queried sample
rate. The current theme restarts from the beginning, mute and cumulative
callback counters survive, and effects/speech from the outage expire.
The app-to-worker and worker-to-callback queues each hold at most 64
commands. Regression tests inject notices, disconnects and busy devices,
check retry deadlines and permanent failures, and hold device discovery
blocked while the app submits commands and changes mute.

An opt-in test opens real CoreAudio output, plays the retail theme, injects
a disconnect through the worker's fault mailbox, opens a replacement,
checks muted callbacks, then checks signal after unmuting:

```
cargo test -p yr-app --lib native_output_recovers_and_preserves_mute -- --ignored --nocapture
```

This test plays audio and requires the retail install and an output device.
The injected disconnect verifies native reopen and callback delivery; it
does not establish physical unplug or Bluetooth behaviour.
