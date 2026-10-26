# Plugin design

## Two repositories

This repository, `ai-automatic-flow`, holds a plugin marketplace with one plugin in it, `ticket-flow`. A separate test repository, still to be chosen, will hold a small web app on GitLab, linked to a sandbox Jira project. Every skill is developed and tried there before the plugin is installed anywhere real.

## Layout

```text
ai-automatic-flow/
├── .claude-plugin/
│   └── marketplace.json
├── reference/existing-skills/     copies of the personal skills the plugin draws on, read only
└── plugins/
    └── ticket-flow/
        ├── .claude-plugin/
        │   └── plugin.json
        ├── README.md
        ├── reference/
        │   └── conventions.md       rules every skill reads first
        ├── skills/
        │   ├── run/SKILL.md          /ticket-flow:run SHOP-142 [--resume]
        │   ├── profile/SKILL.md      /ticket-flow:profile
        │   ├── triage/SKILL.md       /ticket-flow:triage SHOP-142
        │   ├── plan/SKILL.md         /ticket-flow:plan SHOP-142
        │   ├── execute/SKILL.md      /ticket-flow:execute SHOP-142
        │   ├── open-mr/SKILL.md      /ticket-flow:open-mr SHOP-142
        │   ├── ci-gate/SKILL.md      /ticket-flow:ci-gate SHOP-142
        │   ├── review/SKILL.md       /ticket-flow:review SHOP-142
        │   ├── mr-loop/SKILL.md      /ticket-flow:mr-loop SHOP-142
        │   └── handover/             /ticket-flow:handover SHOP-142 [--publish]
        │       ├── SKILL.md
        │       └── example.md
        ├── agents/
        │   ├── triager.md            Haiku
        │   ├── planner.md            Opus, Sonnet when triage says so
        │   ├── implementer.md        Sonnet
        │   ├── implementer-light.md  Haiku
        │   ├── comment-triager.md    Haiku
        │   └── fix-checker.md        Sonnet, read only
        ├── hooks/
        │   └── hooks.json
        ├── scripts/
        │   ├── lib.sh                profile reader
        │   ├── checks.sh
        │   ├── guard-git.sh
        │   ├── forge.sh
        │   ├── watch-mr-comments.sh
        │   └── comments.jq
        └── tests/                    test-guard.sh, test-forge.sh, test-checks.sh, no network needed
```

Plugin skills are namespaced by the plugin name, so `skills/run` is invoked as `/ticket-flow:run`. Each stage is its own skill so it can be run and debugged alone on the test repository. `run` chains them by reading each stage's `SKILL.md`, owns the caps and writes the end state.

Every skill sets `disable-model-invocation: true`. Claude never loads them on its own, so their descriptions add nothing to the context of a session that does not use the flow. The developer starts them with a `/` command. That is also why `run` reads the stage files instead of invoking the stage skills.

## Manifests

`plugins/ticket-flow/.claude-plugin/plugin.json`:

```json
{
  "name": "ticket-flow",
  "version": "0.1.0",
  "description": "Carry a Jira ticket to a reviewed merge request with Claude Code agents",
  "author": { "name": "Your team" },
  "dependencies": [
    { "name": "superpowers", "marketplace": "claude-plugins-official" }
  ]
}
```

`.claude-plugin/marketplace.json`:

```json
{
  "name": "ai-automatic-flow",
  "owner": { "name": "Your team" },
  "allowCrossMarketplaceDependenciesOn": ["claude-plugins-official"],
  "plugins": [
    {
      "name": "ticket-flow",
      "source": "./plugins/ticket-flow",
      "description": "Jira ticket to merge request flow"
    }
  ]
}
```

The `allowCrossMarketplaceDependenciesOn` entry is required: without it Claude Code does not install a dependency from another marketplace, unless the user already has it enabled at the same scope.

The plugin name must not start with `claude-` or `anthropic-`, which are reserved.

## Per-repository settings

Settings that change from one repository to the next live in the committed profile, `.claude/ticket-flow.md`, which the `profile` skill generates and a human reviews:

```markdown
---
forge: gitlab              # detected from the git remote: gitlab or github
jira_project: SHOP
base_branch: develop
branch_pattern: "{type}/{key}-{slug}"   # feat, fix or chore, from the triage type
protected_branches: [main, master, develop, staging, prod, production, "release/*"]
risk_paths: [migrations/, src/auth/, src/payment/]
review_bot: ""             # login of an existing review bot, if the team has one
commands:
  install: npm ci
  lint: npm run lint
  typecheck: npm run typecheck
  test: npm test
  build: npm run build
run_app: npm run dev
app_url: http://localhost:3000
seed_accounts: "see docs/seed.md"
api_client: bruno          # detected: bruno, else postman, else curl
api_collection: bruno/
test_env_url: https://recette.example.com
client_language: fr
team_language: en
---

## Module map

- src/cart: cart, pricing, discount codes
- src/checkout: payment and order confirmation
- src/lib: shared helpers (money, dates, http client)

## Conventions

Conventional Commits. Every new component gets a test next to it.
```

