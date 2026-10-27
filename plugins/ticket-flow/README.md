# ticket-flow

Carries one ticket, from Jira or from the repository's GitHub issues, to a merge request that a human can test and merge. GitLab first, GitHub second. Version 0.2.0 is a draft that has not been run on a real repository yet.

## Prerequisites

- Claude Code with the superpowers plugin (declared as a dependency).
- With Jira tickets: the Atlassian MCP server, `claude mcp add --transport http atlassian https://mcp.atlassian.com/v2/mcp`, then `/mcp` to sign in. With GitHub issues (`tracker: github` in the profile), `gh` is enough.
- `glab` logged in (`glab auth login`) for GitLab, or `gh` (`gh auth login`) for GitHub. `git`, `jq` and `bash`.

## Use

1. `/ticket-flow:profile` once per repository. Review `.claude/ticket-flow.md` and commit it.
2. `/ticket-flow:run SHOP-142`. The flow stops once for you to approve the plan, then runs to a draft MR, waits for CI, runs `/code-review`, handles the comments and posts a handover comment.
3. Test, merge, deploy, then `/ticket-flow:handover SHOP-142 --publish` to post the client test steps on the ticket.

With GitHub issues, give the issue number as `142`, `#142` or `GH-142`. The flow calls it `GH-142` in branch names and run folders. On a public repository, the flow shows you a light version of every post on the issue, without anything sensitive, and posts it only once you approve it.

Each stage can also be run alone: `triage`, `plan`, `execute`, `open-mr`, `ci-gate`, `review`, `mr-loop`, `handover`. `/ticket-flow:run SHOP-142 --resume` continues an interrupted run.

## Safety

A hook denies pushes to protected branches, force pushes, merging, `reset --hard` and `clean -f`, in every session where the plugin is enabled. The flow resolves only the threads it fixed and checked. Client test steps are posted only after a human has read them.

## Tests

`bash tests/test-guard.sh`, `bash tests/test-forge.sh`, `bash tests/test-checks.sh`, `bash tests/test-ticket-key.sh`, `bash tests/test-log-contract.sh`, `bash tests/test-usage.sh`. They need `git` and `jq`, and no network.

The design is documented in the repository root: `architecture.md`, `plugin-design.md`, `handover-report.md`.
