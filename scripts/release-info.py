#!/usr/bin/env python3
"""Validate the tag against Cargo.toml and classify a stable/prerelease build."""
import os
from pathlib import Path
import runpy
import tomllib

VERSION = runpy.run_path(str(Path(__file__).with_name("bump-version.py")))["VERSION"]


def release_info(tag, version):
    match = VERSION.fullmatch(version)
    if not match or tag != "v" + version:
        raise ValueError("Release tag must equal v + the Cargo.toml package version")
    return "true" if match.group(4) else "false"


if __name__ == "__main__":
    version = tomllib.loads(Path("Cargo.toml").read_text())["package"]["version"]
    prerelease = release_info(os.environ["GITHUB_REF_NAME"], version)
    with open(os.environ["GITHUB_OUTPUT"], "a") as output:
        output.write(f"prerelease={prerelease}\n")
