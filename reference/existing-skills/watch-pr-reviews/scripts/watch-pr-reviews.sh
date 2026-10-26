#!/usr/bin/env bash
# Watches the review comments on your open GitHub PRs and prints one line per
# new comment. Built to run under Claude Code's Monitor tool: it prints nothing
# while nothing changes, so no token is spent until a reviewer posts.
# See README.md in this folder.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FILTER="$SCRIPT_DIR/review-comments.jq"

POLL_SECONDS="${POLL_SECONDS:-60}"
QUIET_SECONDS="${QUIET_SECONDS:-180}"
MAX_BACKOFF_SECONDS="${MAX_BACKOFF_SECONDS:-900}"
STATE_DIR="${PR_REVIEW_WATCH_STATE_DIR:-${XDG_STATE_HOME:-$HOME/.local/state}/pr-review-watch}"
IGNORE_AUTHORS="${IGNORE_AUTHORS:-}"
ONLY_AUTHORS="${ONLY_AUTHORS:-}"

# GraphQL page sizes. GitHub charges a query about (PRs x threads) / 100
# points, so these cost 13 points a poll, measured with rateLimit(dryRun).
# Anything beyond them is reported as WATCH_WARNING.
MAX_PRS=25
MAX_REVIEWS=50
MAX_THREADS=50
MAX_COMMENTS=20

# Below this share of the hourly GraphQL points left, warn once.
LOW_RATE_LIMIT_PERCENT=10

QUERY='
query($q: String!) {
  viewer { login }
  rateLimit { limit remaining resetAt }
  search(type: ISSUE, query: $q, first: '"$MAX_PRS"') {
    issueCount
    nodes {
      ... on PullRequest {
        number
        headRefName
        reviews(first: '"$MAX_REVIEWS"') {
          pageInfo { hasNextPage }
          nodes { id author { login } state body url }
        }
        reviewThreads(first: '"$MAX_THREADS"') {
          pageInfo { hasNextPage }
          nodes {
            id
            isResolved
            path
            line
            comments(first: '"$MAX_COMMENTS"') {
              pageInfo { hasNextPage }
              nodes { id author { login } body url }
            }
          }
        }
      }
    }
  }
}'

usage() {
  cat <<'EOF'
Usage: watch-pr-reviews.sh [--once | --mark-seen] [owner/repo ...]

Watches the review comments on your open PRs and prints one line per new one.
With no repo, watches the repo of the current directory.

  --once        Poll once, print what is new straight away, record it, exit.
  --mark-seen   Record every current comment as seen without printing, exit.
                Use it to skip the backlog before the first watch.
  -h, --help    Show this help.

Environment:
  POLL_SECONDS               Seconds between polls (default 60).
  QUIET_SECONDS              New comments are held until none has arrived for
                             this long, then printed together (default 180).
  MAX_BACKOFF_SECONDS        Longest wait between polls while GitHub fails
                             (default 900).
  IGNORE_AUTHORS             Comma-separated logins to ignore, like bots.
  ONLY_AUTHORS               Comma-separated logins to act on. When set,
                             comments from anyone else are ignored.
  PR_REVIEW_WATCH_STATE_DIR  Where seen ids are kept
                             (default $XDG_STATE_HOME/pr-review-watch).

Output, one event per line:
  NEW_REVIEW_COMMENT repo= pr= branch= kind= thread= at= by= url=
  BATCH_END count=<n>          after each group of NEW_REVIEW_COMMENT lines
  WATCH_ERROR repo= <message>  first failure of a streak, then backs off
  WATCH_RECOVERED repo=        GitHub answers again
  WATCH_WARNING repo= <message> results cut at a page size, or GraphQL points low
EOF
}