I chose this over the plugin's `userConfig` because `userConfig` values are saved in the user's settings, not in the repository, so a team would not share them.

`risk_paths` does not block anything. A ticket whose likely files fall under one of these paths is flagged as risky by triage, which raises the planner to Opus and puts the change in the handover's critical points.

## Agents

Agent files set the default model and restrict the tools. Plugin agents are namespaced like skills (`ticket-flow:implementer`). The orchestrator can override the model per call, because the Agent tool's `model` parameter takes precedence over the agent's frontmatter. That is how `planner` runs on Sonnet for a small ticket.

No agent uses `isolation: worktree`. The `execute` skill creates one worktree per ticket at a derived path, `<repository parent>/<repository name>.worktrees/<KEY>`, and every agent that changes code for the ticket works there. CI fixes and comment fixes need the same branch as the first implementation, and an automatic worktree would start a new one each time. Keeping it outside the repository stops the repository's own test runner and linter from scanning it.

The implementer:

```markdown
---
name: implementer
description: Implements a ticket-flow plan, or one fix asked by CI or a review comment, test first, in the run's worktree
model: sonnet
maxTurns: 60
tools: Read, Edit, Write, Bash, Grep, Glob
---

You change code in one git worktree, on a branch that already exists. ...
```

The fix checker, which only reads:

```markdown
---
name: fix-checker
description: Checks that one fix answers one review comment, from the comment and the fix diff only
model: sonnet
maxTurns: 10
tools: Read, Grep, Bash
---
```

`implementer-light` is the same as `implementer` with `model: haiku`. `triager` runs on Haiku and writes `triage.json`. `comment-triager` runs on Haiku and sorts a batch of review comments into the five categories. `planner` runs on Opus by default, reads code, and writes `plan.json` only.

Two limits apply to agents shipped in a plugin. They ignore the `hooks`, `mcpServers` and `permissionMode` fields, so read-only behaviour has to come from `tools`, and hooks go in the plugin's `hooks/hooks.json`. Bash still lets a read-only agent write files, so its prompt forbids it and the git guard below blocks pushes outside the flow's branches.

## Hooks

`hooks/hooks.json` carries one hook. A `PreToolUse` hook on Bash runs `guard-git.sh`, adapted from the existing `git-guardrails` skill. It denies a push to any branch in the profile's `protected_branches` (or a default list when there is no profile), including the indirect forms (`git push origin HEAD:main`, `feat/x:refs/heads/main`, or a bare `git push` while a protected branch is checked out). It also denies any force push (`--force`, `-f` in a group of flags, `--force-with-lease`, a `+` refspec), `push --all` and `--mirror`, `glab mr merge`, `gh pr merge`, the `--auto-merge` flag, `reset --hard` and `clean -f`. Any other branch may be pushed, which covers the flow's own branch and the `ticket-flow-assets` branch on GitHub. Merging stays with a human. `tests/test-guard.sh` checks 29 cases.

A plugin hook runs in every session where the plugin is enabled, not only during a run. That is intended for this guard: protected branches stay protected whatever the developer asks Claude to do in that repository.

Replies to and resolution of threads are limited by the `mr-loop` skill rather than by the hook. It only acts on the MR recorded in the run's `mr.json`, meaning an MR the flow opened itself, and only resolves threads whose fix passed the fix checker.

The design had a second hook, `SubagentStop`, to run the checks when an implementer finishes. It is left out: it would fire for every subagent of every session in the repository, including a plain exploration. The `execute` skill runs `checks.sh` itself instead.

## The forge script

Every call to GitLab or GitHub goes through `scripts/forge.sh`, which reads `forge` from the profile. No skill calls `glab` or `gh` directly. Adding a third forge later means adding one branch to one script.

| Command | GitLab | GitHub |
|---------|--------|--------|
| `find-mr <branch>` | `glab api` GET on `merge_requests` with `state=opened` and `source_branch` | `gh pr list --head` |
| `open-mr` | `glab mr create --draft --source-branch --target-branch --title --description-file --yes` | `gh pr create --draft --head --base --title --body-file` |
| `ci-status`, `ci-failed-log` | `glab api` GET on the MR (`head_pipeline`), then the pipeline's failed jobs and each job's trace | `gh pr checks --json bucket,link`, `gh run view --log-failed` |
| `post-thread <file> <line> <body>` | `glab api` POST on `merge_requests/:iid/discussions` with a `position` | `gh api` POST on the PR's review comments |
| `list-threads` | `glab api` GET on `merge_requests/:iid/discussions` | `gh api graphql`, review threads |
| `reply <thread> <body>` | `glab api` POST on the discussion's notes | `gh api` reply to the review comment |
| `resolve <thread>` | `glab api` PUT on the discussion with `resolved=true` | `gh api graphql`, `resolveReviewThread` |
| `upsert-note <marker> <file>` | find the note carrying the marker, then PUT on `merge_requests/:iid/notes/:id`, or POST a new one | same with `gh api` on issue comments |
| `upload <file>` | `glab api` POST on `projects/:id/uploads`, returns a Markdown link | push to the `ticket-flow-assets` branch, return its link |
| `mark-ready` | `glab mr update --ready` | `gh pr ready` |

