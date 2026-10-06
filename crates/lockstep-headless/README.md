<!-- SPDX-License-Identifier: Apache-2.0 -->

# lockstep-headless

Runs a [Lockstep](https://github.com/igorjs/lockstep) fixture without a host and prints or verifies
its state hash. Continuous integration uses it to prove every example still produces its committed
hash.

```sh
lockstep-headless fixture <name> [--seed <number>] [--steps <number>]
lockstep-headless verify  <name> --expect <file> [--seed <number>] [--steps <number>]
```

- `fixture` prints the hash as sixteen lowercase hexadecimal digits.
- `verify` compares it with a committed file (surrounding whitespace ignored), prints `ok <name> <hash>` on a
  match, and exits with status 1 on a mismatch.

Fixtures: `capsule`, `ledger`, `mars-rovers`, `crowd` and `drone-fleet`. With the `parallel` feature, the crowd
solves its paths on a thread pool, and its hash must not change:

```sh
cargo run -p lockstep-headless -- verify crowd --expect examples/crowd/fixtures/crowd.hash
cargo run -p lockstep-headless --features parallel -- verify crowd --expect examples/crowd/fixtures/crowd.hash
```

It also records and replays sessions:

```sh
cargo run -p lockstep-headless -- record ledger --out ledger.recording --steps 3000
cargo run -p lockstep-headless -- replay ledger.recording
cargo run -p lockstep-headless -- bisect ledger.recording
cargo run -p lockstep-headless -- stats ledger.recording
```

See the [full documentation](https://github.com/igorjs/lockstep/blob/main/docs/lockstep-headless.md).

## License

Licensed under the [Apache License, Version 2.0](https://github.com/igorjs/lockstep/blob/main/LICENSE).
