# Reusable workflows

Shared GitHub Actions workflows for pyrlyn repositories. Each file under
`.github/workflows/` with `on: workflow_call` is called from a thin workflow in the consuming
repository. All third-party actions are pinned to full commit SHAs.

| Workflow | Purpose |
| --- | --- |
| `ci-rust.yml` | fmt, clippy, check, tests on a shared OS/target matrix, optional MSRV |
| `ci-dotnet.yml` | `dotnet test` for every solution; a passing no-op until a .NET project exists |
| `changes.yml` | classify changed files by ecosystem (Rust, Swift, .NET) for suite selection |
| `lint.yml` | actionlint (+ shellcheck) on the caller's workflows |
| `codeql.yml` | CodeQL per language, SARIF to code scanning |
| `semgrep.yml` | Semgrep OSS (`p/default`), SARIF to code scanning |
| `snyk.yml` | Snyk Open Source; off by default (switch), skipped without a token |
| `pipeline.yml` | ci-rust + CodeQL + Semgrep + Snyk in parallel behind a `gate` |
| `bump.yml` | the only release path: version commit, PR, required checks, rebase merge, tag + Release, release build |
| `release-plz.yml` | release PR only; never tags, releases or dispatches (bump does) |
| `release.yml` | release build on bump's tag: checks, verify, build, sign/notarize, smoke, upload, publish |
| `release-apple-desktop.yml` | macOS app release: signed, notarised `.dmg`, Sparkle appcast, GitHub Release (production only) |
| `notify-release-failure.yml` | open or update a `release-failure` issue for a failed release |
| `warnings-to-issues.yml` | one issue per code scanning / SonarCloud warning; closed when the warning is gone |
| `dependabot-automerge.yml` | merge allowed Dependabot updates after green CI; label/flag others |
| `sonarcloud.yml` | SonarCloud scan (+ Rust LCOV coverage); skipped without `SONAR_TOKEN` |
| `cla.yml` | Contributor License Agreement check (`pyrlyn/cla` action, off unless `CLA_ENABLED`), [cla.md](cla.md) |

