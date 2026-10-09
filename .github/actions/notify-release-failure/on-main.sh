#!/usr/bin/env bash
# Print "true" when the failed run belongs to the branch that notifies ($BRANCH, default main),
# else "false". Pull request runs never notify. A tag counts when its commit is on $BRANCH
# (releases dispatched with `--ref <tag>`); when that cannot be checked, it counts (a release
# failure is not silently dropped). From a `workflow_run` watcher, the watched run's event,
# branch or tag, and commit are used.
#
# Env: GH_REPO, GH_TOKEN, GITHUB_EVENT_NAME, GITHUB_EVENT_PATH, GITHUB_REF, SHA (commit of the
# failed run; default GITHUB_SHA), BRANCH (default main).
set -euo pipefail
: "${GH_REPO:?}"
BRANCH="${BRANCH:-main}"
event="${GITHUB_EVENT_NAME:-}"
ref="${GITHUB_REF:-}"
sha="${SHA:-${GITHUB_SHA:-}}"

if [ "$event" = workflow_run ] && [ -f "${GITHUB_EVENT_PATH:-}" ]; then
  event="$(jq -r '.workflow_run.event // ""' "$GITHUB_EVENT_PATH")"
  name="$(jq -r '.workflow_run.head_branch // ""' "$GITHUB_EVENT_PATH")"
  sha="$(jq -r '.workflow_run.head_sha // ""' "$GITHUB_EVENT_PATH")"
  if [ "$name" = "$BRANCH" ]; then
    ref="refs/heads/$name"
  elif [ -n "$name" ] && gh api "repos/$GH_REPO/git/ref/tags/$name" >/dev/null 2>&1; then
    ref="refs/tags/$name"
  else
    ref="refs/heads/$name"
  fi
fi

case "$event" in
  pull_request | pull_request_target)
    echo "Run event $event: pull request runs never notify." >&2
    echo false
    exit 0
    ;;
esac
case "$ref" in
  "refs/heads/$BRANCH")
    echo true
    ;;
  refs/tags/*)
    # behind/identical: the tagged commit is an ancestor of (or equal to) the branch head.
    if status="$(gh api "repos/$GH_REPO/compare/$BRANCH...$sha" --jq .status 2>/dev/null)"; then
      case "$status" in
        behind | identical) echo true ;;
        *)
          echo "Tag ${ref#refs/tags/} ($sha) is not on $BRANCH ($status)." >&2
          echo false
          ;;
      esac
    else
      echo "Could not check whether ${ref#refs/tags/} is on $BRANCH; notifying." >&2
      echo true
    fi
    ;;
  *)
    echo "Ref $ref is not $BRANCH: no notification." >&2
    echo false
    ;;
esac
