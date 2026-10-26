#!/usr/bin/env bash
# Tests the watcher without GitHub: the comment filter against a recorded
# answer, then the script itself against a stubbed `gh` that serves that
# answer. Needs jq only. Takes about 15 seconds, the outage and quiet-window
# cases wait for real polls.

set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")"
HERE="$(pwd)"
WATCH="$HERE/watch-pr-reviews.sh"
FIXTURE="$HERE/fixtures/graphql-response.json"

WORK="$(mktemp -d "${TMPDIR:-/tmp}/pr-review-watch-test.XXXXXX")"
trap 'kill $(jobs -p) 2>/dev/null || true; rm -rf "$WORK"' EXIT

FAILURES=0

check() {
  local name="$1" expected="$2" actual="$3"
  if [[ "$expected" == "$actual" ]]; then
    echo "ok    $name"
  else
    echo "FAIL  $name"
    diff -u <(printf '%s\n' "$expected") <(printf '%s\n' "$actual") | sed 's/^/      /' || true
    FAILURES=$((FAILURES + 1))
  fi
}

# --- the filter ---------------------------------------------------------

filter() {
  jq -r --argjson ignore "$1" --argjson only "$2" -f review-comments.jq "$FIXTURE"
}

check "filter keeps unresolved threads and non-empty reviews, drops own, resolved and ignored" \
  "$(cat fixtures/expected.tsv)" "$(filter '["review-bot"]' '[]')"

check "filter keeps only the allowed reviewers when ONLY_AUTHORS is set" \
  "$(printf 'C_bob\nR_summary')" "$(filter '[]' '["bob"]' | cut -f1)"

# --- the script, against a stubbed gh -----------------------------------

mkdir -p "$WORK/bin"
cat >"$WORK/bin/gh" <<'STUB'
#!/usr/bin/env bash
case "$1" in
  auth) exit 0 ;;
  repo) echo "acme/shop" ;;
  api)
    if [[ -f "$STUB_DIR/down" ]]; then
      echo "error connecting to api.github.com" >&2
      exit 1
    fi
    cat "$STUB_DIR/response.json"
    ;;
esac
STUB
chmod +x "$WORK/bin/gh"

export PATH="$WORK/bin:$PATH"
export STUB_DIR="$WORK"
export POLL_SECONDS=1 QUIET_SECONDS=1 MAX_BACKOFF_SECONDS=1

reset_state() {
  export PR_REVIEW_WATCH_STATE_DIR="$WORK/state-$1"
  cp "$FIXTURE" "$WORK/response.json"
  rm -f "$WORK/down"
}

# Runs the watcher for a few seconds. `timeout` is not on a stock macOS.
run_for() {
  local seconds="$1" output="$2"
  "$WATCH" >"$output" 2>/dev/null &
  local pid=$!
  sleep "$seconds"
  kill "$pid" 2>/dev/null || true
  wait "$pid" 2>/dev/null || true
}

events() {
  sed -E 's/ (thread|at|by|url|branch|kind)=[^ ]*//g'
}

reset_state once
check "--once prints every new comment then BATCH_END" \
  "$(printf '%s\n' \
    'NEW_REVIEW_COMMENT repo=acme/shop pr=12' \
    'NEW_REVIEW_COMMENT repo=acme/shop pr=12' \
    'NEW_REVIEW_COMMENT repo=acme/shop pr=12' \
    'NEW_REVIEW_COMMENT repo=acme/shop pr=12' \
    'BATCH_END count=4')" \
  "$(IGNORE_AUTHORS= "$WATCH" --once | events)"

check "--once prints nothing the second time" "" "$("$WATCH" --once)"

reset_state mark
"$WATCH" --mark-seen 2>/dev/null
check "--mark-seen skips the backlog" "" "$("$WATCH" --once)"

reset_state lock
"$WATCH" >/dev/null &
sleep 1
check "a second watcher on the same repo is refused" \
  "watch-pr-reviews: acme/shop is already watched by process $!" \
  "$("$WATCH" --once 2>&1 || true)"
kill $! 2>/dev/null || true
wait $! 2>/dev/null || true

reset_state stale-lock
mkdir -p "$PR_REVIEW_WATCH_STATE_DIR/acme__shop.lock"
echo 999999 >"$PR_REVIEW_WATCH_STATE_DIR/acme__shop.lock/pid"
check "a lock left by a dead process is taken over" \
  "BATCH_END count=4" "$("$WATCH" --once | tail -1)"

reset_state quiet
QUIET_SECONDS=60 run_for 4 "$WORK/quiet.out"
check "comments are held during the quiet window" "" "$(cat "$WORK/quiet.out")"
check "held comments are not recorded as seen" \
  "BATCH_END count=4" "$("$WATCH" --once | tail -1)"

reset_state outage
touch "$WORK/down"
(sleep 4; rm -f "$WORK/down") &
run_for 8 "$WORK/outage.out"
check "an outage is reported once, then the recovery, then the batch" \
  "$(printf '%s\n' \
    'WATCH_ERROR repo=acme/shop error connecting to api.github.com' \
    'WATCH_RECOVERED repo=acme/shop' \
    'BATCH_END count=4')" \
  "$(grep -v '^NEW_REVIEW_COMMENT' "$WORK/outage.out")"

reset_state warning
jq '.data.search.nodes[0].reviewThreads.nodes[0].comments.pageInfo.hasNextPage = true' \
  "$FIXTURE" >"$WORK/truncated.json"
cp "$WORK/truncated.json" "$WORK/response.json"
first="$("$WATCH" --once | grep '^WATCH_WARNING' || true)"
second="$("$WATCH" --once | grep '^WATCH_WARNING' || true)"
cp "$FIXTURE" "$WORK/response.json"
"$WATCH" --once >/dev/null
cp "$WORK/truncated.json" "$WORK/response.json"
again="$("$WATCH" --once | grep '^WATCH_WARNING' || true)"
expected_warning="WATCH_WARNING repo=acme/shop PR 12 has more reviews, threads or comments than one page, the rest are not watched"
check "a truncated thread warns" "$expected_warning" "$first"
check "the warning is not repeated while it holds" "" "$second"
check "the warning comes back after it cleared" "$expected_warning" "$again"

echo
if [[ "$FAILURES" -gt 0 ]]; then
  echo "$FAILURES check(s) failed" >&2
  exit 1
fi
echo "all checks passed"
