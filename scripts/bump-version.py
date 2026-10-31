#!/usr/bin/env python3
"""Update only the root package version in Cargo.toml and Cargo.lock."""
import argparse
import os
from pathlib import Path
import re
import tomllib

VERSION = re.compile(r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(?:-(alpha|beta|rc)\.(0|[1-9][0-9]*))?")


def next_version(current, bump, channel):
    match = VERSION.fullmatch(current)
    if not match:
        raise ValueError("Expected X.Y.Z or X.Y.Z-alpha.N/beta.N/rc.N")
    if channel not in ("stable", "alpha", "beta", "rc"):
        raise ValueError("Unknown release channel")
    major, minor, patch = map(int, match.group(1, 2, 3))
    old_channel, counter = match.group(4, 5)
    if bump == "promote":
        if not old_channel or channel != "stable":
            raise ValueError("Promote requires a prerelease version and the stable channel")
        return f"{major}.{minor}.{patch}"
    if bump == "prerelease":
        if channel == "stable":
            raise ValueError("Prerelease requires alpha, beta, or rc")
        if old_channel:
            if ("alpha", "beta", "rc").index(channel) < ("alpha", "beta", "rc").index(old_channel):
                raise ValueError("Cannot move a prerelease back to an earlier channel")
            number = int(counter) + 1 if channel == old_channel else 1
        else:
            patch += 1
            number = 1
    else:
        if bump == "major":
            major, minor, patch = major + 1, 0, 0
        elif bump == "minor":
            minor, patch = minor + 1, 0
        elif bump == "patch":
            patch += 1
        else:
            raise ValueError("Unknown version bump")
        number = 1
    base = f"{major}.{minor}.{patch}"
    return base if channel == "stable" else f"{base}-{channel}.{number}"


def replace_version(section, old, new):
    pattern = re.compile(r'(?m)^(version\s*=\s*")' + re.escape(old) + r'(".*)$')
    updated, count = pattern.subn(lambda m: m[1] + new + m[2], section)
    if count != 1:
        raise ValueError("Expected exactly one package version")
    return updated


def update(root, bump, channel):
    manifest_path, lock_path = root / "Cargo.toml", root / "Cargo.lock"
    manifest, lock = manifest_path.read_text(), lock_path.read_text()
    package = tomllib.loads(manifest)["package"]
    old, name = package["version"], package["name"]
    new = next_version(old, bump, channel)
    entries = [p for p in tomllib.loads(lock)["package"]
               if p["name"] == name and "source" not in p]
    if len(entries) != 1 or entries[0]["version"] != old:
        raise ValueError("Root package version in Cargo.lock does not match Cargo.toml")
    # Preserve every unrelated byte, including dependency versions and checksums.
    sections = re.split(r'(?m)(?=^\[)', manifest)
    sections = [replace_version(s, old, new) if s.startswith("[package]\n") else s
                for s in sections]
    lock_sections = re.split(r'(?m)(?=^\[\[package\]\])', lock)
    for i, section in enumerate(lock_sections):
        if section.startswith("[[package]]"):
            entry = tomllib.loads(section)["package"][0]
            if entry["name"] == name and "source" not in entry:
                lock_sections[i] = replace_version(section, old, new)
    manifest_path.write_text("".join(sections))
    lock_path.write_text("".join(lock_sections))
    return new


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("bump", choices=["patch", "minor", "major", "prerelease", "promote"])
    parser.add_argument("channel", choices=["stable", "alpha", "beta", "rc"])
    args = parser.parse_args()
    version = update(Path.cwd(), args.bump, args.channel)
    print(version)
    if os.environ.get("GITHUB_OUTPUT"):
        with open(os.environ["GITHUB_OUTPUT"], "a") as output:
            output.write(f"version={version}\ntag=v{version}\n")
