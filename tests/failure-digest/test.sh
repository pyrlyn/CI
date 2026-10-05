#!/usr/bin/env bash
# Offline test for .github/scripts/failure-digest.sh and the shared
# .github/actions/notify-release-failure/notify-issue.sh, against a fake gh that serves
# fixtures and logs every call: a day with failures (one digest comment, only the failing
# repositories), a day without (nothing posted, no write call at all), and the
# release-failure path (a new issue carries the notification itself).
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
script="$root/.github/scripts/failure-digest.sh"
notify="$root/.github/actions/notify-release-failure/notify-issue.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$tmp/bin"
fail=0
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fail=1; }
has() { grep -qF -- "$2" "$1"; }
expect() { # <description> <command...>: ok when the command succeeds
  local d="$1"
  shift
  if "$@"; then ok "$d"; else bad "$d"; fi
}
refute() { # <description> <command...>: ok when the command fails
  local d="$1"
  shift
  if "$@"; then bad "$d"; else ok "$d"; fi
}
count() { grep -c -- "$2" "$1" || true; }

# Fake gh: fixtures under $FAKE/<owner>_<repo>/, every call appended to $FAKE/log.
cat >"$tmp/bin/gh" <<'GH'
#!/usr/bin/env bash
set -euo pipefail
echo "gh $*" >>"$FAKE/log"
jqf="" path=""
args=("$@")
for ((i = 0; i < ${#args[@]}; i++)); do
  case "${args[i]}" in
    --jq) jqf="${args[i + 1]}"; i=$((i + 1)) ;;
    --paginate | api) ;;
    *) [ -n "$path" ] || path="${args[i]}" ;;
  esac
done
case "$1 $2" in
  "label create") exit 0 ;;
  "issue list") cat "$FAKE/issue-list.json" 2>/dev/null || echo '[]'; exit 0 ;;
  "issue create") echo "https://github.com/$GH_REPO/issues/7"; exit 0 ;;
  "issue comment" | "issue edit") exit 0 ;;
esac
[ "$1" = api ] || { echo "fake gh: unexpected $*" >&2; exit 2; }
repo="$(cut -d/ -f2,3 <<<"$path")"
dir="$FAKE/${repo/\//_}"
serve() { if [ -n "$jqf" ]; then jq -r "$jqf" "$1"; else cat "$1"; fi; }
case "$path" in
  repos/*/actions/runs/*/attempts/*/jobs*)
    id="$(cut -d/ -f6 <<<"$path")"
    serve "$dir/jobs-$id.json" 2>/dev/null || echo '{"jobs":[]}' | serve /dev/stdin ;;
  repos/*/actions/runs\?*) [ -f "$dir/runs.json" ] || echo '{"workflow_runs":[]}' >"$dir/runs.json"
    serve "$dir/runs.json" ;;
  repos/*/git/ref/tags/*) grep -qx "${path##*/}" "$dir/tags" 2>/dev/null ;;
  repos/*/issues\?*) [ -f "$dir/issues.json" ] || echo '[]' >"$dir/issues.json"
    serve "$dir/issues.json" ;;
  repos/*/*) echo '{"default_branch":"main"}' | serve /dev/stdin ;;
  *) echo "fake gh: unexpected path $path" >&2; exit 2 ;;
esac
GH
chmod +x "$tmp/bin/gh"

repos="pyrlyn/ci pyrlyn/rtok,pyrlyn/ketch
pyrlyn/cox pyrlyn/runa"
now=1790834400 # 2026-10-01T06:00:00Z, 09:00 EEST
setup() { rm -rf "$tmp/fake"; mkdir -p "$tmp/fake"/pyrlyn_{infra,rtok,ketch,cox,runa}; : >"$tmp/fake/log"; }
run_digest() {
  env -u LABEL -u WHO -u RELEASE_LABEL -u SINCE_HOURS -u DIGEST_TZ PATH="$tmp/bin:$PATH" \
    FAKE="$tmp/fake" REPOS="$repos" GH_REPO=pyrlyn/ci NOW="$now" GH_TOKEN=fake bash "$script"
}
run() { # <id> <name> <event> <branch> <created> <conclusion>
  jq -cn --argjson id "$1" --arg name "$2" --arg event "$3" --arg branch "$4" \
    --arg created "$5" --arg conclusion "$6" \
    '{id: $id, run_attempt: 1, name: $name, event: $event, head_branch: $branch,
      html_url: "https://github.com/x/actions/runs/\($id)", created_at: $created,
      conclusion: $conclusion}'
}

# --- A day with failures -------------------------------------------------------------------
setup
f="$tmp/fake"
{
  echo '{"workflow_runs":['
  run 11 CI push main 2026-10-01T05:10:00Z failure; echo ,
  run 12 CI push feature/wip 2026-10-01T04:00:00Z failure; echo ,
  run 13 CI pull_request main 2026-10-01T04:00:00Z failure; echo ,
  run 14 Release push v1.2.0 2026-09-30T20:00:00Z failure; echo ,
  run 15 Nightly schedule main 2026-09-30T07:00:00Z timed_out; echo ,
  run 16 Nightly schedule main 2026-10-01T03:00:00Z failure; echo ,
  run 17 CI push main 2026-10-01T02:00:00Z success; echo ,
  run 18 Docs pull_request_target main 2026-10-01T02:00:00Z failure
  echo ']}'
} >"$f/pyrlyn_rtok/runs.json"
echo v1.2.0 >"$f/pyrlyn_rtok/tags"
echo '{"jobs":[{"name":"test (ubuntu)","conclusion":"failure"},{"name":"lint","conclusion":"success"},
  {"name":"test (macos)","conclusion":"timed_out"}]}' >"$f/pyrlyn_rtok/jobs-11.json"
