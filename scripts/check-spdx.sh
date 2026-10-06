#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail
# Every tracked text file starts with the Apache-2.0 SPDX line, exactly, in its comment style
# (after the shebang in scripts). Fixture data, the license text and Cargo.lock are exempt.
git rev-parse --git-dir >/dev/null
missing=0
while IFS= read -r -d '' file; do
  case "$file" in
    LICENSE|Cargo.lock|*.bin|*.hash) continue ;;
  esac
  [ -f "$file" ] || continue
  line=$(head -n 1 "$file")
  case "$line" in
    '#!/'*) line=$(sed -n 2p "$file") ;;
  esac
  line=${line%$'\r'}
  case "$line" in
    '// SPDX-License-Identifier: Apache-2.0' | '# SPDX-License-Identifier: Apache-2.0' | \
      '<!-- SPDX-License-Identifier: Apache-2.0 -->') continue ;;
  esac
  echo "missing SPDX line: $file"
  missing=1
done < <(git ls-files -z --cached --others --exclude-standard)
if [ "$missing" -ne 0 ]; then
  echo "spdx check: start the file with an SPDX-License-Identifier: Apache-2.0 comment"; exit 1
fi
