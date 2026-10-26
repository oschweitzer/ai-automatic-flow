#!/usr/bin/env bash
# Tests checks.sh and the profile reader. Needs git.

set -euo pipefail

SCRIPTS="$(cd "$(dirname "${BASH_SOURCE[0]}")/../scripts" && pwd)"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/checks-test.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT

git -C "$WORK" init -q -b develop
mkdir -p "$WORK/.claude"
cat >"$WORK/.claude/ticket-flow.md" <<'PROFILE'
---
forge: gitlab              # detected
base_branch: "develop"
commands:
  lint: echo linted
  typecheck: "true"
  test: for i in $(seq 1 100); do echo line $i; done; exit 3
run_app: npm run dev
---

## Module map

commands:
  lint: not this one
PROFILE

FAILURES=0
check() {
  if [[ "$2" == "$3" ]]; then echo "ok    $1"; else
    echo "FAIL  $1: expected [$2] got [$3]"; FAILURES=$((FAILURES + 1)); fi
}

# shellcheck source=../scripts/lib.sh
source "$SCRIPTS/lib.sh"
P="$WORK/.claude/ticket-flow.md"
check "flat key, comment stripped" "gitlab" "$(profile_get "$P" forge)"
check "quotes stripped" "develop" "$(profile_get "$P" base_branch)"
check "nested key" "echo linted" "$(profile_get "$P" commands.lint)"
check "key after a nested block" "npm run dev" "$(profile_get "$P" run_app)"
check "missing key" "" "$(profile_get "$P" commands.build)"

out="$(TAIL_LINES=3 bash "$SCRIPTS/checks.sh" "$WORK" || echo "exit=$?")"
check "checks stop at the first failure and print its tail only" \
  "$(printf '%s\n' 'ok    lint' 'ok    typecheck' 'FAIL  test: for i in $(seq 1 100); do echo line $i; done; exit 3' \
    '----- last 3 lines' 'line 98' 'line 99' 'line 100' 'exit=1')" "$out"
check "a missing command is skipped" "skip  build (no command in the profile)" "$(bash "$SCRIPTS/checks.sh" "$WORK" build)"

echo
if [[ "$FAILURES" -gt 0 ]]; then echo "$FAILURES check(s) failed" >&2; exit 1; fi
echo "all checks passed"
