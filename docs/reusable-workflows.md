# Reusable workflows

Shared GitHub Actions workflows for pyrlyn repositories. Each file under
`.github/workflows/` with `on: workflow_call` is called from a thin workflow in the consuming
repository. All third-party actions are pinned to full commit SHAs.

| Workflow | Purpose |
| --- | --- |
| `ci-rust.yml` | fmt, clippy, check, tests on a shared OS/target matrix, optional MSRV |
| `ci-dotnet.yml` | `dotnet test` for every solution; a passing no-op until a .NET project exists |
| `changes.yml` | classify changed files by ecosystem (Rust, Swift, .NET) for suite selection |
| `lint.yml` | actionlint (+ shellcheck) on the caller's workflows; cargo-dist's generated `release.yml` is skipped (`ignore-generated`). In this repository the same file also runs on pull requests and on every push to `main` |
| `codeql.yml` | CodeQL per language, SARIF to code scanning |
| `semgrep.yml` | Semgrep OSS (`p/default`), SARIF to code scanning |
| `snyk.yml` | Snyk Open Source; off by default (switch), skipped without a token |
| `pipeline.yml` | ci-rust + CodeQL + Semgrep + Snyk in parallel behind a `gate` |
| `bump.yml` | the only release path: version commit, PR, required checks, rebase merge, tag + Release, release build |
| `release-plz.yml` | release PR only; never tags, releases or dispatches (bump does) |
| `release.yml` | release build on bump's tag: checks, verify, build, sign/notarize, smoke, upload, publish |
| `release-apple-desktop.yml` | macOS app release: signed, notarised `.dmg`, Sparkle appcast, GitHub Release (production only) |
| `build-cli.yml` | release building block: CLI archives per {os, target}, macOS signed via `macos-keychain`, smoke test, artifacts |
| `build-macos-dmg.yml` | release building block: macOS `.app` in a checked, Developer ID-signed (not notarised) `.dmg` artifact |
| `publish-release.yml` | release building block: one GitHub release from all artifacts; `-test.N` tags become prereleases, never latest |
| `windows-sign.yml` | sign a Windows build and pack one MSIX (production only) |
| `flatpak.yml` | build one Flatpak bundle from the caller's manifest |
| `testflight.yml` | upload an iOS IPA to TestFlight; a pull request does not upload |
| `play.yml` | upload a signed Android App Bundle; a pull request does not upload |
| `notify-release-failure.yml` | open or update a `release-failure` issue for a failed release |
| `warnings-to-issues.yml` | one issue per code scanning / SonarCloud warning; closed when the warning is gone |
| `coderabbit-issues.yml` | one issue per actionable CodeRabbit inline review comment |
| `release-ci-alert.yml` | failed checks on a release PR: comment with a mention, jobs, log tails |
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
- `build-cli.yml`, `build-macos-dmg.yml`: `MACOS_CERTIFICATE`, `MACOS_CERTIFICATE_PWD` (required
  unless `require-signing: false`).
- `publish-release.yml`: none (uses `github.token`).
- `windows-sign.yml`: `WINDOWS_CERTIFICATE`, `WINDOWS_CERTIFICATE_PWD`; a missing one stops the run
  before the layout is packed.
- `flatpak.yml`: none.
- `testflight.yml`: `MACOS_CERTIFICATE`, `MACOS_CERTIFICATE_PWD` (macos-sign; not with
  `developer-id: false`), `APPSTORE_CONNECT_KEY`, `APPSTORE_CONNECT_KEY_ID`,
  `APPSTORE_CONNECT_ISSUER_ID`. A pull request does not upload.
- `play.yml`: `PLAY_SERVICE_ACCOUNT_JSON`. A pull request does not upload.
- `sonarcloud.yml`: `SONAR_TOKEN` (optional; every step skips without it).
- `dependabot-automerge.yml`: none (uses `github.token`).
- `warnings-to-issues.yml`: `SONAR_TOKEN` (optional; public SonarCloud projects need none).
- `coderabbit-issues.yml`, `release-ci-alert.yml`: none (use `github.token`).
- `cla.yml`: `CLA_APP_ID`, `CLA_APP_PRIVATE_KEY` (GitHub App `pyrlyn-cla`; or the fallback PAT
  `CLA_SIGNATURES_TOKEN`), all optional.

`permissions:` the calling job must grant:

- `ci-rust.yml`, `ci-dotnet.yml`, `lint.yml`: `contents: read`, `actions: write`.
- `changes.yml`: `contents: read`.
- `ci.yml`: `contents: read`, `security-events: write`, `pull-requests: read`,
  `actions: write`, `issues: write` (`main-failure`).
- `codeql.yml`, `semgrep.yml`, `snyk.yml`, `pipeline.yml`: `contents: read`,
  `security-events: write` (SARIF upload), `actions: write`.
- `release-plz.yml`: `contents: write`, `pull-requests: write`, `actions: write`,
  `issues: write`.
- `release.yml`: `contents: write` (Release assets), `checks: read`, `actions: write`,
  `issues: write`.
