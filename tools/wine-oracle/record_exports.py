#!/usr/bin/env python3
"""Retain renderer PNG exports across producer exit and a bounded settling window."""
import argparse
from dataclasses import dataclass
import datetime
import hashlib
import json
import math
from pathlib import Path
import re
import stat
import struct
import subprocess
import sys
import time
import zlib

MAX_BYTES = 8 * 1024 * 1024
MAX_LOG_BYTES = 1024 * 1024
REQUEST = re.compile(r"capture_index=([0-9]{1,3}) capture_tick_before=([0-9]{1,10}) after=([0-9]{1,10})")


def png_size(data, expected):
    """Check bounded PNG framing/CRCs, not decompressed pixels or presentation time."""
    if len(data) > MAX_BYTES:
        raise ValueError("PNG exceeds byte limit")
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError("incomplete or invalid PNG signature")
    at, dimensions, image_data = 8, None, False
    while at < len(data):
        if len(data) - at < 12:
            raise ValueError("incomplete PNG chunk")
        length = struct.unpack_from(">I", data, at)[0]
        end = at + 12 + length
        if end > len(data):
            raise ValueError("incomplete PNG payload")
        kind = data[at + 4:at + 8]
        payload = data[at + 8:at + 8 + length]
        crc = struct.unpack_from(">I", data, end - 4)[0]
        if zlib.crc32(kind + payload) != crc:
            raise ValueError("PNG chunk CRC mismatch")
        if dimensions is None:
            if kind != b"IHDR" or length != 13:
                raise ValueError("PNG must start with IHDR")
            dimensions = struct.unpack_from(">II", payload)
            if dimensions != expected:
                raise ValueError("unexpected PNG dimensions")
        elif kind == b"IHDR":
            raise ValueError("duplicate PNG header")
        if kind == b"IDAT":
            image_data = True
        if kind == b"IEND":
            if length != 0 or end != len(data) or not image_data:
                raise ValueError("invalid PNG ending")
            return dimensions
        at = end
    raise ValueError("incomplete PNG ending")


def signature(info):
    return (info.st_dev, info.st_ino, info.st_size, info.st_mtime_ns)


class ExportWatch:
    def __init__(self, directory, out, expected=(800, 600), limit=128, clock=time):
        self.directory, self.out = Path(directory), Path(out)
        self.expected, self.limit = expected, limit
        self.clock = clock
        self.baseline = {p.name: signature(p.lstat()) for p in self.paths()}
        self.states, self.exports, self.problems = {}, [], []
        self.last_activity_ns = 0
        self.limit_hit = False
        (self.out / "exports").mkdir()

    def paths(self):
        return sorted(p for p in self.directory.iterdir() if p.suffix.lower() == ".png")

    def scan(self, now):
        present = set()
        for path in self.paths():
            present.add(path.name)
            try:
                info = path.lstat()
            except FileNotFoundError:
                continue
            key = signature(info)
            previous = self.states.get(path.name)
            if self.baseline.get(path.name) == key and previous is None:
                continue
            if previous is not None and previous["signature"] == key:
                continue
            first = previous["first_seen_ns"] if previous and previous["pending"] else now
            state = dict(signature=key, first_seen_ns=first, pending=True, reason="pending")
            self.states[path.name] = state
            self.last_activity_ns = now
            try:
                if not stat.S_ISREG(info.st_mode):
                    raise ValueError("source is not a regular file")
                if info.st_size > MAX_BYTES:
                    raise ValueError("PNG exceeds byte limit")
                with path.open("rb") as source:
                    data = source.read(MAX_BYTES + 1)
                if signature(path.lstat()) != key:
                    raise ValueError("source changed during read")
                dimensions = png_size(data, self.expected)
                digest = hashlib.sha256(data).hexdigest()
                record = dict(source_name=path.name, source_size=len(data),
                              source_mtime_ns=info.st_mtime_ns, first_seen_ns=first,
                              complete_seen_ns=self.clock.monotonic_ns(), sha256=digest,
                              width=dimensions[0], height=dimensions[1],
                              source_revision=len(self.exports), stored=False,
                              association="unresolved")
                if len(self.exports) < self.limit:
                    dest = self.out / "exports" / f"{len(self.exports):04}.png"
                    with dest.open("xb") as output:
                        output.write(data)
                    record.update(stored=True, file=str(dest.relative_to(self.out)))
                else:
                    self.problems.append(dict(source_name=path.name, reason="export limit reached"))
                    state["reason"] = "export limit reached"
                    self.limit_hit = True
                    return
                self.last_activity_ns = record["complete_seen_ns"]
                self.exports.append(record)
                state.update(pending=False, reason=None)
            except (OSError, ValueError) as error:
                state["reason"] = str(error)
        for name, state in self.states.items():
            if name not in present and state["pending"]:
                state["reason"] = "source disappeared before a complete snapshot"

    def pending(self):
        return [dict(source_name=name, signature=state["signature"],
                     first_seen_ns=state["first_seen_ns"], reason=state["reason"])
                for name, state in sorted(self.states.items()) if state["pending"]]


