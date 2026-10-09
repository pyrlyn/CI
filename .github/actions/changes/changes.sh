#!/usr/bin/env bash
# The predicates below are called through `flag`, which shellcheck cannot follow.
# shellcheck disable=SC2329
# Classify changed files by ecosystem, and whether they are documentation only; see
# action.yml. Writes key=value lines to $GITHUB_OUTPUT (stdout when unset) and a table to
# $GITHUB_STEP_SUMMARY.
#
# Test hook (self-test.yml, local runs): CHANGED_FILES_FILE and TREE_FILES_FILE name files
# with one path per line and replace the API calls; EVENT still decides whether a diff counts.
set -uo pipefail

# Dependency manifests and lock files. A change here forces that ecosystem's full suite.
RUST_DEPS_RE='(^|/)Cargo\.(toml|lock)$'
SWIFT_DEPS_RE='(^|/)Package(@swift-[0-9.]+)?\.swift$|(^|/)Package\.resolved$|(^|/)project\.yml$|\.xcodeproj/project\.pbxproj$'
DOTNET_DEPS_RE='\.(cs|fs|vb)proj$|(^|/)Directory\.Packages\.props$|(^|/)packages\.lock\.json$|(^|/)global\.json$|(^|/)nuget\.config$'
# Other files of each ecosystem (sources and tool configuration).
RUST_SRC_RE='\.rs$|(^|/)\.cargo/|(^|/)rust-toolchain(\.toml)?$|(^|/)\.?(clippy|rustfmt|deny)\.toml$|(^|/)\.config/nextest\.toml$'
SWIFT_SRC_RE='\.(swift|m|mm|metal|xcconfig|entitlements|xcstrings|storyboard|xib)$|\.(xcodeproj|xcworkspace|xcassets)/|(^|/)\.swiftlint\.ya?ml$|(^|/)\.swift-format$'
DOTNET_SRC_RE='\.(cs|fs|fsi|fsx|vb|razor|cshtml|xaml|resx|props|targets|sln|slnx|runsettings)$|(^|/)\.config/dotnet-tools\.json$'
# CI and toolchain pins: every ecosystem runs in full.
FORCE_RE='^\.github/|(^|/)\.?mise\.toml$|(^|/)\.tool-versions$'
# Documentation: Markdown in any directory (`DOCS_PATHS` adds more), minus the `NOT_DOCS_PATHS`
# matches (Markdown that is code: embedded in a binary, run by a test, read by a release).
DOCS_RE='\.md$'
# Project markers in the tree at HEAD.
RUST_PRESENT_RE='(^|/)Cargo\.toml$'
SWIFT_PRESENT_RE='(^|/)Package\.swift$|\.xcodeproj/'
DOTNET_PRESENT_RE='\.(cs|fs|vb)proj$|\.slnx?$'

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
changed="$tmp/changed"
tree="$tmp/tree"
: >"$changed"
: >"$tree"
forced=false
reason=""

force() {
  forced=true
  [ -n "$reason" ] || reason="$1"
}

zero='0000000000000000000000000000000000000000'
BASE="${BASE:-}"
HEAD="${HEAD:-}"
BASE="${BASE// /}"
HEAD="${HEAD// /}"

# 1. The changed files.
case "${EVENT:-}" in
  pull_request | pull_request_target | merge_group | push)
    if [ -n "${CHANGED_FILES_FILE:-}" ]; then
      cp "$CHANGED_FILES_FILE" "$changed"
    elif [ -z "$BASE" ] || [ "$BASE" = "$zero" ] || [ -z "$HEAD" ]; then
      force "no base commit to diff against (event ${EVENT})"
    else
      # Three-dot compare: the changes since the merge base, as on the pull request page.
      # Renames list both paths. The API returns at most 300 files: more means "everything".
      if json="$(gh api "repos/$REPO/compare/$BASE...$HEAD" 2>"$tmp/err")"; then
        count="$(jq '.files | length' <<<"$json")"
        if [ "$count" -ge 300 ]; then
          force "$count+ changed files (API limit)"
        else
          jq -r '.files[] | .filename, (.previous_filename // empty)' <<<"$json" >"$changed"
        fi
      else
        force "compare API failed: $(tr '\n' ' ' <"$tmp/err")"
      fi
    fi
    ;;
  *)
    force "event ${EVENT:-unknown} has no diff"
    ;;
esac

# 2. The tree at HEAD, for the *_present outputs.
tree_ok=true
if [ -n "${TREE_FILES_FILE:-}" ]; then
  cp "$TREE_FILES_FILE" "$tree"
elif ! json="$(gh api "repos/$REPO/git/trees/${HEAD:-HEAD}?recursive=1" 2>"$tmp/err")"; then
  tree_ok=false
  echo "::warning title=changes::tree API failed; every *_present output is true"
