#!/usr/bin/env bash
# One tracking issue per workflow for CI failures on main. A failed run opens the issue (label
# $LABEL, assigned to $WHO, @mention in the body) or comments on the open one; a green run
# closes it. Idempotent: a run attempt that already reported is not reported again, and a green
# run with no open issue does nothing. Cancelled or skipped jobs alone (a manual cancel) are
# neither a failure nor green, so nothing happens.
#
# Env: GH_REPO, GH_TOKEN (issues: write), NEEDS (`toJSON(needs)` of the calling job, gate
# included), GATE (the gate job's id, default gate), WHO (mentioned and assigned), LABEL
# (default ci-main-failure), WORKFLOW, RUN_ID, RUN_ATTEMPT, SHA, BRANCH (default main),
# GITHUB_SERVER_URL.
set -euo pipefail
: "${GH_REPO:?}" "${NEEDS:?}" "${WHO:?}" "${WORKFLOW:?}" "${RUN_ID:?}"
GATE="${GATE:-gate}"
LABEL="${LABEL:-ci-main-failure}"
BRANCH="${BRANCH:-main}"
RUN_ATTEMPT="${RUN_ATTEMPT:-1}"
server="${GITHUB_SERVER_URL:-https://github.com}"
run_url="$server/$GH_REPO/actions/runs/$RUN_ID/attempts/$RUN_ATTEMPT"
marker="<!-- ci-main-failure workflow=$WORKFLOW -->"
run_marker="<!-- ci-main-failure run=$RUN_ID/$RUN_ATTEMPT -->"

gate="$(jq -r --arg g "$GATE" '.[$g].result // ""' <<<"$NEEDS")"
failed="$(jq -r --arg g "$GATE" 'to_entries[] | select(.key != $g)
  | select(.value.result == "failure") | .key' <<<"$NEEDS")"

issue="$(gh issue list --state open --label "$LABEL" --limit 100 --json number,body \
  | jq -r --arg m "$marker" '[.[] | select(.body | contains($m))][0].number // empty')"

if [ "$gate" = success ]; then
  if [ -n "$issue" ]; then
    gh issue close "$issue" --comment "CI is green on \`$BRANCH\` again: $run_url" >/dev/null
    echo "::notice title=$LABEL::closed #$issue ($WORKFLOW is green on $BRANCH)" >&2
  else
    echo "Green on $BRANCH, no open $LABEL issue: nothing to do." >&2
  fi
  exit 0
fi
if [ -z "$failed" ]; then
  echo "Gate '$gate' without a failed job (cancelled or skipped): nothing reported." >&2
  exit 0
fi

# The backticks are Markdown, not shell.
# shellcheck disable=SC2016
jobs="$(sed 's/^/- `/; s/$/`/' <<<"$failed")"
body="$(mktemp)"
# The backticks are Markdown, not shell.
# shellcheck disable=SC2016
{
  printf '@%s %s failed on `%s`.\n\n' "$WHO" "$WORKFLOW" "$BRANCH"
  printf -- '- Run: %s\n- Commit: %s\n\nFailed jobs:\n%s\n\n%s\n' "$run_url" \
    "$server/$GH_REPO/commit/${SHA:-}" "$jobs" "$run_marker"
} >"$body"

if [ -n "$issue" ]; then
  if gh api --paginate "repos/$GH_REPO/issues/$issue/comments" --jq '.[].body' \
    | grep -qF -- "$run_marker"; then
    echo "Run $RUN_ID/$RUN_ATTEMPT already reported on #$issue." >&2
    exit 0
  fi
  gh issue comment "$issue" --body-file "$body" >/dev/null
  echo "::notice title=$LABEL::commented on #$issue" >&2
else
  gh label create "$LABEL" --color B60205 \
    --description "CI failing on $BRANCH (opened by the main-failure action)" >/dev/null 2>&1 || true
  printf '\n%s\n' "$marker" >>"$body"
  url="$(gh issue create --title "CI failing on $BRANCH: $WORKFLOW" --label "$LABEL" \
    --body-file "$body")"
  issue="${url##*/}"
  echo "::notice title=$LABEL::opened $url" >&2
fi
rm -f "$body"
# Assigning can fail (not a collaborator); the mention still notifies.
gh issue edit "$issue" --add-assignee "$WHO" >/dev/null \
  || echo "::warning::could not assign $WHO to #$issue" >&2
