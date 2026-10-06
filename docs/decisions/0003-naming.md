<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0003 Naming (decided)

Status: decided.

The project is called Lockstep and lives under the `igorjs` account.

| Where | Name |
| --- | --- |
| GitHub | `igorjs/lockstep` |
| npm | `@igorjs/lockstep` (WebAssembly build) |
| crates.io core | `lockstep-core` (library `lockstep_core`) |
| crates.io family | `lockstep-macros`, `lockstep-headless`, `lockstep-spatial`, `lockstep-web` |

Why the Rust name has a suffix: the bare `lockstep` crate on crates.io belongs to someone else
(an iterator adaptor). crates.io has no scopes, so `igorjs` cannot act as a scope there. The
`lockstep-` prefix family is the available form. The library name differs from the other crate
so two libraries named `lockstep` never meet in one build.

Documentation must always say `lockstep-core`. `cargo add lockstep` installs the unrelated crate.

Checked free on 2026-10-06: `igorjs/lockstep` on GitHub, `@igorjs/lockstep` on npm (scope
ownership not verified), and every crates.io name in the table.

Open: Cargo namespaces (RFC 3243) are unstable and cannot be published. When they ship, the
owner of a crate named `igorjs` would own `igorjs::*`. The crate `igorjs` is free today. Publishing
a placeholder `igorjs` crate would reserve that root. This needs a publish by the owner and has not
been done.