- `release-apple-desktop.yml`: `contents: write`, `actions: read`, `issues: write`.
- `build-cli.yml`, `build-macos-dmg.yml`: `contents: read`.
- `publish-release.yml`: `contents: write`.
- `windows-sign.yml`: `contents: read`, `actions: write`.
- `flatpak.yml`: `contents: read`, `actions: write`.
- `testflight.yml`: `contents: read`, `actions: write`.
- `play.yml`: `contents: read`, `actions: write`.
- `bump.yml`: `contents: write`, `pull-requests: write`, `actions: write`, `checks: read`,
  `statuses: read`, `issues: write`.
- `notify-release-failure.yml`: `actions: read`, `issues: write`.
- `warnings-to-issues.yml`: `contents: read`, `issues: write`, `security-events: read`.
- `coderabbit-issues.yml`: `issues: write`, `pull-requests: read`.
- `release-ci-alert.yml`: `actions: read`, `contents: read`, `pull-requests: write`.
- `dependabot-automerge.yml`: `contents: write`, `pull-requests: write`, `actions: read`.
- `sonarcloud.yml`: `contents: read`, `pull-requests: read`, `actions: write`.
- `cla.yml`: `contents: read`, `pull-requests: write`, `statuses: write`.

`actions: write` is for cancel-on-failure: every job of every workflow above except
`dependabot-automerge.yml`, `cla.yml`, `warnings-to-issues.yml`, `coderabbit-issues.yml` and
`release-ci-alert.yml` (one job each that only reads results and writes issues or comments)
ends with the `cancel-run` action under `if: failure()`, so the first
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

Composite actions (reference them as `pyrlyn/ci/.github/actions/<name>@<sha>`):

