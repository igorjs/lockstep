#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail
# Release metadata rules, read from `cargo metadata` so that workspace inheritance is resolved.
#  - lockstep-headless, lockstep-fuzz and every example set `publish = false`.
#  - every other crate is publishable, carries keywords, categories and a rust-version, and gives
#    each path dependency (dev-dependencies included) a `version`, which cargo publish needs.
cd "$(dirname "$0")/.."
cargo metadata --format-version 1 --no-deps | python3 -I -c '
import json
import sys

metadata = json.load(sys.stdin)
root = metadata["workspace_root"] + "/"
unpublished_crates = {"lockstep-headless", "lockstep-fuzz"}
problems = []
for package in metadata["packages"]:
    name = package["name"]
    directory = package["manifest_path"][len(root):].rsplit("/", 1)[0]
    # An empty `publish` list is `publish = false`.
    unpublished = package["publish"] == []
    if directory.startswith("examples/") or name in unpublished_crates:
        if not unpublished:
            problems.append(name + ": set publish = false")
        continue
    if unpublished:
        problems.append(name + ": a library crate must not set publish = false")
    for field in ("keywords", "categories"):
        if not package[field]:
            problems.append(name + ": missing " + field)
    if not package["rust_version"]:
        problems.append(name + ": missing rust-version")
    for dependency in package["dependencies"]:
        # A path dependency has no registry source, and without a version its requirement is "*".
        if dependency["source"] is None and dependency["req"] == "*":
            problems.append(name + ": path dependency " + dependency["name"] + " has no version")
if problems:
    print("\n".join(sorted(problems)))
    sys.exit("publish check: fix the manifests above")
'
