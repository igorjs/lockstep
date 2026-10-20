<!-- SPDX-License-Identifier: Apache-2.0 -->
# Continuous integration

Two workflows run on every pull request and every push to `main`.

## determinism

Runs `just ci`. Pull requests run on Linux, and also on macOS and Windows when they touch fixtures,
schemas, scripts, the justfile, line ending rules or workflows. Pushes to `main` always run all
three. The job names `verify (ubuntu-latest)`, `verify (macos-latest)` and `verify (windows-latest)`
are required by branch protection, so do not rename them.

## zizmor

[zizmor](https://docs.zizmor.sh) is a static analyzer for GitHub Actions workflows. The `zizmor`
workflow runs the official `zizmorcore/zizmor-action` over `.github/workflows` and any composite
actions. It reports unpinned actions, excessive permissions, credential persistence, template
injection, cache poisoning and similar problems, and any finding at low severity or above fails
the job.

It uses plain output, not SARIF upload to code scanning, so it needs only `contents: read`, no
`security-events: write`, and it works for pull requests from forks.

Rules the workflows follow:

- Every action is pinned by full commit SHA with a version comment. Look the SHA up with
  `gh api repos/<owner>/<name>/commits/<tag> --jq .sha`; never copy one from memory.
- Permissions start empty at the workflow level and each job asks for `contents: read`.
- Every checkout sets `persist-credentials: false`.
- Tool versions installed by actions (just, wasm-pack, zizmor) are pinned.

Run it locally with `just zizmor`. It runs a pinned zizmor through `uvx` in offline mode, which skips
the audits that query GitHub, so CI is the full check. It needs network access to fetch the tool,
so it is not part of `just check`.
