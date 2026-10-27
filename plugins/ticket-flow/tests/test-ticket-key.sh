#!/usr/bin/env bash
# Tests ticket-key.sh, which normalises the ticket key from the profile's
# tracker. Needs git.

set -euo pipefail

SCRIPTS="$(cd "$(dirname "${BASH_SOURCE[0]}")/../scripts" && pwd)"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/ticket-key-test.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT

git -C "$WORK" init -q -b develop
mkdir -p "$WORK/.claude"
cd "$WORK"

FAILURES=0
check() {
  if [[ "$2" == "$3" ]]; then echo "ok    $1"; else
    echo "FAIL  $1: expected [$2] got [$3]"; FAILURES=$((FAILURES + 1)); fi
}
profile() { { echo ---; printf '%s\n' "$@"; echo ---; } >.claude/ticket-flow.md; }
key() { bash "$SCRIPTS/ticket-key.sh" "$@" 2>/dev/null; }
status() { bash "$SCRIPTS/ticket-key.sh" "$@" >/dev/null 2>&1 && echo 0 || echo $?; }

check "no profile fails" "1" "$(status SHOP-142)"

profile "forge: gitlab"
check "jira key unchanged without tracker" "SHOP-142" "$(key SHOP-142)"

profile "forge: gitlab" "tracker: jira"
check "jira key unchanged with tracker jira" "shop-142" "$(key shop-142)"

profile "forge: github" "tracker: github"
check "github number" "GH-142" "$(key 142)"
check "github hash number" "GH-142" "$(key '#142')"
check "github key any case" "GH-142" "$(key gh-142)"
check "github leading zeros" "GH-142" "$(key GH-0142)"
check "github refuses a jira key" "1" "$(status SHOP-142)"
check "github refuses zero" "1" "$(status 0)"
check "github refuses a bare hash" "1" "$(status '#')"
check "github refuses an empty key" "1" "$(status '')"

profile "forge: gitlab" "tracker: github"
check "github tracker on gitlab refused" "1" "$(status 142)"
message="$(bash "$SCRIPTS/ticket-key.sh" 142 2>&1 >/dev/null || true)"
check "github tracker on gitlab says not supported" "yes" "$([[ "$message" == *"not supported"* ]] && echo yes || echo no)"

profile "forge: github" "tracker: linear"
check "unknown tracker refused" "1" "$(status 142)"

exit "$FAILURES"
