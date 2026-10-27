#!/usr/bin/env bash
# Checks log.jsonl files against the log contract in reference/conventions.md.
# With files as arguments, checks them and exits 1 on any bad line. With no
# argument, checks the fixture and expects a few bad lines to be refused.
# Needs jq.

set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Prints "line N: <reason>" for each line of $1 that breaks the contract.
violations() {
  jq -R -r -n '
    def stages: ["triage","plan","execute","open-mr","ci-gate","review","mr-loop","handover"];
    def events: ["start","end","waiting","end_state","published"];
    def waits: ["developer","ci","reviews"];
    def models: ["haiku","sonnet","opus"];
    def end_states: ["ready_for_qa","split","parked_needs_info","plan_rejected","escalated_checks",
                     "escalated_ci","escalated_comments","timeout","failed_permission"];
    def keys_allowed: ["at","stage","event","outcome","on","model","detail"];
    def problems:
      if type != "object" then ["not a JSON object"] else
      [ (keys - keys_allowed | select(length > 0) | "unknown keys \(join(","))"),
        (select((.at | type) != "string" or (.at | test("^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9:]{8}Z$") | not)) | "bad at"),
        (select(.stage as $s | stages | index([$s]) | not) | "bad stage"),
        (select(.event as $e | events | index([$e]) | not) | "bad event"),
        (select(.event == "waiting" and (.on as $o | waits | index([$o]) | not)) | "waiting without a valid on"),
        (select(.event != "waiting" and has("on")) | "on outside a waiting line"),
        (select(has("model") and (.model as $m | models | index([$m]) | not)) | "bad model"),
        (select(.event == "end_state" and (.outcome as $o | end_states | index([$o]) | not)) | "bad end state"),
        (select(has("detail") and ((.detail | type) != "string" or (.detail | length) > 200)) | "bad detail")
      ] end;
    foreach inputs as $line (0; . + 1;
      . as $n
      | ($line | try fromjson catch null) as $v
      | if $v == null then "line \($n): not valid JSON"
        else ($v | problems[] | "line \($n): \(.)") end)' "$1"
}

if [[ $# -gt 0 ]]; then
  status=0
  for file in "$@"; do
    out="$(violations "$file")"
    [[ -z "$out" ]] || { echo "$file"; echo "$out"; status=1; }
  done
  exit "$status"
fi

FAILURES=0
accepts() {
  local out
  out="$(violations "$2")"
  if [[ -z "$out" ]]; then echo "ok    $1"; else
    echo "FAIL  $1"; echo "$out" | sed 's/^/      /'; FAILURES=$((FAILURES + 1)); fi
}
refuses() {
  local file
  file="$(mktemp)"
  printf '%s\n' "$2" >"$file"
  if [[ -n "$(violations "$file")" ]]; then echo "ok    $1"; else
    echo "FAIL  $1: accepted"; FAILURES=$((FAILURES + 1)); fi
  rm -f "$file"
}

FIXTURE="$HERE/fixtures/log-contract.jsonl"
if [[ -f "$FIXTURE" ]]; then accepts "the fixture follows the contract" "$FIXTURE"; else
  echo "FAIL  the fixture follows the contract: $FIXTURE is missing"; FAILURES=$((FAILURES + 1)); fi

refuses "a waiting line without on" '{"at":"2026-10-26T10:00:00Z","stage":"ci-gate","event":"waiting"}'
refuses "an unknown event" '{"at":"2026-10-26T10:00:00Z","stage":"ci-gate","event":"paused"}'
refuses "a detail of 201 characters" \
  "{\"at\":\"2026-10-26T10:00:00Z\",\"stage\":\"mr-loop\",\"event\":\"end\",\"detail\":\"$(printf 'x%.0s' $(seq 1 201))\"}"

exit "$FAILURES"
