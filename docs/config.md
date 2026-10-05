# Config-driven CI (`ci.yml` + `.github/infra.yml`)

Goal: a consumer switches checks on/off and tunes them by editing YAML, never a workflow file.
The consumer's only CI workflow is a thin caller of `ci.yml` (see docs/migration/).

## Layout (chosen)

Three layers, later wins key by key (maps deep-merge, lists are replaced):

1. `.github/actions/config/defaults.yml` in infra: every key with its default.
2. `config/repos/<repository name>.yml` in infra (optional): central per-repo overrides,
   read at the commit the consumer pinned.
3. `.github/infra.yml` in the consumer (optional; path = `config-file` input): local overrides.

A `config` job (composite action `.github/actions/config`) sparse-checks-out only the config
file, merges with `yq` (preinstalled on GitHub-hosted runners), normalizes with a stdlib
Python script and emits `json`, `jobs`, `skip-ok`, `draft-skip`. Every job in ci.yml has
`needs: config` and `if: fromJSON(needs.config.outputs.json).<check>.run`; inputs come from
the same JSON.

## Options compared

| Option | + | − |
| --- | --- | --- |
| A. Local `.github/infra.yml` only | change ships with the code it tests (same PR); no infra re-pin; reviewers see it | one more file per repo |
| B. Central `config/repos/<repo>.yml` only | all settings in one repo; consumers truly hold only the wrapper | every tweak = infra PR + re-pin in the consumer (the pin also freezes the config); PR in a consumer cannot change its own CI |
| C. Inputs on the caller (`with:`) | no parsing | the caller file grows back into logic; that is what we are removing |
| D. Repository variables (`vars.*`) | no file | not versioned, not reviewable, strings only |
| **Chosen: defaults + optional central + local** | A for day-to-day; B available for fleet-wide policy (e.g. force `snyk: {enabled: false}` everywhere) | two places to look; the step summary prints what ran and why |

## GitHub constraints that shape the design

- `uses:` of a job cannot be an expression, so ci.yml lists every possible reusable call and
  turns each off with `if:`; new check types need an infra change.
- `if:` may read `needs.<job>.outputs`; `with:` may too; matrices take `fromJSON(...)`
  (repository-specific jobs are one matrix job, max 256 entries).
- A skipped required check counts as passed, so `gate` runs with `always()` and fails on any
  non-success except jobs listed in `skip-ok` (disabled by config or event filter). A failed
  `config` job fails the gate. On a draft PR with `skip-drafts`, the gate itself is skipped
  (same as pipeline.yml).
- Permissions are validated for every nested job up front, even disabled ones: the caller
  grants the union (`contents: read`, `security-events: write`, `pull-requests: read`,
  `actions: write`) regardless of what the config enables.
- Steps cannot be generated, so custom jobs have a fixed step shape (checkout, env, mise,
  rustc pin check, rust-cache, taiki-e tools, setup, run). Anything else goes into a script in
  the consumer repository (content, not workflow logic).
- Secrets cannot come from config; ci.yml declares SNYK_TOKEN and SONAR_TOKEN.

## Schema (version 1)

```yaml
version: 1                      # required
cancel-run-on-failure: true     # cancel the run when a job fails (input "false" overrides)
skip-drafts: true               # draft PRs run nothing
upload-sarif: auto              # true | false | auto (= public repositories)
docs-only:                      # skip heavy checks on documentation-only changes (below)
  enabled: false
  events: [pull_request, merge_group]
  paths: []                     # extended regexes of more documentation files
  exclude: []                   # extended regexes of Markdown that is code

# Each check: enabled (true | false | auto = public repos only), events (list of
# github.event_name; [] = all), docs-only (runs on a docs-only change; false except lint)
# + the inputs of the matching reusable workflow.
rust:       {enabled: false, events: [], matrix: [], rust-version: "", fmt-runs-on: ubuntu-latest,
             working-directory: ".", mise-install-args: rust, clippy-args: "", tools: "",
             setup-command: "", test-command: "...", doc-tests: true, build-command: "...",
             package-args: --workspace, feature-args: --all-features, msrv: "",
             msrv-command: "...", timeout-minutes: 60, cache-all-refs: false,
             changed-only: false, full-package-args: --workspace}
dotnet:     {enabled: true, dotnet-version: "", working-directory: ".", test-command: "",
             changed-only: false, runs-on: ubuntu-latest, timeout-minutes: 60}
codeql:     {enabled: auto, languages: [actions], build-mode: none, build-command: "",
             queries: security-and-quality, config-file: "", runs-on: ubuntu-latest}
semgrep:    {enabled: auto, config: p/default, extra-args: "", fail-on-findings: false}
snyk:       {enabled: false, args: --all-projects, monitor: true}  # off org-wide (switch)
sonarcloud: {enabled: false, organization: "", project-key: "", args: "", project-base-dir: ".",
             mise: true, mise-install-args: "", rust: false, setup-command: "",
             coverage-command: "", soft-fail: true, timeout-minutes: 60}
lint:       {enabled: true, actionlint-version: 1.7.12, args: "", extra-command: ""}

jobs:                           # repository-specific jobs -> `ci / <name>`
  - name: footprint             # required
    run: bash scripts/footprint.sh --check   # required (bash, `set -euo pipefail`)
    enabled: true
    events: []                  # e.g. [pull_request]
    runs-on: ubuntu-latest      # or a list: one job per runner
    matrix: []                  # list of maps; keys exported UPPER_CASE; `runs-on` key picks the runner
    mise: true                  # true = all of mise.toml, false = none, "rust node" = install args
    rust-pin: false             # fail unless rustc == mise.toml pin (needs mise)
    rust-cache: false
    tools: ""                   # taiki-e/install-action list, e.g. "nextest,cargo-deny"
    env: {}                     # exported before setup/run
    setup: ""                   # bash before run
    working-directory: "."
    shell: bash                 # bash | pwsh
    timeout-minutes: 60
    fetch-depth: 1
    allow-failure: false
    docs-only: false            # true = also runs on a docs-only change (e.g. a docs linter)
```

