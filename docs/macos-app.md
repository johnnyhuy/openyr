# macOS app

Run `./scripts/package-macos.sh` from the checkout to build a native app for
the current Mac. The result is `target/OpenYR.app`; double-click it in Finder
or run `open target/OpenYR.app`. You can move the app to Applications. It does
not depend on the checkout or its working directory after packaging.

The bundle contains the release executable, a small log wrapper, its
`Info.plist` and the GPL licence. No EA assets are copied. The script uses
local ad-hoc signing, not Developer ID signing or notarization; downloaded
distribution builds remain future packaging work. The bundle layout follows
[Apple's bundle structure](https://developer.apple.com/library/archive/documentation/CoreFoundation/Conceptual/CFBundles/BundleTypes/BundleTypes.html).

## Starting a match

No arguments opens the animated native main menu. Single Player offers saved
matches or the supported EB1.MMX skirmish setup. OpenYR looks for game data in
this order:

1. `OPENYR_GAMEDIR`, when set. An invalid explicit override is an error.
2. The folder remembered in
   `~/Library/Application Support/OpenYR/game-directory.json`.
3. `~/Games/RA2YR/prefix/drive_c/Westwood/RA2`.
4. A macOS folder chooser, only for a launch with no arguments.

Each discovered folder must contain both `ra2.mix` and `ra2md.mix`; filenames
are checked without case sensitivity. A missing or malformed remembered
location falls through to the standard install or chooser. A selected valid
folder is remembered as a JSON path string. Delete that preference to choose
again if the install is outside the standard location. Cancel exits normally.
Before remembering a newly chosen folder, both the initial chooser and the menu
Data Folder action open it through the actual asset VFS (#364). Both required
archives must resolve as nonempty regular loose files and parse as nonempty MIX
archives. Symlink-only archives and ambiguous nonempty loose names reject before
changing the remembered path. Missing, empty, malformed and truncated archives
also reject; existing discovery order and preference-write error policies remain.
Authored Linux filesystem fixtures verify rejection, preserved preferences/source
bytes and successful retry with mixed-case filenames. This validates container
loading, not the presence of every rules, art, map or audio asset needed to play.

The picker uses the standard
[AppleScript folder command and cancellation handling](https://developer.apple.com/library/archive/documentation/AppleScript/Conceptual/AppleScriptLangGuide/reference/ASLR_cmds.html).

Remembering the folder uses the same exclusive sibling replacement as interface
preferences (#362). At most 32 process/serial candidates are attempted with
`create_new`; occupied files, directories and symlinks are skipped. The owned
file is written and synced before rename, and removed on publication failure.
Legacy `game-directory.<pid>.tmp` siblings are left untouched. Existing JSON
path strings and the 16 KiB read limit are unchanged. Linux scratch-file tests
verify occupied file and Unix symlink-target preservation, failed publication,
cleanup and retry. They do not establish native macOS chooser behavior or safety
against a concurrent actor who can replace entries in the containing directory.

The app shows a native alert if startup fails, including invalid data or
display initialization errors. Audio failures keep the existing silent-match
fallback. Finder launches write `~/Library/Logs/OpenYR/openyr.log`; the previous
launch is kept as `previous.log`. CLI launches keep their usual terminal output.

## Command-line compatibility

An explicit game directory bypasses discovery, and an omitted map defaults
to EB1.MMX. Existing directory/map/scale commands remain supported. Flags
alone use the discovered folder and default map without opening any picker,
so headless jobs fail promptly if data is missing:

```sh
cargo run --release -- --frames /tmp/openyr-proof 300 900 1500 3000
cargo run --release -- "/path/to/RA2" --ticks 900 --save /tmp/match.oyrsave
cargo run --release -- "/path/to/RA2" EB1.MMX --load /tmp/match.oyrsave
cargo run --release -- --help
```

Help does not search for data or open a window. `CARGO` and `CARGO_TARGET_DIR`
are supported by the packaging script; its optional first argument selects
the output app path. It packages the host release build and requires macOS.
