#!/usr/bin/env python3
"""Pin sync: moves every pyrlyn/ci reference in the target repositories to one commit of
pyrlyn/ci and keeps the files rendered from templates/ in line, through one pull request per
repository from a bot branch. Config: tools/pin-sync/targets.yml (see its header).

  pin_sync.py --config tools/pin-sync/targets.yml --sha <40-hex> [--dry-run] [--repo NAME]
  pin_sync.py apply --root <checkout> --target-json '<target>' --sha <40-hex> --templates <dir>

`apply` rewrites one checkout in place and prints the changed paths (used by the tests and by
the sync loop). The sync loop clones each target with `gh` (GH_TOKEN), applies, and when
anything changed force-pushes the bot branch and opens or updates its pull request. It never
merges: the target's required checks and a human decide.
"""
from __future__ import annotations

import argparse
import fnmatch
import json
import os
import pathlib
import re
import subprocess
import sys
import tempfile

# `uses: pyrlyn/ci/<path>@<sha>` with an optional trailing comment (replaced by `# main`).
PIN = re.compile(r"(pyrlyn/ci/[\w./-]+)@[0-9a-f]{40}(?:[ \t]*#[^\n]*)?")
SHA = re.compile(r"^[0-9a-f]{40}$")
DEFAULT_PATHS = [".github/**/*.yml", ".github/**/*.yaml"]
# `{{name}}`, but not a GitHub expression `${{ ... }}`.
VAR = re.compile(r"(?<!\$)\{\{\s*([\w-]+)\s*\}\}")


def load_config(path: str) -> dict:
    try:
        import yaml  # type: ignore

        with open(path, encoding="utf-8") as handle:
            return yaml.safe_load(handle)
    except ImportError:
        out = subprocess.run(
            ["yq", "-o=json", ".", path], check=True, capture_output=True, text=True
        ).stdout
        return json.loads(out)


def matches(rel: str, patterns: list[str]) -> bool:
    # fnmatch's `*` crosses `/`, so `.github/**/*.yml` also matches `.github/x.yml`.
    return any(fnmatch.fnmatchcase(rel, p.replace("**/", "*")) for p in patterns)


def render(template: str, variables: dict, sha: str) -> str:
    values = {"sha": sha, **{k: str(v) for k, v in variables.items()}}

    def sub(match: re.Match) -> str:
        name = match.group(1)
        if name not in values:
            raise SystemExit(f"template variable `{name}` is not set")
        return values[name]

    return VAR.sub(sub, template)


def apply(root: pathlib.Path, target: dict, sha: str, templates: pathlib.Path) -> list[str]:
    """Rewrites `root` for `target`; returns the changed paths (relative, sorted)."""
    if not SHA.match(sha):
        raise SystemExit(f"--sha must be 40 lowercase hex characters, got {sha!r}")
    changed: set[str] = set()
    rendered: set[str] = set()
    for item in target.get("templates") or []:
        dest = item["dest"]
        source = templates / item["template"]
        text = render(source.read_text(encoding="utf-8"), item.get("vars") or {}, sha)
        path = root / dest
        rendered.add(dest)
        if not path.exists() or path.read_text(encoding="utf-8") != text:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")
            changed.add(dest)
    paths = target.get("paths") or DEFAULT_PATHS
    exclude = target.get("exclude") or []
    for path in sorted(root.rglob("*")):
        if not path.is_file() or ".git" in path.relative_to(root).parts:
            continue
        rel = path.relative_to(root).as_posix()
        if rel in rendered or not matches(rel, paths) or matches(rel, exclude):
            continue
        text = path.read_text(encoding="utf-8")
        new = PIN.sub(lambda m: f"{m.group(1)}@{sha} # main", text)
        if new != text:
            path.write_text(new, encoding="utf-8")
            changed.add(rel)
    return sorted(changed)


def run(cmd: list[str], cwd: str | None = None, capture: bool = False) -> str:
    result = subprocess.run(cmd, cwd=cwd, check=True, text=True, capture_output=capture)
    return result.stdout if capture else ""


