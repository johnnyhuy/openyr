source /workspace/openyr-env.sh
export CARGO_TARGET_DIR="$PWD/.tmp/baseline-target" CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 OPENYR_WORLD_FEEDBACK_PROOF="$PWD/.tmp/before"
git rev-parse HEAD HEAD^{tree}
cargo +1.100.0 test -p yr-app --bin openyr --locked late_desktop_modifier_compositor -- --nocapture