In the GitLab column, `glab mr create`, `glab mr update --ready`, `glab api` and its field rules, and the discussions, notes and uploads endpoints are checked against GitLab's documentation. The merge request list filters, the `head_pipeline` field, the pipeline jobs and job trace endpoints, and replying in a discussion are not checked yet. `tests/test-forge.sh` checks the script's own logic against a stubbed `glab`, not against GitLab. In the GitHub column, only `gh pr create --draft` and `gh pr ready` are checked. The rest comes from the existing skills and my knowledge of `gh`, and gets checked when the GitHub port is built.

## External services

Jira goes through the Atlassian Rovo MCP server. The plugin does not bundle it, since many users already have it connected. The prerequisite is one command, then `/mcp` in a session to sign in:

```bash
claude mcp add --transport http atlassian https://mcp.atlassian.com/v2/mcp
```

Tool names carry a prefix that depends on how the server was added, so skills describe the operations (search with JQL, read a ticket, comment, list and apply transitions, create a sub-ticket, link two tickets) rather than hard-coding tool names.

GitLab goes through `glab`, authenticated with `glab auth login`. GitHub goes through `gh`, authenticated with `gh auth login`. In version 1 both use the developer's own account, so every flow comment starts with a short line saying it was posted by ticket-flow.

## Installing in a repository

While developing, load the plugin for one session from the test repository:

```bash
claude --plugin-dir /path/to/ai-automatic-flow/plugins/ticket-flow
```

To install it for a team:

```bash
claude plugin marketplace add /path/to/ai-automatic-flow    # or the Git URL once pushed
claude plugin install ticket-flow@ai-automatic-flow --scope project
```

Project scope writes the plugin into `enabledPlugins` in the repository's `.claude/settings.json`, which is committed. That turns the plugin on for collaborators but does not download it, so each of them still runs the install command once. Then run `/ticket-flow:profile`, review the generated file and commit it.

Before each release, `claude plugin validate ./plugins/ticket-flow` checks the manifests, and `claude plugin details ticket-flow` shows how many tokens the plugin adds to every session, which is worth watching with ten skills.

## What comes from existing skills

Copies of these skills sit in `reference/existing-skills/`, so the originals in the personal skills folder are never edited. The plugin's skills are rewritten from them, not linked to them.

- `parallel-tickets` becomes `plan` and `execute`: analysis rules, the fork or Sonnet rule, retry caps, and the warning about worktrees starting on the default branch. Its lots and parallel mode stay out of version 1. Changes: one ticket per branch, write `plan.json`, read the profile, post the plan to Jira.
- `watch-pr-reviews` becomes `mr-loop` and `watch-mr-comments.sh`: the Monitor-based watch, the `jq` filter, comment batching and the persisted list of seen comment ids. Changes: GitLab first through `forge.sh`, push the MR branch (never a protected branch), reply in threads, resolve only checked fixes, Haiku triage into five categories, the caps.
- `git-guardrails` becomes the `PreToolUse` hook.
- `explain-feature` becomes `handover`: its investigation rules, plain-language sections, test checklist, watch-outs, client pitch and Playwright screenshots. See [handover-report.md](handover-report.md).
- superpowers, declared as a dependency, supplies `test-driven-development`, `systematic-debugging` (for bug tickets), `receiving-code-review`, `verification-before-completion` and `using-git-worktrees`. Its `brainstorming` skill is left out because its human gates would stop the flow.

## Build order

Each step ends with a run on the test repository before the next one starts. GitLab comes first, GitHub at the end.

Status on 26 October 2026: a first draft of every skill, agent and script exists. The scripts are tested offline against stubs. The plugin loads, its agents show up as `ticket-flow:<name>`, and `/ticket-flow:run` starts. No step below has been run on a test repository yet, because there is none.

1. Test repository on GitLab, sandbox Jira project with the statuses from [architecture.md](architecture.md#jira-statuses), and the `profile` skill.
2. `triage` and `plan` on the pilot tickets (one per type, see [decisions.md](decisions.md)), without writing code. Read the plans and the split proposal, fix the skills until they are precise enough for Sonnet.
3. `execute`, the agents and the hooks.
4. `forge.sh` for GitLab, `open-mr` and `ci-gate`.
5. `review` and `mr-loop`.
6. `handover`, checked on three MRs: can a developer test and explain each one without opening the diff?
7. `run` to chain the stages, with the pick-up checks and every end state demonstrated at least once.
8. OpenTelemetry on, cost per run and per accepted ticket measured on the pilot tickets, routing tuned.
9. `forge.sh` for GitHub.
