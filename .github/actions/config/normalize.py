"""Normalize the merged ci.yml configuration (JSON on stdin) into what ci.yml's jobs read.

Writes `json`, `jobs`, `skip-ok`, `draft-skip` and `docs-only` to $GITHUB_OUTPUT and a summary
to $GITHUB_STEP_SUMMARY.
Pure standard library: runs on any GitHub-hosted runner's python3.
"""
import json
import os
import re
import sys

CHECKS = ["rust", "dotnet", "codeql", "semgrep", "snyk", "sonarcloud", "lint", "license",
          "commits"]
JOB_DEFAULTS = {
    "enabled": True,
    "events": [],
    "runs-on": "ubuntu-latest",
    "matrix": [],
    "setup": "",
    "mise": True,
    "rust-pin": False,
    "rust-cache": False,
    "tools": "",
    "env": {},
    "working-directory": ".",
    "shell": "bash",
    "timeout-minutes": 60,
    "fetch-depth": 1,
    "allow-failure": False,
    "docs-only": False,
}
DOCS_ONLY_KEYS = {"enabled", "events", "paths", "exclude", "base-exclude"}
COMMIT_TOOLS = ("grep", "commitlint")
KNOWN_JOB_KEYS = set(JOB_DEFAULTS) | {"name", "run"}


def fail(msg):
    print(f"::error title=infra config::{msg}")
    sys.exit(1)


def as_bool(value, private, where):
    if value == "auto":
        return not private
    if isinstance(value, bool):
        return value
    fail(f"{where}: expected true, false or auto, got {value!r}")


def event_ok(events, event, where):
    if events in (None, ""):
        return True
    if not isinstance(events, list):
        fail(f"{where}.events must be a list")
    return not events or event in events


