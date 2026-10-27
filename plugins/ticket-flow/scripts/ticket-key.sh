#!/usr/bin/env bash
# Prints the ticket key every ticket-flow skill uses, from what the user typed.
# Jira keys are kept as typed. GitHub issues become GH-<n>.
# Run it from inside the repository: ticket-key.sh <ticket>

set -euo pipefail

# shellcheck source=lib.sh
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

fail() { echo "ticket-key.sh: $*" >&2; exit 1; }

[[ $# -eq 1 ]] || fail "usage: ticket-key.sh <ticket>"
profile="$(profile_path "$PWD")" || fail "no .claude/ticket-flow.md, run /ticket-flow:profile first"
tracker="$(profile_tracker "$profile")"

case "$tracker" in
  jira)
    [[ -n "$1" ]] || fail "empty ticket key"
    echo "$1"
    ;;
  github)
    [[ "$(profile_get "$profile" forge)" == github ]] ||
      fail "tracker github is not supported with forge $(profile_get "$profile" forge)"
    shopt -s nocasematch
    [[ "$1" =~ ^(#|GH-)?0*([1-9][0-9]*)$ ]] || fail "\"$1\" is not a GitHub issue number"
    echo "GH-${BASH_REMATCH[2]}"
    ;;
  *) fail "unknown tracker \"$tracker\" in the profile" ;;
esac
