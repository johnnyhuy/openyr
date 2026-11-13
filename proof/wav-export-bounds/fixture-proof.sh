set -e
source /workspace/openyr-env.sh
export CARGO_TARGET_DIR="$PWD/.tmp/clippy-target" CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_STRIP=symbols CARGO_PROFILE_TEST_STRIP=symbols
printf 'Recorded CI fixture repair on clean source\n'
git rev-parse HEAD HEAD^{tree}
cargo +1.100.0 test -p yr-render --locked map_extent -- --nocapture --test-threads=16
