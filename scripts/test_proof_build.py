"""Real Cargo/worktree checks without game data or external Rust dependencies."""

import contextlib
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

import proof_build as proof


class ProofBuildTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="openyr-proof-test-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.a, self.b = self.root / "a", self.root / "b"
        self.a.mkdir()
        (self.a / "src").mkdir()
        self.git(self.a, "init", "-q")
        self.git(self.a, "config", "user.email", "fixture@example.invalid")
        self.git(self.a, "config", "user.name", "Proof fixture")
        self.git(self.a, "config", "commit.gpgsign", "false")
        (self.a / "Cargo.toml").write_text(
            '[package]\nname="proof-fixture"\nversion="0.1.0"\nedition="2021"\n'
        )
        (self.a / "src/main.rs").write_text('fn main() { println!("PROOF_A"); }\n')
        subprocess.run([proof.cargo_binary(), "generate-lockfile", "--offline"],
                       cwd=self.a, check=True, capture_output=True)
        self.git(self.a, "add", ".")
        self.git(self.a, "commit", "-qm", "A")
        self.git(self.a, "worktree", "add", "-qb", "fixture-b", str(self.b))
        (self.b / "src/main.rs").write_text('fn main() { println!("PROOF_B"); }\n')
        self.git(self.b, "commit", "-qam", "B")

    @staticmethod
    def git(repo, *args):
        return subprocess.check_output(["git", "-C", str(repo), *args],
                                       stderr=subprocess.STDOUT, text=True).strip()

    def build(self, repo, name):
        with contextlib.redirect_stdout(io.StringIO()):
            return proof.build(repo, self.root / name, "proof-fixture", "proof-fixture")

    def test_shared_target_probe_and_isolated_builds(self):
        # Both worktrees exist before the first build, as in the cloud failure.
        shared = self.root / "shared"
        observations = []
        for repo, expected in [(self.a, "PROOF_A"), (self.b, "PROOF_B"), (self.a, "PROOF_A")]:
            result = subprocess.run(
                [proof.cargo_binary(), "build", "--release", "--locked", "--offline",
                 "--target-dir", str(shared), "--message-format=json"],
                cwd=repo, check=True, text=True, capture_output=True,
            )
            artifact = next(m for m in map(json.loads, result.stdout.splitlines())
                            if m.get("reason") == "compiler-artifact" and m.get("executable"))
            actual = subprocess.check_output([artifact["executable"]], text=True).strip()
            observations.append({"source": repo.name, "expected": expected,
                                 "actual": actual, "fresh": artifact["fresh"],
                                 "stale": actual != expected})
        # Cargo versions/platforms that already fix reuse must also pass.
        print("Shared-target observations: " + json.dumps(observations), flush=True)
        with patch.dict(os.environ, {"CARGO_TARGET_DIR": str(shared)}):
            for repo, expected, name in [(self.a, "PROOF_A", "isolated-a"),
                                         (self.b, "PROOF_B", "isolated-b")]:
                receipt = self.build(repo, name)
                program = proof.verify(repo, receipt)
                self.assertEqual(subprocess.check_output([str(program)], text=True).strip(), expected)
                self.assertFalse(list(receipt.parent.glob("target-*")))
                self.assertEqual(json.loads(receipt.read_text())["source"]["tree"],
                                 proof.git(repo, "rev-parse", "HEAD^{tree}"))

    def test_stale_source_binary_dirty_source_and_reused_output_are_rejected(self):
        receipt_a = self.build(self.a, "proof-a")
        receipt_b = self.build(self.b, "proof-b")
        original = (receipt_a.parent / "program").read_bytes()
        self.git(self.a, "commit", "--allow-empty", "-qm", "same source tree")
        self.assertEqual(proof.verify(self.a, receipt_a), receipt_a.parent / "program")
        with self.assertRaisesRegex(proof.ProofError, "source tree differs"):
            proof.verify(self.a, receipt_b)
        shutil.copyfile(receipt_b.parent / "program", receipt_a.parent / "program")
        with self.assertRaisesRegex(proof.ProofError, "executable differs"):
            proof.verify(self.a, receipt_a)
        cli = subprocess.run(
            [sys.executable, str(Path(proof.__file__)), "--repo", str(self.a), "run",
             "--receipt", str(receipt_a), "--"], text=True, capture_output=True,
        )
        self.assertEqual(cli.returncode, 1)
        self.assertNotIn("PROOF_B\n", cli.stdout)
        self.assertIn("executable differs", cli.stderr)
        (receipt_a.parent / "program").write_bytes(original)
        (self.a / "uncommitted.rs").write_text("// new source\n")
        with self.assertRaisesRegex(proof.ProofError, "commit source changes"):
            proof.verify(self.a, receipt_a)
        (self.a / "uncommitted.rs").unlink()
        with self.assertRaises(FileExistsError):
            self.build(self.a, "proof-a")
        self.assertEqual((receipt_a.parent / "program").read_bytes(), original)

    def test_failed_build_and_invalid_receipts_issue_no_usable_proof(self):
        (self.a / "src/main.rs").write_text("invalid Rust")
        self.git(self.a, "commit", "-qam", "invalid fixture")
        with self.assertRaisesRegex(proof.ProofError, "Cargo failed"):
            self.build(self.a, "failed")
        self.assertFalse((self.root / "failed/receipt.json").exists())
        path = self.root / "bad-receipt.json"
        path.write_bytes(b"x" * 65537)
        with self.assertRaisesRegex(proof.ProofError, "64 KiB"):
            proof.verify(self.a, path)
        for value in ["null", "{}", '{"format":"wrong"}']:
            path.write_text(value)
            with self.assertRaisesRegex(proof.ProofError, "invalid proof receipt"):
                proof.verify(self.a, path)


if __name__ == "__main__":
    unittest.main()
