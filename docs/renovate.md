# Renovate (mise only)

`pyrlyn/ci` runs self-hosted [Renovate](https://docs.renovatebot.com/) for the whole pyrlyn
organization from [`.github/workflows/renovate.yml`](../.github/workflows/renovate.yml). It only
handles [mise](https://mise.jdx.dev/): the tool versions in `mise.toml` and the `mise.lock`
lock files. Cargo and GitHub Actions stay with Dependabot (each repository's
`.github/dependabot.yml`), because Dependabot cannot read `mise.toml`
([dependabot-core#12320](https://github.com/dependabot/dependabot-core/issues/12320)).

Why self-hosted rather than the Mend Renovate app: updating `mise.lock` runs `mise lock`, which
Renovate treats as an unsafe execution. It is allowed only by the global, self-hosted setting
`allowedUnsafeExecutions: ["mise"]`, which repository config cannot set
([mise manager](https://docs.renovatebot.com/modules/manager/mise/),
[`allowedUnsafeExecutions`][unsafe];
checked 2026-10-08, Renovate 44.145.1).

## Files

| File | Kind | What it sets |
| --- | --- | --- |
| [`renovate/global.json`](../renovate/global.json) | self-hosted (global) config | `platform: github`, `autodiscover` with `autodiscoverFilter: ["pyrlyn/*"]`, `onboarding: false`, `requireConfig: optional`, `allowedUnsafeExecutions: ["mise"]`, and `extends` the shared preset |
| [`renovate/mise.json`](../renovate/mise.json) | shared preset (repository config) | `enabledManagers: ["mise"]`, one grouped PR (`mise tools`) per repository, a separate PR for major updates, `lockFileMaintenance`, `minimumReleaseAge: 7 days` (as Dependabot's cooldown), Conventional Commits (`chore(mise): ...`), label `dependencies`, no Dependency Dashboard issue, test fixtures ignored |

No repository needs a `renovate.json`: with `onboarding: false` and `requireConfig: optional`,
Renovate runs on every discovered repository with the preset. A repository that wants something
different adds its own `renovate.json`, which is merged over the preset (for example
`{"enabled": false}` to opt out).

The preset is read as `local>pyrlyn/ci//renovate/mise` from this repository's default branch,
so a change to it takes effect on the next run after it is merged.

## Schedule

The workflow runs every Monday at 03:00 UTC (06:00 Kyiv in summer, 05:00 in winter) and on
demand (`workflow_dispatch`). The preset itself sets `schedule: ["at any time"]` (also for
`lockFileMaintenance`, whose own default is "before 4am on monday"), so the workflow's cron is
the only schedule. Updates that are younger than 7 days wait for a later run.

A manual run takes two inputs:

- `dry-run`: `off` (a real run), `extract`, `lookup` or `full`
  ([`dryRun`](https://docs.renovatebot.com/self-hosted-configuration/#dryrun)).
  `lookup` lists what Renovate found and which updates it would propose, without writing.
- `log-level`: `info` or `debug`.

```bash
gh workflow run renovate.yml -R pyrlyn/ci -f dry-run=lookup -f log-level=debug
```

## Credentials

Renovate needs a token that can push branches and open pull requests in every repository it
updates. The workflow uses a GitHub App and mints an installation token per run with
`actions/create-github-app-token` (`owner: pyrlyn`, so the token covers every repository the
app is installed on). Nothing is created by this repository; the setup is:

1. Create a GitHub App owned by the pyrlyn organization (for example `pyrlyn-renovate`), with
   webhooks off and these repository permissions
   ([Renovate: running as a GitHub App][renovate-app]):

   | Permission | Access |
   | --- | --- |
   | Checks | Read and write |
   | Commit statuses | Read and write |
   | Contents | Read and write |
   | Issues | Read and write |
   | Pull requests | Read and write |
   | Workflows | Read and write |
   | Administration | Read |
   | Dependabot alerts | Read |
   | Metadata | Read |

   and the organization permission Members: Read.
2. Install it on the pyrlyn organization (all repositories, or the ones Renovate should
   update).
3. Add two secrets that `pyrlyn/ci` can read (organization secrets limited to `pyrlyn/ci`, or
   repository secrets of `pyrlyn/ci`):

   | Secret | Value |
   | --- | --- |
   | `RENOVATE_APP_CLIENT_ID` | the app's Client ID |
   | `RENOVATE_APP_PRIVATE_KEY` | a private key generated for the app (PEM) |

Without the secrets a scheduled run is skipped with a warning, and a manual run fails with an
error naming them. Renovate detects the app's bot user and commit author from the token.

Pull requests opened by the app come from `<app-slug>[bot]`. Where the CLA check
([`cla.yml`](cla.md)) is enabled, add that login to its allowlist.

[renovate-app]: https://docs.renovatebot.com/modules/platform/github/#running-as-a-github-app
[unsafe]: https://docs.renovatebot.com/self-hosted-configuration/#allowedunsafeexecutions
