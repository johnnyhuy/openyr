# AGENTS.md

Native Rust rewrite of Red Alert 2: Yuri's Revenge (macOS first). Not a Wine wrapper.

## Commands

- `cargo build --workspace` / `cargo test --workspace` (use `~/.cargo/bin` if cargo is not on PATH)
- `cargo run -p mix-dump -- list <archive.mix>` to inspect game data
- Real-data tests read `~/Games/RA2YR/prefix/drive_c/Westwood/RA2`; they skip when it is absent. Never commit game assets.

## Architecture rules

- `crates/yr-formats` is pure parsing only — no filesystem writes, no platform code.
- Future `sim` crate must be deterministic: fixed-point maths only, no `f32`/`f64`, no `HashMap` iteration in sim paths, fixed tick order. See `docs/architecture.md`.
- Keep renderer/app code out of the simulation crates.
- Parsers treat every header field as hostile: check counts and sizes against the input length (with checked arithmetic) before allocating, and add a hostile-header unit test for each new format.
- `mix-dump scan <all retail .mix>` must stay at zero errors; it decodes every asset in the install.

## Method

- Clean-room: observe behaviour and file formats; do not copy code from YRpp, Phobos, XCC or the original binary into this repo. Format facts go in `docs/formats/` with the evidence.
- Validation is empirical: prefer tests against the retail archives over remembered specs.
- Legal posture is in `docs/legal.md`. Do not add EA assets, EA code, or trademarked logos.

## Docs

- `docs/roadmap.md` — milestones; `docs/architecture.md` — crate plan; `docs/research/` — precedents.