Unknown keys fail the `config` job (typos never silently disable a check).

`dotnet` is on by default and costs one short job: it passes as a no-op until the repository
has a .NET project, then runs the full `dotnet test` (ci-dotnet.yml). `rust.changed-only` and
`dotnet.changed-only` skip the work, never the check, when the change has no file of that
ecosystem; a dependency manifest or lock change always runs the full suite (see
"Dependency-driven suite selection" in docs/reusable-workflows.md).
Example: tests/fixtures/infra.yml (self-test), docs/migration/*/infra.yml.

### Docs-only changes

`docs-only.enabled: true` makes a pull request (or merge group; `docs-only.events`) that
changes documentation only skip every check and custom job whose own `docs-only` is false:
Rust, .NET, CodeQL, Semgrep, Snyk and SonarCloud by default, while lint and custom jobs with
`docs-only: true` still run. The skipped jobs go to `skip-ok`, so `gate` (and a required
`ci / gate` or a caller's `gate`) reports success instead of leaving a required check
pending, which a workflow-level `paths-ignore` would do.

Detection is the `changes` action (`docs_only` output; docs/reusable-workflows.md): the
pull request's three-dot diff, i.e. `git diff --name-only BASE...HEAD`, from the compare API.
It is docs-only when there is a diff, nothing forced the run, and every changed path (both
sides of a rename) matches `\.md$` (any case, any directory, e.g. `docs/uk/*.md`) or a
`paths` regex, and none matches an `exclude` regex. It fails open: no diff (schedule,
dispatch), a `.github/`, `mise.toml` or `.tool-versions` change, 300+ files or an API error
mean not docs-only, so everything runs. Pushes to main are not in the default `events`.

List in `exclude` every Markdown file that is code: embedded in a binary (`include_str!`),
shipped in a package (plugin or skill files, a crate README), executed or asserted on by a
test the skipped checks would run, or read by a release (CHANGELOG). A docs-only change to
Markdown that a test validates still needs that test: give the repository a cheap custom job
(or local job) with `docs-only: true` that runs it.

The config action exposes the verdict as its `docs-only` output (and ci.yml as the `docs-only`
workflow output), so a caller with local jobs can run the action itself and skip them too. The
verdict does not depend on `skip-drafts`: on a draft pull request `draft-skip` turns every check
off as before, and `docs-only` is still `true` for a documentation-only change, so a caller
that runs its local jobs on drafts skips their heavy work there too:

```yaml
  changes:
    runs-on: ubuntu-latest
    outputs:
      docs-only: ${{ steps.config.outputs.docs-only }}
    steps:
      - id: config
        uses: pyrlyn/infra/.github/actions/config@<sha> # main
  heavy:
    needs: changes
    if: needs.changes.outputs.docs-only != 'true'
```

A required check from a matrix job, or from a job of a called workflow, cannot be skipped at
job level (the skipped job reports one check under its raw name and the required ones wait).
For ci-rust.yml pass `skip: ${{ needs.changes.outputs.docs-only == 'true' }}`: the matrix still
expands and every step is a no-op. Gate the steps of a local matrix job the same way.
