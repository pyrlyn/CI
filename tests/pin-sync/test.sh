#!/usr/bin/env bash
# tools/pin-sync/pin_sync.py `apply` on a throwaway checkout: pins move, templates render,
# excluded and non-matching files stay, a second run changes nothing.
set -euo pipefail
cd "$(dirname "$0")/../.."
tool=tools/pin-sync/pin_sync.py
root=$(mktemp -d)
trap 'rm -rf "$root"' EXIT
old=1111111111111111111111111111111111111111
new=2222222222222222222222222222222222222222
mkdir -p "$root/.github/workflows" "$root/.github/actions/x" "$root/src"
cat > "$root/.github/workflows/ci.yml" <<YAML
jobs:
  a:
    uses: pyrlyn/ci/.github/workflows/ci.yml@$old # main
  b:
    steps:
      - uses: pyrlyn/ci/.github/actions/config@$old
      - uses: actions/checkout@$old # v7
YAML
cat > "$root/.github/actions/x/action.yml" <<YAML
runs:
  steps:
    - uses: pyrlyn/ci/.github/actions/setup-rust@$old # v1
YAML
cp "$root/.github/workflows/ci.yml" "$root/.github/workflows/release.yml"
echo "pyrlyn/ci/.github/workflows/ci.yml@$old" > "$root/src/notes.yml"
target='{"exclude": [".github/workflows/release.yml"],
  "templates": [{"template": "dependabot.yml", "dest": ".github/workflows/dependabot.yml",
                 "vars": {"wait-workflow": "ci.yml"}}]}'
changed=$(python3 "$tool" apply --root "$root" --target-json "$target" --sha "$new" \
  --templates templates)
expected=$'.github/actions/x/action.yml\n.github/workflows/ci.yml\n.github/workflows/dependabot.yml'
[ "$changed" = "$expected" ] || { echo "changed: $changed" >&2; exit 1; }
grep -q "ci.yml@$new # main" "$root/.github/workflows/ci.yml"
grep -q "config@$new # main" "$root/.github/workflows/ci.yml"
grep -q "checkout@$old # v7" "$root/.github/workflows/ci.yml"
grep -q "setup-rust@$new # main" "$root/.github/actions/x/action.yml"
grep -q "ci.yml@$old # main" "$root/.github/workflows/release.yml"
grep -q "@$old" "$root/src/notes.yml"
grep -q "dependabot-automerge.yml@$new # main" "$root/.github/workflows/dependabot.yml"
grep -q 'wait-workflow: ci.yml' "$root/.github/workflows/dependabot.yml"
grep -q 'number }}' "$root/.github/workflows/dependabot.yml"
if grep -v '\${{' "$root/.github/workflows/dependabot.yml" | grep -q '{{'; then
  echo "an unfilled template variable is left" >&2
  exit 1
fi
again=$(python3 "$tool" apply --root "$root" --target-json "$target" --sha "$new" \
  --templates templates)
[ -z "$again" ] || { echo "second run changed: $again" >&2; exit 1; }
if python3 "$tool" apply --root "$root" --target-json '{}' --sha nothex --templates templates \
  2>/dev/null; then
  echo "a bad sha was accepted" >&2
  exit 1
fi
bad='{"templates": [{"template": "dependabot.yml", "dest": "x.yml"}]}'
if python3 "$tool" apply --root "$root" --target-json "$bad" --sha "$new" --templates templates \
  2>/dev/null; then
  echo "a missing template variable was accepted" >&2
  exit 1
fi
echo "pin-sync: ok"
