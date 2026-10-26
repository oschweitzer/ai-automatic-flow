#!/usr/bin/env bash
# Tests guard-git.sh against a throwaway repository. Needs git and jq.

set -euo pipefail

GUARD="$(cd "$(dirname "${BASH_SOURCE[0]}")/../scripts" && pwd)/guard-git.sh"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/guard-test.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT

git -C "$WORK" init -q -b develop
git -C "$WORK" -c user.email=t@t -c user.name=t commit -q --allow-empty -m init
mkdir -p "$WORK/.claude"
cat >"$WORK/.claude/ticket-flow.md" <<'EOF'
---
base_branch: develop
protected_branches: [main, develop, "release/*"]
---
EOF

FAILURES=0

run() {
  jq -n --arg c "$1" --arg d "$WORK" '{tool_input: {command: $c}, cwd: $d}' | bash "$GUARD" 2>/dev/null
}

expect() {
  local want="$1" cmd="$2" got=0
  run "$cmd" || got=$?
  if [[ "$want" == allow && "$got" -eq 0 ]] || [[ "$want" == deny && "$got" -eq 2 ]]; then
    echo "ok    $want: $cmd"
  else
    echo "FAIL  $want: $cmd (exit $got)"
    FAILURES=$((FAILURES + 1))
  fi
}

# On develop, a protected branch.
expect deny "git push"
expect deny "git push origin HEAD"
expect deny "git push origin develop"
expect deny "git push origin HEAD:main"
expect deny "git push origin feat/x:refs/heads/main"
expect deny "git push origin release/2.1"
expect deny "cd sub && git push -u origin main"
expect deny "git push --force origin feat/SHOP-1-x"
expect deny "git push -fu origin feat/SHOP-1-x"
expect deny "git push origin +feat/SHOP-1-x"
expect deny "git push --force-with-lease origin feat/SHOP-1-x"
expect deny "git push --all origin"
expect deny "glab mr merge 12"
expect deny "gh pr merge 12 --squash"
expect deny "glab mr create --auto-merge --draft"
expect deny "git reset --hard origin/develop"
expect deny "git clean -fd"
expect allow "git push -u origin feat/SHOP-1-cart"
expect allow "git push origin HEAD:feat/SHOP-1-cart"
expect allow "git push origin ticket-flow-assets"
expect allow "git status && git log --oneline -3"
expect allow "echo push"
expect allow "git push origin staging"   # not in this profile's list, so the profile was read

# On a feature branch, a bare push is fine.
git -C "$WORK" switch -q -c feat/SHOP-1-cart
expect allow "git push"
expect allow "git push origin HEAD"
expect deny "git push origin HEAD:develop"

# Without a profile, the default list applies.
rm "$WORK/.claude/ticket-flow.md"
expect deny "git push origin master"
expect deny "git push origin staging"
expect allow "git push origin feat/SHOP-1-cart"

echo
if [[ "$FAILURES" -gt 0 ]]; then
  echo "$FAILURES check(s) failed" >&2
  exit 1
fi
echo "all checks passed"
