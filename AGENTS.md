# AGENTS.md

If an AGENTS.md or CLAUDE.md exists higher in the tree, follow it too; on conflict, ask the creator.

## What this repo is

Shared, SHA-pinned reusable GitHub Actions workflows for the other repositories
(rtok, ketch, stator, bindsmith, runa, cox). This checkout's `origin` is
`git@github.com:pyrlyn/ci.git`. The README and `docs/index.md` still say
`listepo/infra`.

| Workflow | Role |
| --- | --- |
| `.github/workflows/ci-rust.yml` | Reusable Rust CI: `cargo fmt --check` once, then clippy, check, and tests on the shared five-target matrix. Rust comes from the caller's `mise.toml` unless `rust-version` is set. |
| `.github/workflows/lint.yml` | actionlint on this repo's own workflows. |
| `.github/workflows/self-test.yml` | Calls `ci-rust.yml` against `tests/fixtures/rust-crate`. |

Callers pin a full commit SHA:

```yaml
jobs:
  rust:
    uses: listepo/infra/.github/workflows/ci-rust.yml@<full commit sha>
```

The README still writes that `uses:` line with `listepo/infra`. Inputs, pinning, and secrets are in `docs/index.md`.

## Commands

actionlint is pinned in `mise.toml` (`1.7.12`):

```bash
mise x -- actionlint
```

`tests/fixtures/rust-crate` is the crate `self-test.yml` builds. Its `src/lib.rs` already has a `#[cfg(test)]` module. There is no local script beside `tests/` that those workflows leave uncovered.

## License

[GPL-3.0](LICENSE)
