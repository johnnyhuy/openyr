#!/usr/bin/env python3
"""Build isolated runtime proof and check its source/executable provenance."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile


class ProofError(Exception):
    pass


def git(repo, *args):
    return subprocess.check_output(
        ["git", "-C", str(repo), *args], text=True
    ).strip()


def source(repo):
    if git(repo, "status", "--porcelain", "--untracked-files=all"):
        raise ProofError("commit source changes before building or running proof")
    return {"commit": git(repo, "rev-parse", "HEAD"),
            "tree": git(repo, "rev-parse", "HEAD^{tree}")}


def digest(path):
    result = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            result.update(chunk)
    return result.hexdigest()


def cargo_binary():
    if os.environ.get("CARGO"):
        return os.environ["CARGO"]
    found = shutil.which("cargo")
    if found:
        return found
    return str(Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo")) / "bin/cargo")


def build(repo, output, package="yr-app", binary="openyr"):
    repo, output = repo.resolve(), output.resolve()
    before = source(repo)
    # A new output directory cannot accidentally reuse an earlier proof build.
    output.mkdir(parents=True, exist_ok=False)
    source(repo)  # Reject unignored output inside the checkout before compiling.
    cargo = cargo_binary()
    version = subprocess.check_output([cargo, "--version"], text=True).strip()
    artifact = None
    with tempfile.TemporaryDirectory(prefix="target-", dir=output) as temporary:
        target = Path(temporary).resolve()
        command = [cargo, "build", "--manifest-path", str(repo / "Cargo.toml"),
                   "--package", package, "--bin", binary, "--release", "--locked",
                   "--target-dir", str(target), "--message-format=json-render-diagnostics"]
        print(f"Building {package}/{binary} in a fresh isolated target", flush=True)
        with (output / "cargo.jsonl").open("w") as log:
            with subprocess.Popen(command, cwd=repo, stdout=subprocess.PIPE, text=True) as process:
                for line in process.stdout:
                    log.write(line)
                    message = json.loads(line)
                    if message.get("reason") == "compiler-message":
                        rendered = message.get("message", {}).get("rendered")
                        if rendered:
                            print(rendered, file=sys.stderr, end="")
                    if (message.get("reason") == "compiler-artifact"
                            and message.get("target", {}).get("name") == binary
                            and "bin" in message.get("target", {}).get("kind", [])
                            and message.get("executable")):
                        artifact = Path(message["executable"]).resolve()
                if process.wait() != 0:
                    raise ProofError("Cargo failed; no proof receipt was issued")
        if artifact is None or not artifact.is_relative_to(target) or not artifact.is_file():
            raise ProofError("Cargo did not report the requested isolated executable")
        if source(repo)["tree"] != before["tree"]:
            raise ProofError("source changed during the build; no proof receipt was issued")
        shutil.copy2(artifact, output / "program")
    receipt = {"format": "openyr-proof-build-v1", "source": before,
               "package": package, "binary": binary, "profile": "release",
               "cargo": version, "command": command,
               "sha256": digest(output / "program"),
               "bytes": (output / "program").stat().st_size}
    path = output / "receipt.json"
    path.write_text(json.dumps(receipt, indent=2) + "\n")
    print(f"Receipt: {path}\nSource tree: {before['tree']}\nSHA256: {receipt['sha256']}")
    return path


def verify(repo, path):
    with path.open("rb") as stream:
        data = stream.read(65537)
    if len(data) > 65536:
        raise ProofError("proof receipt exceeds 64 KiB")
    try:
        receipt = json.loads(data)
        if receipt["format"] != "openyr-proof-build-v1":
            raise ValueError("unknown receipt format")
        if source(repo)["tree"] != receipt["source"]["tree"]:
            raise ProofError("source tree differs from the proof build")
        # The program location is fixed, never supplied by receipt contents.
        program = path.resolve().parent / "program"
        if program.stat().st_size != receipt["bytes"] or digest(program) != receipt["sha256"]:
            raise ProofError("executable differs from the proof build")
    except (KeyError, TypeError, ValueError) as error:
        raise ProofError(f"invalid proof receipt: {error}") from error
    return program


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=Path(__file__).resolve().parent.parent)
    commands = parser.add_subparsers(dest="action", required=True)
    make = commands.add_parser("build")
    make.add_argument("--output", type=Path, required=True)
    make.add_argument("--package", default="yr-app")
    make.add_argument("--bin", default="openyr")
    for name in ("verify", "run"):
        check = commands.add_parser(name)
        check.add_argument("--receipt", type=Path, required=True)
        if name == "run":
            check.add_argument("args", nargs=argparse.REMAINDER)
    options = parser.parse_args()
    try:
        if options.action == "build":
            build(options.repo, options.output, options.package, options.bin)
        else:
            program = verify(options.repo, options.receipt)
            print(f"Verified: {program}", flush=True)
            if options.action == "run":
                args = options.args
                if args[:1] == ["--"]:
                    args = args[1:]
                return subprocess.call([str(program), *args], cwd=options.repo)
    except (ProofError, OSError, subprocess.CalledProcessError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
