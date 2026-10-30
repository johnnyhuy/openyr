#!/bin/sh
# Build a local, ad-hoc signed app without any retail game assets.
set -eu
if [ "$(uname -s)" != Darwin ]; then
    printf '%s\n' 'OpenYR.app must be built on macOS.' >&2
    exit 1
fi
repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo"
cargo_bin=${CARGO:-cargo}
if ! command -v "$cargo_bin" >/dev/null 2>&1; then
    cargo_bin="$HOME/.cargo/bin/cargo"
fi
"$cargo_bin" build --release -p yr-app
target_dir=${CARGO_TARGET_DIR:-target}
app=${1:-"$target_dir/OpenYR.app"}
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp "$target_dir/release/openyr" "$app/Contents/MacOS/openyr"
cp packaging/macos/launch-openyr "$app/Contents/MacOS/launch-openyr"
chmod 755 "$app/Contents/MacOS/launch-openyr" "$app/Contents/MacOS/openyr"
cp packaging/macos/Info.plist "$app/Contents/Info.plist"
cp LICENSE "$app/Contents/Resources/LICENSE"
plutil -lint "$app/Contents/Info.plist"
codesign --force --sign - "$app/Contents/MacOS/openyr"
codesign --force --sign - "$app"
codesign --verify --deep --strict "$app"
printf 'Built %s. Open it in Finder to start EB1.MMX.\n' "$app"