def body(sha: str, changed: list[str]) -> str:
    files = "\n".join(f"- `{c}`" for c in changed)
    return (
        f"Moves every pyrlyn/ci reference to [`{sha[:7]}`]"
        f"(https://github.com/pyrlyn/ci/commit/{sha}) and refreshes the files rendered from "
        "pyrlyn/ci's templates.\n\n"
        f"Changed files:\n\n{files}\n\n"
        "Opened by pyrlyn/ci's pin-sync workflow (tools/pin-sync). It is never merged "
        "automatically: check the pyrlyn/ci changes since the previous pin (permissions a "
        "reusable workflow now asks for, renamed inputs) and merge when the required checks "
        "are green. A later sync force-pushes this branch.\n"
    )


def sync(config: dict, sha: str, dry_run: bool, only: str | None, templates: pathlib.Path) -> int:
    branch = config.get("branch", "ci/pin-sync")
    label = config.get("label")
    title = config.get("title", "ci: pin pyrlyn/ci to {short}").format(short=sha[:7], sha=sha)
    failures = 0
    print(f"## pin-sync to `{sha[:7]}`{' (dry run)' if dry_run else ''}\n")
    for target in config["targets"]:
        repo = target["repo"]
        if only and repo not in (only, f"pyrlyn/{only}"):
            continue
        try:
            with tempfile.TemporaryDirectory() as tmp:
                run(["gh", "repo", "clone", repo, tmp, "--", "--depth", "1", "--quiet"])
                changed = apply(pathlib.Path(tmp), target, sha, templates)
                if not changed:
                    print(f"- {repo}: up to date")
                    continue
                if dry_run:
                    print(f"- {repo}: would change {', '.join(changed)}")
                    continue
                run(["git", "switch", "-q", "-C", branch], cwd=tmp)
                run(["git", "add", "--", *changed], cwd=tmp)
                run(
                    [
                        "git", "-c", "user.name=pyrlyn-ci-sync[bot]",
                        "-c", "user.email=pyrlyn-ci-sync[bot]@users.noreply.github.com",
                        "commit", "-q", "-m", title,
                    ],
                    cwd=tmp,
                )
                ref = f"HEAD:refs/heads/{branch}"
                run(["git", "push", "-q", "--force", "origin", ref], cwd=tmp)
                existing = run(
                    ["gh", "pr", "list", "-R", repo, "--head", branch, "--state", "open",
                     "--json", "number", "--jq", ".[0].number // empty"],
                    capture=True,
                ).strip()
                text = body(sha, changed)
                if existing:
                    run(["gh", "pr", "edit", existing, "-R", repo, "--title", title,
                         "--body", text])
                    print(f"- {repo}: updated #{existing}")
                else:
                    cmd = ["gh", "pr", "create", "-R", repo, "--head", branch, "--base",
                           target.get("base", "main"), "--title", title, "--body", text]
                    if label:
                        cmd += ["--label", label]
                    url = run(cmd, capture=True).strip()
                    print(f"- {repo}: opened {url}")
        except subprocess.CalledProcessError as error:
            failures += 1
            print(f"- {repo}: FAILED ({error})")
    return 1 if failures else 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    sub = parser.add_subparsers(dest="cmd")
    one = sub.add_parser("apply")
    one.add_argument("--root", required=True)
    one.add_argument("--target-json", required=True)
    one.add_argument("--sha", required=True)
    one.add_argument("--templates", required=True)
    parser.add_argument("--config", default="tools/pin-sync/targets.yml")
    parser.add_argument("--sha")
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--repo")
    args = parser.parse_args()
    if args.cmd == "apply":
        changed = apply(pathlib.Path(args.root), json.loads(args.target_json), args.sha,
                        pathlib.Path(args.templates))
        print("\n".join(changed))
        return 0
    if not args.sha:
        parser.error("--sha is required")
    if not SHA.match(args.sha):
        parser.error("--sha must be 40 lowercase hex characters")
    config = load_config(args.config)
    templates = pathlib.Path(config.get("templates", "templates"))
    return sync(config, args.sha, args.dry_run, args.repo, templates)


if __name__ == "__main__":
    sys.exit(main())
