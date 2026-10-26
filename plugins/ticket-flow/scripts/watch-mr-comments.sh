#!/usr/bin/env bash
# Watches one MR for new comments and prints them in batches. Built for Claude
# Code's Monitor tool: it prints nothing while nothing changes. Adapted from
# the watch-pr-reviews skill, narrowed to the MR of one ticket-flow run and
# made forge-neutral through forge.sh.
#
# Usage: watch-mr-comments.sh <mr> <run-dir> [--once]
#
# Output, one event per line:
#   NEW_COMMENT note=<id> thread=<id> at=<path:line|general> by=<login>
#   BATCH_END count=<n>
#   WATCH_ERROR <message>        first failure of a streak
#   WATCH_RECOVERED

set -euo pipefail

SCRIPTS="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
POLL_SECONDS="${POLL_SECONDS:-60}"
QUIET_SECONDS="${QUIET_SECONDS:-120}"

mr="${1:?usage: watch-mr-comments.sh <mr> <run-dir> [--once]}"
run_dir="${2:?usage: watch-mr-comments.sh <mr> <run-dir> [--once]}"
once="${3:-}"

seen_file="$run_dir/seen-notes"
touch "$seen_file"
me="$(bash "$SCRIPTS/forge.sh" whoami)"
pending=()
failing=0
last_new=0

poll() {
  local threads seen rows id thread at by
  if ! threads="$(bash "$SCRIPTS/forge.sh" list-threads "$mr" 2>&1)"; then
    ((failing)) || echo "WATCH_ERROR $(tail -1 <<<"$threads")"
    failing=1
    return 0
  fi
  ((failing)) && echo "WATCH_RECOVERED"
  failing=0
  seen="$(jq -Rn '[inputs]' "$seen_file")"
  rows="$(jq -r --arg me "$me" --argjson seen "$seen" -f "$SCRIPTS/comments.jq" <<<"$threads")"
  while IFS=$'\t' read -r id thread at by; do
    [[ -z "$id" ]] && continue
    [[ " ${pending[*]-} " == *" $id|"* ]] && continue
    pending+=("$id|NEW_COMMENT note=$id thread=$thread at=$at by=$by")
    last_new="$(date +%s)"
  done <<<"$rows"
}

# An id is recorded as seen only once printed, so a watch stopped mid-batch loses nothing.
flush() {
  [[ ${#pending[@]} -gt 0 ]] || return 0
  local entry
  for entry in "${pending[@]}"; do
    echo "${entry#*|}"
    echo "${entry%%|*}" >>"$seen_file"
  done
  echo "BATCH_END count=${#pending[@]}"
  pending=()
}

if [[ "$once" == --once ]]; then
  poll
  flush
  exit 0
fi

while true; do
  poll
  if (($(date +%s) - last_new >= QUIET_SECONDS)); then
    flush
  fi
  sleep "$POLL_SECONDS"
done
