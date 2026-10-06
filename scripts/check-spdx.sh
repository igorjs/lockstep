#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail
# Every tracked text file starts with the SPDX line (after the shebang in scripts). Fixture data,
# the license text and the lock file Cargo writes are exempt.
missing=0
while IFS= read -r file; do
  case "$file" in
    LICENSE|Cargo.lock|*.bin|*.hash) continue ;;
  esac
  [ -f "$file" ] || continue
  first=$(head -n 1 "$file")
  line=$first
  case "$first" in
    '#!/'*) line=$(sed -n 2p "$file") ;;
  esac
  case "$line" in
    *'SPDX-License-Identifier: Apache-2.0'*) continue ;;
  esac
  echo "missing SPDX line: $file"
  missing=1
done < <(git ls-files --cached --others --exclude-standard)
if [ "$missing" -ne 0 ]; then
  echo "spdx check: start the file with an SPDX-License-Identifier: Apache-2.0 comment"; exit 1
fi