echo '{"jobs":[{"name":"build","conclusion":"failure"}]}' >"$f/pyrlyn_rtok/jobs-14.json"
echo '[{"number":42,"title":"Release failed: release v0.9.1","html_url":"https://github.com/pyrlyn/ketch/issues/42"}]' \
  >"$f/pyrlyn_ketch/issues.json"
{ echo '{"workflow_runs":['; run 31 release-plz workflow_dispatch main 2026-10-01T01:00:00Z startup_failure; echo ']}'; } \
  >"$f/pyrlyn_cox/runs.json"
{ echo '{"workflow_runs":['; run 51 CI pull_request main 2026-10-01T01:00:00Z failure; echo ']}'; } \
  >"$f/pyrlyn_runa/runs.json"

out="$(DRY_RUN=true run_digest)"
echo "$out" >"$tmp/digest.md"
# The expected lines contain Markdown backticks, not shell.
# shellcheck disable=SC2016
for want in '@listepo failures in the last 24h (since 2026-09-30 09:00 EEST):' \
  '### pyrlyn/rtok' '### pyrlyn/ketch' '### pyrlyn/cox' \
  '- **CI** on `main` (push), 2026-10-01 08:10 EEST: [run](https://github.com/x/actions/runs/11); failed jobs: test (ubuntu), test (macos)' \
  '- **Release** on `v1.2.0` (push), 2026-09-30 23:00 EEST: [run](https://github.com/x/actions/runs/14); failed jobs: build' \
  '[run](https://github.com/x/actions/runs/15); failed jobs: none listed (timed_out)' \
  '[run](https://github.com/x/actions/runs/16)' \
  '- Release failure issue: [#42 Release failed: release v0.9.1](https://github.com/pyrlyn/ketch/issues/42)' \
  '- **release-plz** on `main` (workflow_dispatch), 2026-10-01 04:00 EEST: [run](https://github.com/x/actions/runs/31); failed jobs: none listed (startup_failure)'; do
  if has "$tmp/digest.md" "$want"; then ok "digest has: ${want:0:60}"; else bad "digest lacks: $want"; fi
done
for unwanted in 'pyrlyn/ci' 'pyrlyn/runa' 'runs/12)' 'runs/13)' 'runs/17)' 'runs/18)' 'runs/51)' 'green'; do
  if has "$tmp/digest.md" "$unwanted"; then bad "digest must not have: $unwanted"; else ok "digest omits: $unwanted"; fi
done
expect "24h window in the runs query" has "$f/log" 'created=>=2026-09-30T06:00:00Z'
refute "dry run posts nothing" grep -qE '^gh (issue|label)' "$f/log"

# Posting: no tracking issue yet -> label, issue, one digest comment, assignment.
: >"$f/log"
run_digest >/dev/null 2>&1
expect "one digest comment" [ "$(count "$f/log" '^gh issue comment 7 --body-file')" = 1 ]
expect "tracking issue created" has "$f/log" 'gh issue create --title Daily failure digest --label failure-digest'
expect "label ensured" has "$f/log" 'gh label create failure-digest'
expect "assigned" has "$f/log" 'gh issue edit 7 --add-assignee listepo'
# The tracking issue exists -> only the comment, no new issue.
: >"$f/log"
printf '%s' '[{"number":3,"body":"Tracking\n<!-- failure-digest -->\n"}]' >"$f/issue-list.json"
run_digest >/dev/null 2>&1
expect "comments on the existing issue" has "$f/log" 'gh issue comment 3 --body-file'
refute "no second issue" has "$f/log" 'gh issue create'

# --- A day without failures: complete silence ------------------------------------------------
setup
f="$tmp/fake"
{ echo '{"workflow_runs":['; run 61 CI pull_request main 2026-10-01T01:00:00Z failure; echo ,
  run 62 CI push topic 2026-10-01T01:00:00Z failure; echo ']}'; } >"$f/pyrlyn_rtok/runs.json"
out="$(run_digest 2>&1)"
refute "silent day: no issue, label or comment call" grep -qE '^gh (issue|label)' "$f/log"
expect "silent day: log line only ($out)" [ "$out" = "No failures in the last 24h across: \
pyrlyn/ci pyrlyn/rtok pyrlyn/ketch pyrlyn/cox pyrlyn/runa; nothing posted." ]

# --- release-failure path of the shared script ---------------------------------------------
setup
f="$tmp/fake"
echo "@listepo Release failed for v1." >"$tmp/rf.md"
PATH="$tmp/bin:$PATH" FAKE="$f" GH_REPO=pyrlyn/rtok TITLE="Release failed: Release v1" \
  MARKER="<!-- release-failure ref=v1 -->" LABEL=release-failure WHO=listepo \
  BODY_FILE="$tmp/rf.md" bash "$notify" >/dev/null 2>&1
expect "release failure opens an issue" has "$f/log" 'gh issue create --title Release failed: Release v1 --label release-failure'
refute "release failure: body, no comment" has "$f/log" 'gh issue comment'
printf '%s' '[{"number":9,"body":"x\n<!-- release-failure ref=v1 -->\n"}]' >"$f/issue-list.json"
: >"$f/log"
PATH="$tmp/bin:$PATH" FAKE="$f" GH_REPO=pyrlyn/rtok TITLE="Release failed: Release v1" \
  MARKER="<!-- release-failure ref=v1 -->" LABEL=release-failure WHO=listepo \
  BODY_FILE="$tmp/rf.md" bash "$notify" >/dev/null 2>&1
expect "release failure comments on the open issue" has "$f/log" 'gh issue comment 9'

exit "$fail"
