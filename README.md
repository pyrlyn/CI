# pyrlyn/ci

Shared reusable GitHub workflows and build configs for pyrlyn repositories
(rtok, ketch, stator, bindsmith, runa, cox).

## Reusable workflows

| Workflow | Purpose |
| --- | --- |
| [`ci-rust.yml`](.github/workflows/ci-rust.yml) | Rust CI: fmt, clippy, check, tests on a shared four-target matrix, optional MSRV |
| [`lint.yml`](.github/workflows/lint.yml) | actionlint (+ shellcheck) on the caller's workflows |
| [`codeql.yml`](.github/workflows/codeql.yml) | CodeQL code scanning per language |
| [`semgrep.yml`](.github/workflows/semgrep.yml) | Semgrep OSS scan, SARIF to code scanning |
| [`snyk.yml`](.github/workflows/snyk.yml) | Snyk Open Source scan, off by default (switch) |
| [`pipeline.yml`](.github/workflows/pipeline.yml) | ci-rust + CodeQL + Semgrep + Snyk in parallel behind a `gate` |
| [`release-plz.yml`](.github/workflows/release-plz.yml) | release PR only (versions + changelog); never tags, releases or dispatches |
| [`release.yml`](.github/workflows/release.yml) | release build on bump's tag: checks, verify, build, sign, smoke, upload, publish |
| [`dependabot-automerge.yml`](.github/workflows/dependabot-automerge.yml) | merge allowed Dependabot updates after green CI |
| [`sonarcloud.yml`](.github/workflows/sonarcloud.yml) | SonarCloud scan, skipped without `SONAR_TOKEN` |
| [`ci.yml`](.github/workflows/ci.yml) | **single entrypoint**: config-driven (`.github/infra.yml`) Rust CI, scans, SonarCloud, lint and repo-specific jobs behind a `gate` |
| [`sync-docs.yml`](.github/workflows/sync-docs.yml) | publish docs/ to pyrlyn/landing |
| [`license-check.yml`](.github/workflows/license-check.yml) | fail when the caller's LICENSE / commercial license / README license line drift from [`licenses/`](licenses/README.md) |
| [`license-sync.yml`](.github/workflows/license-sync.yml) | open `chore/license-sync` PRs in the targets when [`licenses/`](licenses/README.md) changes; dry run and a clear error without `LICENSE_SYNC_TOKEN` |
| [`pin-sync.yml`](.github/workflows/pin-sync.yml) | weekly `ci/pin-sync` PRs that move every consumer's pyrlyn/ci pins to one main commit and refresh the files rendered from [`templates/`](templates/); see [docs/pin-sync.md](docs/pin-sync.md) |
| [`pages.yml`](.github/workflows/pages.yml) | build a static site and deploy it to GitHub Pages |
| [`bump.yml`](.github/workflows/bump.yml) | **the only release path**: version commit -> PR -> required checks -> rebase merge -> tag + Release on the landed commit -> dispatch the release build |
| [`notify-release-failure.yml`](.github/workflows/notify-release-failure.yml) | `release-failure` issue (mention + assign) when a release fails |
| [`warnings-to-issues.yml`](.github/workflows/warnings-to-issues.yml) | one issue per code scanning / SonarCloud warning (deduplicated, closed when fixed); infra's own caller is [`warnings.yml`](.github/workflows/warnings.yml) |
| [`revert-on-failure.yml`](.github/workflows/revert-on-failure.yml) | revert a failed push to the default branch |
| [`cla.yml`](.github/workflows/cla.yml) | Contributor License Agreement check (`pyrlyn/cla` action, off unless `CLA_ENABLED`); see [docs/cla.md](docs/cla.md) |

Composite actions: [`gate`](.github/actions/gate/action.yml) (fail unless every needed job
succeeded), [`revert-on-failure`](.github/actions/revert-on-failure/action.yml),
[`macos-sign`](.github/actions/macos-sign/action.yml),
[`setup-xcode`](.github/actions/setup-xcode/action.yml) (select the pinned Xcode 27 on macOS
jobs; ci-rust.yml and release.yml call it) and
[`cancel-run`](.github/actions/cancel-run/action.yml) (cancel the whole run when a job fails)
and [`notify-release-failure`](.github/actions/notify-release-failure/action.yml) (open or
update a `release-failure` issue; release.yml, release-plz.yml and bump.yml run it on failure).
`self-test.yml` runs ci-rust.yml and pipeline.yml against a fixture crate.

See [docs/config.md](docs/config.md) for the config file and [docs/migration/](docs/migration/)
for each consumer's thin callers.

## How to call

```yaml
jobs:
  pipeline:
    uses: pyrlyn/ci/.github/workflows/pipeline.yml@<full commit sha>
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

See [docs/reusable-workflows.md](docs/reusable-workflows.md) for every workflow's inputs,
secrets, required permissions and caller examples, [docs/index.md](docs/index.md) for
ci-rust.yml details, and [docs/centralization-candidates.md](docs/centralization-candidates.md)
for what else could move here, [docs/consumers.md](docs/consumers.md) for
who calls which workflow at which pin, and [docs/github-limits.md](docs/github-limits.md) for
the GitHub constraints (with sources) that shape this repository.

## License

[GPL-3.0](LICENSE)
