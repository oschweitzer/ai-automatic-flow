#!/usr/bin/env bash
# Records the Claude Code sessions a run used, in <run-dir>/sessions, and sums
# their tokens and cost from the session transcripts, subagents included.
#
# Usage: usage.sh add <run-dir> <session-id>   records a session once
#        usage.sh new <run-dir>                records and prints a fresh session ID
#        usage.sh total <run-dir>              prints model, input, output,
#                                              cache_write, cache_read, cost_usd,
#                                              tab separated, one row per model,
#                                              then a total row
#
# A session that has ended carries Claude Code's own cost-state line, which
# also counts its background calls, so it is used as is. A session still
# running has none yet: its messages are priced with the table below, at
# Anthropic API rates in USD (an estimate, not an invoice).
#
# total exits 2 when the sums are partial: a recorded session has no transcript
# on this machine, or a model has no price (its cost shows as ?).
# shortcut: the transcript format is not a documented API, move to OpenTelemetry if it changes.

set -euo pipefail
shopt -s nullglob

fail() { echo "usage.sh: $*" >&2; exit 1; }

cmd="${1:-}"
run_dir="${2:-}"
[[ -n "$cmd" && -n "$run_dir" ]] || fail "usage: usage.sh add <run-dir> <session-id> | new <run-dir> | total <run-dir>"
sessions="$run_dir/sessions"

# USD per million tokens: input, 5m cache write, 1h cache write, cache read, output.
# From https://platform.claude.com/docs/en/about-claude/pricing on 2026-10-27.
PRICES='{
  "claude-fable-5-1": [10, 12.5, 20, 0.25, 50],
  "claude-fable-5": [10, 12.5, 20, 1, 50],
  "claude-opus-5-5": [4, 5, 8, 0.2, 20],
  "claude-opus-5": [5, 6.25, 10, 0.5, 25],
  "claude-opus-4-8": [5, 6.25, 10, 0.5, 25],
  "claude-opus-4-7": [5, 6.25, 10, 0.5, 25],
  "claude-opus-4-6": [5, 6.25, 10, 0.5, 25],
  "claude-sonnet-5-5": [2, 2.5, 4, 0.1, 10],
  "claude-sonnet-5": [2, 2.5, 4, 0.2, 10],
  "claude-sonnet-4-6": [3, 3.75, 6, 0.3, 15],
  "claude-haiku-5-5": [0.1, 0.125, 0.2, 0.01, 0.5],
  "claude-haiku-4-5": [1, 1.25, 2, 0.1, 5]
}'

add() {
  # The ID ends up in a path, so only UUID characters pass.
  [[ "$1" =~ ^[A-Za-z0-9-]+$ ]] || fail "not a session ID: $1"
  [[ -d "$run_dir" ]] || fail "no run folder $run_dir"
  grep -qxF "$1" "$sessions" 2>/dev/null || echo "$1" >>"$sessions"
}

# One JSON record per model for a session, from its transcripts.
# fromjson? skips a line the running session is still writing.
session_rows() {
  jq -Rnc --argjson prices "$PRICES" '
    [inputs | fromjson?] as $lines
    | ($lines | map(select(.type == "cost-state")) | last) as $state
    | if $state then
        $state.modelUsage | to_entries[]
        | {model: .key, input: .value.inputTokens, output: .value.outputTokens,
           cache_write: .value.cacheCreationInputTokens, cache_read: .value.cacheReadInputTokens,
           cost: (if .value.costUSD == 0 and .value.inputTokens + .value.outputTokens > 0 then null else .value.costUSD end)}
      else
        # A transcript repeats a message once per content block, hence unique_by(.id).
        $lines | map(select(.type == "assistant" and .message.usage != null
          and .message.model != "<synthetic>") | .message) | unique_by(.id)[]
        | .usage as $u
        | ($u.cache_creation.ephemeral_1h_input_tokens // 0) as $w1h
        | (($u.cache_creation_input_tokens // 0) - $w1h) as $w5m
        | [$u.input_tokens // 0, $w5m, $w1h, $u.cache_read_input_tokens // 0, $u.output_tokens // 0] as $t
        | (.model | sub("\\[.*\\]$"; "")) as $model
        # Haiku 5.5 prompts over 100k tokens cost 5 times more, fast mode twice, US-only inference 1.1 times.
        | ((if $model == "claude-haiku-5-5" and $t[0] + $t[1] + $t[2] + $t[3] > 100000 then 5 else 1 end)
           * (if $u.speed == "fast" then 2 else 1 end)
           * (if $u.inference_geo == "us" then 1.1 else 1 end)) as $mult
        | {model: .model, input: $t[0], output: $t[4], cache_write: ($t[1] + $t[2]), cache_read: $t[3],
           cost: ($prices[$model] as $p | if $p then ([range(5)] | map($t[.] * $p[.]) | add) * $mult / 1e6 else null end)}
      end' "$@"
}

total() {
  [[ -s "$sessions" ]] || fail "no session recorded in $sessions"
  local projects="${CLAUDE_CONFIG_DIR:-$HOME/.claude}/projects" missing=0 id main rows=""
  while read -r id; do
    [[ -n "$id" ]] || continue
    # Sessions started in a worktree live under another project folder.
    main=("$projects"/*/"$id".jsonl)
    if [[ ${#main[@]} -eq 0 ]]; then
      echo "usage.sh: no transcript for session $id" >&2
      missing=1
      continue
    fi
    rows+="$(session_rows "${main[@]}" "$projects"/*/"$id"/subagents/*.jsonl)"$'\n'
  done <"$sessions"
  [[ -n "${rows//$'\n'/}" ]] || fail "no transcript found for any recorded session"

  printf '%s' "$rows" | jq -rs '
    def money: if . == null then "?" else . * 100 | round / 100 end;
    def sums: {input: (map(.input) | add), output: (map(.output) | add),
               cache_write: (map(.cache_write) | add), cache_read: (map(.cache_read) | add),
               cost: (if any(.cost == null) then null else map(.cost) | add end)};
    map(.model |= sub("\\[.*\\]$"; "")) as $r
    | ["model", "input", "output", "cache_write", "cache_read", "cost_usd"],
      ($r | group_by(.model)[] | sums + {model: .[0].model}
        | [.model, .input, .output, .cache_write, .cache_read, (.cost | money)]),
      ($r | sums | ["total", .input, .output, .cache_write, .cache_read,
        ($r | map(.cost // 0) | add | money)])
    | @tsv'
  printf '%s' "$rows" | jq -e -s 'any(.cost == null)' >/dev/null && {
    echo "usage.sh: no price for some models, their cost is left out of the total" >&2
    missing=1
  }
  return $((missing * 2))
}

case "$cmd" in
  add) add "${3:?usage: usage.sh add <run-dir> <session-id>}" ;;
  new)
    id="$({ uuidgen 2>/dev/null || cat /proc/sys/kernel/random/uuid; } | tr 'A-Z' 'a-z')"
    add "$id"
    echo "$id"
    ;;
  total) total ;;
  *) fail "unknown command $cmd" ;;
esac
