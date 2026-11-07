#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail
# CHANGELOG.md rules: the first line is the SPDX line, no em or en dash appears, and every
# third level entry ends with a "Decisions" line naming only decision numbers that exist.
cd "$(dirname "$0")/.."
python3 -I - <<'PYTHON'
import glob
import os
import re

lines = open("CHANGELOG.md", encoding="utf-8").read().split("\n")
problems = []
if not lines or "SPDX-License-Identifier: Apache-2.0" not in lines[0]:
    problems.append("the first line must be the SPDX line")
for number, line in enumerate(lines, 1):
    if "\u2014" in line or "\u2013" in line:
        problems.append("line %d: no em or en dashes" % number)
known = {os.path.basename(path)[:4] for path in glob.glob("docs/decisions/[0-9][0-9][0-9][0-9]-*.md")}
entries = []
for line in lines:
    if line.startswith("### "):
        entries.append([line, []])
    elif entries:
        match = re.match(r"^Decisions ((?:\d{4})(?:, \d{4})*)\.$", line)
        if match:
            entries[-1][1] = match.group(1).split(", ")
if not entries:
    problems.append("no entries found")
for title, numbers in entries:
    if not numbers:
        problems.append(title + ": cite a decision, for example `Decisions 0001.`")
    for number in numbers:
        if number not in known:
            problems.append(title + ": decision " + number + " does not exist")
for milestone in range(1, 17):
    if not any(re.match(r"^### M%d " % milestone, title) for title, _ in entries):
        problems.append("missing an entry for M%d" % milestone)
if problems:
    raise SystemExit("changelog check:\n" + "\n".join(problems))
PYTHON
