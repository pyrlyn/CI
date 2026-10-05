#!/usr/bin/env python3
"""Mirror code scanning and SonarCloud warnings of a repository as GitHub issues.

One issue per distinct warning, keyed by a fingerprint hidden in the issue body
(`<!-- warning-fingerprint: ... -->`):

- code scanning (every SARIF tool: CodeQL, Semgrep, ...): `code-scanning/<alert number>`;
  GitHub already deduplicates results into alerts by rule and location fingerprint, so the
  alert number is the stable key;
- SonarCloud: `sonar/<project key>/<issue key>`.

A warning in the severity set without an issue gets one (at most MAX_CREATE new or reopened
issues per run). An existing open issue is left alone; when the warning's location changed it
gets one comment and the hidden location is updated. An issue closed as "not planned" is never
touched again; one closed as "completed" is reopened when the warning comes back. An open
issue whose warning is gone (alert fixed or dismissed, Sonar issue resolved) is closed with a
comment. A source that could not be read completely closes nothing.

Effective severity: code scanning uses the alert's security severity (critical, high, medium,
low) when the rule has one, else the rule severity (error, warning, note). SonarCloud
severities map to the same scale: BLOCKER=critical, CRITICAL=high, MAJOR=medium, MINOR=low,
INFO=note.

Env: GH_REPO (owner/name), GH_TOKEN (issues: write, security-events: read; used by gh),
SEVERITIES (comma/space list, default "warning,note,medium,low"), MAX_CREATE (default 20),
LABEL (default "warning"), CODE_SCANNING (true/false, default true), TOOLS (code scanning tool
names to include, comma-separated; empty = all), SONAR_PROJECT_KEY (empty = no Sonar),
SONAR_HOST (default https://sonarcloud.io), SONAR_TOKEN (optional; public projects need
none), SONAR_BRANCH (optional; empty = the project's main branch), COMMENT_ON_CHANGE
(true/false, default true), DRY_RUN (true: print the plan, write nothing).
Needs python3 (stdlib only) and gh.
"""

import json
import os
import re
import subprocess
import sys
import urllib.error
import urllib.parse
import urllib.request

MARKER = re.compile(r"^<!-- warning-fingerprint: (\S+) -->$", re.M)
LOCATION = re.compile(r"^<!-- warning-location: (.*) -->$", re.M)
SONAR_SEVERITY = {"BLOCKER": "critical", "CRITICAL": "high", "MAJOR": "medium",
                  "MINOR": "low", "INFO": "note"}
SEVERITY_COLOR = {"critical": "B60205", "high": "D93F0B", "error": "D93F0B",
                  "medium": "FBCA04", "warning": "FBCA04", "low": "0E8A16", "note": "C5DEF5"}
SOURCE_LABELS = {
    "code-scanning": ("1D76DB", "From GitHub code scanning (SARIF: CodeQL, Semgrep, ...)"),
    "sonar": ("5319E7", "From SonarCloud"),
}
SONAR_PAGE = 500
SONAR_LIMIT = 10000  # api/issues/search refuses to page past 10 000 results


def env(name, default=""):
    return os.environ.get(name, default).strip()


def flag(name, default):
    value = env(name, "true" if default else "false").lower()
    return value in ("true", "1", "yes")


def split_list(value):
    return [v for v in re.split(r"[,\s]+", value) if v]


class Unavailable(Exception):
    """A source that cannot be read (not enabled, no access); it is skipped, not fatal."""


def gh(args, data=None):
    """Run gh; returns stdout. Raises Unavailable on HTTP 403/404, RuntimeError otherwise."""
    proc = subprocess.run(["gh", *args], input=None if data is None else json.dumps(data),
                          capture_output=True, text=True, check=False)
    if proc.returncode != 0:
        err = proc.stderr.strip()
        if re.search(r"HTTP 40[34]", err):
            raise Unavailable(err)
        raise RuntimeError(f"gh {' '.join(args)} failed: {err}")
    return proc.stdout


def gh_pages(path):
    """Every item of a paginated list endpoint (gh --paginate concatenates the JSON arrays)."""
    out = gh(["api", "--paginate", path])
    items, pos, dec = [], 0, json.JSONDecoder()
    while True:
        while pos < len(out) and out[pos].isspace():
            pos += 1
        if pos >= len(out):
            return items
        page, pos = dec.raw_decode(out, pos)
        items.extend(page)


def gh_write(method, path, data):
    out = gh(["api", "--method", method, path, "--input", "-"], data)
    return json.loads(out) if out.strip() else {}


