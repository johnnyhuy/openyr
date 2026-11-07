set -eu
export CARGO_HOME=/workspace/.cargo
export RUSTUP_HOME=/workspace/.rustup
export CARGO_TARGET_DIR=/workspace/openyr/target
export PKG_CONFIG_SYSROOT_DIR=/workspace/openyr-system
export PKG_CONFIG_PATH=/workspace/openyr-system/usr/lib/x86_64-linux-gnu/pkgconfig
python3 -m unittest discover -s scripts -p 'test_*.py' -v
python3 scripts/proof_build.py build --output .tmp/runtime-final
python3 scripts/proof_build.py verify --receipt .tmp/runtime-final/receipt.json
python3 scripts/proof_build.py run --receipt .tmp/runtime-final/receipt.json -- --help
python3 scripts/proof_build.py run --receipt .tmp/runtime-final/receipt.json -- /workspace/game-data/RA2 EB1.MMX --ticks 1
cp .tmp/runtime-final/program .tmp/program-backup
printf x >> .tmp/runtime-final/program
if python3 scripts/proof_build.py run --receipt .tmp/runtime-final/receipt.json -- --help; then
    mv .tmp/program-backup .tmp/runtime-final/program
    exit 1
fi
mv .tmp/program-backup .tmp/runtime-final/program
python3 scripts/proof_build.py verify --receipt .tmp/runtime-final/receipt.json
sha256sum .tmp/runtime-final/program
