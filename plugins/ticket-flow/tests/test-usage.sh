#!/usr/bin/env bash
# Tests usage.sh, which records a run's sessions and sums their tokens and cost
# from fake transcripts. Needs jq.

set -euo pipefail

SCRIPTS="$(cd "$(dirname "${BASH_SOURCE[0]}")/../scripts" && pwd)"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/usage-test.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT

export CLAUDE_CONFIG_DIR="$WORK/config"
RUN="$WORK/run"
P="$CLAUDE_CONFIG_DIR/projects"
mkdir -p "$RUN" "$P/repo/s1/subagents" "$P/worktree"

FAILURES=0
check() {
  if [[ "$2" == "$3" ]]; then echo "ok    $1"; else
    echo "FAIL  $1: expected [$2] got [$3]"; FAILURES=$((FAILURES + 1)); fi
}
usage() { bash "$SCRIPTS/usage.sh" "$@" 2>/dev/null; }
status() { bash "$SCRIPTS/usage.sh" "$@" >/dev/null 2>&1 && echo 0 || echo $?; }
# msg <id> <model> <input> <cache write 5m> <cache write 1h> <cache read> <output> [extra usage fields]
msg() {
  printf '{"type":"assistant","message":{"id":"%s","model":"%s","usage":{"input_tokens":%s,"cache_creation_input_tokens":%s,"cache_creation":{"ephemeral_5m_input_tokens":%s,"ephemeral_1h_input_tokens":%s},"cache_read_input_tokens":%s,"output_tokens":%s%s}}}\n' \
    "$1" "$2" "$3" "$(($4 + $5))" "$4" "$5" "$6" "$7" "${8:-}"
}
row() { usage total "$RUN" | awk -F'\t' -v m="$1" '$1 == m' || true; }

# Session s1, still running: one message written twice (two content blocks), a
# synthetic one, a user line, a half-written last line, and one subagent.
# Opus 5.5: 1M input $4, 1M 5m write $5, 1M 1h write $8, 1M read $0.20, 1M output $20.
{
  msg m1 claude-opus-5-5 1000000 1000000 1000000 1000000 1000000
  msg m1 claude-opus-5-5 1000000 1000000 1000000 1000000 1000000
  msg m0 '<synthetic>' 9 9 9 9 9
  echo '{"type":"user","message":{"content":"hi"}}'
  echo '{"type":"assistant","mess'
} >"$P/repo/s1.jsonl"
# Haiku 5.5 over 100k prompt tokens pays 5 times: 200k input is $0.10, not $0.02.
msg m2 claude-haiku-5-5 200000 0 0 0 0 >"$P/repo/s1/subagents/agent-a.jsonl"
# Session s2 started in the worktree, so it sits in another project folder. It
# has ended: its last cost-state line wins over its messages.
{
  msg m3 claude-opus-5-5 1 1 1 1 1
  echo '{"type":"cost-state","modelUsage":{"claude-opus-5-5[1m]":{"inputTokens":10,"outputTokens":20,"cacheReadInputTokens":30,"cacheCreationInputTokens":40,"costUSD":1}}}'
  echo '{"type":"cost-state","modelUsage":{"claude-opus-5-5[1m]":{"inputTokens":100,"outputTokens":200,"cacheReadInputTokens":300,"cacheCreationInputTokens":400,"costUSD":2.5}}}'
} >"$P/worktree/s2.jsonl"

check "total without sessions fails" "1" "$(status total "$RUN")"
check "add refuses a path" "1" "$(status add "$RUN" ../x)"
check "add refuses a missing run folder" "1" "$(status add "$WORK/nope" s1)"

usage add "$RUN" s1
usage add "$RUN" s1
usage add "$RUN" s2
check "add records each session once" "s1 s2" "$(paste -sd' ' "$RUN/sessions")"

check "header" "model	input	output	cache_write	cache_read	cost_usd" "$(usage total "$RUN" | head -1)"
check "opus priced, deduplicated, cost-state added, suffix dropped" "claude-opus-5-5	1000100	1000200	2000400	1000300	39.7" "$(row claude-opus-5-5)"
check "haiku long prompt pays 5 times" "claude-haiku-5-5	200000	0	0	0	0.1" "$(row claude-haiku-5-5)"
check "total row" "total	1200100	1000200	2000400	1000300	39.8" "$(row total)"
check "total exits 0 when everything is found" "0" "$(status total "$RUN")"

msg m4 claude-opus-5-5 1000000 0 0 0 0 ',"speed":"fast"' >"$P/repo/s1/subagents/agent-b.jsonl"
check "fast mode pays twice" "claude-opus-5-5	2000100	1000200	2000400	1000300	47.7" "$(row claude-opus-5-5)"
rm "$P/repo/s1/subagents/agent-b.jsonl"

msg m5 claude-unknown-9 10 0 0 0 0 >"$P/repo/s1/subagents/agent-c.jsonl"
check "unknown model shows ?" "claude-unknown-9	10	0	0	0	?" "$(row claude-unknown-9)"
check "total leaves the unknown model out" "39.8" "$(row total | cut -f6)"
check "unknown model makes total partial" "2" "$(status total "$RUN")"
rm "$P/repo/s1/subagents/agent-c.jsonl"

usage add "$RUN" gone
check "total exits 2 when a transcript is missing" "2" "$(status total "$RUN")"
check "partial total still prints" "39.8" "$(row total | cut -f6)"

id="$(usage new "$RUN")"
check "new prints a lowercase UUID" "yes" "$([[ "$id" =~ ^[0-9a-f-]{36}$ ]] && echo yes || echo no)"
check "new records it" "$id" "$(tail -1 "$RUN/sessions")"

exit "$FAILURES"
