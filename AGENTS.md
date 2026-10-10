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
| `.github/workflows/lint.yml` | jactionlint on this repo's own workflows. |
| `.github/workflows/self-test.yml` | Calls `ci-rust.yml` against `tests/fixtures/rust-crate`. |

Callers pin a full commit SHA:

```yaml
jobs:
  rust:
    uses: listepo/infra/.github/workflows/ci-rust.yml@<full commit sha>
```

The README still writes that `uses:` line with `listepo/infra`. Inputs, pinning, and secrets are in `docs/index.md`.

## Commands

jactionlint and hk are pinned in `mise.toml`. `.github/jactionlint.yaml` applies the default profile and the
baseline `.github/jactionlint-baseline.json`. `hk.pkl` runs jactionlint as a pre-push hook:

```bash
mise x -- jactionlint                  # lint workflows, composite actions, dependabot.yml
mise x -- jactionlint --baseline-write # refresh the baseline after fixing findings
mise x -- hk install --mise            # install the pre-push hook once per clone
```

`tests/fixtures/rust-crate` is the crate `self-test.yml` builds. Its `src/lib.rs` already has a `#[cfg(test)]` module. There is no local script beside `tests/` that those workflows leave uncovered.

## License

[GPL-3.0](LICENSE)
