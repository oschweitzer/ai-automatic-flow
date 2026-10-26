#!/usr/bin/env bash
# Runs the profile's local checks in a directory (default: the current one).
# Prints one line per check, and only the tail of the first failure, so an
# agent never reads a whole log.
#
# Usage: checks.sh [dir] [check ...]   checks default to: lint typecheck test

set -uo pipefail

# shellcheck source=lib.sh
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

dir="${1:-$PWD}"
shift || true
checks=("$@")
[[ ${#checks[@]} -gt 0 ]] || checks=(lint typecheck test)
tail_lines="${TAIL_LINES:-60}"

profile="$(profile_path "$dir")" || {
  echo "checks: no .claude/ticket-flow.md profile, run /ticket-flow:profile" >&2
  exit 1
}

out="$(mktemp "${TMPDIR:-/tmp}/ticket-flow-checks.XXXXXX")"
trap 'rm -f "$out"' EXIT

for check in "${checks[@]}"; do
  command="$(profile_get "$profile" "commands.$check")"
  if [[ -z "$command" ]]; then
    echo "skip  $check (no command in the profile)"
    continue
  fi
  if (cd "$dir" && bash -c "$command") >"$out" 2>&1; then
    echo "ok    $check"
  else
    echo "FAIL  $check: $command"
    echo "----- last $tail_lines lines"
    tail -n "$tail_lines" "$out"
    exit 1
  fi
done