def clean(text, limit=None):
    """Untrusted scanner text for an issue: no mentions, no fake markers, one line."""
    text = " ".join(str(text or "").split())
    text = text.replace("@", "@\u200b").replace("<!--", "&lt;!--").replace("-->", "--&gt;")
    if limit and len(text) > limit:
        text = text[: limit - 1] + "\u2026"
    return text


def code_scanning_warnings(repo, tools):
    """All open alerts on the default branch, keyed by fingerprint."""
    out = {}
    for a in gh_pages(f"repos/{repo}/code-scanning/alerts?state=open&per_page=100"):
        tool = (a.get("tool") or {}).get("name") or "code scanning"
        if tools and tool not in tools:
            continue
        rule = a.get("rule") or {}
        inst = a.get("most_recent_instance") or {}
        loc = inst.get("location") or {}
        path, line = loc.get("path") or "", loc.get("start_line")
        severity = (rule.get("security_severity_level") or rule.get("severity") or "").lower()
        out[f"code-scanning/{a['number']}"] = {
            "source": "code-scanning",
            "tool": tool,
            "severity": severity or "warning",
            "rule": rule.get("id") or rule.get("name") or "unknown-rule",
            "rule_text": rule.get("description") or rule.get("name") or "",
            "message": ((inst.get("message") or {}).get("text")) or "",
            "location": f"{path}:{line}" if path and line else path,
            "url": a.get("html_url") or "",
            "number": a["number"],
        }
    return out


def sonar_get(host, token, path, params):
    url = f"{host.rstrip('/')}/{path}?{urllib.parse.urlencode(params)}"
    req = urllib.request.Request(url, headers={"Accept": "application/json"})
    if token:
        req.add_header("Authorization", f"Bearer {token}")
    try:
        with urllib.request.urlopen(req, timeout=60) as resp:
            return json.load(resp)
    except urllib.error.HTTPError as e:
        if e.code in (401, 403, 404):
            raise Unavailable(f"SonarCloud {path}: HTTP {e.code}") from e
        raise


def sonar_warnings(host, token, key, branch):
    """All unresolved issues of the project, keyed by fingerprint; complete or Unavailable."""
    out, page = {}, 1
    while True:
        params = {"componentKeys": key, "resolved": "false", "ps": SONAR_PAGE, "p": page}
        if branch:
            params["branch"] = branch
        data = sonar_get(host, token, "api/issues/search", params)
        total = (data.get("paging") or {}).get("total", data.get("total", 0))
        if total > SONAR_LIMIT:
            raise Unavailable(f"SonarCloud {key}: {total} open issues, more than the API "
                              f"lists ({SONAR_LIMIT}); nothing synced")
        for i in data.get("issues") or []:
            component = i.get("component") or ""
            path = component.split(":", 1)[1] if ":" in component else component
            line = i.get("line") or (i.get("textRange") or {}).get("startLine")
            link = (f"{host.rstrip('/')}/project/issues?id={urllib.parse.quote(key)}"
                    f"&open={urllib.parse.quote(i['key'])}")
            out[f"sonar/{key}/{i['key']}"] = {
                "source": "sonar",
                "tool": "SonarCloud",
                "severity": SONAR_SEVERITY.get((i.get("severity") or "").upper(), "medium"),
                "rule": i.get("rule") or "unknown-rule",
                "rule_text": (i.get("type") or "").replace("_", " ").lower(),
                "message": i.get("message") or "",
                "location": f"{path}:{line}" if path and line else path,
                "url": link,
                "number": i["key"],
            }
        if page * SONAR_PAGE >= total or not data.get("issues"):
            return out
        page += 1


def short_rule(rule):
    """Semgrep's dotted ids repeat the rule name: keep the last segment for the title."""
    return rule.rsplit(".", 1)[-1] if "." in rule and "/" not in rule else rule


def title_of(w):
    where = f" ({w['location']})" if w["location"] else ""
    return clean(f"[{w['tool']}] {w['severity']}: {short_rule(w['rule'])}{where}", 250)


