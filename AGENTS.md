# AGENTS.md

Native Rust rewrite of Red Alert 2: Yuri's Revenge (macOS first). Not a Wine wrapper.

## Commands

- `mise trust && mise install` installs project tools; `mise run check` runs the
  local Rust and proof checks. `mise tasks` lists launch/proof/package tasks.
  Rust versions/components live in `rust-toolchain.toml`; Python/tasks in `mise.toml`.

- `cargo build --workspace` / `cargo test --workspace` (use `~/.cargo/bin` if cargo is not on PATH)
- `cargo run -p mix-dump -- list <archive.mix>` to inspect game data
- Real-data tests read `~/Games/RA2YR/prefix/drive_c/Westwood/RA2`; they skip when it is absent. Never commit game assets.
- CI (`.github/workflows/ci.yml`, job `test`: fmt + clippy `-D warnings` + test) is a required check: never merge a PR while `test` is failing or pending. Server-side branch protection needs GitHub Pro or a public repo (private/Free returns 403, attempted 2026-10-17), so until the owner enables one of those, this rule is enforced by convention: check `gh pr checks <n>` before every merge.

## Sweep and merge

Run this before starting new work, every time. Stale PRs and branches are the biggest source of wasted context here.

- `gh pr list --repo johnnyhuy/openyr --state open --limit 50 --json number,title,headRefName,mergeable,mergeStateStatus,isDraft`
- Merge every conflict-free PR with green checks: `gh pr merge <n> --squash --delete-branch`.
- Merge complete drafts. Leave genuinely incomplete ones and say so.
- Resolve merge conflicts on the branch rather than closing and reopening. Formatting-only conflicts are usually rustfmt colliding with hand-written code, take the rustfmt side, then confirm `cargo fmt --check`.
- Close duplicates and superseded PRs with a comment explaining why. Never close a PR carrying work that exists nowhere else without saying so.
- Delete branches on origin once their PR is merged or closed.
- Check whether a branch is actually dead before deleting it. Squash merges break ancestry, so `merge-base --is-ancestor` always reports false. Compare trees instead (`git diff --name-only origin/main origin/<branch>`) and confirm the work exists in the current layout under `crates/`.
- If auto mode blocks a merge, fall back to `gh api -X PUT repos/johnnyhuy/openyr/pulls/<n>/merge -f merge_method=squash -f sha=<head-sha>` rather than just reporting it. Cap retries at two per run, then name the blocked PR in the output.
- The PR list should be empty when a run finishes.

## Issues

The issue list is the backlog of record. Keep it honest in both directions.

- Read `gh issue list --repo johnnyhuy/openyr --state open --limit 50` before picking work, alongside `docs/roadmap.md`. Roadmap bullets and issues should describe the same work; where they disagree, the roadmap is the source of truth and the issue needs correcting.
- **Close what shipped.** When a merge lands the work behind an issue, close it. Prefer a `Closes #N` in the commit message so the link survives the squash; otherwise `gh issue close <n> --comment "<evidence>"`.
- Verify before closing. Check the issue's acceptance criteria against `main`, not against the branch that closed it. A `[x]` roadmap bullet plus a live path in `main` is the standard test. Close it even when a follow-up remains, but note the remainder on the follow-up issue.
- **Open what was discovered.** A bug found during a run gets an issue before the run ends, even when it is not fixed yet. One issue per distinct defect.
- A bug found and fixed in the same run: still file it, then close it in the same run with the fix reference. The history is worth more than the empty count.
- Title format follows the existing convention: `[M<n>] <what> (<area>)`. Keep the milestone prefix so the list stays sortable.
- Label with `bug` for defects and `enhancement` for planned work. Add `wontfix` when declining, with the reason in a comment, then close.
- Before filing, search: `gh issue list --repo johnnyhuy/openyr --state all --search "<terms>"`. Comment on the existing issue rather than opening a near-duplicate.
- Never close an issue as a side effect of unrelated work. Close it in the run that delivered it, or leave it and say why not.

## Evidence on every PR

A PR with no screenshot, GIF, video, or terminal recording is an incomplete slice.