main() {
  local repos=()

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --once) MODE="once" ;;
      --mark-seen) MODE="mark-seen" ;;
      -h | --help) usage; exit 0 ;;
      -*) fail "unknown option $1, see --help" ;;
      *) repos+=("$1") ;;
    esac
    shift
  done

  check_prerequisites

  if [[ ${#repos[@]} -eq 0 ]]; then
    repos=("$(gh repo view --json nameWithOwner -q .nameWithOwner 2>/dev/null)") ||
      fail "not in a GitHub repo, pass owner/repo"
  fi

  local repo
  for repo in "${repos[@]}"; do
    [[ "$repo" =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ ]] || fail "\"$repo\" is not owner/repo"
  done

  exec 3>&1
  mkdir -p "$STATE_DIR"
  PENDING_FILE="$(mktemp "${TMPDIR:-/tmp}/pr-review-watch.XXXXXX")"
  trap cleanup EXIT
  trap 'exit 130' INT TERM

  for repo in "${repos[@]}"; do
    acquire_lock "$repo"
  done

  case "$MODE" in
    mark-seen) mark_seen "${repos[@]}" ;;
    once)
      local status=0
      poll "${repos[@]}" || status=1
      flush
      exit "$status"
      ;;
    watch) watch "${repos[@]}" ;;
  esac
}

watch() {
  local last_new_at=0
  local wait="$POLL_SECONDS"

  while true; do
    if poll "$@"; then
      wait="$POLL_SECONDS"
    else
      wait=$((wait * 2))
      ((wait > MAX_BACKOFF_SECONDS)) && wait="$MAX_BACKOFF_SECONDS"
    fi

    if [[ "$NEW_IN_LAST_POLL" -gt 0 ]]; then
      last_new_at="$(date +%s)"
    fi

    if [[ -s "$PENDING_FILE" ]] && (($(date +%s) - last_new_at >= QUIET_SECONDS)); then
      flush
    fi

    sleep "$wait"
  done
}

# Adds the comments not seen yet to the pending file. Returns non-zero when a
# repo could not be read. Sets NEW_IN_LAST_POLL.
poll() {
  local repo seen_file rows id pr branch kind thread location author url
  local status=0
  NEW_IN_LAST_POLL=0

  for repo in "$@"; do
    seen_file="$(state_file "$repo" seen)"
    touch "$seen_file"

    if ! rows="$(fetch_rows "$repo")"; then
      status=1
      continue
    fi

    while IFS=$'\t' read -r id pr branch kind thread location author url; do
      [[ -z "$id" ]] && continue
      grep -qxF "$id" "$seen_file" && continue
      grep -qF "	$id	" "$PENDING_FILE" && continue

      printf '%s\t%s\t%s\n' "$seen_file" "$id" \
        "NEW_REVIEW_COMMENT repo=$repo pr=$pr branch=$branch kind=$kind thread=$thread at=$location by=$author url=$url" \
        >>"$PENDING_FILE"
      NEW_IN_LAST_POLL=$((NEW_IN_LAST_POLL + 1))
    done <<<"$rows"
  done

  return "$status"
}

# Prints the pending comments as one batch, then records them as seen. An id
# is only recorded once printed, so a watch stopped mid-batch loses nothing.
flush() {
  [[ -s "$PENDING_FILE" ]] || return 0

  local seen_file id line count=0
  while IFS=$'\t' read -r seen_file id line; do
    echo "$line"
    echo "$id" >>"$seen_file"
    count=$((count + 1))
  done <"$PENDING_FILE"

  echo "BATCH_END count=$count"
  : >"$PENDING_FILE"
}

mark_seen() {
  local repo seen_file rows count
  for repo in "$@"; do
    seen_file="$(state_file "$repo" seen)"
    touch "$seen_file"
    rows="$(fetch_rows "$repo")" || exit 1
    count=0
    while IFS=$'\t' read -r id _; do
      [[ -z "$id" ]] && continue
      grep -qxF "$id" "$seen_file" || { echo "$id" >>"$seen_file"; count=$((count + 1)); }
    done <<<"$rows"
    echo "Marked $count comment(s) as seen in $repo." >&2
  done
}

