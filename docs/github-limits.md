# GitHub constraints for a central workflow repository

listepo is a personal account on the Free plan, not an organization. What that means for
"every workflow lives in pyrlyn/ci":

| Constraint | Consequence | Source |
| --- | --- | --- |
| A reusable workflow runs only when a caller workflow in the consuming repository triggers it | every consumer keeps a thin `.github/workflows/*.yml` with the `on:` triggers and a `uses:` job | [Reusing workflow configurations](https://docs.github.com/en/actions/reference/workflows-and-actions/reusing-workflow-configurations) |
| "Require workflows to pass before merging" rulesets are configured at organization or enterprise level | not available on a user account; nothing can inject a workflow into a repository | [Available rules for rulesets](https://docs.github.com/en/enterprise-cloud@latest/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/available-rules-for-rulesets) |
| Public caller -> only public reusable workflows; private caller -> public or private (private host needs its Access policy set) | pyrlyn/ci is public, so every listepo repository can call it; the Access policy API answers 422 for public repositories | same as row 1 |
| Up to 10 levels of nesting; at most 50 unique reusable workflows per top-level caller file (tree included) | deepest chain today: caller -> pipeline.yml -> ci-rust.yml (3 levels) | same as row 1 |
| `GITHUB_TOKEN` permissions can only stay the same or shrink down the chain; a nested job asking for more than its caller grants fails the run at startup, even if the job would be skipped | the calling job must grant the union of what every called job declares (see docs/reusable-workflows.md); since cancel-run this includes `actions: write` | same as row 1 |
| Workflow-level `env` of the caller is not propagated; `github` context is the caller's | pass values as `inputs`; `github.run_id`, `github.workflow`, `github.repository` inside infra refer to the consumer | same as row 1 |
| `concurrency` with the same group in caller and callee cancels the running caller | callee groups must not reuse `${{ github.workflow }}` alone (it is the caller's name) | same as row 1 |
| Secrets never flow implicitly | declare them in the caller's `secrets:` (explicit) or `secrets: inherit` | same as row 1 |
| `$/` self-repository references resolve to the file's own repository at the running commit (github.com, runner >= 2.336.0; no `@ref`) | infra's workflows call siblings with `$/` so a consumer's SHA pin covers the whole tree | [Changelog 2026-07-30](https://github.blog/changelog/2026-07-30-reference-same-repository-actions-with-self-repository-syntax/), [Reuse workflows](https://docs.github.com/en/actions/how-tos/reuse-automations/reuse-workflows) |
| Dependabot-triggered `pull_request` runs get a read-only `GITHUB_TOKEN` and only Dependabot secrets; `permissions:` may raise the token | SNYK_TOKEN/SONAR_TOKEN are empty there (jobs skip); cancel-run needs `actions: write` granted by the caller | [Troubleshooting Dependabot on GitHub Actions](https://docs.github.com/en/code-security/dependabot/troubleshooting-dependabot/troubleshooting-dependabot-on-github-actions) |
| Dependabot's `github-actions` ecosystem updates `uses:` refs of actions and reusable workflows, including SHA pins | consumers' `@<sha>` pins on infra move by Dependabot PR | [Keeping your actions up to date with Dependabot](https://docs.github.com/en/code-security/dependabot/working-with-dependabot/keeping-your-actions-up-to-date-with-dependabot) |
| `.github/dependabot.yml`, CODEOWNERS, rulesets are read per repository | cannot be centralized; keep templates here | – |
| Code scanning upload is free only for public repositories | private callers pass `upload-sarif: false` | docs/reusable-workflows.md |
| A required check that is `skipped` counts as passed | `pipeline / gate` runs with `always()` and fails on anything but success or an input-disabled job | [Troubleshooting required status checks](https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/collaborating-on-repositories-with-code-quality-features/troubleshooting-required-status-checks) |
