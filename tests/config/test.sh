#!/usr/bin/env bash
# Table test for .github/actions/config/normalize.py's docs-only handling: a merged config
# (JSON) plus the `changes` verdict (DOCS_ONLY) in, the outputs out. Runs offline, no yq.
set -euo pipefail
script="$(cd "$(dirname "$0")/../.." && pwd)/.github/actions/config/normalize.py"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fail=0

# The checks as defaults.yml sets them (enabled, docs-only), two custom jobs.
base='{"version": 1, "skip-drafts": true, "upload-sarif": false,
  "docs-only": {"enabled": true, "events": ["pull_request"], "paths": [], "exclude": []},
  "rust": {"enabled": true, "docs-only": false}, "dotnet": {"enabled": true, "docs-only": false},
  "codeql": {"enabled": true, "docs-only": false}, "semgrep": {"enabled": true, "docs-only": false},
  "snyk": {"enabled": false, "docs-only": false}, "sonarcloud": {"enabled": false, "docs-only": false},
  "lint": {"enabled": true, "docs-only": true},
  "license": {"enabled": false, "docs-only": true},
  "commits": {"enabled": false, "events": ["pull_request"], "docs-only": true, "tool": "grep",
    "types": ["feat", "fix"]},
  "jobs": [{"name": "heavy", "run": "true"}, {"name": "docs", "run": "true", "docs-only": true}]}'

check() { # <name> <event> <DOCS_ONLY> <jq patch> <expected key=value ...>
  local name="$1" event="$2" docs="$3" patch="$4"
  shift 4
  : >"$tmp/out"
  if ! jq "$patch" <<<"$base" | EVENT="$event" DOCS_ONLY="$docs" PRIVATE=false DRAFT="${DRAFT:-false}" \
      GITHUB_OUTPUT="$tmp/out" GITHUB_STEP_SUMMARY='' python3 "$script" >"$tmp/log" 2>&1; then
    echo "FAIL $name: normalize.py failed: $(cat "$tmp/log")"
    fail=1
    return
  fi
  local want bad=0 got
  for want in "$@"; do
    case "$want" in
      run:*) # run:<check>=<bool>
        want="${want#run:}"
        got="$(sed -n 's/^json=//p' "$tmp/out" | jq -r --arg c "${want%%=*}" '.[$c].run')"
        [ "$got" = "${want#*=}" ] || { echo "FAIL $name: ${want%%=*}.run=$got, want ${want#*=}"; bad=1; } ;;
      *)
        grep -qx -- "$want" "$tmp/out" || { echo "FAIL $name: want $want, got: $(grep -- "^${want%%=*}=" "$tmp/out")"; bad=1; } ;;
    esac
  done
  if [ "$bad" -eq 0 ]; then echo "ok   $name"; else fail=1; fi
}

check docs-only-pr pull_request true . \
  docs-only=true "skip-ok=rust dotnet codeql semgrep snyk sonarcloud license commits" \
  run:rust=false run:codeql=false run:lint=true
if grep -q '"name":"docs"' "$tmp/out" && ! grep -q '"name":"heavy"' "$tmp/out"; then
  echo "ok   docs-only-pr custom jobs"
else
  echo "FAIL docs-only-pr custom jobs"
  fail=1
fi
check code-pr pull_request false . \
  docs-only=false "skip-ok=snyk sonarcloud license commits" run:rust=true run:codeql=true
check docs-only-push push true . docs-only=false run:rust=true
check docs-only-disabled pull_request true '."docs-only".enabled = false' \
  docs-only=false run:codeql=true
check no-docs-only-section pull_request true 'del(."docs-only")' docs-only=false run:rust=true
check docs-only-custom-all-skipped pull_request true '.jobs = [{"name": "heavy", "run": "true"}]' \
  docs-only=true "skip-ok=rust dotnet codeql semgrep snyk sonarcloud license commits custom"

# Draft PR with skip-drafts: every check is skipped (draft-skip), but `docs-only` still reports
# the change, so a caller's local jobs can skip their heavy work on a docs-only draft too.
DRAFT=true check docs-only-draft pull_request true . \
  docs-only=true draft-skip=true \
  "skip-ok=rust dotnet codeql semgrep snyk sonarcloud lint license commits custom" \
  run:rust=false run:lint=false
DRAFT=true check code-draft pull_request false . \
  docs-only=false draft-skip=true \
  "skip-ok=rust dotnet codeql semgrep snyk sonarcloud lint license commits custom" run:rust=false
DRAFT=true check docs-only-draft-no-skip-drafts pull_request true '."skip-drafts" = false' \
  docs-only=true draft-skip=false run:rust=false run:lint=true

# ci.yml passes the merged `skip-drafts` to the reusable workflows' boolean `skip-drafts` input,
# so the JSON always carries it as a boolean, default true.
json_skip_drafts() { # <name> <jq patch> <expected>
  local got
  check "$1" pull_request false "$2" docs-only=false
  got="$(sed -n 's/^json=//p' "$tmp/out" | jq -c '."skip-drafts"')"
  [ "$got" = "$3" ] && echo "ok   $1 json" || { echo "FAIL $1: skip-drafts=$got, want $3"; fail=1; }
}
json_skip_drafts skip-drafts-default 'del(."skip-drafts")' true
json_skip_drafts skip-drafts-off '."skip-drafts" = false' false

# license and commits: on when enabled, both run on a docs-only change; commits only on
# pull_request by default.
on='.license.enabled = true | .commits.enabled = true'
check toggles-docs-only-pr pull_request true "$on" run:license=true run:commits=true run:rust=false
check toggles-push push false "$on" run:license=true run:commits=false
json_field() { # <name> <jq patch> <jq path> <expected>
  local got
  check "$1" pull_request false "$2" docs-only=false
  got="$(sed -n 's/^json=//p' "$tmp/out" | jq -c "$3")"
  if [ "$got" = "$4" ]; then echo "ok   $1 json"; else echo "FAIL $1: $3=$got, want $4"; fail=1; fi
}
json_field commits-types "$on" '.commits."types-list"' '"feat fix"'
json_field locked-default . '.rust.locked' '"auto"'
json_field locked-bool '.rust.locked = false' '.rust.locked' '"false"'
json_field base-exclude-allowed '."docs-only"."base-exclude" = []' '."docs-only"."base-exclude"' '[]'

# Bad config fails the job.
must_fail() { # <name> <jq patch>
  : >"$tmp/out"
  if jq "$2" <<<"$base" | EVENT=pull_request DOCS_ONLY=false \
      GITHUB_OUTPUT="$tmp/out" python3 "$script" >/dev/null 2>&1; then
    echo "FAIL $1 accepted"; fail=1
  else
    echo "ok   $1"
  fi
}
must_fail "unknown docs-only key" '."docs-only".typo = 1'
must_fail "unknown commits tool" '.commits.tool = "gitlint"'
must_fail "bad commit type" '.commits.types = ["Feat"]'
must_fail "bad rust.locked" '.rust.locked = "yes"'

exit "$fail"
