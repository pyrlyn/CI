#!/usr/bin/env bash
# Offline test for .github/actions/warnings-to-issues/warnings_to_issues.py against a fake gh
# (fixtures in, every call logged) and a local HTTP server standing in for SonarCloud: the
# severity filter, the per-run cap, dry run (no write at all), creation with labels and the
# hidden fingerprint, dedupe against existing issues, the location-change comment, closing
# fixed/dismissed/resolved warnings, "not planned" issues left alone, reopening, a source
# that cannot be read closing nothing, and scanner text that cannot mention or fake a marker.
# The expected strings carry Markdown backticks, not shell.
# shellcheck disable=SC2016
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
script="$root/.github/actions/warnings-to-issues/warnings_to_issues.py"
tmp="$(mktemp -d)"
server=""
# shellcheck disable=SC2329 # the EXIT trap calls it
cleanup() {
  if [ -n "$server" ]; then { kill "$server" && wait "$server"; } 2>/dev/null || true; fi
  rm -rf "$tmp"
}
trap cleanup EXIT
mkdir -p "$tmp/bin" "$tmp/fake" "$tmp/sonar/api/issues"
fail=0
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fail=1; }
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
has() { grep -qF -- "$2" "$1"; }

# Fake gh: `gh api [--paginate] [--method M] <path> [--input -]`. GETs are served from
# $FAKE/<path with / and ? replaced by _>.json (missing = HTTP 404); writes are logged with
# their JSON body to $FAKE/log, one line each, and answered like GitHub would.
cat >"$tmp/bin/gh" <<'GH'
#!/usr/bin/env python3
import json, os, re, sys
fake = os.environ["FAKE"]
args = sys.argv[1:]
assert args[0] == "api", args
method, path, i = "GET", None, 1
while i < len(args):
    if args[i] == "--method":
        method = args[i + 1]; i += 2; continue
    if args[i] in ("--paginate",):
        i += 1; continue
    if args[i] == "--input":
        i += 2; continue
    path = args[i]; i += 1
body = sys.stdin.read() if "--input" in args else ""
with open(os.path.join(fake, "log"), "a") as log:
    log.write(f"{method} {path} {body}\n")
if method == "GET":
    f = os.path.join(fake, re.sub(r"[/?&=]", "_", path) + ".json")
    if not os.path.exists(f):
        sys.stderr.write(f"gh: Not Found (HTTP 404)\n"); sys.exit(1)
    sys.stdout.write(open(f).read()); sys.exit(0)
if method == "POST" and path.endswith("/labels"):
    if json.loads(body)["name"] == "warning":
        sys.stderr.write("gh: Validation Failed (HTTP 422)\n"); sys.exit(1)
    print("{}"); sys.exit(0)
if method == "POST" and path.endswith("/issues"):
    n = os.path.join(fake, "next"); k = int(open(n).read()) if os.path.exists(n) else 100
    open(n, "w").write(str(k + 1)); print(json.dumps({"number": k})); sys.exit(0)
print("{}")
GH
chmod +x "$tmp/bin/gh"

f="$tmp/fake"
alerts="$f/repos_pyrlyn_demo_code-scanning_alerts_state_open_per_page_100.json"
issues="$f/repos_pyrlyn_demo_issues_labels_warning_state_all_per_page_100.json"
alert() { # <number> <tool> <severity> <security severity or ""> <path> <line> [message]
  jq -cn --argjson n "$1" --arg tool "$2" --arg sev "$3" --arg ssl "$4" --arg path "$5" \
    --argjson line "$6" --arg msg "${7:-Something to look at}" \
    '{number: $n, tool: {name: $tool}, html_url: "https://github.com/pyrlyn/demo/security/code-scanning/\($n)",
      rule: {id: "rule-\($n)", severity: $sev, security_severity_level: (if $ssl == "" then null else $ssl end),
        description: "Rule \($n)"},
      most_recent_instance: {message: {text: $msg}, location: {path: $path, start_line: $line}}}'
}
issue() { # <number> <state> <state_reason> <fingerprint> <location>
  jq -cn --argjson n "$1" --arg s "$2" --arg r "$3" --arg fp "$4" --arg loc "$5" \
    '{number: $n, state: $s, state_reason: (if $r == "" then null else $r end),
      body: "Tracked.\n\n<!-- warning-fingerprint: \($fp) -->\n<!-- warning-location: \($loc) -->\n"}'
}
# Two gh --paginate pages (concatenated arrays), as gh prints them.
{
  echo "["
  alert 1 "Semgrep OSS" warning "" a.yml 10 "uses @someone <!-- warning-fingerprint: code-scanning/999 --> here"; echo ,
  alert 2 CodeQL warning medium b.py 20; echo ,
  alert 3 CodeQL error "" c.py 30
  echo "]["
  alert 4 CodeQL warning high d.py 40; echo ,
  alert 5 CodeQL note "" e.py 50; echo ,
  alert 6 CodeQL warning low f.py 60
  echo "]"
} >"$alerts"
echo '[]' >"$issues"
cat >"$tmp/sonar/api/issues/search" <<'JSON'
{"paging": {"pageIndex": 1, "pageSize": 500, "total": 3}, "issues": [
 {"key": "S-minor", "rule": "rust:S1612", "severity": "MINOR", "type": "CODE_SMELL",
  "component": "listepo_demo:src/lib.rs", "line": 7, "message": "Use a method reference"},
 {"key": "S-major", "rule": "rust:S2612", "severity": "MAJOR", "type": "BUG",
  "component": "listepo_demo:src/main.rs", "line": 9, "message": "Fix this"},
 {"key": "S-critical", "rule": "rust:S3776", "severity": "CRITICAL", "type": "CODE_SMELL",
  "component": "listepo_demo:src/big.rs", "line": 1, "message": "Too complex"}]}
