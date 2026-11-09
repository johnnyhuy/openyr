#!/usr/bin/env bash
set -euo pipefail
source /workspace/openyr-env.sh
cd /workspace/openyr-map-extents-20261109-1000
export CARGO_TARGET_DIR="$PWD/.tmp/cargo-target" CARGO_BUILD_JOBS=2
proof_root=/workspace/scratch/map-bounds-294-proof
receipt=.tmp/map-bounds-release/receipt.json
git rev-parse HEAD HEAD^{tree}
test -z "$(git status --porcelain)"
mise exec -- python3 scripts/proof_build.py verify --receipt "$receipt"
for fixture in small cancel-offset; do
    mise exec -- python3 scripts/proof_build.py run --receipt "$receipt" -- render-map "$proof_root/inputs/$fixture" SMALL.MAP "$proof_root/$fixture.png" 1
done
cmp "$proof_root/small.png" "$proof_root/cancel-offset.png"
sha256sum "$proof_root/small.png" "$proof_root/cancel-offset.png"
for fixture in huge-coordinates huge-extra; do
    for scale in 1 2 4; do
        output="$proof_root/rejected-$fixture-$scale.png"
        log="$proof_root/cli-$fixture-$scale.txt"
        set +e
        mise exec -- python3 scripts/proof_build.py run --receipt "$receipt" -- render-map "$proof_root/inputs/$fixture" SMALL.MAP "$output" "$scale" > "$log" 2>&1
        result=$?
        set -e
        cat "$log"
        test "$result" -eq 1
        test ! -e "$output"
        if test "$fixture" = huge-coordinates; then
            rg -q 'terrain coordinate extent exceeds 262144 cells' "$log"
        else
            rg -q 'map raster exceeds 1 GiB native RGBA budget' "$log"
        fi
        printf 'Verified %s scale %s: exit %s, no output PNG.\n' "$fixture" "$scale" "$result"
    done
done