elif [ "$(jq -r '.truncated' <<<"$json")" = "true" ]; then
  tree_ok=false
  echo "::notice title=changes::tree listing truncated; every *_present output is true"
else
  jq -r '.tree[] | select(.type == "blob") | .path' <<<"$json" >"$tree"
fi

matches() { # <file> <regex> [i = ignore case]
  if [ "${3:-}" = i ]; then grep -Eqi -e "$2" "$1"; else grep -Eq -e "$2" "$1"; fi
}
extra() { # <file> <newline-separated regexes>
  local re
  while IFS= read -r re; do
    [ -n "$re" ] || continue
    grep -Eq -e "$re" "$1" && return 0
  done <<<"$2"
  return 1
}
flag() { if "$@"; then echo true; else echo false; fi; }

if [ "$forced" = false ] && matches "$changed" "$FORCE_RE"; then
  force "CI or toolchain files changed: $(grep -E "$FORCE_RE" "$changed" | head -3 | paste -sd " " -)"
fi

rust_deps=$(flag matches "$changed" "$RUST_DEPS_RE")
swift_deps=$(flag matches "$changed" "$SWIFT_DEPS_RE")
dotnet_deps=$(flag matches "$changed" "$DOTNET_DEPS_RE" i)

any() { # <ecosystem deps flag> <src regex> <extra regexes> [i = ignore case]
  [ "$forced" = true ] || [ "$1" = true ] || matches "$changed" "$2" "${4:-}" \
    || extra "$changed" "$3"
}
rust=$(flag any "$rust_deps" "$RUST_SRC_RE" "${RUST_PATHS:-}")
swift=$(flag any "$swift_deps" "$SWIFT_SRC_RE" "${SWIFT_PATHS:-}")
dotnet=$(flag any "$dotnet_deps" "$DOTNET_SRC_RE" "${DOTNET_PATHS:-}" i)

full() { [ "$forced" = true ] || [ "$1" = true ]; }

# docs_only: a diff exists, nothing forced it, and every changed path (both sides of a rename)
# is documentation. The `git diff --name-only BASE...HEAD` of the pull request, file by file.
is_doc() { # <path>
  printf '%s\n' "$1" >"$tmp/one"
  { matches "$tmp/one" "$DOCS_RE" i || extra "$tmp/one" "${DOCS_PATHS:-}"; } \
    && ! extra "$tmp/one" "${NOT_DOCS_PATHS:-}"
}
docs_only=false
not_doc=""
if [ "$forced" = false ] && [ -s "$changed" ]; then
  docs_only=true
  while IFS= read -r f; do
    [ -n "$f" ] || continue
    if ! is_doc "$f"; then
      docs_only=false
      not_doc="$f"
      break
    fi
  done <"$changed"
fi
present() { [ "$tree_ok" = false ] || matches "$tree" "$1" "${2:-}"; }

files=""
[ "$forced" = true ] && [ ! -s "$changed" ] || files="$(grep -c . "$changed" || true)"

out="${GITHUB_OUTPUT:-/dev/stdout}"
{
  echo "forced=$forced"
  echo "reason=$reason"
  echo "files=$files"
  echo "rust=$rust"
  echo "rust_deps=$rust_deps"
  echo "rust_full=$(flag full "$rust_deps")"
  echo "rust_present=$(flag present "$RUST_PRESENT_RE")"
  echo "swift=$swift"
  echo "swift_deps=$swift_deps"
  echo "swift_full=$(flag full "$swift_deps")"
  echo "swift_present=$(flag present "$SWIFT_PRESENT_RE")"
  echo "dotnet=$dotnet"
  echo "dotnet_deps=$dotnet_deps"
  echo "dotnet_full=$(flag full "$dotnet_deps")"
  echo "dotnet_present=$(flag present "$DOTNET_PRESENT_RE" i)"
  echo "docs_only=$docs_only"
} >>"$out"

if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
  {
    echo "### changes"
    echo
    if [ "$forced" = true ]; then echo "Forced (every ecosystem in full): $reason"; echo; fi
    echo "| ecosystem | changed | deps changed |"
    echo "| --- | --- | --- |"
    echo "| Rust | $rust | $rust_deps |"
    echo "| Swift | $swift | $swift_deps |"
    echo "| .NET | $dotnet | $dotnet_deps |"
    echo
    if [ "$docs_only" = true ]; then
      echo "Documentation only: every changed file is documentation."
    elif [ -n "$not_doc" ]; then
      echo "Not documentation only: \`$not_doc\`."
    fi
  } >>"$GITHUB_STEP_SUMMARY"
fi
[ "$forced" = false ] || echo "::notice title=changes::every ecosystem runs in full: $reason"
exit 0
