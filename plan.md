# infra (pyrlyn/ci)

<https://github.com/pyrlyn/ci>

Shared, SHA-pinned reusable GitHub Actions workflows (mainly `ci-rust.yml`, a five-target Rust CI) called by the other pyrlyn repositories.

| # | Status | Priority | Complexity | Readiness | Agent |
| --- | --- | --- | --- | --- | --- |
| T1 | todo | P1 | 1 | 0% | |
| T2 | done | P2 | 1 | 100% | |
| T3 | todo | P3 | 2 | 0% | |
| T4 | todo | P3 | 1 | 0% | |

Audit note (2026-10-07): the local checkout was 96 commits behind `origin/main` and README/docs still pointed at the nonexistent `listepo/infra` repo (renamed to `pyrlyn/ci` upstream in `933ef57`). T2–T4 were found in the stale tree — re-verify each against current `origin/main` before working on it.

### T1. Sync the local checkout with origin/main

Local `main` sits 96 commits behind; the checked-out `feat/ci-rust-pin-check` branch is already merged upstream. Done means: local `main` is fast-forwarded to `origin/main`, the merged branch is rebased or deleted, and the docs no longer reference `listepo/infra`.

### T2. Run actionlint on pushes to main

Done. `.github/workflows/lint.yml` now triggers on `push: branches: [main]` as well as `pull_request` and `workflow_dispatch`. A merge or a direct commit to main runs actionlint. `workflow_call` is unchanged, so SHA-pinned callers are unaffected.

### T3. Extend self-test coverage to the custom inputs

`self-test.yml` exercises only the two toolchain paths; `tools`, `setup-command`, `test-command`, `build-command`, `clippy-args`, `cache-all-refs`, and the `"test": false` matrix option are never executed. Done means: a self-test job runs a one-entry custom matrix overriding the main inputs, so regressions there surface before callers pin a new SHA.

### T4. Rename the misleading CLIPPY_ARGS env in the check step

In `ci-rust.yml`, the `cargo check` step maps `inputs.clippy-args` to the env name `CLIPPY_ARGS`. Done means: the env is renamed to something neutral (e.g. `CARGO_ARGS`) in both the clippy and check steps, or the input is passed directly.
