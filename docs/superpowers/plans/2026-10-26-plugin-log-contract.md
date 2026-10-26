# Plugin log contract implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the ticket-flow plugin write the few extra `log.jsonl` events the dashboard needs, so it can read where a run stands instead of guessing.

**Architecture:** The contract lives in `plugins/ticket-flow/reference/conventions.md`, which every skill reads first. A fixture log and a jq-based test pin the contract. Each skill that waits, launches a model or publishes gets one sentence telling it which line to write. Triage learns to restart a parked run.

**Tech Stack:** Markdown skills, bash, jq (already used by the plugin's tests and `comments.jq`).

**Spec:** [docs/superpowers/specs/2026-10-26-ticket-flow-dashboard-design.md](../specs/2026-10-26-ticket-flow-dashboard-design.md), section "Log contract with the plugin".

## Global Constraints

- One JSON object per line, appended, never rewritten. Existing fields keep their meaning: `at` (UTC, RFC 3339 with `Z`), `stage`, `event`, `outcome`.
- `event` is one of `start`, `end`, `waiting`, `end_state`, `published`.
- `waiting` carries `on`, one of `developer`, `ci`, `reviews`.
- `start` lines of `plan` and `execute` carry `model`, one of `haiku`, `sonnet`, `opus`.
- An optional `detail` string (at most 200 characters) may carry counts or a URL. The dashboard never displays it.
- Never write a secret, a token or a full log into `log.jsonl` (unchanged rule).
- Stage names stay: `triage`, `plan`, `execute`, `open-mr`, `ci-gate`, `review`, `mr-loop`, `handover`.

## Review Focus

- A parked ticket run again after the client answers: triage must append to the same log, not refuse and not delete the folder.
- A plan rework round: the `waiting` line is written only when the gate question is asked, so the dashboard shows "plan à valider" only while the developer is the one blocking.
- `handover --publish` run on a ticket whose last line is not `ready_for_qa`: it still logs `published`, and the dashboard only reads it after `ready_for_qa`.
- mr-loop without a review bot does not wait, so it writes no `waiting` line.
- The `detail` field must not become a place to paste a CI log or a comment body.

---

### Task 1: Write the contract and pin it with a test

**Files:**
- Modify: `plugins/ticket-flow/reference/conventions.md` (section "Log", lines 35-43)
- Create: `plugins/ticket-flow/tests/fixtures/log-contract.jsonl`
- Create: `plugins/ticket-flow/tests/test-log-contract.sh`

**Interfaces:**
- Produces: the fixture `tests/fixtures/log-contract.jsonl`, a full run that the dashboard plan copies as its first test fixture. `test-log-contract.sh <file>...` exits 0 when every line of every file follows the contract, 1 otherwise, printing `line N: <reason>` for each bad line.

- [ ] **Step 1: Write `tests/test-log-contract.sh`** in the style of `tests/test-checks.sh` (`set -euo pipefail`, a `FAILURES` counter, `PASS`/`FAIL` lines). It runs one jq program per file that checks each line against these rules:
  - it is a valid JSON object with no keys beyond the ones below
  - `at` matches `^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9:]{8}Z$`
  - `stage` is in the stage list and `event` in the event list
  - `on` is present and valid exactly when `event` is `waiting`
  - `model`, when present, is in the model list
  - `outcome` is in the end-state list when `event` is `end_state`
  - `detail`, when present, is a string of 200 characters or fewer

  With no argument it checks `tests/fixtures/log-contract.jsonl` and also runs three inline bad lines (a `waiting` without `on`, an unknown `event`, a `detail` of 201 characters) and expects each to fail.

- [ ] **Step 2: Run it to see it fail**

Run: `bash plugins/ticket-flow/tests/test-log-contract.sh`
Expected: FAIL, the fixture file does not exist yet. The three bad-line cases already print PASS (rejected as expected).

- [ ] **Step 3: Write the fixture.** A complete run of `SHOP-142` with these lines in order, times increasing: triage start/end `continue`, plan start with `"model":"sonnet"`, plan `waiting` `on:"developer"`, plan end `approved`, execute start with `"model":"sonnet"`, execute end, open-mr start/end with `"detail":"https://gitlab.example.com/shop/front/-/merge_requests/87"`, ci-gate start, ci-gate `waiting` `on:"ci"`, ci-gate end `green`, review start/end, mr-loop start, mr-loop `waiting` `on:"reviews"`, mr-loop end with `"detail":"actionable 2, question 1, resolved 2"`, handover start, `end_state` `ready_for_qa`, then handover `published`.

- [ ] **Step 4: Run it to see it pass**

Run: `bash plugins/ticket-flow/tests/test-log-contract.sh`
Expected: PASS for the fixture and for each bad line, exit code 0.

- [ ] **Step 5: Rewrite the "Log" section of `conventions.md`.** Keep the first example line and the secrets rule. Add, in prose and one table: the five events and when each is written, the `on` values and which stage writes each, the `model` field on `plan` and `execute` start lines, the optional `detail` field and its 200-character limit, the rule "the next `start`, `end` or `end_state` line closes a wait", and a pointer to `tests/fixtures/log-contract.jsonl` as the full example. State that the dashboard reads this file and that changing the contract means changing that fixture and the dashboard together.

- [ ] **Step 6: Run all plugin tests**

Run: `for t in plugins/ticket-flow/tests/test-*.sh; do bash "$t" || exit 1; done`
Expected: every script ends without a FAIL line and the loop exits 0.

- [ ] **Step 7: Commit**

```bash
git add plugins/ticket-flow/reference/conventions.md plugins/ticket-flow/tests/fixtures/log-contract.jsonl plugins/ticket-flow/tests/test-log-contract.sh
git commit -m "feat(ticket-flow): log contract for waits, models and publishing"
```

### Task 2: Make each skill write the new lines

**Files:**
- Modify: `plugins/ticket-flow/skills/plan/SKILL.md` (section 1 and section 4)
- Modify: `plugins/ticket-flow/skills/execute/SKILL.md` (where the stage starts)
- Modify: `plugins/ticket-flow/skills/ci-gate/SKILL.md` (section "Wait")
- Modify: `plugins/ticket-flow/skills/mr-loop/SKILL.md` (section "Rounds and waiting")
- Modify: `plugins/ticket-flow/skills/handover/SKILL.md` (section "Publishing the client steps")
- Modify: `plugins/ticket-flow/skills/triage/SKILL.md` (section "1. Already running?")
- Modify: `plugins/ticket-flow/skills/run/SKILL.md` (the `--resume` rule)

**Interfaces:**
- Consumes: the contract from Task 1, by reference to `conventions.md`. Skills name the event, never restate the schema.

- [ ] **Step 1: plan.** In section 1, replace "Record the choice in the log" with: write the `plan` start line with `model` set to the chosen model. In section 4, before "Ask the developer", add: append a `waiting` line with `on: "developer"`. On a rework answer, the next planner launch needs no new start line. The following `waiting` line marks the next question.

- [ ] **Step 2: execute.** Where the stage starts, state that the start line carries `model` set to `plan.json`'s `executor`.

- [ ] **Step 3: ci-gate.** At the top of "Wait", before the Monitor loop: append a `waiting` line with `on: "ci"`. After a fix is pushed and the skill waits again, append another.

- [ ] **Step 4: mr-loop.** In "Rounds and waiting", before waiting for the review bot: append a `waiting` line with `on: "reviews"`. Say explicitly that without a review bot no `waiting` line is written. Change "Log the end with the counts per category" to put those counts in `detail`.

- [ ] **Step 5: handover.** In "Publishing the client steps", replace "Log the event." with: append a line with `stage: "handover"` and `event: "published"`.

- [ ] **Step 6: triage and run.** In triage's first check, split the existing-folder case in two. When the last `end_state` in `log.jsonl` is `parked_needs_info`, tell the user the run restarts after the client's answer, overwrite `ticket.md` with a fresh snapshot, and continue from step 2, appending to the same log. Any other existing folder keeps today's behaviour (offer `--resume`, stop). In `run/SKILL.md`, next to the rule that a run with an end state does not resume, add that a parked run restarts through triage instead.

- [ ] **Step 7: Check every skill names the contract**

Run: `grep -n '"waiting"\|waiting line\|`model`\|published' plugins/ticket-flow/skills/{plan,execute,ci-gate,mr-loop,handover}/SKILL.md && grep -n parked_needs_info plugins/ticket-flow/skills/triage/SKILL.md plugins/ticket-flow/skills/run/SKILL.md`
Expected: at least one hit in each of the seven files.

- [ ] **Step 8: Load the plugin**

Run: `claude --plugin-dir plugins/ticket-flow -p "List the ticket-flow skills you can see, one per line." --max-turns 1`
Expected: the eight stage skills and `run`, `profile` are listed, so the edited frontmatter still parses.

- [ ] **Step 9: Commit**

```bash
git add plugins/ticket-flow/skills
git commit -m "feat(ticket-flow): skills write waits, models, publishing, parked runs restart"
```
