<!-- SPDX-License-Identifier: Apache-2.0 -->

# lockstep-headless

Runs a named fixture without a host and prints its state hash.

```
lockstep-headless fixture <name> [--seed <number>] [--steps <number>]
lockstep-headless verify  <name> --expect <file> [--seed <number>] [--steps <number>]
```

- `fixture` prints the hash as sixteen lowercase hexadecimal digits.
- `verify` compares the hash with a committed file, ignoring surrounding whitespace. It prints `ok <name> <hash>` on a match. On a mismatch it prints the expected and actual hashes to standard error and exits with status 1.
- Without `--seed` and `--steps`, each fixture uses its own committed defaults.

Fixtures in milestone M1: `capsule` and `ledger`. Their committed hashes live in
`examples/capsule/fixtures/capsule.hash` and `examples/ledger/fixtures/ledger.hash`.

A fixture hash changes only with a commit message that names the rule that changed.

`replay`, `bisect`, and `stats` arrive with record and replay in milestone M5.
