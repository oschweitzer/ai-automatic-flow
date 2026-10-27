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

# ---------------------------------------------------------------- GitHub issues

printf -- '---\nforge: github\ntracker: github\n---\n' >"$WORK/.claude/ticket-flow.md"
F="$HERE/fixtures"
cat >"$WORK/bin/gh" <<STUB
#!/usr/bin/env bash
echo "\$*" >>"$WORK/calls"
case "\$*" in
  "repo view --json nameWithOwner -q .nameWithOwner") echo acme/shop ;;
  "repo view --json visibility -q .visibility") echo "\${VISIBILITY:-PUBLIC}" ;;
  "api repos/acme/shop/issues/142") cat "$F/gh-issue-142.json" ;;
  "api repos/acme/shop/issues/143") cat "$F/gh-issue-143-pr.json" ;;
  "api repos/acme/shop/issues/150") cat "$F/gh-issue-150.json" ;;
  "api --paginate repos/acme/shop/issues/142/comments") cat "$F/gh-issue-142-comments.json" ;;
  "api --paginate repos/acme/shop/issues/150/comments") echo '[]' ;;
  "api --paginate repos/acme/shop/issues/142/sub_issues") cat "$F/gh-issue-142-sub-issues.json" ;;
  "api --paginate repos/acme/shop/issues/150/sub_issues") echo "gh: Not Found (HTTP 404)" >&2; exit 1 ;;
  "api repos/acme/shop/issues/160") jq '.number = 160' "$F/gh-issue-142.json" ;;
  "api --paginate repos/acme/shop/issues/160/comments") cat "$WORK/long-comments.json" ;;
  "api --paginate repos/acme/shop/issues/160/sub_issues") echo '[]' ;;
  "api repos/acme/shop/issues/160/parent") echo '{"message": "Not Found"}'; exit 1 ;;
  "api repos/acme/shop/issues/142/parent") echo '{"message": "Not Found", "status": "404"}'; echo "gh: Not Found (HTTP 404)" >&2; exit 1 ;;
  "api repos/acme/shop/issues/150/parent") cat "$F/gh-issue-150-parent.json" ;;
  "api repos/acme/shop/issues -f title=Part 1 -f body="*) echo '{"id": 987654, "number": 152}' ;;
  "api repos/acme/shop/issues/142/sub_issues -F sub_issue_id="*)
    [[ -z "\${ATTACH_FAIL:-}" ]] || { echo "gh: Validation Failed (HTTP 422)" >&2; exit 1; }
    echo '{"number": 152}' ;;
  *) echo "stub: unexpected gh \$*" >&2; exit 1 ;;
esac
STUB
chmod +x "$WORK/bin/gh"
: >"$WORK/calls"

check "visibility maps public" "public" "$(forge visibility)"
check "visibility maps private" "private" "$(VISIBILITY=PRIVATE forge visibility)"
check "visibility maps internal" "internal" "$(VISIBILITY=INTERNAL forge visibility)"
check "issue-read keeps the author and collaborators" \
  "client-anna dev-bob" "$(forge issue-read 142 | jq -r '[.comments[].author] | join(" ")')"
check "issue-read counts dropped comments" "1" "$(forge issue-read 142 | jq -r .dropped_comments)"
check "issue-read reports type, labels and state" \
  "Bug bug,checkout open" "$(forge issue-read 142 | jq -r '"\(.type) \(.labels | join(",")) \(.state)"')"
check "issue-read reports the URL" \
  "https://github.com/acme/shop/issues/142" "$(forge issue-read 142 | jq -r .url)"
check "issue-read lists sub-issues" "150 151" "$(forge issue-read 142 | jq -r '.sub_issues | join(" ")')"
check "an issue without parent has a null parent" "null" "$(forge issue-read 142 | jq -r .parent)"
check "an issue with a parent" "142 null" "$(forge issue-read 150 | jq -r '"\(.parent) \(.type)"')"
check "issue-read survives a repository without sub-issues" "150 0" "$(forge issue-read 150 | jq -r '"\(.number) \(.sub_issues | length)"')"
# 200 comments of about 1 KB each pass 128 KiB, the limit of one command-line argument on Linux.
jq -n --arg body "$(printf 'x%.0s' $(seq 1 1000))" \
  '[range(200) | {user: {login: "dev-bob"}, author_association: "COLLABORATOR", body: $body, created_at: "2026-10-20T08:00:00Z"}]' \
  >"$WORK/long-comments.json"
check "issue-read handles a long discussion" "200" "$(forge issue-read 160 | jq '.comments | length')"
check "issue-read refuses a pull request" "forge.sh: #143 is a pull request" "$(forge issue-read 143 2>&1 >/dev/null || true)"

echo "[ticket-flow] Part 1 of #142" >"$WORK/part.md"
: >"$WORK/calls"
check "create-sub prints the new number" "152" "$(forge issue-create-sub 142 "Part 1" "$WORK/part.md")"
check "create-sub attaches with the database id" "yes" \
  "$(grep -q 'sub_issue_id=987654' "$WORK/calls" && ! grep -q 'sub_issue_id=152' "$WORK/calls" && echo yes || echo no)"
check "a failed attach still prints the number" "152" \
  "$(ATTACH_FAIL=1 forge issue-create-sub 142 "Part 1" "$WORK/part.md" 2>/dev/null || true)"
check "a failed attach exits non-zero" "1" \
  "$(ATTACH_FAIL=1 forge issue-create-sub 142 "Part 1" "$WORK/part.md" >/dev/null 2>&1; echo $?)"

echo
if [[ "$FAILURES" -gt 0 ]]; then echo "$FAILURES check(s) failed" >&2; exit 1; fi
echo "all checks passed"