| Action | Purpose |
| --- | --- |
| `gate` | fail unless every job in a `needs` JSON succeeded (`skip-ok` lists allowed skips) |
| `revert-on-failure` | revert a failed push, push the revert, open a draft re-apply PR |
| `macos-sign` | Developer ID codesign (or identity discovery for cargo-dist) + notarization |
| `macos-keychain` | import the Developer ID identity into a job keychain kept for later steps (the release builds' one signing setup) |
| `setup-xcode` | select the pinned Xcode (default 27) with `xcode-select`; fails when it is missing |
| `cancel-run` | cancel the current workflow run (last step, `if: failure()`); `actions: write` |
| `setup-rust` | Rust from the caller's mise.toml as the active toolchain; fails on any other rustc |
| `commits` | Conventional Commits subjects of a pull request's commits (grep or commitlint) |
| `notify-release-failure` | `release-failure` issue (mention + assign) for a failed release run |
| `warnings-to-issues` | sync code scanning / SonarCloud warnings with GitHub issues (the workflow's step) |
| `coderabbit-issues` | CodeRabbit inline findings to GitHub issues (the workflow's step) |
| `release-ci-alert` | comment on a release pull request whose CI failed (the workflow's step) |
| `changes` | changed files by ecosystem: `rust`/`swift`/`dotnet`, `*_deps`, `*_full`, `*_present`; `docs_only` |

Private repositories: no scans (CodeQL, Semgrep, Snyk, SonarCloud) by pyrlyn policy.

## Referencing and pinning

```yaml
uses: pyrlyn/ci/.github/workflows/pipeline.yml@<full-sha> # main 2026-09-27
```

- Pin to a full commit SHA (optionally with a `# vX.Y.Z` comment once tags exist). Dependabot
  (`package-ecosystem: github-actions`) updates SHA-pinned reusable workflow refs like action
  refs, so the pin moves by pull request.
- Inside this repository, workflows call each other with `$/.github/workflows/<file>` (GitHub's
  self-repository syntax, July 2026): the nested call resolves to pyrlyn/ci at the commit
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

## Draft pull requests (`skip-drafts`)

A draft pull request runs no CI; marking it ready for review runs everything as usual.

- ci-rust, ci-dotnet, codeql, semgrep, snyk, sonarcloud, lint, license-check and pages take
  `skip-drafts` (boolean, default `true`): on a draft `pull_request` their first job is skipped
  at job level (`if:`), so no runner starts. ci-rust skips `plan` and with it every job that
  needs it; pages skips `build` and `deploy`. Pass `skip-drafts: false` to run drafts.
- ci.yml passes its `skip-drafts` config key ([config](config.md)) on to every workflow it
  calls; pipeline.yml skips every job, the gate included, on a draft.
- The caller's `pull_request` trigger lists `ready_for_review`, or marking a draft ready starts
  no run and the checks of the last (skipped) draft run stay on the head commit:

  ```yaml
  on:
    pull_request:
      branches: [main]
      types: [opened, synchronize, reopened, ready_for_review]
  ```

- The caller's own jobs on `pull_request` take the same guard:
  `if: github.event_name != 'pull_request' || !github.event.pull_request.draft` (with `&&`
  when the job has an `if` already). A job with `!cancelled()` or `always()` runs even when
  the jobs it needs were skipped, so it needs the guard itself.
- Required checks: a job skipped at job level counts as passed, while a skipped matrix job or a
  skipped call of a reusable workflow reports one check under its raw name, so required checks
  such as `rust / fmt` stay pending on a draft. Neither lets a draft through: GitHub does not
  merge a draft, and `ready_for_review` runs the full set on the head commit before a merge.
- Not skipped: `changes.yml` (a non-success reads as "unknown, run everything" in callers that
  fail open, so skipping it would start their heavy jobs; the callers guard those jobs),
  `pull_request_target` workflows (cla.yml), Dependabot flows (Dependabot never opens drafts)
  and release workflows.

## Free plan and private repositories

pyrlyn/ci is public, so any repository (public or private) can call it. Code scanning
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
| `locked` | `auto` | `--locked` (`LOCKED_ARGS`) for clippy, check, doctests and the default test/build commands; auto = the workspace has a Cargo.lock |
| `tools` | `""` | taiki-e/install-action tools (e.g. `nextest`) |
| `setup-command` | `""` | bash before clippy (system packages) |
| `test-command` | `cargo test $PACKAGE_ARGS --all-targets $FEATURE_ARGS $LOCKED_ARGS` | native targets only |
| `doc-tests` | `true` | `cargo test $PACKAGE_ARGS --doc $FEATURE_ARGS`; no lib: skipped |
| `build-command` | `cargo build $PACKAGE_ARGS --all-targets $FEATURE_ARGS $LOCKED_ARGS --target "$TARGET"` | |
| `msrv` | `""` | e.g. `1.85`; adds an `msrv` job |
| `msrv-command` | `cargo check $PACKAGE_ARGS --all-targets $FEATURE_ARGS` | |
| `changed-only` | `false` | no work (jobs still pass under their names) when no Rust file changed |
| `skip` | `false` | no work (jobs still pass under their names) whatever changed, e.g. docs-only |
| `skip-runs-on` | `ubuntu-latest` | runner of every matrix entry when there is no work (`""` = each entry's `os`) |
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
the targets report success instead of waiting. Without work (`skip`, or `changed-only` and no
Rust change) every entry runs on `skip-runs-on` (`ubuntu-latest`) instead of its own `os`: the
check names (`target`) stay, and a docs-only change starts no macOS (Xcode), Windows or ARM
runner.

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
    uses: pyrlyn/ci/.github/workflows/changes.yml@<sha> # main
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
    uses: pyrlyn/ci/.github/workflows/pipeline.yml@<sha> # main
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
      - uses: pyrlyn/ci/.github/actions/gate@<sha> # main
        with:
          needs: ${{ toJSON(needs) }}
```

## Notifications

Failures notify the maintainer only when they happen on `main`. Pull requests and other
branches never notify, and CodeRabbit findings are filed without notifying anyone.

| Notifier | Notifies | Default | Turn off / on |
| --- | --- | --- | --- |
| `main-failure` job of `ci.yml` (the `main-failure` action) | a failed push, schedule or workflow_dispatch run on `main`: one `ci-main-failure` issue per workflow, mentioning and assigning `main-failure-maintainer`; the next green run on `main` closes it | on, `listepo` | `notify-main-failure: false` |
| `notify-release-failure` (action and workflow); `notify-failure` jobs of `release.yml`, `release-plz.yml`, `bump.yml`, `release-apple-desktop.yml` | a failed release run on `branch` (default `main`; a tag whose commit is on `main` counts) | on, `listepo` | `maintainer` / `notify-maintainer: ""` |
| `failure-digest.yml` (this repository, daily) | failed runs on each scanned repository's default branch in the last 24 hours, plus active `release-failure` issues | on (cron `0 6 * * *`) | remove the cron |
| `release-ci-alert` (action and workflow) | nothing: release pull requests are pull request branches | off (`maintainer: ""`) | set `maintainer` |
| `dependabot-automerge.yml` `notify-failure` | nothing: Dependabot pull requests are pull request branches | off (`notify-failures: false`) | `notify-failures: true` |
| `coderabbit-issues` (action and workflow) | nothing: issues are filed unassigned | off (`assignee: ""`) | set `assignee` |

A consumer picks this up when its `pyrlyn/ci` pin moves past this change. A `ci.yml` caller
must then grant `issues: write` as well (see docs/consumers.md). GitHub's own Actions
notifications (workflow runs you triggered) are an account setting, not a workflow setting:
"Notification settings" -> "System" -> "Actions"
([Managing GitHub Actions notifications](https://docs.github.com/en/subscriptions-and-notifications/how-tos/managing-github-actions-notifications)).

## Release failure notifications

Only a failure on `main` notifies (see [Notifications](#notifications)). `release.yml`,
`release-plz.yml` and `bump.yml` end with a `notify-failure` job (`if: always() &&
contains(needs.*.result, 'failure')`; `always()` because `cancel-run` has cancelled the rest
of the run by then) that runs the `notify-release-failure` action: it opens
`Release failed: <workflow> <ref>` labeled `release-failure` (created when missing), mentions
and assigns `notify-maintainer` (default `listepo`; empty turns it off), and lists the run link
and the failed jobs. An open `release-failure` issue for the same ref (a hidden
`<!-- release-failure ref=... -->` marker) gets a comment instead. The ref is the tag where
one is known (`release.yml` `tag`), else the branch. The action notifies only when the run is
on `branch` (default `main`): a push, schedule or dispatch on `main`, or a tag whose commit is
on `main` (checked with the compare API; when that check fails, it notifies). Pull request runs
and other branches post nothing. From a `workflow_run` watcher, the watched run's event,
branch or tag and commit are checked.
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
    uses: pyrlyn/ci/.github/workflows/notify-release-failure.yml@<sha> # main
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
    uses: pyrlyn/ci/.github/workflows/warnings-to-issues.yml@<sha> # main
    with:
      dry-run: ${{ inputs.dry-run || false }}
      sonar-project-key: listepo_rtok # the repository's sonar.projectKey; omit without Sonar
```

`workflow_run` only fires for a workflow file on the default branch, so a new caller first
runs on its schedule or by hand. Start with a manual `dry-run` to see what the first live run
would open. `self-test.yml` runs the offline test (`tests/warnings-to-issues/test.sh`) and a
live dry run on this repository.

## CodeRabbit findings as issues (`coderabbit-issues.yml`)

CodeRabbit runs on demand and its findings should not get lost in a pull request's
conversation, while its other comments stay quiet (the repositories' `.coderabbit.yaml` turns
off the summary, walkthrough extras, review status, fortune and poem, and chat auto-replies).
`coderabbit-issues.yml` (the `coderabbit-issues` action, `actions/github-script`) opens one
issue per actionable inline review comment of a CodeRabbit review.

- **Trigger**: the caller's `pull_request_review` (`submitted`) and, as a fallback,
  `pull_request_review_comment` (`created`), gated on `coderabbitai[bot]` as the author, so
  human reviews never start a runner. Either event covers every comment of that review.
- **Skipped**: replies, summary and status comments, nitpicks, `[!WARNING]`-style notices,
  `Warning:` notes, rate-limit and skipped-review notes. CodeRabbit's `⚠️ Potential issue` badge
  is a finding, not a warning, and is kept. Comments in the review body (nitpicks, outside-diff
  notes) are not filed.
- **Issue**: title `CodeRabbit: <first line of the comment>`, the pull request link, file and
  line(s) with a link to the comment, the commit, the comment body (no `@` mentions outside code
  blocks, no HTML comments), label `coderabbit` (created when missing), assigned to `assignee`
  (default empty: nobody is assigned, so a new issue notifies no one).
- **Dedupe**: a hidden `<!-- coderabbit-comment-id: N -->` marker; an issue with the label and
  that marker (open or closed) means the comment is never filed again.
- **Cap**: at most `max-create` (default 20) issues per run; `dry-run` only logs.
- **Forks**: no secrets; on a fork's pull request the token is read-only, so the run logs a
  warning instead of failing.

Caller (`.github/workflows/coderabbit-issues.yml`):

```yaml
name: coderabbit-issues
on:
  pull_request_review:
    types: [submitted]
  pull_request_review_comment:
    types: [created]
concurrency:
  group: coderabbit-issues-${{ github.event.pull_request.number }}-${{ github.event.sender.login }}
  cancel-in-progress: false
permissions: {}
jobs:
  issues:
    if: >-
      github.event.review.user.login == 'coderabbitai[bot]'
      || github.event.comment.user.login == 'coderabbitai[bot]'
    permissions:
      issues: write
      pull-requests: read
    uses: pyrlyn/ci/.github/workflows/coderabbit-issues.yml@<sha> # main
```

`pull_request_review` runs the workflow file of the pull request's merge commit, so it works on
pull requests opened before the caller landed once they are rebased or updated.

## Release pull request CI alerts (`release-ci-alert.yml`)

Off by default ([Notifications](#notifications)): a release pull request is a pull request
branch, so with `maintainer` empty the watcher posts nothing. With `maintainer` set, ordinary
pull requests still never notify about failed checks; a release pull request does: `release-ci-alert.yml`
(the `release-ci-alert` action) runs from a `workflow_run` watcher of the repository's pull
request workflows and comments on the pull request when it is a release pull request.

- **Release pull request**: head branch `release-plz-*` (release-plz) or `release/bump-*`
  (`bump.yml`), label `release`, or a title starting with `chore: release`, `chore(release)` or
  `release: v`.
- **Pull request of the run**: `workflow_run.pull_requests`, else every open pull request whose
  head is the run's commit (fork pull requests included).
- **Comment**: mentions `maintainer` (default empty: no comment at all; the mention is the
  notification) and
  lists the workflow run, the failed or timed-out jobs and steps with links, the number of jobs
  cancelled after the failure, and the last `tail-lines` (default 30) log lines up to the last
  error of up to five failed jobs. One comment per workflow and commit (hidden
  `<!-- release-ci-alert: <workflow id>/<sha> -->` marker); a re-run that fails again updates
  it, a new commit that fails gets a new comment.
- Runs that are not pull request runs, did not fail, or belong to a pull request that is not a
  release pull request post nothing (the caller's `if` keeps most of them from starting a
  runner). Release workflows keep their `release-failure` issues.

Caller (`.github/workflows/release-ci-alert.yml`; list the workflows that run on
`pull_request`):

```yaml
name: release-ci-alert
on:
  workflow_run:
    workflows: [pipeline, ci, license-check]
    types: [completed]
concurrency:
  group: >-
    ${{ github.workflow }}-${{ github.event.workflow_run.workflow_id }}-${{
    github.event.workflow_run.head_sha }}
  cancel-in-progress: false
permissions: {}
jobs:
  alert:
    if: >-
      contains(fromJSON('["pull_request", "pull_request_target"]'),
      github.event.workflow_run.event)
      && contains(fromJSON('["failure", "timed_out"]'), github.event.workflow_run.conclusion)
    permissions:
      actions: read
      contents: read
      pull-requests: write
    uses: pyrlyn/ci/.github/workflows/release-ci-alert.yml@<sha> # main
```

`workflow_run` only fires for a workflow file on the default branch.

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

Prerelease is the caller's decision: `prerelease` is a required input with no default, so each
repository sets it in its own bump workflow (a version with a `-` suffix is a prerelease
either way). `release-title-suffix` (appended to the tag in the title) and
`release-notes-header` (Markdown above the notes) label a release, e.g. as a dev build.

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
  same for a dispatched run). Empty `ci-workflows` means `ci.yml` and `pipeline.yml`, each
  that exists and has `workflow_dispatch`; empty `release-script` (without
  `release-plz-update`) means `scripts/release.sh` when the repository has it.
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
    uses: pyrlyn/ci/.github/workflows/bump.yml@<sha> # main
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
      prerelease: false # required: this repository's decision
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
    uses: pyrlyn/ci/.github/workflows/release.yml@<sha> # main
    permissions:
      contents: write
      checks: read
      actions: write
      issues: write # notify-failure
    with:
      tag: ${{ inputs.tag }}
      dry-run: ${{ inputs.dry-run }}
      prerelease: false # required: this repository's decision
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
`build` per `build-matrix` entry (`setup-command`, `build-command` — by default
`cargo build --profile "$CARGO_PROFILE"` with `cargo-profile`, default `release` — collect
`bins` from `bin-dir`, default `target/$TARGET/$PROFILE_DIR`, `debug` for the `dev` profile, codesign + notarize on macOS when `macos-sign` and the secrets exist, otherwise a
notice unless `require-macos-sign`, `smoke-command` on native targets, `.tar.gz`/`.zip` +
`.sha256`; macOS `verify` and `build` jobs first select Xcode `xcode-version`, default `27`,
through `setup-xcode`, and the default `verify-os`/`build-matrix` use the `xcode-27` image)
-> `release` (uploads to bump's Release and publishes it; `notes-command` replaces
bump's notes; prerelease when the required `prerelease` input is true or the tag has a `-`
suffix; `draft` keeps it a draft)
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
    uses: pyrlyn/ci/.github/workflows/release-apple-desktop.yml@<sha> # main
    permissions:
      contents: write
      actions: read
      issues: write # notify-failure
    with:
      version: ${{ inputs.version }}
      prerelease: false # required: this repository's decision
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
with the `.dmg`, its checksum and the appcast (a prerelease when the required `prerelease`
input is true; `release-notes-header` goes above its notes), and the appcast on the feed
prerelease. Every
release is created with `--latest=false`, and the run fails (restoring it) if
`/releases/latest` moved, so a CLI in the same repository keeps its installers' target.
Notarisation uses the App Store Connect API key, or the Apple ID trio when no key is set.

A single job, so it does not end with `cancel-run`; `notify-failure` opens the
`release-failure` issue as in release.yml (not for a dry run).

## Release orchestration: build-cli.yml + build-macos-dmg.yml + publish-release.yml

Three building blocks for a repository that ships a CLI and a macOS app from one tag, into one
GitHub release, with one signing setup (`macos-keychain`, the Developer ID organization
certificate). The caller keeps one release workflow with one trigger: bump.yml dispatches it on
its tag, exactly as for `release.yml`. That workflow checks that the run is on the tag,
runs its own verify gate, calls the two builds and then publishes:

```yaml
name: release
on:
  workflow_dispatch:
    inputs:
      tag: { description: "vX.Y.Z (bump.yml) or vX.Y.Z-test.N; run with --ref <tag>", required: true, type: string }
permissions:
  contents: read
jobs:
  version:   # repository-specific: the tag names this commit and matches the manifest version
    runs-on: ubuntu-latest
    outputs: { ref: "${{ steps.v.outputs.ref }}" }
    steps: [ ... ]
  cli:
    needs: version
    uses: pyrlyn/ci/.github/workflows/build-cli.yml@<sha>
    with:
      ref: ${{ needs.version.outputs.ref }}
      matrix: >-
        [{"os": "xcode-27", "target": "aarch64-apple-darwin", "sign": true},
         {"os": "ubuntu-latest", "target": "x86_64-unknown-linux-gnu", "sign": false}]
      # Signs the macOS binary with "$MACOS_SIGN_IDENTITY" when it is set.
      package-command: scripts/package.sh "$TARGET" "$OUT_DIR"
      smoke-command: tar -xJf "$OUT_DIR/app-$TARGET.tar.xz" -C "$RUNNER_TEMP" && "$RUNNER_TEMP/app" --version
    secrets:
      MACOS_CERTIFICATE: ${{ secrets.MACOS_CERTIFICATE }}
      MACOS_CERTIFICATE_PWD: ${{ secrets.MACOS_CERTIFICATE_PWD }}
  desktop:
    needs: version
    uses: pyrlyn/ci/.github/workflows/build-macos-dmg.yml@<sha>
    with:
      ref: ${{ needs.version.outputs.ref }}
      setup-command: bash scripts/desktop/xcframework.sh
      # Builds App.app, signs it with "$MACOS_SIGN_IDENTITY", packs the DMG, prints its path last.
      build-command: bash scripts/desktop/app.sh CURRENT_PROJECT_VERSION="$BUILD_NUMBER" && bash scripts/desktop/dmg.sh
      app-name: App
    secrets:
      MACOS_CERTIFICATE: ${{ secrets.MACOS_CERTIFICATE }}
      MACOS_CERTIFICATE_PWD: ${{ secrets.MACOS_CERTIFICATE_PWD }}
  publish:
    needs: [version, cli, desktop]
    uses: pyrlyn/ci/.github/workflows/publish-release.yml@<sha>
    permissions:
      contents: write
    with:
      tag: ${{ inputs.tag }}
  tap:       # package managers, taps, announcements: skip them for test releases
    needs: publish
    if: needs.publish.outputs.test != 'true'
    ...
```

- **build-cli.yml**: `matrix` (JSON list of `{os, target, sign}`), `package-command` (required;
  runs with `TARGET`, `OUT_DIR` and, on signed macOS entries, the identity in `identity-env`
  (default `MACOS_SIGN_IDENTITY`) plus `SIGNING_KEYCHAIN`), `smoke-command`, `setup-command`,
  `ref`, `xcode-version` (`27`), `mise-install-args` (`rust`), `rust-cache` (true), `out-dir`
  (`dist`), `artifact-files` (`*.tar.xz *.tar.gz *.zip`), `artifact-prefix` (`cli-`),
  `require-signing` (true). Artifacts: `<artifact-prefix><target>`.
- **build-macos-dmg.yml**: `build-command` (required; builds and signs the app, makes the DMG,
  prints its path as the last stdout line; gets `BUILD_NUMBER` = run number), `app-name`
  (required), `setup-command`, `ref`, `runs-on` (`xcode-27`), `xcode-version` (`27`),
  `timeout-minutes` (90), `mise-install-args`, `archs` (`arm64`), `require-applications-link`
  (true), `artifact-name` (`macos-dmg`), `require-signing` (true), `identity-env`. Before the
  upload it mounts the DMG and requires `<app-name>.app`, the executable its Info.plist names,
  every arch in `archs`, the /Applications link and a valid (Developer ID) signature. Debug or
  Release is the build-command's choice; there is no notarisation (production app releases with
  notarisation and Sparkle are release-apple-desktop.yml).
- **publish-release.yml**: `tag` (required), `test-tag-regex` (`-test\.[0-9]+$`),
  `artifact-pattern` (`*`), `files` (`*.tar.xz *.tar.gz *.zip *.dmg`), `checksums` (true:
  `SHA256SUMS` plus a `.sha256` per file), `test-notes`. Output `test`. A normal tag fills and
  publishes bump's draft (missing draft = failure). A test tag gets its release created here:
  published with `--prerelease --latest=false`, and the run fails if `/releases/latest` moved to
  it, so installers and `self update` that read /releases/latest stay on the last real release.
  A test release needs no bump: tag the commit by hand (`git tag -a vX.Y.Z-test.N`, pushed) and
  dispatch the release workflow with `--ref vX.Y.Z-test.N -f tag=vX.Y.Z-test.N`.

### Dependabot callers of a ci.yml that uses ci-rust.yml

A repository's `dependabot.yml` that runs its own `ci.yml` (`uses: ./.github/workflows/ci.yml`)
must grant that calling job everything ci.yml's jobs ask for. ci-rust.yml cancels the run on a
failed job and so asks for `actions: write`; without it on the calling job GitHub rejects the
whole Dependabot run as `startup_failure` (on every pull request, Dependabot's or not):

```yaml
  ci:
    uses: ./.github/workflows/ci.yml
    permissions:
      contents: write   # whatever ci.yml's other jobs need
      actions: write    # ci-rust.yml (cancel-run-on-failure)
```

## windows-sign.yml

Production packaging of one Windows layout. Never call it for an unsigned test build: the
certificate is required and a missing secret stops the run before `makeappx`. The caller owns
the trigger (dispatch or a tag). This workflow does not register a required check.

```yaml
jobs:
  sign:
    uses: pyrlyn/ci/.github/workflows/windows-sign.yml@<sha> # main
    permissions:
      contents: read
      actions: write
    with:
      layout: desktop/windows/layout
      output: Mailune.msix
    secrets: inherit
```

| Input | Default |
| --- | --- |
| `layout` (required) | directory with `AppxManifest.xml` at its root, relative to the repository |
| `output` | `app.msix` (a file name, not a path) |
| `build-command` | `""` (bash in `working-directory` that produces the layout) |
| `working-directory` | `.` |
| `dotnet-version` | `""` (skips `actions/setup-dotnet`) |
| `runs-on` | `windows-latest` |
| `timeout-minutes` | `60` |
| `cancel-run-on-failure` | `true` |

Secrets: `WINDOWS_CERTIFICATE` (base64 `.pfx`) and `WINDOWS_CERTIFICATE_PWD`. The job checks
them, optionally installs the SDK and runs `build-command`, packs with the newest x64
`makeappx` from the Windows SDK, signs with `signtool` (SHA256, Microsoft timestamp), verifies
with `signtool verify /pa`, and uploads the MSIX as an artifact named after `output`. The
`.pfx` is deleted before the step ends, including when signing fails. The password is not
printed. Output: `package` (the file name).

## flatpak.yml

Builds one Flatpak from the caller's manifest and uploads the bundle. The manifest chooses the
runtime; this workflow installs that SDK from `repo-url` (default Flathub) and does not sign
the repository. Flathub submission stays in the app repository.

```yaml
jobs:
  flatpak:
    uses: pyrlyn/ci/.github/workflows/flatpak.yml@<sha> # main
    permissions:
      contents: read
      actions: write
    with:
      manifest: desktop/linux/app.mailune.yml
      app-id: app.mailune.Mailune
```

| Input | Default |
| --- | --- |
| `manifest` (required) | path relative to the repository root |
| `app-id` (required) | application id for `flatpak build-bundle` |
| `bundle` | `dist/app.flatpak` |
| `arch` | `x86_64` |
| `branch` | `""` (manifest default) |
| `repo-url` | Flathub's `flathub.flatpakrepo` |
| `runs-on` | `ubuntu-latest` |
| `timeout-minutes` | `90` |
| `cancel-run-on-failure` | `true` |

`flatpak-builder` runs as the user with `--disable-rofiles-fuse` because the hosted runner has
no FUSE device, and `--install-deps-from=flathub` so the SDK is the one the manifest names.
Output: `bundle` (the relative path). The file is also uploaded as `flatpak-<arch>`.

## testflight.yml

Uploads one signed IPA to TestFlight. The certificate check is the `macos-sign` action in
`discover` mode (the same `.p12` import as the other Apple workflows). The upload uses that
action's App Store Connect API key (`APPSTORE_CONNECT_KEY`, key id, issuer) with `altool`.

A pull request does not upload. The gate treats `pull_request` and `pull_request_target` as
skip, and the upload step exits if it is ever reached on those events. `build-command` still
runs, so a caller can compile on a pull request without sending a build.

```yaml
jobs:
  testflight:
    uses: pyrlyn/ci/.github/workflows/testflight.yml@<sha> # main
    permissions:
      contents: read
      actions: write
    with:
      ipa: build/Mailune.ipa
      build-command: xcodebuild -scheme Mailune -destination 'generic/platform=iOS' build
    secrets: inherit
```

| Input | Default |
| --- | --- |
| `ipa` (required) | path relative to the repository root |
| `working-directory` | `.` |
| `build-command` | `""` |
| `runs-on` | `macos-26` |
| `xcode-version` | `""` (image default; otherwise `setup-xcode`) |
| `setup-mise` | `false` (`true` installs the caller's `mise.toml` tools before the build) |
| `developer-id` | `true` (`false` skips the Developer ID check, which TestFlight does not use) |
| `timeout-minutes` | `90` |
| `cancel-run-on-failure` | `true` |

On a run that may upload, the `.p8` is written to `~/private_keys/AuthKey_<id>.p8` before
`build-command`, which also gets `APPSTORE_CONNECT_KEY_ID` and `APPSTORE_CONNECT_ISSUER_ID`, so
an `xcodebuild -exportArchive` with `-authenticationKeyPath` can sign through App Store Connect.
`altool` reads the same file. A pull request build gets neither. The file is removed when the job
ends, even after a failure. The key bytes are not printed.

## play.yml

Uploads one signed Android App Bundle to Google Play. The caller signs the bundle. This
workflow checks the JAR signature with `jarsigner -verify -strict` and then uploads with
`r0adkll/upload-google-play` (pinned). The service-account JSON is a secret and is not printed.

A pull request does not upload. `pull_request` and `pull_request_target` skip the check and
the upload.

```yaml
jobs:
  play:
    uses: pyrlyn/ci/.github/workflows/play.yml@<sha> # main
    permissions:
      contents: read
      actions: write
    with:
      aab: app/build/outputs/bundle/release/app-release.aab
      package-name: app.mailune
      track: internal
    secrets: inherit
```

| Input | Default |
| --- | --- |
| `aab` (required) | path of the signed bundle, relative to the repository |
| `package-name` (required) | Android application id |
| `track` | `internal` (`alpha`, `beta`, `production`) |
| `status` | `completed` (`draft`, `inProgress`, `halted`) |
| `changes-not-sent-for-review` | `false` |
| `runs-on` | `ubuntu-latest` |
| `timeout-minutes` | `30` |
| `cancel-run-on-failure` | `true` |

## dependabot-automerge.yml

Unifies rtok/ketch/cox. The caller runs its CI and passes the result. Only Dependabot's own
PRs from a branch of the repository are touched. Update types listed in
`allowed-update-types` are merged with `gh pr merge --<merge-method> --match-head-commit`
(never `--admin`, never `--auto`); others get `review-label`, and a major update is assigned
to `maintainer` with a review request. With `notify-failures: true`, a failed CI or merge
assigns and mentions `maintainer`; by default failures notify no one.

| Input | Default |
| --- | --- |
| `ci-result` (required) | – (pass `needs.ci.result`) |
| `allowed-update-types` | `version-update:semver-patch` (space-separated) |
| `merge-method` | `squash` (`merge`, `rebase`) |
| `wait-workflow` | `""` (e.g. `pipeline.yml`: its latest PR run on the head commit must be green) |
| `wait-minutes` | `90` (max 90) |
| `review-label` | `needs-review` |
| `maintainer` | `listepo` (empty: nobody is assigned or mentioned) |
| `notify-failures` | `false` (`true`: a failed CI or merge assigns and mentions `maintainer`) |

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
  automerge:
    if: >-
      github.actor == 'dependabot[bot]'
      && github.event.pull_request.user.login == 'dependabot[bot]'
      && github.event.pull_request.head.repo.full_name == github.repository
    uses: pyrlyn/ci/.github/workflows/dependabot-automerge.yml@<sha> # main
    permissions:
      contents: write
      pull-requests: write
      actions: read
    with:
      ci-result: success # the real gate is wait-workflow
      wait-workflow: ci.yml # the workflow that reports the required checks
```

Do not call the repository's CI (`uses: ./.github/workflows/ci.yml`) from this workflow: the
Dependabot pull request already runs it through its own `pull_request` trigger, so the call is
a second full CI run on the same commit. `wait-workflow` waits for that run and refuses to
merge unless it is green (docs/migration/_common/dependabot.yml is the caller to copy).

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
    uses: pyrlyn/ci/.github/workflows/sonarcloud.yml@<sha> # main
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
      - uses: pyrlyn/ci/.github/actions/revert-on-failure@<sha> # main
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
- uses: pyrlyn/ci/.github/actions/macos-sign@<sha> # main
  if: runner.os == 'macOS'
  with:
    mode: discover
    certificate: ${{ secrets.MACOS_CERTIFICATE }}
    certificate-password: ${{ secrets.MACOS_CERTIFICATE_PWD }}
    require: "true"
```

## setup-rust (composite action)

The Rust of the caller's `mise.toml` for a repository's own jobs (FFI builds, packaging,
platform tests): `jdx/mise-action` installs it, `RUSTUP_TOOLCHAIN` makes it the active
toolchain for every later rustup/cargo call, and the step fails when `rustc` is anything else
(e.g. the runner image's stable). The same steps ci-rust.yml runs before clippy. Replaces the
per-repository `.github/actions/rust` copies.

| Input | Default | Notes |
| --- | --- | --- |
| `install-args` | `rust` | `mise install` arguments; must include `rust` (e.g. `rust node`) |
| `working-directory` | `.` | directory with the mise.toml |
| `targets` | `""` | extra rustup targets, space-separated |
| `components` | `""` | extra rustup components, space-separated |
| `cache` | `"false"` | `"true"` adds Swatinem/rust-cache (saved on the default branch) |
| `cache-key` | `""` | extra rust-cache key |

Output: `version` (the pinned rustc version).

```yaml
      - uses: actions/checkout@<sha> # v7.0.1
      - uses: pyrlyn/ci/.github/actions/setup-rust@<sha> # main
        with:
          targets: wasm32-unknown-unknown
```

## commits (composite action)

Conventional Commits subjects of a pull request's commits, from the base branch to HEAD: merge
commits and git's own `Revert "..."` subjects are skipped, every offender is listed as an
`::error::` before the step fails. Outside `pull_request` there is no range: a notice, success.
Needs a checkout with `fetch-depth: 0`. ci.yml runs it as the `commits` check
(`commits: {enabled: true}` in infra.yml); a repository whose ruleset requires a check named
`commits` calls the action from its own `commits` job instead, so the name stays.

| Input | Default | Notes |
| --- | --- | --- |
| `tool` | `grep` | `grep`: `type(scope)!: subject` with `types`; `commitlint`: the repository's commitlint config (`npm ci`, Node from mise.toml) |
| `types` | `feat fix docs ci test chore style refactor perf build` | space-separated, `grep` only |
| `working-directory` | `.` | package.json and the commitlint config (`commitlint` only) |

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
      - uses: pyrlyn/ci/.github/actions/setup-xcode@<sha> # main
        with:
          version: "27"
```

A caller whose own workflows use `runs-on: xcode-27` and lint them with actionlint 1.7.12 adds
the label under `self-hosted-runner.labels` in `.github/actionlint.yaml` (as this repository
does): that actionlint release predates the image.
