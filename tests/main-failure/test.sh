#!/usr/bin/env bash
# Offline test for .github/actions/main-failure/main-failure.sh (open, comment, dedupe, close,
# cancel-only silence) and .github/actions/notify-release-failure/on-main.sh (main, other
# branch, pull request, tag on main, tag off main, workflow_run watcher), against a fake gh
# that serves fixtures and logs every call.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
script="$root/.github/actions/main-failure/main-failure.sh"
onmain="$root/.github/actions/notify-release-failure/on-main.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$tmp/bin"
fail=0
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fail=1; }
# shellcheck disable=SC2317 # called through expect/refute
has() { grep -qF -- "$2" "$1"; }
expect() { local d="$1"; shift; if "$@"; then ok "$d"; else bad "$d"; fi; }
refute() { local d="$1"; shift; if "$@"; then bad "$d"; else ok "$d"; fi; }

# Fake gh: $F/issue-list.json, $F/comments.json, $F/tags (one per line), $F/compare (status).
cat >"$tmp/bin/gh" <<'GH'
#!/usr/bin/env bash
set -euo pipefail
echo "gh $*" >>"$F/log"
case "$1 $2" in
  "label create" | "issue comment" | "issue edit" | "issue close") exit 0 ;;
  "issue list") cat "$F/issue-list.json" 2>/dev/null || echo '[]'; exit 0 ;;
  "issue create") echo "https://github.com/$GH_REPO/issues/7"; exit 0 ;;
esac
path="" jqf=""
args=("$@")
for ((i = 1; i < ${#args[@]}; i++)); do
  case "${args[i]}" in
    --jq) jqf="${args[i + 1]}"; i=$((i + 1)) ;;
    --paginate) ;;
    *) [ -n "$path" ] || path="${args[i]}" ;;
  esac
