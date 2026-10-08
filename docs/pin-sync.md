# Pin sync

Consumers pin pyrlyn/ci by commit (`pyrlyn/ci/<path>@<sha> # main`). Moving those pins by hand
in every repository drifts: repositories end up on different commits and copies of the same
caller file diverge. pin-sync keeps them on one commit.

## What it does

[`pin-sync.yml`](../.github/workflows/pin-sync.yml) runs weekly (Monday 05:23 UTC) or by hand
(`sha`, `repo`, `dry-run` inputs). For each target in
[`tools/pin-sync/targets.yml`](../tools/pin-sync/targets.yml) it:

1. clones the default branch;
2. rewrites every `pyrlyn/ci/<path>@<40-hex>` reference in `.github/**/*.yml` (the target's
   `paths`, minus `exclude`) to the synced commit, with the comment `# main`;
3. renders each `templates` entry: the file in [`templates/`](../templates/) with `{{sha}}` and
   the entry's `vars` filled in, replacing `dest` whole;
4. when anything changed, force-pushes `ci/pin-sync` and opens (or updates) one pull request.

It never merges. The pull request runs the target's own CI; a human merges it after checking
the pyrlyn/ci changes since the previous pin (see [consumers.md](consumers.md) for pin upgrades
that need caller changes, such as wider permissions).

The script is [`tools/pin-sync/pin_sync.py`](../tools/pin-sync/pin_sync.py) (Python standard
library, `gh` and `git`). `pin_sync.py apply --root <checkout> ...` rewrites one local checkout
and is what [`tests/pin-sync/test.sh`](../tests/pin-sync/test.sh) runs.

## Templates

[`templates/dependabot.yml`](../templates/dependabot.yml) is the shared Dependabot caller
(`wait-workflow` var). A file rendered from a template is overwritten on every sync, so a
repository that needs a different file drops its entry from targets.yml. sync-docs callers are
not templated: their `with:` blocks differ per repository.

## Setup

Writing needs the `PIN_SYNC_TOKEN` org secret, visible to pyrlyn/ci: a GitHub App installation
token or a fine-grained PAT with Contents, Pull requests and Workflows read/write on every
target (Workflows because every change is under `.github/`). Without it the run is a dry run
that lists what would change; a manual run then fails with a clear error, the weekly one only
warns.

Pull requests opened with a GitHub App or PAT token run the target's CI; ones opened with
`GITHUB_TOKEN` would not, which is why that token is not used.
