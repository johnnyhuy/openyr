# Bink metadata and native menu playback

`yr-formats::BinkFile` parses Bink 1 metadata and frame packet ranges without
I/O or codec work. `yr-app::movie` uses the separately licensed
`infinitier_bik_decoder` Cargo dependency for video decoding. Its version and
attribution are in [the dependency notes](../third-party.md). Decoder source
is not copied into this repository.

## Retail observations

The 2026-11-01 scan validates all 146 Bink entries in the 16 retail archives.
The header has a BIK signature and revision, a file size excluding the first
eight bytes, frame count, maximum packet size, repeated count, dimensions,
rational frame rate, flags and audio-track count. Tracks precede the frame
index. Index low bits mark keyframes; adjacent offsets bound packets and the
terminal offset matches EOF. Counts and ranges are checked against the input
before allocating either table. No compressed payload is copied by this parser.

The field interpretation was checked against the retail bytes and
[Mike Melanson's format observations](https://multimedia.cx/bink-format.txt),
with the updated [container description](https://wiki.multimedia.cx/index.php/Bink_Container).
Revision b and Bink 2 are outside this parser's scope. Unknown header fields
are not used as allocation sizes.

The menu's `LANGMD.MIX/RA2TS_L.BIK`, ID `33665128`, is BIKi, 7,185,136 bytes,
632x570, 431 frames, 15/1 fps, zero flags and zero audio tracks. Its first
packet begins at byte 1772. The encoded maximum packet is 49,984 bytes.
The declared duration is 431/15 seconds. Menu music is separate: the retail
`THEMEMD.INI` INTRO section selects the sample through SoundBank.

All 232,895,160 decoded YUV bytes match FFmpeg 8.1.2. Both streams have SHA-256
`08d017092b70bd3b9d679abbec6c0d12421be125862ee211111c7b4a3247b559`.
The retail test can repeat this comparison with `OPENYR_BINK_ORACLE` pointing
to independently decoded planar YUV420 data. `OPENYR_BINK_RGB_ORACLE` adds an
RGBA comparison from the same input.

## Display conversion and scheduling

The decoder's generic RGB mode leaves limited-range black/white levels
unexpanded. The native menu reads the verified planes and uses nearest chroma
sampling with separate signed BT.601 contributions truncated toward zero
(#136). The front compositor then packs RGB565. Fourteen visible movie phases
match original renderer capture fingerprints exactly; the captured sequence
and compatible integer matrix are in the [oracle observations](front-animation-oracle.md).
The comparison against all 431 independently decoded YUV/RGB frames remains
separate from this original display check. Captured-frame agreement does not
calibrate original playback cadence or prove uncaptured original phases.
Packed black/white and truncated-plane checks also run without retail data.

Playback runs on a worker. The compressed clip is held once, the decoder keeps
its dependent-frame state, and a single pending frame replaces older pending
results. The UI owns only its current frame. No whole decoded movie is cached.
A rational deadline from the start instant avoids per-frame rounding drift;
decoding continues sequentially when the UI stalls. Looping rebuilds the
codec from frame zero. Cancellation wakes the deadline wait and joins the
worker; spurious wakeups do not advance the clock. Decoder errors become a
recoverable menu message.

Before codec construction, the native menu limits input to 16 MiB, each
canvas dimension to 1024, frame count to 30,000 and rate to 1-60 fps. It accepts
BIKi without audio/scaling/alpha flags. Other movie classes require a separate
playback policy. Hostile header/index cases run without retail assets.

`OPENYR_FRONT_PROOF` exports sampled source and native composite frames.
Adding `OPENYR_FRONT_VIDEO=1` exports all 431 native compositions for a proof
video at the declared 15 fps. These are shared-renderer captures, not a desktop
recording. Keep every derived image/video outside git.