done
case "$path" in
  repos/*/issues/*/comments) jq -r "$jqf" "$F/comments.json" 2>/dev/null || echo '[]' | jq -r "$jqf" ;;
  repos/*/git/ref/tags/*) grep -qx "${path##*/}" "$F/tags" 2>/dev/null ;;
  repos/*/compare/*) [ -f "$F/compare" ] || exit 1; cat "$F/compare" ;;
  *) echo "fake gh: unexpected $*" >&2; exit 2 ;;
esac
GH
chmod +x "$tmp/bin/gh"

F="$tmp/fake"
setup() { rm -rf "$F"; mkdir -p "$F"; : >"$F/log"; }
needs() { # <rust result> <gate result>
  jq -cn --arg r "$1" --arg g "$2" \
    '{config: {result: "success"}, rust: {result: $r}, lint: {result: "skipped"}, gate: {result: $g}}'
}
run_mf() { # <needs json> [attempt]
  PATH="$tmp/bin:$PATH" F="$F" GH_REPO=pyrlyn/rtok GH_TOKEN=fake NEEDS="$1" WHO=listepo \
    WORKFLOW=pipeline RUN_ID=100 RUN_ATTEMPT="${2:-1}" SHA=abc BRANCH=main \
    bash "$script" >"$F/out" 2>&1
}
marker='<!-- ci-main-failure workflow=pipeline -->'

# --- main-failure ---------------------------------------------------------------------------
setup
run_mf "$(needs failure failure)"
expect "failure: opens an issue" has "$F/log" 'gh issue create --title CI failing on main: pipeline --label ci-main-failure'
expect "failure: assigns" has "$F/log" 'gh issue edit 7 --add-assignee listepo'
expect "failure: label ensured" has "$F/log" 'gh label create ci-main-failure'

setup
printf '[{"number":5,"body":"x\\n%s\\n"}]' "$marker" >"$F/issue-list.json"
echo '[{"body":"old <!-- ci-main-failure run=99/1 -->"}]' >"$F/comments.json"
run_mf "$(needs failure failure)"
expect "failure, open issue: comments" has "$F/log" 'gh issue comment 5 --body-file'
refute "failure, open issue: no second issue" has "$F/log" 'gh issue create'

setup
printf '[{"number":5,"body":"x\\n%s\\n"}]' "$marker" >"$F/issue-list.json"
echo '[{"body":"@listepo <!-- ci-main-failure run=100/1 -->"}]' >"$F/comments.json"
run_mf "$(needs failure failure)"
refute "same run attempt: not reported twice" has "$F/log" 'gh issue comment'
run_mf "$(needs failure failure)" 2
expect "re-run attempt: reported" has "$F/log" 'gh issue comment 5'

setup
printf '[{"number":5,"body":"x\\n%s\\n"}]' "$marker" >"$F/issue-list.json"
run_mf "$(needs success success)"
expect "green: closes the open issue" has "$F/log" 'gh issue close 5 --comment'

setup
run_mf "$(needs success success)"
refute "green, no issue: no write" grep -qE '^gh issue (create|comment|close|edit)' "$F/log"

setup
run_mf "$(needs cancelled failure)"
refute "cancel only: nothing reported" grep -qE '^gh issue (create|comment|close|edit)' "$F/log"

setup
printf '[{"number":5,"body":"x\\n<!-- ci-main-failure workflow=other -->\\n"}]' >"$F/issue-list.json"
run_mf "$(needs failure failure)"
expect "another workflow's issue is not reused" has "$F/log" 'gh issue create'

# --- on-main --------------------------------------------------------------------------------
om() { # <event> <ref> [event file]
  PATH="$tmp/bin:$PATH" F="$F" GH_REPO=pyrlyn/cox GH_TOKEN=fake GITHUB_EVENT_NAME="$1" \
    GITHUB_REF="$2" GITHUB_EVENT_PATH="${3:-/nonexistent}" SHA=abc bash "$onmain" 2>/dev/null
}
setup
expect "on-main: push to main" [ "$(om push refs/heads/main)" = true ]
expect "on-main: dispatch on main" [ "$(om workflow_dispatch refs/heads/main)" = true ]
expect "on-main: other branch" [ "$(om push refs/heads/feature)" = false ]
expect "on-main: pull request" [ "$(om pull_request refs/heads/main)" = false ]
echo behind >"$F/compare"
expect "on-main: tag on main" [ "$(om workflow_dispatch refs/tags/v1.0.0)" = true ]
echo diverged >"$F/compare"
expect "on-main: tag off main" [ "$(om workflow_dispatch refs/tags/v1.0.0)" = false ]
rm -f "$F/compare"
expect "on-main: unknown tag ancestry notifies" [ "$(om workflow_dispatch refs/tags/v1.0.0)" = true ]
echo '{"workflow_run":{"event":"workflow_dispatch","head_branch":"main","head_sha":"abc"}}' >"$F/ev.json"
expect "on-main: watcher, run on main" [ "$(om workflow_run refs/heads/main "$F/ev.json")" = true ]
echo '{"workflow_run":{"event":"workflow_dispatch","head_branch":"topic","head_sha":"abc"}}' >"$F/ev.json"
expect "on-main: watcher, run on a branch" [ "$(om workflow_run refs/heads/main "$F/ev.json")" = false ]
echo '{"workflow_run":{"event":"pull_request","head_branch":"main","head_sha":"abc"}}' >"$F/ev.json"
expect "on-main: watcher, pull request run" [ "$(om workflow_run refs/heads/main "$F/ev.json")" = false ]
echo v2.0.0 >"$F/tags"
echo identical >"$F/compare"
echo '{"workflow_run":{"event":"workflow_dispatch","head_branch":"v2.0.0","head_sha":"abc"}}' >"$F/ev.json"
expect "on-main: watcher, tag on main" [ "$(om workflow_run refs/heads/main "$F/ev.json")" = true ]

exit "$fail"