- Capture in this order, falling through on failure without retrying a known-blocked path: live `screencapture -x` → `screencapture -V` recording → headless `--frames` (always available, same draw code as the window) → `script`/`asciinema` for non-visual slices.
- Build runtime proof with `python3 scripts/proof_build.py build --output .tmp/proof-build-<unique-name>` from a clean committed tree. Run via `python3 scripts/proof_build.py run --receipt <output>/receipt.json -- <arguments>`. This isolates Cargo targets and rejects changed source/executables (#238); see `docs/runtime-proof.md`. A successful shared-target build alone does not establish executable provenance.
- Headless capture through the verified copy: `python3 scripts/proof_build.py run --receipt <output>/receipt.json -- <gamedir> EB1.MMX --frames <dir> 300 900 1500 2400`. Choose ticks that actually show the change.
- The frames contain EA artwork, so **never commit them**. Upload to a draft release (`gh release create <tag> --draft` then `gh release upload`) and link from the PR body. `proof/<slice>/` on a PR branch is the fallback only, and must be dropped before merge.
- PR body order: what changed → exact verification commands and their pass/fail results → proof images with tick numbers or timestamps → reviewer notes. Write it to a file under `.tmp/` and use `gh pr edit <n> --body-file`.
- Run output must carry the PR URL and the proof link, and must state which capture paths were permission-blocked so the next run skips them.

## Architecture rules

- `crates/yr-formats` is pure parsing only, no filesystem writes, no platform code.
- Future `sim` crate must be deterministic: fixed-point maths only, no `f32`/`f64`, no `HashMap` iteration in sim paths, fixed tick order. See `docs/architecture.md`.
- Keep renderer/app code out of the simulation crates.
- Parsers treat every header field as hostile: check counts and sizes against the input length (with checked arithmetic) before allocating, and add a hostile-header unit test for each new format.
- `mix-dump scan <all retail .mix>` must stay at zero errors; it decodes every asset in the install.

## Writing style

The natural-writing skill rules apply to every prose output here: PR bodies, run summaries, issue text, commits, `docs/` prose, and code comments. The most important rule, with concrete substitutions for this repo:

- **No em-dashes anywhere in prose.** Replace `X — Y` with a context-appropriate form:
  - Headings and label openers: `X: Y` (e.g. `## Sweep and merge`, `**Status:** in progress`).
  - Citation/appositive: `X. [link]` or `X, via `gh api ...`` — the em dash almost always introduces a source.
  - In-quote appositive: `text "...", more text` — comma, not dash.
  - Window title / user-facing strings: `OpenYR: {}` not `OpenYR — {}`.
  - Mid-sentence aside between two independent clauses: `,` or `. `, never ` — `.
- **No `Label:` colon openers** in prose. `Key insight:`, `Note:`, `Status:`, `Why this matters:`, `**Status:**` are all filler. Use a full sentence or a real section heading.
- **No bold-as-speech.** Reserve bold for one specific term, not every sentence lead.
- **No `## Why This Matters` sections.** Fold the why into the surrounding content.
- **No opener fluff** like "I hope this finds you well", "Let me explain", "Below you'll find". Start with the point.
- **No "Please don't hesitate to reach out"** sign-offs. End at the call to action or just stop.
- **One idea per paragraph** for medium-length replies. Fragments are fine.
- **Plain ASCII preferred.** No en-dashes, smart quotes, ellipsis, or arrow symbols in prose.

The em-dash rule is the single strongest tell of AI-generated text. An em-dash in any prose artefact is a prompt-engineering failure. This rule covers Rust doc comments, Markdown, GitHub issue and PR bodies, commit messages, and user-facing strings (window titles, status text). Code identifiers and table cells (`—` as a placeholder) are exempt.

## Method

- Clean-room: observe behaviour and file formats; do not copy code from YRpp, Phobos, XCC or the original binary into this repo. Format facts go in `docs/formats/` with the evidence.
- Validation is empirical: prefer tests against the retail archives over remembered specs.
- Legal posture is in `docs/legal.md`. Do not add EA assets, EA code, or trademarked logos.

## Docs

- `docs/roadmap.md`, milestones; `docs/architecture.md`, crate plan; `docs/research/`, precedents.
