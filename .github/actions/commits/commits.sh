#!/usr/bin/env bash
# Conventional Commits check for a pull request's commits (see action.yml).
# Env: TOOL (grep | commitlint), TYPES, BASE_REF, WORKDIR, GITHUB_EVENT_NAME.
set -euo pipefail

if [ "${GITHUB_EVENT_NAME:-}" != "pull_request" ] || [ -z "${BASE_REF:-}" ]; then
  echo "::notice title=commits::not a pull request (${GITHUB_EVENT_NAME:-none}); nothing to check"
  exit 0
fi
case "$TOOL" in
  grep | commitlint) ;;
  *) echo "::error title=commits::tool must be grep or commitlint, got '$TOOL'"; exit 1 ;;
esac

# An unresolvable base must be an error: rev-list would print nothing and the check would pass.
base="origin/$BASE_REF"
git rev-parse -q --verify "$base^{commit}" >/dev/null || {
  echo "::error title=commits::base branch $base not found (checkout with fetch-depth: 0)"
  exit 1
}

alternatives="$(tr -s ' \n' '|' <<<"${TYPES:-}" | sed 's/^|//; s/|$//')"
[ -n "$alternatives" ] || { echo "::error title=commits::no commit types"; exit 1; }
pattern="^(${alternatives})(\\([a-z0-9._/-]+\\))?!?: .+"

echo "checking $(git rev-list --count --no-merges "$base..HEAD") commit(s) from $base"
bad=0
for sha in $(git rev-list --no-merges --reverse "$base..HEAD"); do
  subject="$(git log -1 --format=%s "$sha")"
  # git writes reverts in its own format, not as Conventional Commits.
  case "$subject" in
    "Revert "*) continue ;;
  esac
  if [ "$TOOL" = grep ]; then
    printf '%s\n' "$subject" | grep -Eq -- "$pattern" && continue
  else
    (cd "$WORKDIR" && npx --no-install commitlint --from "$sha^" --to "$sha") && continue
  fi
  echo "::error title=commits::not a Conventional Commit: ${sha:0:12} $subject"
  bad=1
done
exit "$bad"