def body_of(fp, w):
    # The backticks are Markdown.
    lines = [
        f"**{clean(w['tool'])}** reports a `{w['severity']}` warning"
        + (" on the default branch." if w["source"] == "code-scanning" else "."),
        "",
        f"- Rule: `{clean(w['rule'], 200)}`" + (f" ({clean(w['rule_text'], 300)})"
                                                 if w["rule_text"] else ""),
    ]
    if w["location"]:
        lines.append(f"- Location: `{clean(w['location'], 300)}`")
    if w["message"]:
        lines.append(f"- Message: {clean(w['message'], 1000)}")
    if w["url"]:
        lines.append(f"- Details: {w['url']}")
    lines += [
        "",
        "Opened by pyrlyn/infra `warnings-to-issues`. It is closed automatically when the "
        "warning is fixed or dismissed at the source. Close it as *not planned* to stop "
        "tracking it.",
        "",
        f"<!-- warning-fingerprint: {fp} -->",
        f"<!-- warning-location: {clean(w['location'])} -->",
    ]
    return "\n".join(lines) + "\n"


def labels_of(label, w):
    return [label, w["source"], f"severity:{w['severity']}"]


def existing_issues(repo, label):
    """fingerprint -> issue (open ones preferred over closed duplicates)."""
    found = {}
    for issue in gh_pages(f"repos/{repo}/issues?labels={urllib.parse.quote(label)}"
                          f"&state=all&per_page=100"):
        if issue.get("pull_request"):
            continue
        m = MARKER.search(issue.get("body") or "")
        if not m:
            continue
        fp = m.group(1)
        old = found.get(fp)
        if old is None or (old["state"] != "open" and issue["state"] == "open"):
            found[fp] = issue
    return found


def main():
    repo = env("GH_REPO")
    if not repo:
        sys.exit("GH_REPO is required")
    dry = flag("DRY_RUN", False)
    severities = {s.lower() for s in split_list(env("SEVERITIES", "warning,note,medium,low"))}
    max_create = int(env("MAX_CREATE", "20") or "20")
    label = env("LABEL", "warning") or "warning"
    comment_on_change = flag("COMMENT_ON_CHANGE", True)
    tools = set(t.strip() for t in env("TOOLS").split(",") if t.strip())
    sonar_key = env("SONAR_PROJECT_KEY")
    sonar_host = env("SONAR_HOST", "https://sonarcloud.io") or "https://sonarcloud.io"

    # Every source read in full first: `present` holds all its open warnings (any severity),
    # so a warning outside the severity set never closes its issue.
    present, sources, notes = {}, [], []
    if flag("CODE_SCANNING", True):
        try:
            present.update(code_scanning_warnings(repo, tools))
            sources.append("code-scanning/")
        except Unavailable as e:
            notes.append(f"code scanning skipped: {e}")
    if sonar_key:
        try:
            present.update(sonar_warnings(sonar_host, env("SONAR_TOKEN"), sonar_key,
                                          env("SONAR_BRANCH")))
            sources.append(f"sonar/{sonar_key}/")
        except Unavailable as e:
            notes.append(f"SonarCloud skipped: {e}")
    for n in notes:
        print(f"::warning title=warnings-to-issues::{n}")

    issues = existing_issues(repo, label)
    wanted = {fp: w for fp, w in present.items() if w["severity"] in severities}
    plan = {"create": [], "reopen": [], "comment": [], "close": [], "deferred": [], "kept": 0}

    def order(fp):  # oldest first: alert numbers ascending, Sonar keys alphabetically
        n = present[fp]["number"]
        return (present[fp]["source"], n if isinstance(n, int) else 0, str(n))

    budget = max_create
    for fp in sorted(wanted, key=order):
        w, issue = wanted[fp], issues.get(fp)
        if issue is None or (issue["state"] == "closed"
                             and issue.get("state_reason") == "completed"):
            if budget <= 0:
                plan["deferred"].append((fp, w))
                continue
            budget -= 1
            plan["create" if issue is None else "reopen"].append((fp, w, issue))
        elif issue["state"] == "open":
            m = LOCATION.search(issue.get("body") or "")
            old = m.group(1) if m else ""
            if comment_on_change and m and old != clean(w["location"]):
                plan["comment"].append((fp, w, issue, old))
            else:
                plan["kept"] += 1
        # closed as not planned (or duplicate): left alone for good
    for fp, issue in sorted(issues.items(), key=lambda kv: kv[1]["number"]):
        if issue["state"] == "open" and fp not in present \
                and any(fp.startswith(s) for s in sources):
            plan["close"].append((fp, issue))

    if not dry:
        apply(repo, label, plan)
    report(repo, dry, sources, notes, plan, len(wanted), max_create)


