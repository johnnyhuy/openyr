import base64
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import sys
import tempfile
import threading
import time
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


class ExportTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.directory = self.root / "Screenshots"
        self.directory.mkdir()
        self.image = png()
        self.encoded = base64.b64encode(self.image).decode()
        self.producers = []

    def tearDown(self):
        for producer in self.producers:
            producer.wait(timeout=3)
        self.temp.cleanup()

    def run_producer(self, code, **options):
        setup = ("import base64,subprocess,sys,time\nfrom pathlib import Path\n"
                 f"root=Path({str(self.directory)!r})\n"
                 f"image=base64.b64decode({self.encoded!r})\n")
        result = record(self.directory, self.root / "recording",
                        [sys.executable, "-c", setup + code], expected=(2, 2),
                        quiet=options.pop("quiet", .18), settle_limit=options.pop("settle_limit", .6),
                        run_limit=options.pop("run_limit", 2), poll=.005, **options)
        self.producers.append(result.producer)
        return result.receipt

    def test_late_duplicate_after_producer_exit_is_retained(self):
        original_popen = subprocess.Popen
        threads, observed_codes = [], []

        def start(*args, **kwargs):
            producer = original_popen(*args, **kwargs)

            def late_renderer():
                observed_codes.append(producer.wait())
                time.sleep(.04)
                (self.directory / "late.png").write_bytes(self.image)

            thread = threading.Thread(target=late_renderer)
            thread.start()
            threads.append(thread)
            return producer

        try:
            with patch("record_exports.subprocess.Popen", side_effect=start):
                receipt = self.run_producer(
                    "print('capture_index=0 capture_tick_before=40 after=40',flush=True)\n"
                    "(root/'first.png').write_bytes(image)\n", quiet=.5)
        finally:
            for thread in threads:
                thread.join(timeout=3)
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

    def test_partial_png_is_not_accepted_until_complete(self):
        receipt = self.run_producer(
            "print('capture_index=0 capture_tick_before=4 after=4',flush=True)\n"
            "p=root/'partial.png';p.write_bytes(image[:30]);time.sleep(.08);p.write_bytes(image)\n")
        self.assertEqual(len(receipt["exports"]), 1)
        export = receipt["exports"][0]
        self.assertGreater(export["complete_seen_ns"], export["first_seen_ns"] + 20_000_000)
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
        receipt = self.run_producer(
            "print('capture_index=0 capture_tick_before=',end='',flush=True)\n"
            "(root/'early.png').write_bytes(image);time.sleep(.04)\n"
            "print('4294967290 after=3',flush=True)\n")
        self.assertEqual(len(receipt["requests"]), 1)
        self.assertEqual(receipt["request_errors"], [])
        self.assertFalse(receipt["temporal_association_validated"])

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
        receipt = self.run_producer("time.sleep(.2)\n", run_limit=.04)
        producer = self.producers[-1]
        self.assertEqual(receipt["producer_pid"], producer.pid)
        self.assertIsNone(producer.poll())
        self.assertEqual(receipt["producer_status"], "running")
        self.assertEqual(receipt["collector_status"], "producer_deadline")
        self.assertFalse(receipt["export_accounting_complete"])

    def test_export_bound_is_explicit_and_does_not_kill_producer(self):
        receipt = self.run_producer(
            "(root/'a.png').write_bytes(image);(root/'b.png').write_bytes(image);time.sleep(.2)\n",
            limit=1)
        self.assertEqual(receipt["collector_status"], "export_limit")
        self.assertFalse(receipt["export_accounting_complete"])
        self.assertIsNone(self.producers[-1].poll())
        self.assertEqual(len(receipt["exports"]), 1)

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
        original_popen = subprocess.Popen
        threads = []

        def start(*args, **kwargs):
            producer = original_popen(*args, **kwargs)

            def renderer():
                producer.wait()
                until = time.monotonic() + .4
                index = 0
                while time.monotonic() < until:
                    (self.directory / "changing.png").write_bytes(png(color=index))
                    index += 1
                    time.sleep(.02)

            thread = threading.Thread(target=renderer)
            thread.start()
            threads.append(thread)
            return producer

        try:
            with patch("record_exports.subprocess.Popen", side_effect=start):
                receipt = self.run_producer("pass\n", quiet=.07, settle_limit=.15)
        finally:
            for thread in threads:
                thread.join(timeout=3)
        self.assertEqual(receipt["producer_exit_code"], 0)
        self.assertEqual(receipt["collector_status"], "settling_deadline")
        self.assertFalse(receipt["export_accounting_complete"])
        self.assertTrue(receipt["exports"])

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
            "--settle-limit", ".2", "--run-limit", "2", "--",
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
