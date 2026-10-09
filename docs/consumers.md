# Consumers (2026-09-29)

Repositories with Actions enabled that call pyrlyn/ci, and the infra commit they pin.

| Repository | Workflow -> infra workflow | Pin | Caller permissions |
| --- | --- | --- | --- |
| cox (main, PR #65 p37-desktop) | ci.yml `rust` -> ci-rust.yml | `189816a` | none on the job (workflow `contents: read`) |
| cox | pipeline.yml -> pipeline.yml (`rust: false`) | `189816a` | `contents: read`, `security-events: write`, `actions: read` |
| cox | release-plz.yml -> release-plz.yml | `189816a` | `contents: write`, `pull-requests: write`, `actions: write` |
| ketch | ci.yml `rust` -> ci-rust.yml | `059c0c5` | none on the job (workflow `contents: read`) |
| ketch | pipeline.yml -> pipeline.yml | `189816a` | `contents: read`, `security-events: write`, `actions: read` |
| ketch | release-apple-desktop.yml -> release-apple-desktop.yml | ci/release-apple-desktop head (repin to the merge commit) | `contents: write`, `actions: read`, `issues: write` |
| rtok | pipeline.yml -> pipeline.yml + `gate` action | `189816a` | `contents: read`, `security-events: write`, `actions: read` |
| rtok | release-plz.yml -> release-plz.yml | `189816a` | `contents: write`, `pull-requests: write`, `actions: write` |

Actions disabled (not consumers today): bindsmith, crates-packages, cross-code, swarfr, runa,
slint_dart, stator; private airtalk-cli, budget-app, riverpod, slint-dotnet. landing (Pages only)
does not call infra.

## Upgrading a pin past cancel-run (416dbf9 / main 1be9b44)

Every workflow except dependabot-automerge.yml now declares `actions: write`, and GitHub rejects
a nested job that asks for more than its caller grants, even when `cancel-run-on-failure` is
false. Before moving a pin past 416dbf9, change the calling job's permissions:

- ci-rust.yml callers (cox, ketch `rust` jobs): add `permissions: {contents: read, actions: write}`
  on the job. Where ci.yml is itself called (dependabot.yml `ci` job), that job must grant
  `actions: write` too.
- pipeline.yml callers (cox, ketch, rtok): `actions: read` -> `actions: write`.
- release-plz.yml callers already grant `actions: write`.

## Upgrading a pin past notify-release-failure

`release.yml`, `release-plz.yml` and `bump.yml` end with a `notify-failure` job that asks for
`issues: write` (a failed release opens a `release-failure` issue; docs/reusable-workflows.md,
"Release failure notifications"). Before moving a pin of any of the three past it, add
`issues: write` to the calling job's permissions, or the run fails at startup.

## Upgrading a pin past main-failure

`ci.yml` gained a `main-failure` job (on by default) that asks for `issues: write`. Before
moving a `ci.yml` pin past it, add `issues: write` to the calling job's permissions, or the run
fails at startup even with `notify-main-failure: false`.
