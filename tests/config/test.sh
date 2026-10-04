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
  "jobs": [{"name": "heavy", "run": "true"}, {"name": "docs", "run": "true", "docs-only": true}]}'

check() { # <name> <event> <DOCS_ONLY> <jq patch> <expected key=value ...>
  local name="$1" event="$2" docs="$3" patch="$4"
  shift 4
  : >"$tmp/out"
  if ! jq "$patch" <<<"$base" | EVENT="$event" DOCS_ONLY="$docs" PRIVATE=false DRAFT=false \
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
  docs-only=true "skip-ok=rust dotnet codeql semgrep snyk sonarcloud" \
  run:rust=false run:codeql=false run:lint=true
if grep -q '"name":"docs"' "$tmp/out" && ! grep -q '"name":"heavy"' "$tmp/out"; then
  echo "ok   docs-only-pr custom jobs"
else
  echo "FAIL docs-only-pr custom jobs"
  fail=1
fi
check code-pr pull_request false . \
  docs-only=false "skip-ok=snyk sonarcloud" run:rust=true run:codeql=true
check docs-only-push push true . docs-only=false run:rust=true
check docs-only-disabled pull_request true '."docs-only".enabled = false' \
  docs-only=false run:codeql=true
check no-docs-only-section pull_request true 'del(."docs-only")' docs-only=false run:rust=true
check docs-only-custom-all-skipped pull_request true '.jobs = [{"name": "heavy", "run": "true"}]' \
  docs-only=true "skip-ok=rust dotnet codeql semgrep snyk sonarcloud custom"

# Bad config fails the job.
: >"$tmp/out"
if jq '."docs-only".typo = 1' <<<"$base" | EVENT=pull_request DOCS_ONLY=false \
    GITHUB_OUTPUT="$tmp/out" python3 "$script" >/dev/null 2>&1; then
  echo "FAIL unknown docs-only key accepted"; fail=1
else
  echo "ok   unknown docs-only key"
fi

exit "$fail"