def main():
    cfg = json.load(sys.stdin) or {}
    private = os.environ.get("PRIVATE") == "true"
    event = os.environ.get("EVENT", "")
    draft = os.environ.get("DRAFT") == "true"
    cancel_override = os.environ.get("CANCEL_OVERRIDE", "")
    # The `changes` action's docs_only, computed by action.yml only when docs-only applies.
    docs_detected = os.environ.get("DOCS_ONLY") == "true"

    if cfg.get("version") != 1:
        fail(f"version must be 1, got {cfg.get('version')!r}")
    unknown = set(cfg) - set(CHECKS) - {"version", "cancel-run-on-failure", "skip-drafts",
                                          "upload-sarif", "jobs", "docs-only"}
    if unknown:
        fail(f"unknown top-level keys: {sorted(unknown)}")

    cfg["skip-drafts"] = bool(cfg.get("skip-drafts", True))
    skip_all = cfg["skip-drafts"] and draft

    docs = cfg.get("docs-only") or {}
    if not isinstance(docs, dict):
        fail("docs-only must be a map")
    extra = set(docs) - DOCS_ONLY_KEYS
    if extra:
        fail(f"docs-only: unknown keys {sorted(extra)}")
    if not isinstance(docs.get("enabled", False), bool):
        fail("docs-only.enabled must be true or false")
    for key in ("paths", "exclude", "base-exclude"):
        if not isinstance(docs.get(key) or [], list):
            fail(f"docs-only.{key} must be a list of regexes")
    # Independent of skip_all: a caller's local jobs read `docs-only` on a draft PR too (their own
    # draft handling is theirs). The checks below are off on a skipped draft either way.
    docs_only = bool(docs.get("enabled", False)
                     and event_ok(docs.get("events"), event, "docs-only") and docs_detected)
    docs["active"] = docs_only
    cfg["docs-only"] = docs
    cfg["upload-sarif"] = as_bool(cfg.get("upload-sarif", "auto"), private, "upload-sarif")
    if cancel_override in ("true", "false"):
        cfg["cancel-run-on-failure"] = cancel_override == "true"

    skip_ok = []
    for name in CHECKS:
        c = cfg.get(name) or {}
        run = (as_bool(c.get("enabled", False), private, f"{name}.enabled")
               and event_ok(c.get("events"), event, name) and not skip_all
               and not (docs_only and not c.get("docs-only", False)))
        c["run"] = run
        if not run:
            skip_ok.append(name)
        cfg[name] = c

    # Inputs of type string that take JSON.
    rust = cfg["rust"]
    rust["matrix-json"] = json.dumps(rust.get("matrix") or []) if rust.get("matrix") else ""
    locked = rust.get("locked", "auto")
    if isinstance(locked, bool):
        locked = "true" if locked else "false"
    if locked not in ("auto", "true", "false"):
        fail(f"rust.locked: expected true, false or auto, got {locked!r}")
    rust["locked"] = locked

    commits = cfg["commits"]
    tool = commits.get("tool", "grep")
    if tool not in COMMIT_TOOLS:
        fail(f"commits.tool: expected one of {list(COMMIT_TOOLS)}, got {tool!r}")
    types = commits.get("types") or []
    if not isinstance(types, list) or not all(
            isinstance(t, str) and re.fullmatch(r"[a-z][a-z0-9-]*", t) for t in types):
        fail("commits.types must be a list of lowercase commit types")
    commits["tool"] = tool
    commits["types-list"] = " ".join(types)
    cfg["codeql"]["languages-json"] = json.dumps(cfg["codeql"].get("languages") or ["actions"])

    jobs = []
    for i, raw in enumerate(cfg.get("jobs") or []):
        where = f"jobs[{i}]"
        if not isinstance(raw, dict) or not raw.get("name") or not raw.get("run"):
            fail(f"{where}: `name` and `run` are required")
        extra = set(raw) - KNOWN_JOB_KEYS
        if extra:
            fail(f"{where} ({raw['name']}): unknown keys {sorted(extra)}")
        job = {**JOB_DEFAULTS, **raw}
        if not job["enabled"] or skip_all or not event_ok(job["events"], event, where):
            continue
        if docs_only and not job["docs-only"]:
            continue
        if job["shell"] not in ("bash", "pwsh"):
            fail(f"{where}: shell must be bash or pwsh")
        runs_on = job["runs-on"] if isinstance(job["runs-on"], list) else [job["runs-on"]]
        variants = job["matrix"] or [{}]
        for base_label in runs_on:
            for variant in variants:
                variant = dict(variant)
                # A matrix entry may pick its own runner: {runs-on: macos-latest, target: ...}.
                os_label = variant.pop("runs-on", base_label)
                env = {str(k): str(v) for k, v in (job["env"] or {}).items()}
                env.update({re.sub(r"[^A-Z0-9_]", "_", str(k).upper()): str(v)
                            for k, v in variant.items()})
                label = ", ".join([*(str(v) for v in variant.values()),
                                   *([os_label] if len(runs_on) > 1 else [])])
                mise = job["mise"]
                jobs.append({
                    "name": f"{job['name']} ({label})" if label else job["name"],
                    "runs-on": os_label,
                    "run": job["run"],
                    "setup": job["setup"],
                    "mise": "false" if mise is False else "true",
                    "mise-install-args": "" if mise in (True, False) else str(mise),
                    "rust-pin": bool(job["rust-pin"]),
                    "rust-cache": bool(job["rust-cache"]),
                    "tools": job["tools"],
                    "env": json.dumps(env),
                    "working-directory": job["working-directory"],
                    "shell": job["shell"],
                    "timeout-minutes": int(job["timeout-minutes"]),
                    "fetch-depth": int(job["fetch-depth"]),
                    "allow-failure": bool(job["allow-failure"]),
                })
    if len(jobs) > 256:
        fail(f"{len(jobs)} custom jobs; a matrix holds at most 256")
    if not jobs:
        skip_ok.append("custom")
    cfg["jobs"] = jobs

    out = os.environ["GITHUB_OUTPUT"]
    with open(out, "a", encoding="utf-8") as fh:
        fh.write(f"json={json.dumps(cfg, separators=(',', ':'))}\n")
        fh.write(f"jobs={json.dumps(jobs, separators=(',', ':'))}\n")
        fh.write(f"skip-ok={' '.join(skip_ok)}\n")
        fh.write(f"draft-skip={'true' if skip_all else 'false'}\n")
        fh.write(f"docs-only={'true' if docs_only else 'false'}\n")
    summary = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary:
        with open(summary, "a", encoding="utf-8") as fh:
            fh.write("### infra config\n\n| check | runs |\n| --- | --- |\n")
            for name in CHECKS:
                fh.write(f"| {name} | {cfg[name]['run']} |\n")
            fh.write(f"| custom jobs | {len(jobs)} |\n")
            if docs_only:
                fh.write("\nDocs-only change: checks without `docs-only: true` are skipped.\n")


if __name__ == "__main__":
    main()
