# ticket-flow

Carries one Jira ticket to a merge request that a human can test and merge. GitLab first, GitHub second. Version 0.1.0 is a first draft that has not been run on a real repository yet.

## Prerequisites

- Claude Code with the superpowers plugin (declared as a dependency).
- The Atlassian MCP server: `claude mcp add --transport http atlassian https://mcp.atlassian.com/v2/mcp`, then `/mcp` to sign in.
- `glab` logged in (`glab auth login`) for GitLab, or `gh` (`gh auth login`) for GitHub. `git`, `jq` and `bash`.

## Use

1. `/ticket-flow:profile` once per repository. Review `.claude/ticket-flow.md` and commit it.
2. `/ticket-flow:run SHOP-142`. The flow stops once for you to approve the plan, then runs to a draft MR, waits for CI, runs `/code-review`, handles the comments and posts a handover comment.
3. Test, merge, deploy, then `/ticket-flow:handover SHOP-142 --publish` to post the client test steps on Jira.

Each stage can also be run alone: `triage`, `plan`, `execute`, `open-mr`, `ci-gate`, `review`, `mr-loop`, `handover`. `/ticket-flow:run SHOP-142 --resume` continues an interrupted run.

## Safety

A hook denies pushes to protected branches, force pushes, merging, `reset --hard` and `clean -f`, in every session where the plugin is enabled. The flow resolves only the threads it fixed and checked. Client test steps are posted only after a human has read them.

## Tests

`bash tests/test-guard.sh`, `bash tests/test-forge.sh`, `bash tests/test-checks.sh`. They need `git` and `jq`, and no network.

The design is documented in the repository root: `architecture.md`, `plugin-design.md`, `handover-report.md`.
