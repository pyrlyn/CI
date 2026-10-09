#!/usr/bin/env bash
# Table test for .github/actions/commits/commits.sh (tool: grep) on a scratch repository.
set -euo pipefail
script="$(cd "$(dirname "$0")/../.." && pwd)/.github/actions/commits/commits.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fail=0
cd "$tmp"
git init -q -b main
git -c user.name=t -c user.email=t@t commit -q --allow-empty -m "chore: init"
git update-ref refs/remotes/origin/main HEAD
git checkout -q -b pr
commit() { git -c user.name=t -c user.email=t@t commit -q --allow-empty -m "$1"; }

run() { # <name> <want rc> <event> <base ref>
  local rc=0
  GITHUB_EVENT_NAME="$3" BASE_REF="$4" TOOL=grep TYPES="feat fix chore" WORKDIR=. \
    bash "$script" >"$tmp/log" 2>&1 || rc=$?
  if [ "$rc" = "$2" ]; then
    echo "ok   $1"
  else
    echo "FAIL $1: rc=$rc, want $2"
    cat "$tmp/log"
    fail=1
  fi
}

commit "feat(ci/x): scoped"
commit "fix!: breaking"
commit 'Revert "fix!: breaking"'
run conventional 0 pull_request main
run push-has-no-range 0 push ""
run missing-base 1 pull_request nope
commit "docs: not in types"
run unknown-type 1 pull_request main
git reset -q --hard HEAD~1
commit "no type at all"
run no-type 1 pull_request main
exit "$fail"