class Requests:
    def __init__(self, clock=time):
        self.offset, self.tail = 0, b""
        self.events, self.errors = [], []
        self.clock = clock

    def error(self, message):
        if message not in self.errors:
            self.errors.append(message)

    def scan(self, path, now):
        with path.open("rb") as source:
            source.seek(self.offset)
            data = source.read(MAX_LOG_BYTES - self.offset + 1)
        self.offset += len(data)
        if self.offset > MAX_LOG_BYTES:
            if "producer stdout exceeds metadata limit" not in self.errors:
                self.error("producer stdout exceeds metadata limit")
            self.offset = MAX_LOG_BYTES + 1
            return
        lines = (self.tail + data).split(b"\n")
        self.tail = lines.pop()
        if len(self.tail) > 4096:
            self.error("producer metadata line exceeds limit")
            self.tail = b""
        for line in lines:
            text = line.decode("utf-8", errors="replace").strip()
            if not text.startswith("capture_index="):
                continue
            match = REQUEST.fullmatch(text)
            if match is None:
                self.error("malformed capture request metadata")
                continue
            index, before, after = map(int, match.groups())
            if (index != len(self.events) or index >= 512 or before > 0xffffffff
                    or after > 0xffffffff or ((after - before) & 0xffffffff) > 60000):
                self.error("invalid capture request index or ticks")
                continue
            if self.events and ((before - self.events[-1]["tick_before"]) & 0xffffffff) > 0x7fffffff:
                self.error("capture request ticks moved backward")
                continue
            self.events.append(dict(index=index, tick_before=before, tick_after=after,
                                    host_seen_ns=self.clock.monotonic_ns()))


@dataclass
class Recording:
    receipt: dict
    producer: subprocess.Popen


def record(directory, out, command, *, expected=(800, 600), quiet=2.0,
           settle_limit=10.0, run_limit=120.0, poll=0.02, limit=128, clock=time):
    directory, out = Path(directory).resolve(), Path(out).resolve()
    if (not directory.is_dir() or out == directory or directory in out.parents
            or out in directory.parents):
        raise ValueError("use an existing source and a separate new output directory")
    if (not all(math.isfinite(n) for n in [quiet, settle_limit, run_limit, poll])
            or quiet <= 0 or settle_limit < quiet or run_limit <= 0 or poll <= 0
            or limit < 1 or limit > 512 or min(expected) < 1
            or max(expected) > 8192 or expected[0] * expected[1] > 16_777_216):
        raise ValueError("invalid observer bounds")
    out.mkdir(parents=True, exist_ok=False)
    watch = ExportWatch(directory, out, expected, limit, clock)
    requests = Requests(clock)
    started = clock.monotonic_ns()
    utc = datetime.datetime.now(datetime.timezone.utc).isoformat()
    exit_seen, status, last_checkpoint = None, "running", 0
    stdout_path, stderr_path = out / "producer.stdout", out / "producer.stderr"
    with stdout_path.open("wb") as stdout, stderr_path.open("wb") as stderr:
        producer = subprocess.Popen(command, stdout=stdout, stderr=stderr)
        while True:
            now = clock.monotonic_ns()
            requests.scan(stdout_path, now)
            watch.scan(now)
            now = clock.monotonic_ns()
            code = producer.poll()
            if code is not None and exit_seen is None:
                exit_seen = now
            if exit_seen is not None:
                if now - max(exit_seen, watch.last_activity_ns) >= quiet * 1e9:
                    status = "settled"
                elif now - exit_seen >= settle_limit * 1e9:
                    status = "settling_deadline"
            elif now - started >= run_limit * 1e9:
                status = "producer_deadline"
            if watch.limit_hit:
                status = "export_limit"
            if status != "running" and requests.tail.startswith(b"capture_index="):
                requests.error("incomplete capture request metadata line")
            pending = watch.pending()
            receipt = dict(command=command, producer_pid=producer.pid,
                           producer_status="exited" if code is not None else "running",
                           producer_exit_code=code, producer_exit_seen_ns=exit_seen,
                           started_utc=utc, started_ns=started, observed_until_ns=now,
                           collector_status=status, quiet_seconds=quiet, poll_seconds=poll,
                           settling_limit_seconds=settle_limit, run_limit_seconds=run_limit,
                           expected_size=list(expected), exports=watch.exports,
                           pending_exports=pending, problems=watch.problems,
                           requests=requests.events, request_errors=requests.errors,
                           temporal_association_validated=False,
                           export_accounting_complete=(status == "settled" and not pending
                                                       and not watch.problems))
            # Poll ordering is not evidence of which input caused a PNG.
            for export in watch.exports:
                export["request_candidates"] = [event["index"] for event in requests.events]
            if status != "running" or now - last_checkpoint >= 1_000_000_000:
                (out / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
                last_checkpoint = now
            if status != "running":
                break
            clock.sleep(poll)
    return Recording(receipt, producer)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--directory", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--quiet-seconds", type=float, default=2.0)
    parser.add_argument("--settle-limit", type=float, default=10.0)
    parser.add_argument("--run-limit", type=float, default=120.0)
    parser.add_argument("--width", type=int, default=800)
    parser.add_argument("--height", type=int, default=600)
    parser.add_argument("--max-exports", type=int, default=128)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    if not command:
        parser.error("a producer command is required after --")
    result = record(args.directory, args.out, command, expected=(args.width, args.height),
                    quiet=args.quiet_seconds, settle_limit=args.settle_limit,
                    run_limit=args.run_limit, limit=args.max_exports)
    r = result.receipt
    print(json.dumps(dict(receipt=str(args.out / "receipt.json"),
                          producer_pid=r["producer_pid"], producer_status=r["producer_status"],
                          producer_exit_code=r["producer_exit_code"],
                          collector_status=r["collector_status"], exports=len(r["exports"]),
                          pending=len(r["pending_exports"]),
                          request_events=len(r["requests"]), request_errors=r["request_errors"],
                          export_accounting_complete=r["export_accounting_complete"],
                          temporal_association_validated=False)), flush=True)
    if r["producer_status"] == "running" or r["collector_status"] != "settled":
        return 7
    if r["producer_exit_code"]:
        return r["producer_exit_code"] if 0 < r["producer_exit_code"] < 256 else 1
    return 0 if (r["export_accounting_complete"] and r["exports"] and r["requests"]
                 and not r["request_errors"]) else 1


if __name__ == "__main__":
    sys.exit(main())
