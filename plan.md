# infra (pyrlyn/ci)

<https://github.com/pyrlyn/ci>

Shared, SHA-pinned reusable GitHub Actions workflows (mainly `ci-rust.yml`, a five-target Rust CI) called by the other pyrlyn repositories.

| # | Status | Priority | Complexity | Readiness | Agent |
| --- | --- | --- | --- | --- | --- |
| T1 | todo | P1 | 1 | 0% | |
| T2 | done | P2 | 1 | 100% | |
| T3 | done | P3 | 2 | 100% | |
| T4 | done | P3 | 1 | 100% | |

Audit note (2026-10-07): the local checkout was 96 commits behind `origin/main` and README/docs still pointed at the nonexistent `listepo/infra` repo (renamed to `pyrlyn/ci` upstream in `933ef57`). T2–T4 were found in the stale tree — re-verify each against current `origin/main` before working on it.

### T1. Sync the local checkout with origin/main

Local `main` sits 96 commits behind; the checked-out `feat/ci-rust-pin-check` branch is already merged upstream. Done means: local `main` is fast-forwarded to `origin/main`, the merged branch is rebased or deleted, and the docs no longer reference `listepo/infra`.

### T2. Run actionlint on pushes to main

Done. `.github/workflows/lint.yml` now triggers on `push: branches: [main]` as well as `pull_request` and `workflow_dispatch`. A merge or a direct commit to main runs actionlint. `workflow_call` is unchanged, so SHA-pinned callers are unaffected.

### T3. Extend self-test coverage to the custom inputs

Done. The `custom` job in `self-test.yml` calls `ci-rust.yml` with a one-entry matrix (`ubuntu-latest`, `x86_64-unknown-linux-gnu`, `"test": false`) and sets `tools` (`nextest`), `setup-command`, `test-command`, `build-command`, `clippy-args` (`--quiet`), and `cache-all-refs`. Setup writes a marker after checking that `cargo-nextest` is on `PATH`; the build command checks that marker and runs `cargo build`. `test-command` exits 1, so the job fails if `"test": false` still runs the tests.

### T4. Rename the misleading CLIPPY_ARGS env in the check step

Done. `ci-rust.yml` maps `inputs.clippy-args` to `CARGO_ARGS` in the pinned-toolchain step (the `--locked` detection) and in both the `cargo clippy` and `cargo check` steps. The input name `clippy-args` is unchanged.
