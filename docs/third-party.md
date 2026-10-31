# Dependency provenance

The native engine is written here. Standard Rust libraries and independent
codec libraries are consumed through Cargo, with versions and checksums in
Cargo.lock. No decoder implementation is transplanted into this source tree.

| Dependency | Version | Declared license | Upstream |
| --- | --- | --- | --- |
| infinitier_bik_decoder | 0.0.17 | GPL-3.0-or-later | [infinitier](https://github.com/ufoscout/infinitier) |
| infinitier_datasource | 0.0.17 | GPL-3.0-or-later | [infinitier](https://github.com/ufoscout/infinitier) |

The decoder's upstream README attributes its Bink implementation to a port of
FFmpeg's bink.c and binkaudio.c from release/6.1. OpenYR uses its public decoder
API, validates the retail container before construction, and performs its own
limited-range display conversion. FFmpeg is used separately as a verification
oracle; the application does not launch or link an external FFmpeg runtime.

Assets, fonts, movie packets, menu labels and music still come from the user's
retail install. These dependencies do not supply EA data or game-engine code.
