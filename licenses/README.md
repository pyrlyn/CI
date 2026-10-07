# Canonical license files

pyrlyn/ci is the single source of truth for the license files of every GPL-licensed
pyrlyn repository:

| File | What |
| --- | --- |
| [`LICENSE`](LICENSE) | GNU GPL v3, verbatim FSF text (same bytes as this repository's root `LICENSE`) |
| [`PRICING.md`](PRICING.md) | Commercial license terms |
| [`readme-snippet.md`](readme-snippet.md) | Line kept in each target README between `<!-- license-sync:start -->` and `<!-- license-sync:end -->`; `{commercial}` becomes the link to that repository's commercial file |
| [`targets.yml`](targets.yml) | Target repositories and the path of each file there |

Only GPL repositories (GPL-3.0-or-later) are targets: cox, rtok, ketch and runa get both
files and the README line; crates-packages is GPL only (`commercial: null`, `readme: null`). `tools/license-kit` decides from the default branch: a
license file that reads as the GNU GPL and no root `Cargo.toml` / `package.json` license
expression without GPL. A repository whose `LICENSE` is GPL but whose manifest says e.g.
`MIT OR Apache-2.0` is a conflict and is skipped for the maintainer to decide.

## Drift check (each target)

Each target calls the reusable [`license-check.yml`](../.github/workflows/license-check.yml)
on push and pull request. It builds `tools/license-kit` and fails with a unified diff in the
job summary when a copy differs from the canonical file (byte-exact: CRLF line endings, a
different final newline and a missing file count) or the README block is missing or stale.

```yaml
name: license-check
on:
  push:
    branches: [main]
  pull_request:
  workflow_dispatch:
concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true
permissions:
  contents: read
jobs:
  license:
    uses: pyrlyn/ci/.github/workflows/license-check.yml@<full commit sha>
    permissions:
      contents: read
      actions: write # cancel-run on failure
```

Inputs: `canonical-ref` (pyrlyn/ci ref the canonical files are read from, default `main`),
`check-headers` (also require an `SPDX-License-Identifier` header in every tracked source
file, default `false`), `header-exclude` (extra excluded paths, one per line),
`cancel-run-on-failure` (default `true`).

## Sync

`license-kit sync` brings each target in line Dependabot-style: when the default branch
differs from the canonical files, one commit lands on the long-lived `chore/license-sync`
branch (created from the default branch or fast-forwarded, never forced) and a pull request
labelled `license` is opened unless one is already open from that branch. A target that
already matches gets no commit and no pull request. Sync pull requests are drafts; set
`auto-merge: true` on a target to open it ready for review with GitHub auto-merge (the
`protect-main` ruleset's required checks gate the merge).
[`license-sync.yml`](../.github/workflows/license-sync.yml)
runs it on push to main touching `licenses/` and on demand (`workflow_dispatch`, with an
optional `dry-run`). Writing needs the org secret `LICENSE_SYNC_TOKEN`, a fine-grained PAT or
GitHub App token with Contents, Pull requests and Workflows read/write on the target
repositories; the `GITHUB_TOKEN` cannot write to other repositories, and its pull requests
would not trigger CI. Without the secret the run is a read-only dry run (the targets are
public) that lists what would change and then fails with an error naming the missing secret.

`license-kit automerge-decide` is the merge gate for such automation: it answers `merge` only
for a ready (non-draft) pull request by `dependabot[bot]`, or one labelled `license` on a
license-sync branch (`chore/license-*`), and `refuse` otherwise.

## Local use

```sh
cargo run --manifest-path tools/license-kit/Cargo.toml -- \
  drift-check --config licenses/targets.yml --repo pyrlyn/cox ../cox
cargo run --manifest-path tools/license-kit/Cargo.toml -- \
  readme-line --config licenses/targets.yml --repo pyrlyn/cox ../cox
cargo test --manifest-path tools/license-kit/Cargo.toml
```
