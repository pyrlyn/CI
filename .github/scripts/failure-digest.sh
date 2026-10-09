#!/usr/bin/env bash
# Daily failure digest: one @mention comment on the "Daily failure digest" issue of $GH_REPO
# listing, per repository, what failed in the last $SINCE_HOURS hours on the default branch
# only: failed workflow runs (failure, timed_out, startup_failure) of push, schedule,
# workflow_dispatch and workflow_run events on the default branch (pull requests, other
# branches and tags do not count), and open release-failure issues with activity in the window. Only failing repositories are listed;
# when nothing failed anywhere it posts nothing at all.
#
# Env: REPOS (owner/name list, separated by spaces, commas or newlines), GH_REPO (where the
# digest issue lives), GH_TOKEN (actions: read on REPOS, issues: write on GH_REPO), WHO
# (mentioned and assigned, default listepo), SINCE_HOURS (default 24), DIGEST_TZ (default
# Europe/Kyiv), LABEL (default failure-digest), RELEASE_LABEL (default release-failure),
# DRY_RUN=true prints the digest instead of posting it, NOW (epoch seconds; test hook).
# Needs gh, jq and GNU date.
set -euo pipefail
: "${REPOS:?}" "${GH_REPO:?}"
WHO="${WHO:-listepo}"
SINCE_HOURS="${SINCE_HOURS:-24}"
DIGEST_TZ="${DIGEST_TZ:-Europe/Kyiv}"
LABEL="${LABEL:-failure-digest}"
RELEASE_LABEL="${RELEASE_LABEL:-release-failure}"
here="$(cd "$(dirname "$0")" && pwd)"
notify="$here/../actions/notify-release-failure/notify-issue.sh"

now="${NOW:-$(date +%s)}"
since_epoch=$((now - SINCE_HOURS * 3600))
since="$(date -u -d "@$since_epoch" +%Y-%m-%dT%H:%M:%SZ)"
local_time() { TZ="$DIGEST_TZ" date -d "$1" '+%Y-%m-%d %H:%M %Z'; }

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
: >"$tmp/digest"

read -r -a repos <<<"$(tr ',\n' '  ' <<<"$REPOS")"
for repo in "${repos[@]}"; do
  default="$(gh api "repos/$repo" --jq .default_branch)"
  # Completed failed runs created in the window, pull request events left out.
  gh api --paginate "repos/$repo/actions/runs?created=>=$since&status=completed&per_page=100" \
    --jq '.workflow_runs[]
      | select(.conclusion == "failure" or .conclusion == "timed_out"
        or .conclusion == "startup_failure")
      | select(.event == "push" or .event == "schedule" or .event == "workflow_dispatch"
        or .event == "workflow_run")
      | [.id, .run_attempt, .name, .event, .head_branch, .html_url, .created_at, .conclusion]
      | @tsv' >"$tmp/runs"
  : >"$tmp/section"
  while IFS=$'\t' read -r id attempt name event ref url created conclusion; do
    # Only the default branch notifies; other branches and tags are not main.
    [ "$ref" = "$default" ] || continue
    jobs="$(gh api --paginate "repos/$repo/actions/runs/$id/attempts/$attempt/jobs?per_page=100" \
      --jq '.jobs[] | select(.conclusion == "failure" or .conclusion == "timed_out") | .name' \
      2>/dev/null | paste -sd, - | sed 's/,/, /g' || true)"
    [ -n "$jobs" ] || jobs="none listed ($conclusion)"
    # The backticks are Markdown, not shell.
    # shellcheck disable=SC2016
    printf -- '- **%s** on `%s` (%s), %s: [run](%s); failed jobs: %s\n' \
      "$name" "$ref" "$event" "$(local_time "$created")" "$url" "$jobs" >>"$tmp/section"
  done <"$tmp/runs"
  # Open release-failure issues with activity (opened or commented) in the window.
  gh api "repos/$repo/issues?labels=$RELEASE_LABEL&state=open&since=$since&per_page=100" \
    --jq '.[] | select(.pull_request == null)
      | "- Release failure issue: [#\(.number) \(.title)](\(.html_url))"' >>"$tmp/section"
  if [ -s "$tmp/section" ]; then
    { printf '### %s\n\n' "$repo"; cat "$tmp/section"; printf '\n'; } >>"$tmp/digest"
  fi
done

if [ ! -s "$tmp/digest" ]; then
  echo "No failures in the last ${SINCE_HOURS}h across: ${repos[*]}; nothing posted."
  exit 0
fi

{
  printf '@%s failures in the last %sh (since %s):\n\n' "$WHO" "$SINCE_HOURS" \
    "$(local_time "$since")"
  cat "$tmp/digest"
} >"$tmp/body"
if [ "${DRY_RUN:-false}" = true ]; then
  cat "$tmp/body"
  exit 0
fi
cat >"$tmp/issue" <<BODY
Tracking issue for the daily failure digest (pyrlyn/ci \`failure-digest.yml\`): every day
that something failed in ${repos[*]}, a comment here lists it and mentions @$WHO.
Days without failures post nothing.
BODY
TITLE="Daily failure digest" MARKER="<!-- failure-digest -->" LABEL="$LABEL" \
  LABEL_COLOR=D93F0B LABEL_DESCRIPTION="Daily failure digest (failure-digest.yml)" \
  WHO="$WHO" BODY_FILE="$tmp/body" ISSUE_BODY_FILE="$tmp/issue" \
  bash "$notify" >/dev/null
