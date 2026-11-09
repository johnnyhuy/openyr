# Preference replacement proof for #299

`terminal.txt` is a real `script` recording made on Linux. It shows the failure
from the original native EdgeScroll handler, then five passing regression tests
from the fixed native handler/private preference writer using owned scratch
files. The regression actually changes preferences, preserves the unrelated
symlink target, reopens the stored JSON, reaches Controls after a publication
error and retries after removing the fixture's obstruction. Helper checks cover
occupied file/directory/symlink candidates, bounded 32-collision exhaustion and
cleanup of only the owned failed candidate.

`run-proof.sh` is the exact recording command script. `targeted.log` is the Cargo
invocation for those tests. The recording names/hashes the actual debug test
executable; this is separate from the isolated release receipt.

`receipt.json` comes from `scripts/proof_build.py build` at clean committed
source `0e818e3041d5b8dcf161a79796a291412d2f11f9`, tree
`68b0e7f69a314e0976394e2af4ddbade56511fa4`. The release CLI `--help` smoke verifies
that isolated program's provenance and launch. It does not exercise preference
handlers. The copied program remains outside Git.

`full-check.log` records `mise run check`: locked workspace tests, strict
all-target clippy, fmt, and three proof-script tests passed. Rust reported 557
passed, one ignored and zero failed. Retail-labelled tests self-skip without
retail data and are not retail passes.

No retail data, native window, physical Mac, Retina, audio or original-game
runtime checks ran. This nonvisual storage slice uses terminal evidence.
Previously blocked original-game/Mac capture and HTTP401 release-upload paths
were not retried. This does not establish safety against concurrent directory
mutation by another actor.

These files are temporary PR-branch proof. Their pinned commit remains linked
from the PR; every proof file must be removed before squash merge so the final
source tree exactly equals the tested and receipted tree.
