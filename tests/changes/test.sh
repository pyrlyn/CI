#!/usr/bin/env bash
# Table test for .github/actions/changes/changes.sh: each case is a list of changed files and
# the outputs it must produce. Runs offline (CHANGED_FILES_FILE/TREE_FILES_FILE test hook).
set -euo pipefail
script="$(cd "$(dirname "$0")/../.." && pwd)/.github/actions/changes/changes.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
printf '%s\n' Cargo.toml crates/a/Cargo.toml desktop/macos/Packages/A/Package.swift >"$tmp/tree"
fail=0

check() { # <name> <event> <changed files, comma-separated> <expected key=value ...>
  local name="$1" event="$2" files="$3"
  shift 3
  tr ',' '\n' <<<"$files" | sed '/^$/d' >"$tmp/changed"
  local got
  got="$(EVENT="$event" CHANGED_FILES_FILE="$tmp/changed" TREE_FILES_FILE="$tmp/tree" \
    SWIFT_PATHS='^scripts/desktop/' DOCS_PATHS='^docs/.*\.png$' NOT_DOCS_PATHS='^src/' \
    GITHUB_OUTPUT="$tmp/out" GITHUB_STEP_SUMMARY='' \
    bash -c ': >"$GITHUB_OUTPUT"; bash "$0" >/dev/null; cat "$GITHUB_OUTPUT"' "$script")"
  local want bad=0
  for want in "$@"; do
    if ! grep -qx -- "$want" <<<"$got"; then
      echo "FAIL $name: want $want, got: $(grep -- "^${want%%=*}=" <<<"$got")"
      bad=1
    fi
  done
  if [ "$bad" -eq 0 ]; then echo "ok   $name"; else fail=1; fi
}

check docs-only pull_request README.md,docs/a.md \
  forced=false rust=false rust_deps=false swift=false dotnet=false docs_only=true
check docs-translations pull_request docs/uk/a.md,docs/ru/b.MD,plan.md docs_only=true
check docs-extra-path pull_request docs/a.md,docs/img/shot.png docs_only=true
check docs-not-docs-path pull_request docs/a.md,src/agents/README.md docs_only=false
check docs-and-code pull_request docs/a.md,crates/a/src/lib.rs docs_only=false rust=true
check docs-other-file pull_request docs/a.md,docs/a.txt docs_only=false
check docs-github-md pull_request .github/pull_request_template.md forced=true docs_only=false
check docs-empty-diff pull_request "" docs_only=false
check docs-schedule schedule "" docs_only=false
check cargo-lock pull_request Cargo.lock \
  rust=true rust_deps=true rust_full=true swift=false swift_full=false
check member-manifest pull_request crates/a/Cargo.toml rust_deps=true rust_full=true
check rust-source pull_request crates/a/src/lib.rs rust=true rust_deps=false rust_full=false
check package-resolved pull_request desktop/macos/Packages/A/Package.resolved \
  swift=true swift_deps=true swift_full=true rust=false
check xcodegen-project pull_request desktop/macos/project.yml swift_deps=true
check pbxproj-pins pull_request App/App.xcodeproj/project.pbxproj swift_deps=true
check swift-source pull_request desktop/macos/App/A.swift swift=true swift_deps=false
check swift-extra-path pull_request scripts/desktop/app.sh swift=true swift_deps=false
check csproj pull_request src/App/App.csproj dotnet=true dotnet_deps=true dotnet_full=true
check central-packages pull_request Directory.Packages.props dotnet_deps=true
check nuget-config-case pull_request NuGet.Config dotnet_deps=true
check global-json pull_request global.json dotnet_deps=true
check lock-json pull_request src/App/packages.lock.json dotnet_deps=true
check cs-source pull_request src/App/Program.cs dotnet=true dotnet_deps=false
check workflow-change pull_request .github/workflows/ci.yml \
  forced=true rust_full=true swift_full=true dotnet_full=true rust_deps=false
check mise-pin push mise.toml forced=true rust=true
check schedule schedule "" forced=true rust_full=true swift_full=true dotnet_full=true
check presence pull_request README.md rust_present=true swift_present=true dotnet_present=false

exit "$fail"
