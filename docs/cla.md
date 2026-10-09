# Contributor License Agreement (CLA)

The pyrlyn projects are published under `GPL-3.0-only` and also offered under a royalty-free
license and a paid commercial license (`LICENSE`, `LICENSE-ROYALTY-FREE.md`, `PRICING.md` in each
project). Offering contributed code under the second and third requires a license from every
contributor, so outside contributions need a signed CLA.

- [`CLA.md`](../CLA.md): the agreement (English, canonical).
- [`CLA.ru.md`](../CLA.ru.md): Russian translation; the English text prevails.
- [`.github/workflows/cla.yml`](../.github/workflows/cla.yml): the check, reusable from every
  repository through a thin caller. It runs the [`pyrlyn/cla`](https://github.com/pyrlyn/cla)
  action (design: its `PLAN.md`).

> These are drafts, not legal advice. Have a lawyer review `CLA.md` before it is used.

## What the agreement says

The contributor keeps the copyright and grants Ivan Tugay a perpetual, worldwide,
non-exclusive, royalty-free, irrevocable copyright license, with the right to sublicense and to
license the contribution under any terms (GPL, the royalty-free and commercial licenses,
proprietary products), plus an Apache-style patent license. In return, a contribution in a
published GPL version of a project stays available in that project under `GPL-3.0-only`. It also
covers: representations (original work, employer permission), no obligation to use, no warranty,
a moral rights waiver where the law allows it (a consent to the uses otherwise), transfer to a
successor, and signing by pull request comment. One signature covers all pyrlyn repositories.

It is modelled on the Apache Individual CLA v2.2 (copyright and patent grants, representations),
the Harmony Agreements HA-CLA-I with the "any license" outbound option (relicensing plus a
commitment to keep the project license), and the SAP CLA that CLA Assistant uses by default
(signing by GitHub identity). The text is new; none of it is copied.

## Current state: the check blocks nobody

The check is **off**: the `cla` job runs only when the variable `CLA_ENABLED` is `true`
(organization or repository variable, Settings > Secrets and variables > Actions > Variables).
`CLA_ENABLED` is not set anywhere, so the job is skipped in every repository, no comment is
posted, no `pyrlyn/cla` status is written, and nothing is asked of anyone: neither organization
members nor outside contributors.

`pyrlyn/cla` v1.0.0 has no advisory mode. Once `CLA_ENABLED=true`:

- **Organization members** of `pyrlyn` (`members-pass: true`, set in `cla.yml`) and allowlisted
  bots are covered without signing: status `success`.
- **Outside contributors** who have not signed get the sticky comment and a **`failure`** status
  `pyrlyn/cla` until they sign. The status blocks a merge only if `pyrlyn/cla` is a required
  check in the repository's ruleset; it is not required anywhere today. Do not add it while the
  CLA must not block anyone.

## How signing works (when enabled)

1. A contributor opens a pull request. The `cla` check collects the opener, every commit author,
   every committer other than `web-flow` and every `Co-authored-by:` trailer, and decides for each
   one: organization member, allowlisted bot, signed (at least `minimum-version`), missing, or
   unknown email. It posts or edits **one** comment (marker `<!-- pyrlyn-cla -->`) with that
   table and sets the commit status **`pyrlyn/cla`** on the head commit.
2. Each person who is missing comments on the pull request, signed in to GitHub, alone:
   `I have read the CLA Document and I hereby sign the CLA`
3. The comment triggers the check again (`issue_comment`). It appends a signature (GitHub login,
   numeric user ID, comment ID and URL, time, repository, pull request, CLA version, document URL
   and SHA-256) to `signatures/cla.json` in `pyrlyn/cla-signatures` and writes the status on the
   pull request head itself, so nothing is re-run. Commenting `recheck` runs the check again
   without signing.
4. After the merge, the conversation is locked so signature comments cannot be edited or deleted
   (`lock-after-merge`, default `true`); it is unlocked if the pull request is reopened.

The decision is the status `pyrlyn/cla`, not the job: a green job with a failing status means
someone still has to sign; a red job means the action itself failed and no status was set.

Covered without signing: current members of `pyrlyn` (checked at run time through the app token,
so private membership counts) and the bots in `allowlist` (`dependabot[bot]`,
`github-actions[bot]`, `renovate[bot]`, matched by login and numeric ID). The allowlist accepts
bot logins (ending in `[bot]`) only; `listepo` passes as a member. If the opener is not an author
or co-author of any commit, the check fails unless the opener is a member. Commits whose email is
not linked to a GitHub account cannot sign; the comment tells the author to link the email or
rewrite the commits. Dependabot pull requests skip the job entirely.

## Why the pyrlyn/cla action

Until this change `cla.yml` wrapped CLA Assistant Lite (`contributor-assistant/github-action`
v2.6.1). It was archived on 2026-03-23, declares `node20` (GitHub runs it on Node 24), cannot
see private organization membership and re-runs workflows by name (`actions: write`).
[`pyrlyn/cla`](https://github.com/pyrlyn/cla) replaces it: `node24`, organization members pass,
one sticky comment, a status written from the comment run (no re-run, no `actions: write`), a
versioned signature schema with the document SHA-256, Apache-2.0 with the ported parts credited
in its `NOTICE`. It is pinned by commit SHA (`# v1.0.0`) like every action here; Dependabot bumps
the pin.

Like Lite, it records the GitHub identity of the signer and keeps the signatures in a
repository we own, with no third-party app holding organization access. The hosted alternative,
cla-assistant.io (SAP), would keep the signatures in its own service.

## Where signatures are stored

A dedicated **private repository `pyrlyn/cla-signatures`**, branch `main`, file
`signatures/cla.json` (schema 1; the defaults of `cla.yml`).

- The credential only needs write access to that one repository. Storing signatures on a branch
  of `pyrlyn/ci` would need a credential that can push to infra, the repository every
  workflow pin points into.
- Signature commits stay out of the infra history and its pins, Dependabot and `sync-docs`.
- One file for the whole organization: a contributor signs once for every repository.
- Private keeps the list of signers from being scraped. The evidence itself (the signing comment
  on the pull request) stays public either way.

`main` of `pyrlyn/cla-signatures` must **not** be protected (the action commits to it directly).
Existing entries are never rewritten. Back it up like any legal record and never delete it.

## Versions of the agreement

`cla.yml` passes `document-url` and `document-url-ru` pinned to an infra commit, `cla-version`
(`1.0`), `minimum-version` (`1.0`) and `document-sha256` (SHA-256 of `CLA.md` at that commit).
When `CLA.md` changes, update all of them together in `cla.yml`:

```sh
git show <commit>:CLA.md | shasum -a 256
```

Raising `minimum-version` asks people who signed an older version to sign again.

## Setup (manual, by an organization owner)

Done: `pyrlyn/cla-signatures` exists and holds `signatures/cla.json`. Not done: the app and the
secrets below, `CLA_ENABLED`, a required check.

### 1. Credentials: the `pyrlyn-cla` GitHub App

The check uses two tokens:

- `github.token` of the repository the pull request is in, for the comment, the status and the
  lock. The caller job grants it `contents: read`, `pull-requests: write`, `statuses: write`.
  Nothing to create.
- A token for organization membership and the signatures file, minted per run with
  `actions/create-github-app-token` from the **`pyrlyn-cla`** GitHub App: Organization
  permission **Members: read**, Repository permission **Contents: read and write**, installed on
  `pyrlyn/cla-signatures` only. Store its ID and private key as organization secrets:

```sh
gh secret set CLA_APP_ID --org pyrlyn --visibility selected \
  --repos cox,rtok,ketch,runa,crates-packages,infra
gh secret set CLA_APP_PRIVATE_KEY --org pyrlyn --visibility selected \
  --repos cox,rtok,ketch,runa,crates-packages,infra < pyrlyn-cla.private-key.pem
```

Instead of the app, a fine-grained PAT (resource owner `pyrlyn`, repository `cla-signatures`,
Contents: Read and write, Organization Members: Read) can be stored as `CLA_SIGNATURES_TOKEN`; it
expires and must be rotated. The app wins when both are set. Without either, membership falls
back to public membership and signatures cannot be written (status `error`).

On the Free plan organization secrets reach public repositories only; all of these are public.

### 2. Add the caller to each repository

`.github/workflows/cla.yml` in the calling repository:

```yaml
name: cla

on:
  pull_request_target:
    types: [opened, synchronize, reopened, closed]
  issue_comment:
    types: [created]

permissions:
  contents: read

jobs:
  cla:
    uses: pyrlyn/ci/.github/workflows/cla.yml@<full commit sha>
    permissions:
      contents: read
      pull-requests: write
      statuses: write
    secrets:
      CLA_APP_ID: ${{ secrets.CLA_APP_ID }}
      CLA_APP_PRIVATE_KEY: ${{ secrets.CLA_APP_PRIVATE_KEY }}
```

Pass secrets by name; do not use `secrets: inherit`. Existing callers that pass
`CLA_SIGNATURES_TOKEN` and grant `actions: write` keep working (the extra permission is unused).
Inputs (all optional): `document-url`, `document-url-ru`, `document-sha256`, `cla-version`,
`minimum-version`, `signatures-organization`, `signatures-repository`, `signatures-branch`,
`signatures-path`, `allowlist` (bot logins ending in `[bot]` only; `*` wildcard),
`lock-after-merge`. `pyrlyn/ci` itself runs `cla.yml` directly on its own pull requests.

`pull_request_target` and `issue_comment` always run the workflow file of the **default
branch**, so the check starts working only after the caller is merged. The workflow never checks
out pull request code and puts no event text into `run:`; keep it that way, since
`pull_request_target` gives fork pull requests a write token and secrets.

Actions must be enabled in the repository (crates-packages has them disabled today).

### 3. Turn it on (later, only when the CLA should be enforced)

Pilot with `CLA_ENABLED=true` as a repository variable in one repository, test from a
non-member account through a fork (unsigned: comment and red status; sign: green without a
re-run; `recheck`; merge: locked; signature in `cla-signatures`), then set it per repository or
for the organization.

### 4. Make the check required (later)

Only after step 3 passed in that repository; a required check that never reports blocks every
pull request. Organization rulesets need GitHub Team (`pyrlyn` is on Free), so add the status
context **`pyrlyn/cla`** (source GitHub Actions) to each repository's `protect-main` ruleset:
Repository > Settings > Rules > Rulesets > `protect-main` > Require status checks to pass. With
`gh`:

```sh
gh api repos/pyrlyn/<repo>/rulesets            # find the protect-main id
gh api repos/pyrlyn/<repo>/rulesets/<id> > ruleset.json
jq '(.rules[] | select(.type=="required_status_checks") | .parameters.required_status_checks)
    += [{"context":"pyrlyn/cla"}]
    | {name, target, enforcement, conditions, rules, bypass_actors}' ruleset.json \
  | gh api -X PUT repos/pyrlyn/<repo>/rulesets/<id> --input -
```

Pull requests opened by `bump.yml`, `release-plz.yml` and `revert-on-failure` are committed as
`github-actions[bot]` or `listepo` (bot and member), so they pass.

### Rollback

1. Remove `pyrlyn/cla` from the required checks (unblocks pull requests at once).
2. Delete `CLA_ENABLED` or set it to anything but `true`; the job is skipped.
3. Revert this workflow change or pin `pyrlyn/cla` to an earlier release.

Always 1 before 2, otherwise pull requests wait for a status that never comes.

## Troubleshooting

- **Status `error`, "could not read signatures (HTTP 403/404)"**: the app (or PAT) is missing,
  not visible to the repository, or not installed on `pyrlyn/cla-signatures`.
- **A member is asked to sign, "membership could not be verified"**: no app token, so only
  public membership is visible; set `CLA_APP_ID` / `CLA_APP_PRIVATE_KEY`.
- **Job fails with "allowlist entry ... must be a bot login"**: a caller passes a non-bot login in
  `allowlist`; drop it (members pass anyway).
- **Signed, but the status is still red**: comment `recheck`; check that the comment holds the
  phrase alone and comes from the account that made the commits.
