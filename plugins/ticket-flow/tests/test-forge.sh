#!/usr/bin/env bash
# Tests the GitLab side of forge.sh and the comment watcher against a stubbed
# glab that serves recorded answers. Needs git and jq.

set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCRIPTS="$HERE/../scripts"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/forge-test.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT

git -C "$WORK" init -q -b develop
mkdir -p "$WORK/.claude" "$WORK/bin" "$WORK/run"
printf -- '---\nforge: gitlab\n---\n' >"$WORK/.claude/ticket-flow.md"

cat >"$WORK/bin/glab" <<STUB
#!/usr/bin/env bash
echo "\$*" >>"$WORK/calls"
case "\$*" in
  "api user") echo '{"username": "dev"}' ;;
  "api --method GET projects/:id/merge_requests -f state=opened -f source_branch=feat/SHOP-1-x")
    echo '[{"iid": 12, "web_url": "https://gitlab.example.com/shop/-/merge_requests/12"}]' ;;
  "api --method GET projects/:id/merge_requests -f state=opened -f source_branch="*) echo '[]' ;;
  "api projects/:id/merge_requests/12") echo '{"head_pipeline": {"id": 5, "status": "'"\${PIPELINE_STATUS:-failed}"'"}}' ;;
  "api projects/:id/merge_requests/13") echo '{"head_pipeline": null}' ;;
  "api --paginate projects/:id/merge_requests/12/discussions") cat "$HERE/fixtures/gitlab-discussions.json" ;;
  *) echo "stub: unexpected glab \$*" >&2; exit 1 ;;
esac
STUB
chmod +x "$WORK/bin/glab"
export PATH="$WORK/bin:$PATH"
cd "$WORK"

FAILURES=0
check() {
  if [[ "$2" == "$3" ]]; then echo "ok    $1"; else
    echo "FAIL  $1"; diff <(printf '%s\n' "$2") <(printf '%s\n' "$3") | sed 's/^/      /' || true
    FAILURES=$((FAILURES + 1)); fi
}
forge() { bash "$SCRIPTS/forge.sh" "$@"; }

check "kind comes from the profile" "gitlab" "$(forge kind)"
check "find-mr prints number and URL" \
  "$(printf '12\thttps://gitlab.example.com/shop/-/merge_requests/12')" "$(forge find-mr feat/SHOP-1-x)"
check "find-mr prints nothing without an MR" "" "$(forge find-mr feat/SHOP-2-y)"
check "ci-status maps failed" "failed" "$(forge ci-status 12)"
check "ci-status maps canceled to failed" "failed" "$(PIPELINE_STATUS=canceled forge ci-status 12)"
check "ci-status maps pending to running" "running" "$(PIPELINE_STATUS=pending forge ci-status 12)"
check "ci-status maps manual to blocked" "blocked" "$(PIPELINE_STATUS=manual forge ci-status 12)"
check "ci-status without pipeline" "none" "$(forge ci-status 13)"
check "list-threads drops system notes and keeps the rest" \
  "d-open d-done d-general" "$(forge list-threads 12 | jq -r '[.[].thread] | join(" ")')"
check "list-threads keeps the location" \
  "src/cart/discount.ts:18" "$(forge list-threads 12 | jq -r '.[0] | "\(.path):\(.line)"')"
check "unknown command fails" "1" "$(forge frobnicate >/dev/null 2>&1; echo $?)"

check "the watcher prints new comments from others in unresolved threads" \
  "$(printf '%s\n' \
    'NEW_COMMENT note=10 thread=d-open at=src/cart/discount.ts:18 by=reviewer' \
    'NEW_COMMENT note=30 thread=d-general at=general by=po' \
    'BATCH_END count=2')" \
  "$(bash "$SCRIPTS/watch-mr-comments.sh" 12 "$WORK/run" --once)"
check "the watcher prints nothing the second time" "" "$(bash "$SCRIPTS/watch-mr-comments.sh" 12 "$WORK/run" --once)"

echo
if [[ "$FAILURES" -gt 0 ]]; then echo "$FAILURES check(s) failed" >&2; exit 1; fi
echo "all checks passed"