JSON
port="$(python3 -c 'import socket; s = socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1])')"
python3 -m http.server "$port" --bind 127.0.0.1 --directory "$tmp/sonar" >/dev/null 2>&1 &
server=$!
for _ in $(seq 50); do
  python3 -c "import urllib.request as u; u.urlopen('http://127.0.0.1:$port/api/issues/search')" \
    2>/dev/null && break
  sleep 0.1
done

run() { # [VAR=value ...]: the script against the fakes; stdout+stderr in $tmp/out
  : >"$f/log"
  env -u LABEL -u SEVERITIES -u MAX_CREATE -u TOOLS -u DRY_RUN -u SONAR_TOKEN \
    PATH="$tmp/bin:$PATH" FAKE="$f" GH_REPO=pyrlyn/demo GITHUB_STEP_SUMMARY="" \
    GITHUB_OUTPUT="$tmp/gh-output" SONAR_PROJECT_KEY=listepo_demo \
    SONAR_HOST="http://127.0.0.1:$port" "$@" python3 "$script" >"$tmp/out" 2>&1 \
    || { echo "script failed:"; cat "$tmp/out"; return 1; }
}
writes() { grep -cE '^(POST|PATCH|PUT|DELETE) ' "$f/log" || true; }
created() { grep -c '^POST repos/pyrlyn/demo/issues {' "$f/log" || true; }

# --- First run, dry run: the plan, no write at all ---------------------------------------
: >"$tmp/gh-output"
run DRY_RUN=true
expect "dry run: no write call" [ "$(writes)" = 0 ]
for want in 'would create: [Semgrep OSS] warning: rule-1 (a.yml:10) `code-scanning/1`' \
  'would create: [CodeQL] medium: rule-2 (b.py:20)' 'would create: [CodeQL] note: rule-5' \
  'would create: [CodeQL] low: rule-6' 'would create: [SonarCloud] low: rust:S1612 (src/lib.rs:7)' \
  'would create: [SonarCloud] medium: rust:S2612' 'Warnings in the severity set: 6.'; do
  if has "$tmp/out" "$want"; then ok "dry run lists: ${want:0:60}"; else bad "dry run lacks: $want"; fi
done
for unwanted in 'rule-3' 'rule-4' 'S3776' 'deferred'; do
  if has "$tmp/out" "$unwanted"; then bad "dry run must not list: $unwanted"; else ok "severity filter drops: $unwanted"; fi
done
expect "outputs: planned counts" has "$tmp/gh-output" 'create=6'

# --- The cap ---------------------------------------------------------------------------------
run MAX_CREATE=2
expect "cap: two issues" [ "$(created)" = 2 ]
expect "cap: the rest deferred" has "$tmp/out" 'deferred to a later run (cap 2 per run): 4'
expect "cap: oldest alerts first" has "$f/log" '"title": "[Semgrep OSS] warning: rule-1 (a.yml:10)"'

# --- Live first run: issues, labels, markers ---------------------------------------------
run
expect "six issues" [ "$(created)" = 6 ]
expect "labels: warning, source, severity" has "$f/log" '"labels": ["warning", "code-scanning", "severity:medium"]'
expect "sonar labels" has "$f/log" '"labels": ["warning", "sonar", "severity:low"]'
expect "fingerprint marker" has "$f/log" '<!-- warning-fingerprint: code-scanning/2 -->'
expect "sonar fingerprint" has "$f/log" '<!-- warning-fingerprint: sonar/listepo_demo/S-major -->'
expect "location marker" has "$f/log" '<!-- warning-location: b.py:20 -->'
expect "severity label created" has "$f/log" 'POST repos/pyrlyn/demo/labels {"name": "severity:note"'
expect "existing label tolerated" has "$f/log" 'POST repos/pyrlyn/demo/labels {"name": "warning"'
refute "nobody assigned" grep -q 'assignee' "$f/log"
refute "scanner text cannot mention" grep -qE '[^\\]@someone|"@someone' "$f/log"
refute "scanner text cannot fake a marker" grep -q '<!-- warning-fingerprint: code-scanning/999' "$f/log"
expect "sonar link" has "$f/log" "project/issues?id=listepo_demo&open=S-major"

