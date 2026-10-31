import importlib.util
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("bump", Path(__file__).with_name("bump-version.py"))
bump = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bump)


class VersionTests(unittest.TestCase):
    def test_release_transitions(self):
        cases = [
            ("0.1.0", "patch", "stable", "0.1.1"),
            ("0.1.2", "minor", "stable", "0.2.0"),
            ("0.1.2", "major", "stable", "1.0.0"),
            ("0.1.0", "minor", "alpha", "0.2.0-alpha.1"),
            ("0.2.0-alpha.1", "prerelease", "alpha", "0.2.0-alpha.2"),
            ("0.2.0-alpha.2", "prerelease", "beta", "0.2.0-beta.1"),
            ("0.2.0-beta.2", "prerelease", "rc", "0.2.0-rc.1"),
            ("0.1.0", "prerelease", "rc", "0.1.1-rc.1"),
            ("0.2.0-rc.1", "promote", "stable", "0.2.0"),
        ]
        for current, kind, channel, expected in cases:
            with self.subTest(current=current, kind=kind, channel=channel):
                self.assertEqual(bump.next_version(current, kind, channel), expected)

    def test_invalid_transitions(self):
        for args in [
            ("01.2.3", "patch", "stable"), ("1.2.3", "promote", "stable"),
            ("1.2.3", "prerelease", "stable"), ("1.2.3-rc.1", "promote", "rc"),
            ("1.2.3-rc.1", "prerelease", "alpha"), ("1.2.3", "patch", "bad"),
        ]:
            with self.subTest(args=args), self.assertRaises(ValueError):
                bump.next_version(*args)

    def test_updates_only_root_package(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest = '[package]\nname = "nes-rs"\nversion = "0.1.0"\n\n[dependencies]\nother = "0.1.0"\n'
            lock = 'version = 4\n\n[[package]]\nname = "other"\nversion = "0.1.0"\nsource = "registry+example"\n\n[[package]]\nname = "nes-rs"\nversion = "0.1.0"\n'
            (root / "Cargo.toml").write_text(manifest)
            (root / "Cargo.lock").write_text(lock)
            self.assertEqual(bump.update(root, "minor", "beta"), "0.2.0-beta.1")
            self.assertEqual((root / "Cargo.toml").read_text(), manifest.replace('version = "0.1.0"', 'version = "0.2.0-beta.1"'))
            self.assertEqual((root / "Cargo.lock").read_text(), lock.replace('name = "nes-rs"\nversion = "0.1.0"', 'name = "nes-rs"\nversion = "0.2.0-beta.1"'))
            before = (root / "Cargo.toml").read_text()
            (root / "Cargo.lock").write_text(lock)
            with self.assertRaises(ValueError):
                bump.update(root, "patch", "stable")
            self.assertEqual((root / "Cargo.toml").read_text(), before)

class ReleaseInfoTests(unittest.TestCase):
    def test_tag_matches_code_and_prerelease_is_explicit(self):
        import runpy
        classify = runpy.run_path(str(Path(__file__).with_name("release-info.py")))["release_info"]
        self.assertEqual(classify("v1.2.3", "1.2.3"), "false")
        self.assertEqual(classify("v1.2.3-rc.1", "1.2.3-rc.1"), "true")
        for tag, version in [("v1.2.4", "1.2.3"), ("1.2.3", "1.2.3"),
                             ("v01.2.3", "01.2.3"), ("v1.2.3-rc", "1.2.3-rc")]:
            with self.subTest(tag=tag), self.assertRaises(ValueError):
                classify(tag, version)


if __name__ == "__main__":
    unittest.main()