# Prints the TSV rows of one repo. On failure prints WATCH_ERROR once per
# failure streak and returns non-zero. Runs in a command substitution, so its
# events go to fd 3, which main points at the real stdout.
fetch_rows() {
  local repo="$1"
  local error_file stderr_file response rows
  error_file="$(state_file "$repo" error)"
  stderr_file="$(state_file "$repo" stderr)"

  if ! response="$(gh api graphql -f query="$QUERY" \
    -f q="repo:$repo is:pr is:open author:@me sort:updated-desc" 2>"$stderr_file")"; then
    report_error "$repo" "$error_file" "$(tail -1 "$stderr_file")"
    return 1
  fi

  if ! rows="$(jq -r --argjson ignore "$(logins_json "$IGNORE_AUTHORS")" \
    --argjson only "$(logins_json "$ONLY_AUTHORS")" -f "$FILTER" <<<"$response" 2>&1)"; then
    report_error "$repo" "$error_file" "unexpected answer from GitHub: $(head -1 <<<"$rows")"
    return 1
  fi

  if [[ -f "$error_file" ]]; then
    rm -f "$error_file"
    echo "WATCH_RECOVERED repo=$repo" >&3
  fi

  report_warnings "$repo" "$response"
  printf '%s\n' "$rows"
}

report_error() {
  local repo="$1" error_file="$2" message="$3"
  if [[ "$MODE" != "watch" || ! -f "$error_file" ]]; then
    echo "WATCH_ERROR repo=$repo $message" >&3
    touch "$error_file"
  fi
}

# Prints each warning once while it holds. The file keeps only the warnings of
# the last poll, so one that clears and comes back is printed again.
report_warnings() {
  local repo="$1" response="$2" warnings
  warnings="$(jq -r --argjson max "$MAX_PRS" --argjson low "$LOW_RATE_LIMIT_PERCENT" '
    [ (if .data.search.issueCount > $max then "more than \($max) open PRs, only the \($max) most recently updated are watched" else empty end),
      (.data.search.nodes[]
        | select(.number != null)
        | select(.reviews.pageInfo.hasNextPage or .reviewThreads.pageInfo.hasNextPage
            or any(.reviewThreads.nodes[]; .comments.pageInfo.hasNextPage))
        | "PR \(.number) has more reviews, threads or comments than one page, the rest are not watched"),
      (.data.rateLimit // empty
        | select(.remaining * 100 < .limit * $low)
        | "GitHub GraphQL points low, \(.remaining) of \(.limit) left until \(.resetAt)") ]
    | .[]' <<<"$response")"

  local warning_file message
  warning_file="$(state_file "$repo" warned)"
  touch "$warning_file"
  while IFS= read -r message; do
    [[ -z "$message" ]] && continue
    grep -qxF "$message" "$warning_file" || echo "WATCH_WARNING repo=$repo $message" >&3
  done <<<"$warnings"
  printf '%s\n' "$warnings" | grep -v '^$' >"$warning_file" || true
}

logins_json() {
  jq -cn --arg list "$1" '$list | split(",") | map(gsub("^\\s+|\\s+$"; "")) | map(select(length > 0))'
}

# One watcher per repo: two sessions reacting to the same comment would fix it twice.
acquire_lock() {
  local repo="$1" lock_dir
  lock_dir="$(state_file "$repo" lock)"

  if ! mkdir "$lock_dir" 2>/dev/null; then
    local holder
    holder="$(cat "$lock_dir/pid" 2>/dev/null || true)"
    if [[ -n "$holder" ]] && kill -0 "$holder" 2>/dev/null; then
      fail "$repo is already watched by process $holder"
    fi
    rm -rf "$lock_dir"
    mkdir "$lock_dir" || fail "cannot lock $lock_dir"
  fi

  echo $$ >"$lock_dir/pid"
  LOCKS+=("$lock_dir")
}

cleanup() {
  rm -f "${PENDING_FILE:-}"
  local lock_dir
  for lock_dir in ${LOCKS[@]+"${LOCKS[@]}"}; do
    rm -rf "$lock_dir"
  done
}

state_file() {
  echo "$STATE_DIR/${1//\//__}.$2"
}

check_prerequisites() {
  command -v gh >/dev/null || fail "gh is not installed, see https://cli.github.com"
  command -v jq >/dev/null || fail "jq is not installed"
  [[ -f "$FILTER" ]] || fail "missing $FILTER"
  gh auth status >/dev/null 2>&1 || fail "gh is not logged in, run gh auth login"
}

fail() {
  echo "watch-pr-reviews: $*" >&2
  exit 1
}

MODE="watch"
LOCKS=()
NEW_IN_LAST_POLL=0
PENDING_FILE=""

main "$@"