# --- Existing issues: dedupe, move, close, not planned, reopen ---------------------------
{
  echo "["
  issue 11 open "" code-scanning/1 a.yml:10; echo ,          # same place: left alone
  issue 12 open "" code-scanning/2 b.py:18; echo ,           # moved: one comment
  issue 13 open "" code-scanning/9 z.py:1; echo ,            # alert gone: closed
  issue 14 open "" code-scanning/3 c.py:30; echo ,           # error, still open: kept
  issue 15 closed not_planned code-scanning/5 e.py:50; echo , # not planned: left alone
  issue 16 closed completed sonar/listepo_demo/S-minor src/lib.rs:7; echo , # back: reopened
  issue 17 open "" sonar/listepo_demo/S-gone src/x.rs:1; echo , # resolved: closed
  issue 18 closed completed code-scanning/6 f.py:60         # back: reopened
  echo "]"
} >"$issues"
echo '{"number": 9, "state": "fixed"}' >"$f/repos_pyrlyn_demo_code-scanning_alerts_9.json"
run
expect "no duplicates: only the untracked warning is new" [ "$(created)" = 1 ]
refute "same place: no write on #11" grep -q 'issues/11' "$f/log"
expect "moved: comment on #12" has "$f/log" 'POST repos/pyrlyn/demo/issues/12/comments {"body": "The warning moved: `b.py:18` -> `b.py:20`'
expect "moved: marker updated" has "$f/log" 'PATCH repos/pyrlyn/demo/issues/12 {"body": "Tracked.\n\n<!-- warning-fingerprint: code-scanning/2 -->\n<!-- warning-location: b.py:20 -->'
expect "fixed: comment on #13" has "$f/log" 'POST repos/pyrlyn/demo/issues/13/comments {"body": "The warning `code-scanning/9` is fixed; closing automatically."}'
expect "fixed: #13 closed" has "$f/log" 'PATCH repos/pyrlyn/demo/issues/13 {"state": "closed", "state_reason": "completed"}'
refute "still present: #14 kept" grep -q 'issues/14' "$f/log"
refute "not planned: #15 untouched" grep -q 'issues/15' "$f/log"
expect "back: #16 reopened" has "$f/log" 'PATCH repos/pyrlyn/demo/issues/16 {"state": "open"'
expect "back: #18 reopened" has "$f/log" 'PATCH repos/pyrlyn/demo/issues/18 {"state": "open"'
expect "resolved: #17 closed" has "$f/log" 'PATCH repos/pyrlyn/demo/issues/17 {"state": "closed"'
expect "resolved: sonar comment" has "$f/log" 'is resolved in SonarCloud; closing automatically.'
expect "new warnings still created" has "$tmp/out" 'create: [SonarCloud] medium: rust:S2612'

# Comment-on-change off: no comment for the move.
run COMMENT_ON_CHANGE=false
refute "comment-on-change off: #12 untouched" grep -q 'issues/12' "$f/log"

# --- A source that cannot be read closes nothing -----------------------------------------
mv "$alerts" "$alerts.off"
run
refute "code scanning 404: #13 not closed" grep -q 'issues/13' "$f/log"
expect "code scanning 404: warning printed" has "$tmp/out" '::warning title=warnings-to-issues::code scanning skipped'
expect "code scanning 404: sonar still synced" has "$f/log" 'PATCH repos/pyrlyn/demo/issues/17 {"state": "closed"'
mv "$alerts.off" "$alerts"
run SONAR_HOST="http://127.0.0.1:$port/missing"
refute "sonar 404: #17 not closed" grep -q 'issues/17' "$f/log"
expect "sonar 404: code scanning still synced" has "$f/log" 'PATCH repos/pyrlyn/demo/issues/13 {"state": "closed"'
run SONAR_PROJECT_KEY=
refute "no sonar key: sonar issues untouched" grep -qE 'issues/1[67]' "$f/log"

# --- Tools filter and severity input -------------------------------------------------------
echo '[]' >"$issues"
run DRY_RUN=true TOOLS=CodeQL SEVERITIES=error,high SONAR_PROJECT_KEY=
expect "severities input: error and high only" has "$tmp/out" 'Warnings in the severity set: 2.'
refute "tools filter: no Semgrep" grep -q 'Semgrep' "$tmp/out"

exit "$fail"