def ensure_labels(repo, names):
    for name in names:
        if name.startswith("severity:"):
            color = SEVERITY_COLOR.get(name.split(":", 1)[1], "BFDADC")
            desc = f"Warning severity {name.split(':', 1)[1]}"
        elif name in SOURCE_LABELS:
            color, desc = SOURCE_LABELS[name]
        else:
            color, desc = "FBCA04", "Linter or security warning (warnings-to-issues)"
        try:
            gh_write("POST", f"repos/{repo}/labels",
                     {"name": name, "color": color, "description": desc})
        except (RuntimeError, Unavailable):
            pass  # exists already (HTTP 422)


def apply(repo, label, plan):
    needed = set()
    for _fp, w, _ in plan["create"] + plan["reopen"]:
        needed.update(labels_of(label, w))
    if needed:
        ensure_labels(repo, sorted(needed))
    for fp, w, _ in plan["create"]:
        made = gh_write("POST", f"repos/{repo}/issues",
                        {"title": title_of(w), "body": body_of(fp, w),
                         "labels": labels_of(label, w)})
        print(f"::notice title=warnings-to-issues::opened #{made.get('number')} for {fp}")
    for fp, w, issue in plan["reopen"]:
        n = issue["number"]
        gh_write("PATCH", f"repos/{repo}/issues/{n}",
                 {"state": "open", "body": body_of(fp, w), "labels": labels_of(label, w)})
        gh_write("POST", f"repos/{repo}/issues/{n}/comments",
                 {"body": f"The warning is back ({w['url'] or fp}); reopening."})
    for fp, w, issue, old in plan["comment"]:
        n = issue["number"]
        marker = f"<!-- warning-location: {clean(w['location'])} -->"
        new_body = LOCATION.sub(lambda _m, m=marker: m, issue.get("body") or "", count=1)
        gh_write("POST", f"repos/{repo}/issues/{n}/comments",
                 {"body": f"The warning moved: `{clean(old)}` -> `{clean(w['location'])}` "
                          f"({w['url'] or fp})."})
        gh_write("PATCH", f"repos/{repo}/issues/{n}", {"body": new_body})
    for fp, issue in plan["close"]:
        n = issue["number"]
        why = "is no longer reported"
        if fp.startswith("code-scanning/"):
            try:
                alert = json.loads(gh(["api", f"repos/{repo}/code-scanning/alerts/"
                                              f"{fp.split('/', 1)[1]}"]))
                state = alert.get("state")
                if state and state != "open":
                    why = f"is {state}"
            except (RuntimeError, Unavailable, ValueError):
                pass
        else:
            why = "is resolved in SonarCloud"
        gh_write("POST", f"repos/{repo}/issues/{n}/comments",
                 {"body": f"The warning `{fp}` {why}; closing automatically."})
        gh_write("PATCH", f"repos/{repo}/issues/{n}",
                 {"state": "closed", "state_reason": "completed"})


def report(repo, dry, sources, notes, plan, wanted, max_create):
    verb = "would " if dry else ""
    lines = [f"## warnings-to-issues: {repo}{' (dry run)' if dry else ''}", "",
             f"Sources read: {', '.join(s.rstrip('/') for s in sources) or 'none'}. "
             f"Warnings in the severity set: {wanted}. Already tracked: {plan['kept']}.", ""]
    lines += [f"- {n}" for n in notes]
    for fp, w, _ in plan["create"]:
        lines.append(f"- {verb}create: {title_of(w)} `{fp}`")
    for fp, w, issue in plan["reopen"]:
        lines.append(f"- {verb}reopen #{issue['number']}: {title_of(w)} `{fp}`")
    for _fp, w, issue, old in plan["comment"]:
        lines.append(f"- {verb}comment on #{issue['number']}: moved `{clean(old)}` -> "
                     f"`{clean(w['location'])}`")
    for fp, issue in plan["close"]:
        lines.append(f"- {verb}close #{issue['number']} `{fp}`")
    if plan["deferred"]:
        lines.append(f"- deferred to a later run (cap {max_create} per run): "
                     f"{len(plan['deferred'])}")
    text = "\n".join(lines) + "\n"
    print(text)
    summary = env("GITHUB_STEP_SUMMARY")
    if summary:
        with open(summary, "a", encoding="utf-8") as f:
            f.write(text)
    output = env("GITHUB_OUTPUT")
    if output:
        with open(output, "a", encoding="utf-8") as f:
            for k in ("create", "reopen", "comment", "close", "deferred"):
                f.write(f"{k}={len(plan[k])}\n")


if __name__ == "__main__":
    main()
