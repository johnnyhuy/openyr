#!/usr/bin/env python3
"""Build and run fixed-scope, public Win32 UI diagnostics under Wine."""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time


SOURCE = Path(__file__).resolve().parent
IMPORTS = {
    "GetStdHandle": 4,
    "WriteFile": 20,
    "ExitProcess": 4,
    "LoadLibraryA": 4,
    "GetProcAddress": 8,
    "GetLastError": 0,
    "OpenProcess": 12,
    "CloseHandle": 4,
    "QueryFullProcessImageNameA": 16,
}


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def build(out, prefix):
    rustc = shutil.which("rustc") or str(Path.home() / ".cargo/bin/rustc")
    sysroot = Path(subprocess.check_output([rustc, "--print", "sysroot"], text=True).strip())
    linkers = list(sysroot.glob("lib/rustlib/*/bin/gcc-ld/lld-link"))
    if len(linkers) != 1:
        raise RuntimeError("Expected one lld-link in the active Rust toolchain")
    linker = str(linkers[0])
    clang = shutil.which("clang") or "/usr/bin/clang"
    definition = out / "kernel32.def"
    definition.write_text("LIBRARY kernel32.dll\nEXPORTS\n" + "\n".join(IMPORTS) + "\n")
    library = out / "kernel32.lib"
    subprocess.run([linker, "/lib", "/machine:x86", "/def:" + str(definition),
                    "/out:" + str(library)], check=True)
    fixture = {"TARGET_IMAGE": r"C:\openyr-oracle\fixture32.exe",
               "TARGET_DIRECTORY": r"C:\openyr-oracle"}
    no_window = {"TARGET_IMAGE": r"C:\openyr-oracle\no-window32.exe",
                 "TARGET_DIRECTORY": r"C:\openyr-oracle"}
    variants = [
        ("observe32", "observer.c", {}),
        ("launch32", "observer.c", {"LAUNCH": 1}),
        ("fixture-observe32", "observer.c", fixture),
        ("fixture-launch32", "observer.c", dict(fixture, LAUNCH=1)),
        ("fixture32", "fixture.c", {}),
        ("no-window32", "fixture.c", {"NO_WINDOW": 1}),
        ("no-window-observe32", "observer.c", no_window),
        ("no-window-launch32", "observer.c", dict(no_window, LAUNCH=1)),
    ]
    destination = prefix / "drive_c/openyr-oracle"
    destination.mkdir(exist_ok=True)
    binaries = {}
    for name, source, defines in variants:
        obj, exe = out / (name + ".obj"), out / (name + ".exe")
        flags = ["-D" + key + "=" + (json.dumps(value) if isinstance(value, str) else str(value))
                 for key, value in defines.items()]
        subprocess.run([clang, "--target=i686-pc-windows-msvc", "-O2", "-Wall", "-Wextra",
                        "-Werror", "-fno-stack-protector", "-fno-builtin", *flags,
                        "-c", str(SOURCE / source), "-o", str(obj)], check=True)
        aliases = [f"/alternatename:__imp__{name}@{size}=__imp__{name}"
                   for name, size in IMPORTS.items()]
        subprocess.run([linker, "/entry:mainCRTStartup", "/subsystem:console", "/nodefaultlib",
                        "/machine:x86", "/out:" + str(exe), str(obj), str(library), *aliases], check=True)
        shutil.copy2(exe, destination / exe.name)
        binaries[exe.name] = digest(exe)
        print("Built and staged " + exe.name, flush=True)
    receipt = {"sources": {p.name: digest(p) for p in [*SOURCE.glob("*.c"), SOURCE / "oracle.py"]},
               "binaries": binaries, "clang": clang, "linker": linker}
    (out / "build.json").write_text(json.dumps(receipt, indent=2) + "\n")


def run_helper(name, args, phase):
    command = [str(args.wine), "C:\\openyr-oracle\\" + name + ".exe"]
    env = dict(os.environ, WINEPREFIX=str(args.prefix), WINEDEBUG="err+all")
    stdout, stderr = args.out / (phase + ".stdout"), args.out / (phase + ".stderr")
    started = datetime.datetime.now(datetime.timezone.utc).isoformat()
    start = time.monotonic_ns()
    with stdout.open("wb") as output, stderr.open("wb") as errors:
        result = subprocess.run(command, cwd=args.prefix / "drive_c/openyr-oracle", env=env,
                                stdout=output, stderr=errors)
    result.stdout, result.stderr = stdout.read_bytes(), stderr.read_bytes()
    receipt = {"phase": phase, "command": command, "started_utc": started,
               "elapsed_ms": (time.monotonic_ns() - start) / 1e6,
               "exit_code": result.returncode, "stdout_sha256": digest(stdout),
               "stderr_sha256": digest(stderr)}
    (args.out / (phase + ".json")).write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps(receipt), flush=True)
    print(result.stdout.decode(errors="replace"), end="", flush=True)
    return result


def verify(args):
    good = run_helper("fixture-launch32", args, "fixture-valid")
    text = good.stdout.decode(errors="replace")
    if good.returncode != 0 or "verified target=C:\\openyr-oracle\\fixture32.exe" not in text:
        raise RuntimeError("Synthetic target was not verified")
    expected = ["page_client=800,600", "window_xyxy=40,50,185,166 client_xywh=40,50,145,116",
                "class=Edit", "label_key=GUI:FixtureHeading"]
    if not all(value in text for value in expected):
        raise RuntimeError("Synthetic control geometry or metadata differs")
    if "PRIVATE_FIXTURE_VALUE_7d39" in text:
        raise RuntimeError("Observer disclosed an edit value")
    silent = run_helper("no-window-launch32", args, "fixture-silent-exit")
    if silent.returncode != 4 or b"child_exit_code=0" not in silent.stdout:
        raise RuntimeError("A successful process exit was mistaken for a verified UI")
    absent = run_helper("no-window-observe32", args, "fixture-absent")
    if absent.returncode != 4 or b"eligible_roots=0" not in absent.stdout:
        raise RuntimeError("Absent target was not rejected")
    print("PASS: own UI geometry, private edit exclusion, silent-exit detection and absent-target guard", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=["build", "verify", "observe", "launch"])
    parser.add_argument("--prefix", type=Path, default=Path.home() / "Games/RA2YR/prefix")
    parser.add_argument("--wine", type=Path, default=Path.home() /
                        "Games/RA2YR/wine/Wine Stable.app/Contents/Resources/wine/bin/wine")
    parser.add_argument("--out", type=Path, default=SOURCE.parents[1] / ".tmp/wine-oracle")
    args = parser.parse_args()
    args.prefix, args.wine, args.out = [p.expanduser().resolve() for p in (args.prefix, args.wine, args.out)]
    if not (args.prefix / "drive_c").is_dir():
        parser.error("An existing Wine prefix with drive_c is required")
    args.out.mkdir(parents=True, exist_ok=True)
    build(args.out, args.prefix)
    if args.mode == "verify":
        verify(args)
    elif args.mode in ("observe", "launch"):
        stamp = datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%dT%H%M%S%fZ")
        result = run_helper(args.mode + "32", args, args.mode + "-" + stamp)
        return result.returncode
    return 0


if __name__ == "__main__":
    sys.exit(main())