Secrets (declared in each workflow's `on.workflow_call.secrets`):

- `snyk.yml`, `pipeline.yml`: `SNYK_TOKEN` (optional).
- `bump.yml`: `BUMP_TOKEN` (pass `RELEASE_PLZ_TOKEN`; opens the PR so its CI runs),
  `CARGO_REGISTRY_TOKEN` (optional, `publish-command`).
- `release-plz.yml`: `RELEASE_PLZ_TOKEN` (required).
- `release.yml`: `MACOS_CERTIFICATE`, `MACOS_CERTIFICATE_PWD`, `APPSTORE_CONNECT_KEY`,
  `APPSTORE_CONNECT_KEY_ID`, `APPSTORE_CONNECT_ISSUER_ID`, `APPLE_ID`, `APPLE_TEAM_ID`,
  `APPLE_APP_PASSWORD`, `CARGO_REGISTRY_TOKEN`, `PUBLISH_TOKEN` (all optional).
- `release-apple-desktop.yml`: `MACOS_CERTIFICATE`, `MACOS_CERTIFICATE_PWD`,
  `APPSTORE_CONNECT_KEY`, `APPSTORE_CONNECT_KEY_ID`, `APPSTORE_CONNECT_ISSUER_ID` (or `APPLE_ID`,
  `APPLE_TEAM_ID`, `APPLE_APP_PASSWORD`), `SPARKLE_ED_PRIVATE_KEY` (with `sparkle`); organization
  secrets, passed with `secrets: inherit`; a missing one stops the run before the build.
- `sonarcloud.yml`: `SONAR_TOKEN` (optional; every step skips without it).
- `dependabot-automerge.yml`: none (uses `github.token`).
- `warnings-to-issues.yml`: `SONAR_TOKEN` (optional; public SonarCloud projects need none).
- `cla.yml`: `CLA_APP_ID`, `CLA_APP_PRIVATE_KEY` (GitHub App `pyrlyn-cla`; or the fallback PAT
  `CLA_SIGNATURES_TOKEN`), all optional.

`permissions:` the calling job must grant:

- `ci-rust.yml`, `ci-dotnet.yml`, `lint.yml`: `contents: read`, `actions: write`.
- `changes.yml`: `contents: read`.
- `codeql.yml`, `semgrep.yml`, `snyk.yml`, `pipeline.yml`: `contents: read`,
  `security-events: write` (SARIF upload), `actions: write`.
- `release-plz.yml`: `contents: write`, `pull-requests: write`, `actions: write`,
  `issues: write`.
- `release.yml`: `contents: write` (Release assets), `checks: read`, `actions: write`,
  `issues: write`.
- `release-apple-desktop.yml`: `contents: write`, `actions: read`, `issues: write`.
- `bump.yml`: `contents: write`, `pull-requests: write`, `actions: write`, `checks: read`,
  `statuses: read`, `issues: write`.
- `notify-release-failure.yml`: `actions: read`, `issues: write`.
- `warnings-to-issues.yml`: `contents: read`, `issues: write`, `security-events: read`.
- `dependabot-automerge.yml`: `contents: write`, `pull-requests: write`, `actions: read`.
- `sonarcloud.yml`: `contents: read`, `pull-requests: read`, `actions: write`.
- `cla.yml`: `contents: read`, `pull-requests: write`, `statuses: write`.

`actions: write` is for cancel-on-failure: every job of every workflow above except
`dependabot-automerge.yml`, `cla.yml` and `warnings-to-issues.yml` (one job that only
reads results and writes issues) ends with the `cancel-run` action under `if: failure()`, so the first
failing job (a test, clippy, fmt, CodeQL, Semgrep, Snyk, SonarCloud, a release step) cancels
the whole run at once: every other running or queued job, the caller's own jobs included
(`github.run_id` inside a reusable workflow is the caller's run). Matrices use
`fail-fast: true`. `pipeline.yml`'s `gate` runs with `always()`, so after such a cancel it
fails instead of being skipped (a skipped required check counts as passed). A calling job that
grants less than `actions: write` makes the run fail at startup (GitHub refuses a nested job
that asks for more than its caller grants), even with the input off. Pass
`cancel-run-on-failure: false` where a later job of the caller must still run after a failure,
e.g. a Dependabot flow whose notify job reports a failed CI. `dependabot-automerge.yml` has no
cancel step for the same reason: its `notify-failure` must run after `automerge` fails.

Composite actions (reference them as `pyrlyn/infra/.github/actions/<name>@<sha>`):

| Action | Purpose |
| --- | --- |
| `gate` | fail unless every job in a `needs` JSON succeeded (`skip-ok` lists allowed skips) |
| `revert-on-failure` | revert a failed push, push the revert, open a draft re-apply PR |
| `macos-sign` | Developer ID codesign (or identity discovery for cargo-dist) + notarization |
| `setup-xcode` | select the pinned Xcode (default 27) with `xcode-select`; fails when it is missing |
| `cancel-run` | cancel the current workflow run (last step, `if: failure()`); `actions: write` |
| `notify-release-failure` | `release-failure` issue (mention + assign) for a failed release run |
| `warnings-to-issues` | sync code scanning / SonarCloud warnings with GitHub issues (the workflow's step) |
| `changes` | changed files by ecosystem: `rust`/`swift`/`dotnet`, `*_deps`, `*_full`, `*_present`; `docs_only` |

Private repositories: no scans (CodeQL, Semgrep, Snyk, SonarCloud) by pyrlyn policy.

## Referencing and pinning

```yaml
uses: pyrlyn/infra/.github/workflows/pipeline.yml@<full-sha> # main 2026-09-27
```

- Pin to a full commit SHA (optionally with a `# vX.Y.Z` comment once tags exist). Dependabot
  (`package-ecosystem: github-actions`) updates SHA-pinned reusable workflow refs like action
  refs, so the pin moves by pull request.
- Inside this repository, workflows call each other with `$/.github/workflows/<file>` (GitHub's
  self-repository syntax, July 2026): the nested call resolves to pyrlyn/infra at the commit
  the caller pinned, never to the caller's repository and never to `main`.
- GitHub limits (github.com): 10 levels of nesting, 50 unique reusable workflows per run.
  The deepest chain here is caller -> pipeline.yml -> ci-rust.yml (3 levels).
- Secrets never flow implicitly: declare each one in `secrets:` of the calling job (preferred)
  or use `secrets: inherit`. Every workflow here declares its secrets explicitly.
- Permissions only go down: a called workflow gets at most what the calling job grants. Grant
  the table's permissions on the calling job.
- The calling repository's Actions policy also applies to the actions used here. With
  "Allow listepo, and select non-listepo, actions" allow: GitHub-owned actions,
  `jdx/mise-action@*`, `Swatinem/rust-cache@*`, `taiki-e/install-action@*`,
  `snyk/actions/*`, `release-plz/action@*`.

## Concurrency (only the newest run executes)

Cancelling older runs is the caller's job: put a workflow-level `concurrency` in the thin
caller, keyed by workflow and ref, so a new push cancels the older queued or in-progress run of
the same workflow on the same branch or PR and the newest run runs to the end.

```yaml
concurrency:
  group: ${{ github.workflow }}-${{ github.event.pull_request.number || github.ref }}
  cancel-in-progress: ${{ github.event_name == 'pull_request' }}
```

- `cancel-in-progress: true` cancels older runs on every ref. The consumer callers here
  (docs/migration) cancel on pull requests only: on the default branch each push keeps its
  run, so `revert-on-failure` reverts the push that actually failed rather than the newest
  one. Use `true` where no job acts on a single push.
- The reusable workflows here set no `concurrency` of their own for CI (`ci.yml`,
  `pipeline.yml`, `ci-rust.yml`, scans, `sonarcloud.yml`): inside a called workflow the
  `github` context is the caller's, so a group built from `github.workflow` equals the
  caller's group and cancels or deadlocks the caller (docs/github-limits.md), and a
  cancel inside an older caller run fails its `gate` instead of cancelling the run cleanly.
- `lint.yml` cancels older runs only when it runs directly in this repository; called, it
  uses a group of its own run and never cancels.
- This repository's own triggered workflows (`action-pins.yml`, `lint.yml`, `self-test.yml`)
  use `group: ${{ github.workflow }}-${{ github.ref }}` and cancel older runs, except on the
  default branch: a push to main (a merge) gets a group of its own run (`github.run_id`) and
  `cancel-in-progress: false`, so a newer merge neither cancels nor supersedes it.
- Release and bump are never cancelled mid-run: bump merges a version commit, then tags it
  and dispatches publishing, and a cancel half-way can leave a merged version with no tag.
  `bump.yml` (`release-${{ github.repository }}`) and `release-plz.yml`
  (`release-plz-${{ github.repository }}`) set `cancel-in-progress: false`: a newer run waits,
  and GitHub keeps only the newest pending run per group. `release.yml` sets nothing; its
  caller should:

```yaml
# caller of release.yml / bump.yml (workflow_dispatch)
concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: false
```

## Free plan and private repositories

pyrlyn/infra is public, so any repository (public or private) can call it. Code scanning
(uploading SARIF from CodeQL, Semgrep or Snyk) is free for public repositories only; private
repositories need GitHub Code Security (formerly Advanced Security), which a personal Free
plan does not have. For private callers pass `upload: false` (or `upload-sarif: false` to
`pipeline.yml`): the scans still run, the SARIF is kept as a workflow artifact, and the job
does not need to upload. Rulesets/branch protection are also unavailable for private repos on
Free, so `gate` is advisory there unless something `needs:` it.

## ci-rust.yml

Rust comes from the caller's `mise.toml` (pyrlyn convention) unless `rust-version` is set;
every job fails if `rustc --version` is not the pinned version.

| Input | Default | Notes |
| --- | --- | --- |
| `matrix` | `""` (shared four-target matrix) | JSON `[{"os", "target", "test"?}]` |
| `rust-version` | `""` | exact toolchain via rustup instead of mise |
| `xcode-version` | `27` | Xcode selected on macOS jobs (`setup-xcode`); `""` = image default |
| `working-directory` | `.` | Cargo workspace |
| `package-args` | `--workspace` | packages for every cargo command (`-p x`; `""` = root only) |
| `feature-args` | `--all-features` | used by clippy, check, test, doctests, build, MSRV |
| `clippy-args` | `""` | extra args before `--` for clippy and check |
| `tools` | `""` | taiki-e/install-action tools (e.g. `nextest`) |
| `setup-command` | `""` | bash before clippy (system packages) |
| `test-command` | `cargo test $PACKAGE_ARGS --all-targets $FEATURE_ARGS` | native targets only |
| `doc-tests` | `true` | `cargo test $PACKAGE_ARGS --doc $FEATURE_ARGS`; no lib: skipped |
| `build-command` | `cargo build $PACKAGE_ARGS --all-targets $FEATURE_ARGS --target "$TARGET"` | |
| `msrv` | `""` | e.g. `1.85`; adds an `msrv` job |
| `msrv-command` | `cargo check $PACKAGE_ARGS --all-targets $FEATURE_ARGS` | |
| `changed-only` | `false` | no work (jobs still pass under their names) when no Rust file changed |
| `skip` | `false` | no work (jobs still pass under their names) whatever changed, e.g. docs-only |
| `full-package-args` | `--workspace` | replaces `package-args` when a Cargo.toml/Cargo.lock changed |
| `fmt-runs-on`, `mise-install-args`, `cache-all-refs`, `timeout-minutes` | | |

The shared matrix runs its one macOS target (arm64, `aarch64-apple-darwin`) on `xcode-27`,
the only GitHub-hosted image with Xcode 27, and every macOS job selects Xcode `xcode-version`
first (see [setup-xcode](#setup-xcode-composite-action)). A `matrix` entry on another macOS
runner fails there unless it also passes an `xcode-version` that image has (or `""`).

The `plan` job runs the `changes` action. A Cargo.toml or Cargo.lock change (or a run without
a diff: schedule, workflow_dispatch, a `.github/` or `mise.toml` change) always runs the full
suite: `full-package-args` instead of `package-args`, and `changed-only` never skips it.
`changed-only` is off by default because tests often read non-Rust files (docs, fixtures);
when on, the matrix still expands and every step is a no-op, so required checks named after
the targets report success instead of waiting.

## Dependency-driven suite selection (`changes`)

`changes.yml` (reusable, one `changes` job) and the `changes` composite action classify the
files a pull request, merge-group or push changed (GitHub compare API, three-dot, so exactly
the pull request's diff; `contents: read`, no checkout):

| Ecosystem | `<eco>_deps` (dependency manifests and locks) | `<eco>` also counts |
| --- | --- | --- |
| Rust | `Cargo.toml`, `Cargo.lock` (any directory) | `*.rs`, `.cargo/`, `rust-toolchain*`, `clippy/rustfmt/deny.toml`, `.config/nextest.toml` |
| Swift | `Package.swift`, `Package@swift-*.swift`, `Package.resolved`, XcodeGen `project.yml`, `*.xcodeproj/project.pbxproj` | `*.swift`, `*.m`, `*.mm`, `*.metal`, `*.xcconfig`, `*.entitlements`, `*.xcstrings`, `*.xcodeproj/`, `*.xcworkspace/`, `*.xcassets/`, `.swiftlint.yml`, `.swift-format` |
| .NET | `*.csproj`, `*.fsproj`, `*.vbproj`, `Directory.Packages.props`, `packages.lock.json`, `global.json`, `NuGet.config` (any case) | `*.cs`, `*.fs`, `*.vb`, `*.razor`, `*.xaml`, `*.props`, `*.targets`, `*.sln`, `*.slnx`, ... |

Outputs (strings `true`/`false`): `<eco>` (anything of that ecosystem changed),
`<eco>_deps`, `<eco>_full` (= `<eco>_deps` or `forced`: run the whole suite, never a narrowed
one), `<eco>_present` (the tree has such a project; `changes.yml` exports it for .NET only),
`forced` and `reason`. Inputs `rust-paths`, `swift-paths`, `dotnet-paths` add
repository-specific regexes (one per line) that count as that ecosystem, e.g. a script that
builds the app.

`docs_only` is `true` when there is a diff, nothing forced the run, and every changed file is
documentation: `*.md` (any directory, any case) or a `docs-paths` regex, and no
`not-docs-paths` regex (Markdown that is code: `include_str!`, shipped, run by tests). ci.yml
uses it for `docs-only` (docs/config.md, "Docs-only changes").

It fails open: an event without a diff, a `.github/`, `mise.toml` or `.tool-versions` change,
300 or more changed files, or any API error sets `forced` and every `<eco>`/`<eco>_full` to
`true`. So a broken classification runs more, never less.

Required checks: gate a job on the outputs with `!cancelled() && (needs.changes.result !=
'success' || needs.changes.outputs.swift == 'true')` so a failed `changes` job runs the suite
instead of skipping it. A non-matrix job skipped by its `if:` reports `skipped`, which branch
protection treats as passed, under its usual name. A matrix job must not be skipped at job
level (the check would be named after the raw `${{ matrix.* }}` expression and a required
target check would wait forever): gate its steps, as `ci-rust.yml` does. A local `gate` job
that `needs:` a job skipped this way passes it in `skip-ok`.

Caller example (the Swift suite of an app that links a Rust library):

```yaml
jobs:
  changes:
    uses: pyrlyn/infra/.github/workflows/changes.yml@<sha> # main
    permissions:
      contents: read
    with:
      swift-paths: |
        ^desktop/
  swift:
    needs: changes
    if: >-
      !cancelled() && (needs.changes.result != 'success'
      || needs.changes.outputs.swift == 'true' || needs.changes.outputs.rust == 'true')
    runs-on: macos-26
    steps:
      - run: swift test   # always the whole package; swift_full is true on a pin change
```

## ci-dotnet.yml

One job, `dotnet`, that always runs and reports under that name. Without a `*.csproj`,
`*.fsproj`, `*.vbproj`, `*.sln` or `*.slnx` in the repository it only classifies and passes
with a notice (no .NET project exists in any pyrlyn repository yet). Once one exists it
restores (`--locked-mode` when a `packages.lock.json` is tracked) and runs `dotnet test` for
every solution, or every project when there is none: the full suite, also for any .NET
dependency change.

| Input | Default | Notes |
| --- | --- | --- |
| `dotnet-version` | `""` | actions/setup-dotnet version; empty = `global.json`, else the runner's SDK |
| `working-directory` | `.` | |
| `test-command` | `""` | bash replacing restore + `dotnet test` |
| `changed-only` | `false` | no-op when no .NET file changed; a dependency change still runs it |
| `runs-on`, `timeout-minutes`, `cancel-run-on-failure` | | |

`ci.yml` runs it as the `dotnet` check (`enabled: true` by default, see docs/config.md).

## codeql.yml / semgrep.yml / snyk.yml

- codeql: `languages` (JSON, default `["actions"]`), `build-mode` (`none`), `build-command`
  (for `manual`), `queries` (`security-and-quality`), `config-file`, `runs-on`, `upload`.
  Keep the repository's CodeQL *default setup* off.
- semgrep: `config` (`p/default`), `extra-args`, `fail-on-findings` (`false`), `upload`.
  Results suppressed in source (`# nosemgrep: <rule>`) are removed from the SARIF before the
  upload: Semgrep keeps them with a `suppressions` mark that code scanning ignores, so each
  would otherwise stay an open alert (and become an issue through warnings-to-issues).
- snyk: `args` (`--all-projects`), `monitor` (`true`), `upload`; secret `SNYK_TOKEN`.
  Snyk CLI does not test Cargo projects; it covers npm, pub, Go, Python, NuGet manifests.
- Snyk is switched off org-wide (kept, not removed): `snyk.enabled: false` in ci.yml's
  defaults.yml and `snyk: false` as pipeline.yml's default. The job is then `skipped` and the
  gate counts it as passed, so Snyk never fails a check, blocks a merge or spends CI minutes.
  To turn it back on for one repository: `snyk: {enabled: true}` (or `auto`) in its
  `.github/infra.yml`, or `snyk: true` on a pipeline.yml call; org-wide: flip the default.

## pipeline.yml

Inputs:

- `rust` (false) runs ci-rust.yml. Every ci-rust.yml input is passed through as `rust-<name>`
  with the same default: `rust-matrix`, `rust-rust-version`, `rust-fmt-runs-on`,
  `rust-working-directory`, `rust-mise-install-args`, `rust-clippy-args`, `rust-tools`,
  `rust-setup-command`, `rust-test-command`, `rust-doc-tests`, `rust-build-command`,
  `rust-package-args`, `rust-feature-args`, `rust-msrv`, `rust-msrv-command`,
  `rust-timeout-minutes`, `rust-cache-all-refs`, `rust-changed-only`, `rust-full-package-args`.
- `codeql` (true), `codeql-languages`, `codeql-build-mode`, `codeql-build-command`,
  `codeql-queries`, `codeql-config-file`, `codeql-runs-on`.
- `semgrep` (true), `semgrep-config`, `semgrep-extra-args`, `semgrep-fail-on-findings`.
- `snyk` (false: off org-wide, the gate treats it as passed), `snyk-args`, `snyk-monitor`.
- `upload-sarif` (true).

Output: `result` (`success`). Draft PRs skip every job, including the gate, so a draft never
shows a green `gate`.

Copy-paste caller (`.github/workflows/pipeline.yml`):

```yaml
name: pipeline

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]
    types: [opened, synchronize, reopened, ready_for_review]
  schedule:
    - cron: "17 4 * * 1"
  workflow_dispatch:

concurrency:
  group: pipeline-${{ github.event.pull_request.number || github.ref }}
  cancel-in-progress: ${{ github.event_name == 'pull_request' }}

permissions:
  contents: read

jobs:
  pipeline:
    uses: pyrlyn/infra/.github/workflows/pipeline.yml@<sha> # main
    permissions:
      contents: read
      security-events: write
      actions: write
    with:
      rust: true
      codeql-languages: '["actions", "rust"]'
    secrets:
      SNYK_TOKEN: ${{ secrets.SNYK_TOKEN }}
```

The required check is `pipeline / gate`. A repository with its own CI (e.g. `just`
recipes) sets `rust: false`, keeps its local `ci.yml`, and adds a local gate:

```yaml
  ci:
    uses: ./.github/workflows/ci.yml
  gate:
    needs: [ci, pipeline]
    if: >-
      !cancelled()
      && (github.event_name != 'pull_request' || !github.event.pull_request.draft)
    runs-on: ubuntu-latest
    steps:
      - uses: pyrlyn/infra/.github/actions/gate@<sha> # main
        with:
          needs: ${{ toJSON(needs) }}
```

## Release failure notifications

GitHub cannot filter Actions notifications per workflow, so the maintainer's personal Actions
notifications stay off and only release workflows notify, by issue. `release.yml`,
`release-plz.yml` and `bump.yml` end with a `notify-failure` job (`if: always() &&
contains(needs.*.result, 'failure')`; `always()` because `cancel-run` has cancelled the rest
of the run by then) that runs the `notify-release-failure` action: it opens
`Release failed: <workflow> <ref>` labeled `release-failure` (created when missing), mentions
and assigns `notify-maintainer` (default `listepo`; empty turns it off), and lists the run link
and the failed jobs. An open `release-failure` issue for the same ref (a hidden
`<!-- release-failure ref=... -->` marker) gets a comment instead. The ref is the tag where
one is known (`release.yml` `tag`), else the branch.
`release.yml` skips it on `dry-run`. Callers of these three must grant `issues: write`: GitHub
rejects a nested job that asks for more than the caller grants, even with the input empty.

Release workflows that live in the repository (cargo-dist's `release.yml`, a desktop release)
call `notify-release-failure.yml` from a last job:

```yaml
  notify-failure:
    needs: [plan, build-local-artifacts, build-global-artifacts, host, announce]
    if: >-
      always() && github.event_name != 'pull_request'
      && contains(needs.*.result, 'failure')
    permissions:
      actions: read
      issues: write
    uses: pyrlyn/infra/.github/workflows/notify-release-failure.yml@<sha> # main
    with:
      ref: ${{ inputs.tag || github.ref_name }}
      needs: ${{ toJSON(needs) }}
```

A dist `release.yml` without `allow-dirty = ["ci"]` must not be edited (`dist plan` fails); a
separate `workflow_run` watcher calls it with `run-id`, `run-attempt`, `workflow`, `ref` and
`sha` from `github.event.workflow_run` when `conclusion == 'failure'`. Ordinary CI never calls
it.

## Warnings as issues (`warnings-to-issues.yml`)

Scans report warnings without failing CI (`semgrep.fail-on-findings: false`, CodeQL and
Semgrep only upload SARIF, `sonarcloud.soft-fail`), so warnings pile up unseen in the Security
tab and on SonarCloud. `warnings-to-issues.yml` (the `warnings-to-issues` action,
`.github/actions/warnings-to-issues/warnings_to_issues.py`, Python stdlib + `gh`) turns each
one into a GitHub issue in the caller's repository and closes it when it is gone. It runs on
its own triggers, never inside CI, so it cannot fail a CI run.

Sources:

- **Code scanning**: open alerts on the default branch from every SARIF tool (CodeQL,
  Semgrep OSS, Snyk, ...); `tools` narrows the list (e.g. `CodeQL`). A repository without
  code scanning (HTTP 403/404) is skipped with a warning.
- **SonarCloud**: unresolved issues of `sonar-project-key` (`api/issues/search`; public
  projects are read without a token, a private one needs `SONAR_TOKEN`). Empty key = off.

Rules:

- **Severity set** (`severities`, default `warning,note,medium,low`: warnings, not errors).
  A code scanning alert's effective severity is its security severity (critical, high,
  medium, low) when the rule has one, else the rule severity (error, warning, note).
  SonarCloud: BLOCKER = critical, CRITICAL = high, MAJOR = medium, MINOR = low, INFO = note.
- **One issue per warning**, found again by the fingerprint hidden in its body:
  `<!-- warning-fingerprint: code-scanning/<alert number> -->` (GitHub already merges results
  into alerts by rule and location fingerprint, so the alert number is stable across runs and
  line moves) or `<!-- warning-fingerprint: sonar/<project key>/<issue key> -->`. Issues are
  looked up by the `label` (default `warning`); removing that label from an issue makes the
  next run open a new one.
- **Existing issue**: left as it is. When the warning moved (`<!-- warning-location: ... -->`
  differs), one comment says where to, and the hidden location is updated
  (`comment-on-change: false` turns that off).
- **Warning gone** (alert fixed or dismissed, Sonar issue resolved): the open issue gets a
  comment (`... is fixed`, `... is dismissed`, `... is resolved in SonarCloud`) and is closed
  as completed. A source that could not be read completely closes nothing.
- **Closed by hand**: *not planned* means "do not track" and the issue is never touched again;
  *completed* while the warning is still (or again) there reopens it.
- **Labels**: `warning`, the source (`code-scanning` or `sonar`) and `severity:<level>`,
  created when missing (an existing label is kept as it is). Nobody is assigned.
- **Cap**: at most `max-create` (default 20) issues opened or reopened per run, oldest alerts
  first; the rest are listed as deferred and come in later runs, so a first run does not
  flood the repository.
- **Dry run** (`dry-run: true`): the plan (would create / comment / close / deferred) goes to
  the log and the job summary; nothing is written, labels included.
- Scanner text is untrusted: messages are flattened to one line, `@` cannot mention anyone
  and `<!--`/`-->` cannot forge a marker.

Inputs: `dry-run` (false), `severities`, `max-create` (20), `label` (`warning`),
`code-scanning` (true), `tools` (all), `sonar-project-key` (off), `sonar-host`
(`https://sonarcloud.io`), `sonar-branch` (main branch), `comment-on-change` (true).
Outputs: `created`, `closed`, `deferred` (planned counts in a dry run).

Caller (one file per repository, e.g. `.github/workflows/warnings.yml`; infra's own is the
first consumer). One run at a time: two concurrent runs could open the same issue twice.

```yaml
name: warnings
on:
  schedule:
    - cron: "30 6 * * *" # daily
  workflow_run: # right after CI on main uploaded fresh SARIF
    workflows: [pipeline] # the consumer's caller of ci.yml (pipeline.yml in pyrlyn repos)
    types: [completed]
    branches: [main]
  workflow_dispatch:
    inputs:
      dry-run:
        type: boolean
        default: false
concurrency:
  group: ${{ github.workflow }}
  cancel-in-progress: false
permissions:
  contents: read
jobs:
  warnings:
    permissions:
      contents: read
      issues: write
      security-events: read
    uses: pyrlyn/infra/.github/workflows/warnings-to-issues.yml@<sha> # main
    with:
      dry-run: ${{ inputs.dry-run || false }}
      sonar-project-key: listepo_rtok # the repository's sonar.projectKey; omit without Sonar
```

`workflow_run` only fires for a workflow file on the default branch, so a new caller first
runs on its schedule or by hand. Start with a manual `dry-run` to see what the first live run
would open. `self-test.yml` runs the offline test (`tests/warnings-to-issues/test.sh`) and a
live dry run on this repository.

## bump.yml

The one way a version is released, for every repository. Nothing else creates a release tag:
not release-plz (`git_tag_enable = false`, `git_release_enable = false`, no `release` command),
not cargo-dist (`dispatch-releases = true` + `create-release = false`: it only fills bump's
draft Release), not a script (release scripts only make the local version commit).

1. `<release-script> <level> --local` (or `release-plz update` with `release-plz-update: true`)
   makes one version commit on top of the default branch.
2. It is pushed to `release/bump-<tag>` and a PR into the default branch is opened. bump
   waits until every required status check of the default branch's rules (read from
   `GET /repos/{repo}/rules/branches/{branch}`, or `required-checks`) has concluded
   `success`/`skipped`/`neutral` on the branch head.
3. `gh pr merge --rebase --match-head-commit <head>` with `GITHUB_TOKEN` (no bypass actor, so
   the rules decide). Rebase rewrites the SHA: the landed commit is read back from the PR's
   `mergeCommit`, and must have the tested tree and the tested base as its only parent.
4. Only then: the tag on that commit (git refs API), the GitHub Release (a draft unless
   `release-draft: false`; notes from the version's CHANGELOG.md section), and every
   `release-workflows` file dispatched with `--ref <tag> -f tag=<tag>` (a tag or Release made
   with `GITHUB_TOKEN` triggers no `push: tags` / `release` workflow). Then `publish-command`.

A failure, a timeout or a closed PR before the merge closes the PR, deletes the branch and
fails the run (and opens a `release-failure` issue): no tag, no Release. If the default branch
moves during the checks the branch is rebuilt on the new head (`max-attempts`, 3).
`dry-run: true` opens the PR, waits for the checks and closes it. `release-untagged-head: true`
releases an untagged version already on the default branch (nothing to commit) after its
required checks are green; off by default, so such a version is never released by accident.

GitHub limits it designs around:

- `pull_request` CI never starts for a PR opened or pushed with `GITHUB_TOKEN`, and pyrlyn has
  "Allow GitHub Actions to create and approve pull requests" off. So `BUMP_TOKEN` (the existing
  `RELEASE_PLZ_TOKEN`) pushes the branch and opens the PR; the PR's own CI reports the checks.
  It never merges (its owner may be a bypass actor). Without it, `GITHUB_TOKEN` opens the PR
  and `ci-workflows` lists the workflows to dispatch on the branch (each needs
  `workflow_dispatch`; check run names, e.g. `pipeline / gate` for a reusable call, are the
  same for a dispatched run).
- The repository must allow rebase merging (`allow_rebase_merge`), and the ruleset's
  `pull_request` rule must list `rebase` in `allowed_merge_methods`.

```yaml
name: Bump and release
on:
  workflow_dispatch:
    inputs:
      level:
        type: choice
        options: [patch, minor, major]
        default: patch
      dry-run:
        type: boolean
        default: false
permissions:
  contents: read
jobs:
  bump:
    uses: pyrlyn/infra/.github/workflows/bump.yml@<sha> # main
    permissions:
      contents: write
      pull-requests: write
      actions: write
      checks: read
      statuses: read
      issues: write # notify-failure
    with:
      level: ${{ inputs.level }}
      dry-run: ${{ inputs.dry-run }}
      release-script: tools/release.sh
      ci-workflows: |
        pipeline.yml
    secrets:
      BUMP_TOKEN: ${{ secrets.RELEASE_PLZ_TOKEN }}
```

## release-plz.yml

On push to the default branch: open or refresh the release PR (`release-plz release-pr`),
held back while the current version is untagged. It never releases: merging the PR tags
nothing and dispatches nothing (the old `detect`/`verify`/`dispatch` jobs released without
bump and are gone). Prefer bump.yml alone; a merged release PR leaves an untagged version that
bump releases only with `release-untagged-head`. Inputs: `tag-prefix` (`v`), `package` (`""` =
first workspace package), `mise` (true). Secret `RELEASE_PLZ_TOKEN` is required (fine-grained
PAT, contents + pull requests write): a PR opened with `GITHUB_TOKEN` would run no CI.

## release.yml

The release build for repositories not built with cargo-dist. bump.yml tags the merged commit,
creates the draft Release and dispatches the caller's workflow with `--ref <tag> -f tag=<tag>`;
this workflow never tags. The caller owns `workflow_dispatch`:

```yaml
name: release
on:
  workflow_dispatch:
    inputs:
      tag:
        description: Release tag (vX.Y.Z)
        required: true
        type: string
      dry-run:
        type: boolean
        default: false
permissions:
  contents: read
jobs:
  release:
    uses: pyrlyn/infra/.github/workflows/release.yml@<sha> # main
    permissions:
      contents: write
      checks: read
      actions: write
      issues: write # notify-failure
    with:
      tag: ${{ inputs.tag }}
      dry-run: ${{ inputs.dry-run }}
      required-checks: |
        gate
      verify-command: just check
      bins: mytool
      smoke-command: '"$BIN_DIR/mytool" --version'
      macos-sign: true
    secrets:
      MACOS_CERTIFICATE: ${{ secrets.MACOS_CERTIFICATE }}
      MACOS_CERTIFICATE_PWD: ${{ secrets.MACOS_CERTIFICATE_PWD }}
      APPLE_ID: ${{ secrets.APPLE_ID }}
      APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}
      APPLE_APP_PASSWORD: ${{ secrets.APPLE_APP_PASSWORD }}
```

Stages: `checks` (the tag exists and names the commit the run is on, `required-checks`
concluded `success` on it,
publish secrets present when asked for) -> `verify` (`verify-command` on `verify-os`) ->
`build` per `build-matrix` entry (`setup-command`, `build-command`, collect `bins` from
`bin-dir`, codesign + notarize on macOS when `macos-sign` and the secrets exist, otherwise a
notice unless `require-macos-sign`, `smoke-command` on native targets, `.tar.gz`/`.zip` +
`.sha256`; macOS `verify` and `build` jobs first select Xcode `xcode-version`, default `27`,
through `setup-xcode`, and the default `verify-os`/`build-matrix` use the `xcode-27` image)
-> `release` (uploads to bump's Release and publishes it; `notes-command` replaces
bump's notes; prerelease when the tag has a `-` suffix; `draft` keeps it a draft)
-> `publish` (`publish-crates` with `CARGO_REGISTRY_TOKEN`, and/or `publish-command` with
`PUBLISH_TOKEN`, archives in `./dist`). `dry-run: true` stops after `build`.

Repositories built with cargo-dist (rtok, ketch, swarfr, runa) keep dist's generated
`release.yml`: dist regenerates it and fails `dist plan` on a hand-edited copy. Their
dist-workspace.toml sets `dispatch-releases = true` and `create-release = false`, so the
workflow bump dispatches uploads to bump's draft Release and undrafts it, and never tags.

What stays in each repository: the thin callers above, `.github/dependabot.yml` (GitHub reads
it only from the repository itself), repository-specific jobs (e.g. rtok's webui/wasm checks,
plugin-version checks, revert-on-failure), cargo-dist's `release.yml` and `build-setup.yml`.

## release-apple-desktop.yml

Production release of a macOS desktop app; the job ketch's `desktop-release.yml` used to run,
with the app-specific parts turned into inputs. Never call it for test builds: CI and local
builds stay unsigned or ad-hoc signed and need none of its secrets. The caller owns a
dispatch-only trigger (the workflow creates the tag itself, so a tag or release trigger would
fire on its own output):

```yaml
name: release-apple-desktop
on:
  workflow_dispatch:
    inputs:
      version:
        description: App version to release, X.Y.Z (tagged desktop-vX.Y.Z)
        required: true
        type: string
permissions:
  contents: read
jobs:
  release:
    uses: pyrlyn/infra/.github/workflows/release-apple-desktop.yml@<sha> # main
    permissions:
      contents: write
      actions: read
      issues: write # notify-failure
    with:
      version: ${{ inputs.version }}
      working-directory: desktop/macos
      project: Ketch.xcodeproj
      scheme: Ketch
      app-name: Ketch
      export-options: desktop/macos/ExportOptions.plist
      info-plist: desktop/macos/Ketch/Info.plist
      minimum-system-version: "26.0"
      cliff-config: desktop/cliff.toml
    secrets: inherit
```

Stages, all in one job on `runs-on` (default `macos-26`; `xcode-version` selects Xcode through
`setup-xcode`): the version is plain X.Y.Z, untagged and above every earlier `tag-prefix`
(default `desktop-v`) tag -> every secret present (and the source `SUPublicEDKey` not a
placeholder) -> `pre-build-command` -> a throwaway keychain with the one Developer ID
Application identity -> `xcodegen` (`xcodegen-spec`, empty skips) and `xcodebuild archive`
(`archs`, hardened runtime, `MARKETING_VERSION` = `CURRENT_PROJECT_VERSION` = version) ->
`-exportArchive` with `export-options`, checking the signature, the hardened runtime, the
architectures, the version and `minimum-system-version` -> notarise and staple the app, then
build `<app-name>-X.Y.Z.dmg` (the app and an /Applications link, UDZO), sign, notarise and staple
it -> `spctl` on both -> `.sha256` -> with `sparkle` (default on): fetch `appcast.xml` from the
`appcast-tag` prerelease (default `desktop-appcast`), run the resolved Sparkle package's
`generate_appcast` (`sparkle-bin`) with the key on standard input, and verify the new item's
EdDSA signature against the exported app's `SUPublicEDKey` -> release notes (`notes-command`,
or git-cliff with `cliff-config`) -> publish (skipped by `dry-run`): the release under the tag
with the `.dmg`, its checksum and the appcast, and the appcast on the feed prerelease. Every
release is created with `--latest=false`, and the run fails (restoring it) if
`/releases/latest` moved, so a CLI in the same repository keeps its installers' target.
Notarisation uses the App Store Connect API key, or the Apple ID trio when no key is set.

A single job, so it does not end with `cancel-run`; `notify-failure` opens the
`release-failure` issue as in release.yml (not for a dry run).

## dependabot-automerge.yml

Unifies rtok/ketch/cox. The caller runs its CI and passes the result. Only Dependabot's own
PRs from a branch of the repository are touched. Update types listed in
`allowed-update-types` are merged with `gh pr merge --<merge-method> --match-head-commit`
(never `--admin`, never `--auto`); others get `review-label`, and a major update is assigned
to `maintainer` with a review request. A failed CI or merge assigns and mentions `maintainer`.

| Input | Default |
| --- | --- |
| `ci-result` (required) | – (pass `needs.ci.result`) |
| `allowed-update-types` | `version-update:semver-patch` (space-separated) |
| `merge-method` | `squash` (`merge`, `rebase`) |
| `wait-workflow` | `""` (e.g. `pipeline.yml`: its latest PR run on the head commit must be green) |
| `wait-minutes` | `90` (max 90) |
| `review-label` | `needs-review` |
| `maintainer` | `listepo` (empty: nobody is assigned or mentioned) |

```yaml
name: Dependabot
on:
  pull_request:
    branches: [main]
    types: [opened, synchronize, reopened]
permissions: {}
concurrency:
  group: dependabot-pr-${{ github.event.pull_request.number }}
  cancel-in-progress: true
jobs:
  ci:
    if: github.actor == 'dependabot[bot]'
    uses: ./.github/workflows/ci.yml
    permissions:
      contents: read
  automerge:
    needs: ci
    if: ${{ !cancelled() && github.actor == 'dependabot[bot]' }}
    uses: pyrlyn/infra/.github/workflows/dependabot-automerge.yml@<sha> # main
    permissions:
      contents: write
      pull-requests: write
      actions: read
    with:
      ci-result: ${{ needs.ci.result }}
      wait-workflow: pipeline.yml
```

The repository must allow squash merges (the default method) and, for the review label, the
token needs `pull-requests: write`.

## sonarcloud.yml

Unifies the sonarcloud.yml of rtok, ketch, cox, runa, crates-packages, slint_dart and stator.
Every step skips with a notice when `SONAR_TOKEN` is empty; coverage and scan are soft-fail
unless `soft-fail: false`.

| Input | Default |
| --- | --- |
| `organization`, `project-key` | `""` (use `sonar-project.properties`) |
| `args` | `""` extra scanner args |
| `project-base-dir` | `.` |
| `mise`, `mise-install-args` | `true`, `""` |
| `rust` | `false` (llvm-tools-preview, cargo-llvm-cov, rust-cache) |
| `setup-command` | `""` |
| `coverage-command` | `""` (Rust: `cargo llvm-cov --locked --lcov`, to `coverage/lcov.info`) |
| `soft-fail` | `true` |
| `timeout-minutes` | `60` |

```yaml
jobs:
  sonarcloud:
    uses: pyrlyn/infra/.github/workflows/sonarcloud.yml@<sha> # main
    permissions:
      contents: read
      pull-requests: read
      actions: write
    with:
      rust: true
      organization: listepo
      project-key: listepo_ketch
    secrets:
      SONAR_TOKEN: ${{ secrets.SONAR_TOKEN }}
```

## revert-on-failure (composite action)

Unifies both generations found in six repositories: the older one (bindsmith, cox, runa,
slint_dart, stator: skip new branches, missing bases, bot commits, earlier auto-reverts and
pushes touching `.github/`) and rtok's (adds the draft re-apply PR).

Inputs: `before` (`github.event.before`), `after` (`github.sha`), `branch`
(`github.ref_name`), `reapply-pr` (`true`), `skip-workflow-changes` (`true`), `dry-run`
(`false`), `token` (`github.token`). Outputs: `reverted`, `pr-url`. The action checks out the
repository itself. Job permissions: `contents: write`, plus `pull-requests: write` for the
re-apply PR (and the repository setting "Allow GitHub Actions to create and approve pull
requests").

```yaml
  revert-on-failure:
    needs: [lint, test]
    if: >-
      always() && github.event_name == 'push' && github.ref == 'refs/heads/main'
      && contains(join(needs.*.result, ','), 'failure')
    runs-on: ubuntu-latest
    permissions:
      contents: write
      pull-requests: write
    steps:
      - uses: pyrlyn/infra/.github/actions/revert-on-failure@<sha> # main
```

## macos-sign (composite action)

Extracted from rtok/swarfr/ketch `build-setup.yml` (identity discovery for cargo-dist) and
ketch `build-check.yml` (notarization). A no-op on non-macOS runners. Every credential is
optional: missing ones skip with a notice unless `require: "true"`.

| Input | Notes |
| --- | --- |
| `mode` | `sign` (default), `discover` (export `CODESIGN_IDENTITY` for dist), `notarize` |
| `paths` | newline/space-separated files |
| `certificate`, `certificate-password` | base64 `.p12` Developer ID Application + password |
| `notarize` | `true` (sign mode) |
| `api-key`, `api-key-id`, `api-issuer` | App Store Connect API key (base64 `.p8`) |
| `apple-id`, `team-id`, `app-password` | alternative Apple ID auth |
| `require` | `false` |

Outputs: `identity`, `signed`, `notarized`. `release.yml` uses it (`macos-sign` input).
In a cargo-dist `build-setup.yml`:

```yaml
- uses: pyrlyn/infra/.github/actions/macos-sign@<sha> # main
  if: runner.os == 'macOS'
  with:
    mode: discover
    certificate: ${{ secrets.MACOS_CERTIFICATE }}
    certificate-password: ${{ secrets.MACOS_CERTIFICATE_PWD }}
    require: "true"
```

## setup-xcode (composite action)

Pins the Xcode of macOS jobs for the whole organization: `ci-rust.yml` (`rust` jobs) and
`release.yml` (`verify`, `build`) call it with their `xcode-version` input (default `27`). It
selects the newest stable `/Applications/Xcode_<version>*.app` (symlinks resolved; beta and RC
installs skipped, so `27` picks 27.0 over a 27.2 beta) with `sudo xcode-select -s`, prints
`xcodebuild -version`, and fails with an `::error::` when the runner has no such Xcode. A
no-op on non-macOS runners and with `version: ""`.

| Input | Default | Notes |
| --- | --- | --- |
| `version` | `27` | major (`27`) or major.minor (`27.1`); `""` keeps the image default |

Outputs: `path` (selected Xcode.app), `version` (e.g. `27.0`).

Xcode 27 ships only on GitHub's `xcode-27` image (`xcode-27`, `xcode-27-xlarge`; preview,
arm64, announced in actions/runner-images#14404): `macos-latest`/`macos-26` default to Xcode
26.6 and `macos-15` to 16.4. A repository-specific macOS job:

```yaml
jobs:
  macos-app:
    runs-on: xcode-27
    steps:
      - uses: actions/checkout@<sha> # v7.0.1
      - uses: pyrlyn/infra/.github/actions/setup-xcode@<sha> # main
        with:
          version: "27"
```

A caller whose own workflows use `runs-on: xcode-27` and lint them with actionlint 1.7.12 adds
the label under `self-hosted-runner.labels` in `.github/actionlint.yaml` (as this repository
does): that actionlint release predates the image.
