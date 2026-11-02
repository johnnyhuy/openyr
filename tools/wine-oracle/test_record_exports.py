import base64
import hashlib
import json
import os
from pathlib import Path
import struct
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
import zlib

from record_exports import ExportWatch, MAX_BYTES, Requests, png_size, record


def png(width=2, height=2, color=0):
    def chunk(kind, data):
        return (struct.pack(">I", len(data)) + kind + data
                + struct.pack(">I", zlib.crc32(kind + data)))
    return (b"\x89PNG\r\n\x1a\n"
            + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress((b"\x00" + bytes([color]) * width * 3) * height))
            + chunk(b"IEND", b""))


class ControlledClock:
    """Order synthetic filesystem events without relying on host scheduling."""
    def __init__(self, events):
        self.now = 1_000_000_000
        self.events = [(self.now + round(seconds * 1e9), action)
                       for seconds, action in sorted(events, key=lambda event: event[0])]

    def monotonic_ns(self):
        return self.now

    def sleep(self, seconds):
        target = self.now + round(seconds * 1e9)
        while self.events and self.events[0][0] <= target:
            self.now, action = self.events.pop(0)
            action()
        self.now = target


class ControlledProducer:
    pid = 12345

    def __init__(self, clock, exit_at):
        self.clock, self.exit_at = clock, clock.now + round(exit_at * 1e9)

    def poll(self):
        return 0 if self.clock.now >= self.exit_at else None


class ExportTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.directory = self.root / "Screenshots"
        self.directory.mkdir()
        self.image = png()
        self.encoded = base64.b64encode(self.image).decode()
        self.producers = []
        self.release = self.root / "release-producer"

    def tearDown(self):
        self.release.touch()
        for producer in self.producers:
            producer.wait(timeout=10)
        self.temp.cleanup()

    def run_producer(self, code, **options):
        setup = ("import base64,subprocess,sys,time\nfrom pathlib import Path\n"
                 f"root=Path({str(self.directory)!r})\n"
                 f"image=base64.b64decode({self.encoded!r})\n")
        result = record(self.directory, self.root / "recording",
                        [sys.executable, "-c", setup + code], expected=(2, 2),
                        quiet=options.pop("quiet", .18), settle_limit=options.pop("settle_limit", 5),
                        run_limit=options.pop("run_limit", 10), poll=.005, **options)
        self.producers.append(result.producer)
        return result.receipt

    def run_controlled(self, events, *, exit_at=.01, validation_seconds=0, **options):
        clock = ControlledClock(events)

        def validate(data, expected):
            clock.sleep(validation_seconds)
            return png_size(data, expected)

        def start(*args, **kwargs):
            kwargs["stdout"].write(b"capture_index=0 capture_tick_before=40 after=40\n")
            kwargs["stdout"].flush()
            producer = ControlledProducer(clock, exit_at)
            clock.sleep(0)
            return producer

        with patch("record_exports.subprocess.Popen", side_effect=start), \
                patch("record_exports.png_size", side_effect=validate):
            return record(self.directory, self.root / "recording", ["synthetic-producer"],
                          expected=(2, 2), quiet=options.pop("quiet", .07),
                          settle_limit=options.pop("settle_limit", .15),
                          run_limit=1, poll=.005, clock=clock, **options).receipt

    def test_real_producer_export_after_exit_is_retained(self):
        original_popen = subprocess.Popen
        observed_codes = []

        def start(*args, **kwargs):
            producer = original_popen(*args, **kwargs)
            observed_codes.append(producer.wait(timeout=10))
            (self.directory / "late.png").write_bytes(self.image)
            return producer

        with patch("record_exports.subprocess.Popen", side_effect=start):
            receipt = self.run_producer(
                "print('capture_index=0 capture_tick_before=40 after=40',flush=True)\n"
                "(root/'first.png').write_bytes(image)\n")
        self.assertEqual(observed_codes, [0])
        self.assertEqual(receipt["producer_exit_code"], 0)
        self.assertEqual(len(receipt["exports"]), 2)
        self.assertTrue(receipt["export_accounting_complete"])
        self.assertEqual({e["sha256"] for e in receipt["exports"]},
                         {hashlib.sha256(self.image).hexdigest()})
        for export in receipt["exports"]:
            self.assertEqual((self.root / "recording" / export["file"]).read_bytes(), self.image)
            self.assertEqual(export["association"], "unresolved")
        self.assertFalse(receipt["temporal_association_validated"])

    def test_late_duplicate_after_observed_producer_exit_is_retained(self):
        receipt = self.run_controlled([
            (0, lambda: (self.directory / "first.png").write_bytes(self.image)),
            (.05, lambda: (self.directory / "late.png").write_bytes(self.image)),
        ])
        self.assertEqual(receipt["producer_exit_seen_ns"], 1_010_000_000)
        self.assertEqual(len(receipt["exports"]), 2)
        self.assertEqual(receipt["exports"][1]["complete_seen_ns"], 1_050_000_000)
        self.assertEqual(receipt["observed_until_ns"], 1_120_000_000)
        self.assertEqual(receipt["collector_status"], "settled")
        self.assertTrue(receipt["export_accounting_complete"])

    def test_validation_cost_can_exhaust_settling_budget_with_both_exports_retained(self):
        receipt = self.run_controlled([
            (0, lambda: (self.directory / "first.png").write_bytes(self.image)),
            (.12, lambda: (self.directory / "late.png").write_bytes(self.image)),
        ], validation_seconds=.1)
        self.assertEqual(len(receipt["exports"]), 2)
        self.assertEqual(receipt["producer_exit_seen_ns"], 1_100_000_000)
        self.assertEqual(receipt["observed_until_ns"], 1_250_000_000)
        self.assertEqual(receipt["collector_status"], "settling_deadline")
        self.assertFalse(receipt["export_accounting_complete"])
        self.assertEqual(receipt["pending_exports"], [])
        self.assertEqual(receipt["problems"], [])

    def test_partial_png_is_not_accepted_until_complete(self):
        path = self.directory / "partial.png"
        receipt = self.run_controlled([
            (0, lambda: path.write_bytes(self.image[:30])),
            (.04, lambda: path.write_bytes(self.image)),
        ], exit_at=.05)
        self.assertEqual(len(receipt["exports"]), 1)
        export = receipt["exports"][0]
        self.assertEqual(export["first_seen_ns"], 1_000_000_000)
        self.assertEqual(export["complete_seen_ns"], 1_040_000_000)
        self.assertTrue(receipt["export_accounting_complete"])

    def test_unfinished_or_corrupt_export_is_explicitly_incomplete(self):
        receipt = self.run_producer("(root/'broken.png').write_bytes(image[:-4])\n")
        self.assertFalse(receipt["export_accounting_complete"])
        self.assertEqual(len(receipt["pending_exports"]), 1)
        self.assertEqual(receipt["exports"], [])

    def test_existing_files_are_ignored_but_overwritten_revision_is_kept(self):
        (self.directory / "old.png").write_bytes(self.image)
        receipt = self.run_producer(
            "print('capture_index=0 capture_tick_before=1 after=1',flush=True)\n"
            "(root/'old.png').write_bytes(image)\n")
        self.assertEqual([e["source_name"] for e in receipt["exports"]], ["old.png"])
        self.assertTrue(receipt["export_accounting_complete"])

    def test_two_request_events_do_not_imply_unique_export_ownership(self):
        receipt = self.run_producer(
            "print('capture_index=0 capture_tick_before=1 after=1',flush=True)\n"
            "print('capture_index=1 capture_tick_before=2 after=2',flush=True)\n"
            "(root/'late.png').write_bytes(image)\n")
        self.assertEqual(receipt["exports"][0]["request_candidates"], [0, 1])
        self.assertEqual(receipt["exports"][0]["association"], "unresolved")

    def test_split_metadata_line_is_reassembled_without_guessing(self):
        path = self.root / "stdout"
        path.write_bytes(b"capture_index=0 capture_tick_before=")
        clock = ControlledClock([])
        requests = Requests(clock)
        requests.scan(path, clock.now)
        self.assertEqual(requests.events, [])
        self.assertEqual(requests.errors, [])
        with path.open("ab") as output:
            output.write(b"4294967290 after=3\n")
        clock.sleep(.04)
        requests.scan(path, clock.now)
        self.assertEqual(requests.events, [dict(index=0, tick_before=4294967290,
                                              tick_after=3, host_seen_ns=1_040_000_000)])
        self.assertEqual(requests.errors, [])

    def test_malformed_request_log_is_retained_as_error(self):
        receipt = self.run_producer(
            "print('capture_index=99 capture_tick_before=0 after=0',flush=True)\n"
            "(root/'frame.png').write_bytes(image)\n")
        self.assertEqual(len(receipt["exports"]), 1)
        self.assertTrue(receipt["request_errors"])
        self.assertEqual(receipt["requests"], [])

    def test_failed_producer_still_has_its_exports_observed(self):
        receipt = self.run_producer("(root/'frame.png').write_bytes(image);sys.exit(4)\n")
        self.assertEqual(receipt["producer_exit_code"], 4)
        self.assertEqual(len(receipt["exports"]), 1)
        self.assertTrue(receipt["export_accounting_complete"])

    def test_deadline_reports_the_same_producer_still_running(self):
        receipt = self.run_producer(self.wait_for_release(), run_limit=.04)
        producer = self.producers[-1]
        self.assertEqual(receipt["producer_pid"], producer.pid)
        self.assertIsNone(producer.poll())
        self.assertEqual(receipt["producer_status"], "running")
        self.assertEqual(receipt["collector_status"], "producer_deadline")
        self.assertFalse(receipt["export_accounting_complete"])

    def test_export_bound_is_explicit_and_does_not_kill_producer(self):
        receipt = self.run_producer(
            "(root/'a.png').write_bytes(image);(root/'b.png').write_bytes(image)\n"
            + self.wait_for_release(),
            limit=1)
        self.assertEqual(receipt["collector_status"], "export_limit")
        self.assertFalse(receipt["export_accounting_complete"])
        self.assertIsNone(self.producers[-1].poll())
        self.assertEqual(len(receipt["exports"]), 1)

    def wait_for_release(self):
        return (f"release=Path({str(self.release)!r})\n"
                "deadline=time.monotonic()+20\n"
                "while not release.exists():\n"
                "    if time.monotonic() >= deadline: raise SystemExit(9)\n"
                "    time.sleep(.01)\n")

    def test_new_png_revisions_are_preserved(self):
        out = self.root / "snapshots"
        out.mkdir()
        watch = ExportWatch(self.directory, out, (2, 2))
        p = self.directory / "frame.png"
        p.write_bytes(self.image)
        watch.scan(100)
        replacement = png(color=50)
        p.write_bytes(replacement)
        watch.scan(200)
        self.assertEqual(len(watch.exports), 2)
        self.assertNotEqual(watch.exports[0]["sha256"], watch.exports[1]["sha256"])

    def test_continuing_exports_reach_a_bounded_settling_deadline(self):
        path = self.directory / "changing.png"

        def write_revision(index):
            path.write_bytes(png(color=index))
            os.utime(path, ns=(1_000_000_000 + index, 1_000_000_000 + index))

        receipt = self.run_controlled([
            (index * .02, lambda revision=index: write_revision(revision))
            for index in range(12)
        ])
        self.assertEqual(receipt["producer_exit_code"], 0)
        self.assertEqual(receipt["collector_status"], "settling_deadline")
        self.assertFalse(receipt["export_accounting_complete"])
        self.assertEqual(len(receipt["exports"]), 9)
        self.assertEqual(receipt["observed_until_ns"], 1_160_000_000)

    def test_hostile_png_headers_and_crc_are_rejected(self):
        for bad in [self.image[:-1], self.image + b"extra",
                    self.image[:8] + struct.pack(">I", 0xffffffff) + b"IHDR",
                    self.image[:30] + bytes([self.image[30] ^ 1]) + self.image[31:],
                    png(3, 2), b"x" * (MAX_BYTES + 1)]:
            with self.subTest(size=len(bad)), self.assertRaises(ValueError):
                png_size(bad, (2, 2))
        self.assertEqual(png_size(self.image, (2, 2)), (2, 2))

    def test_oversized_request_numbers_are_rejected_without_integer_allocation(self):
        path = self.root / "stdout"
        path.write_text("capture_index=0 capture_tick_before=" + "9" * 10000 + " after=0\n")
        requests = Requests()
        requests.scan(path, 0)
        self.assertEqual(requests.events, [])
        self.assertTrue(requests.errors)

    def cli(self, producer_code):
        return subprocess.run([
            sys.executable, str(Path(__file__).with_name("record_exports.py")),
            "--directory", str(self.directory), "--out", str(self.root / "cli"),
            "--width", "2", "--height", "2", "--quiet-seconds", ".04",
            "--settle-limit", "5", "--run-limit", "10", "--",
            sys.executable, "-c", producer_code,
        ], capture_output=True, text=True)

    def test_cli_does_not_succeed_when_requests_produce_no_exports(self):
        result = self.cli("print('capture_index=0 capture_tick_before=1 after=1',flush=True)")
        self.assertEqual(result.returncode, 1, result.stderr)
        summary = json.loads(result.stdout)
        self.assertEqual((summary["exports"], summary["request_events"]), (0, 1))
        self.assertFalse(summary["temporal_association_validated"])

    def test_cli_preserves_nonzero_producer_status_and_its_export(self):
        code = ("from pathlib import Path;"
                f"Path({str(self.directory / 'frame.png')!r}).write_bytes({self.image!r});"
                "raise SystemExit(4)")
        result = self.cli(code)
        self.assertEqual(result.returncode, 4, result.stderr)
        summary = json.loads(result.stdout)
        self.assertEqual((summary["exports"], summary["producer_exit_code"]), (1, 4))

    def test_nonfinite_time_bounds_are_rejected_before_starting_a_producer(self):
        for bound in [float("nan"), float("inf")]:
            with self.subTest(bound=bound), self.assertRaises(ValueError):
                record(self.directory, self.root / "invalid", ["never-started"], quiet=bound)
        self.assertFalse((self.root / "invalid").exists())


if __name__ == "__main__":
    unittest.main()
