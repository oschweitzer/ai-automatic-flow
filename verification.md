# Verification

Checked on 26 October 2026 against the Claude Code documentation, Atlassian's documentation, GitLab's documentation (version 19.3), the GitLab CLI and GitHub CLI manuals (read through Context7), and the files installed on this machine.

## Corrections

The first proposal said `superpowers:subagent-driven-development` runs two reviews per task. That is wrong. Version 7.0.0 runs one combined review (spec compliance and code quality) after each task, allows up to five fix rounds, and ends with a review of the whole branch. The recommendation to review once per lot still stands, since that is fewer passes than once per task, but the gap is smaller than I said.

The first proposal presented "parallel cost 1.8 times as much as inline" as a finding. It is a single measurement on four small tickets, recorded in the `parallel-tickets` skill. It supports the default of working inline. It is not a general ratio.

## Verified

| Claim | Source |
|-------|--------|
| Subagent files accept `model` (`sonnet`, `opus`, `haiku`, `fable`, a full model ID, or `inherit`), `tools`, `disallowedTools`, `isolation: worktree`, `maxTurns`, `skills` and `effort` | [Subagents](https://code.claude.com/docs/en/sub-agents) |
| Plugin subagents ignore `hooks`, `mcpServers` and `permissionMode` | [Subagents](https://code.claude.com/docs/en/sub-agents) |
| The Agent tool's `model` parameter takes precedence over the agent's frontmatter | Agent tool description in this Claude Code session |
| Skill frontmatter accepts `argument-hint`, `arguments`, `disable-model-invocation` (Claude cannot load the skill on its own) and `allowed-tools` (granted without asking, does not restrict). `$0`, `$1` and `$ARGUMENTS` are replaced by the arguments. `${CLAUDE_SKILL_DIR}` is the skill's own folder, also inside a plugin | [Skills](https://code.claude.com/docs/en/skills.md) |
| The plugin loads with `--plugin-dir`: its six agents show up as `ticket-flow:<name>`, its skills stay out of the model's context, `/ticket-flow:run` starts the skill, and `${CLAUDE_SKILL_DIR}` resolves to the plugin's `skills/run` folder | test sessions on Haiku, 26 October 2026 |
| `claude plugin validate` passes for the plugin and the marketplace. It warns about a missing README (since added), licence and icon, which only matter for the public plugin directory | `claude plugin validate`, 26 October 2026 |
| Plugin layout (`.claude-plugin/plugin.json`, `skills/`, `agents/`, `hooks/hooks.json`, `.mcp.json`, `workflows/`, `bin/`), components namespaced as `plugin:name`, `${CLAUDE_PLUGIN_ROOT}` and `${CLAUDE_PLUGIN_DATA}`, reserved name prefixes | [Plugin manifest reference](https://code.claude.com/docs/en/plugins-reference) |
| `userConfig` values are stored under `pluginConfigs` in the user's settings, sensitive ones in the system's credential store | [Plugin manifest reference](https://code.claude.com/docs/en/plugins-reference) |
| A marketplace can be added from a local path, plugins are checked with `claude plugin validate` | [Create a marketplace](https://code.claude.com/docs/en/plugin-marketplaces) |
| Project scope writes `enabledPlugins` to `.claude/settings.json`, and each collaborator still runs the install once. `--plugin-dir` loads a plugin for one session. `claude plugin details` shows the tokens a plugin adds to every session | [Install and manage plugins](https://code.claude.com/docs/en/plugins/install.md) |
| A dependency from another marketplace needs `allowCrossMarketplaceDependenciesOn` in the root marketplace | [Plugin dependencies](https://code.claude.com/docs/en/plugins/dependencies.md) |
| superpowers is installed from `claude-plugins-official`, version 7.0.0, with the skills named in these documents | local plugin cache |
| Skill frontmatter accepts `model`, `effort`, `allowed-tools`, `context: fork` and `agent`. `${CLAUDE_SKILL_DIR}` points at the skill's folder | [Skills](https://code.claude.com/docs/en/skills.md) |
| `Stop` and `SubagentStop` hooks can block stopping with exit code 2. `PreToolUse` hooks can deny a tool call | [Hooks](https://code.claude.com/docs/en/hooks) |
| The Monitor tool runs a command in the background and feeds each output line back to Claude | [Tools reference](https://code.claude.com/docs/en/tools-reference.md) |
| Workflows: a script can choose a model per agent, cannot take user input mid-run, can be resumed in the same session, can ship in a plugin's `workflows/` folder, needs an explicit opt-in or a `Workflow(<name>)` allow rule, and shares the prompt cache across a fan-out | [Workflows](https://code.claude.com/docs/en/workflows.md) |
| `anthropics/claude-code-action@v1` runs in automation mode when given a `prompt`, takes CLI flags such as `--model` and `--max-turns` through `claude_args`, can install plugins, and rejects bot actors unless they are listed in `allowed_bots` | [GitHub Actions](https://code.claude.com/docs/en/github-actions) |
| OpenTelemetry is enabled with `CLAUDE_CODE_ENABLE_TELEMETRY=1`. `claude_code.cost.usage` and `claude_code.token.usage` carry `model`, `query_source`, `agent.name`, `skill.name` and `plugin.name`, with custom names redacted unless `OTEL_LOG_TOOL_DETAILS=1`. Cost figures are estimates | [Monitoring](https://code.claude.com/docs/en/monitoring-usage) |
| `/code-review` is a bundled skill that works on any plan and bills like the rest of the session. It runs as a forked subagent with its own context, accepts an effort level, a target (PR number, branch, range, path), `--fix` and `--comment` (inline comments on GitHub, one note on GitLab through `glab`). In a `-p` run it runs in the foreground and returns the findings as text or JSON. It follows `CLAUDE.md`, not `REVIEW.md` | [Review locally with /code-review](https://code.claude.com/docs/en/code-review-local) |
| The managed Code Review service needs a Team or Enterprise plan, runs on Anthropic infrastructure through the Claude GitHub App, posts inline comments and a check run, and never approves or blocks a PR | [Code Review](https://code.claude.com/docs/en/code-review.md) |
| A Claude Console service account (`svac_...`) is a non-human identity with no email, password or Console login. It acts through service account keys or Workload Identity Federation (short-lived tokens from GitHub Actions, Kubernetes, AWS, Google Cloud, Entra ID, Okta or any OIDC issuer). Usage is attributed to a workspace like an API key | [Workload Identity Federation](https://platform.claude.com/docs/en/manage-claude/workload-identity-federation) |
| `claude-code-action` accepts federation inputs (`anthropic_federation_rule_id`, `anthropic_organization_id`, `anthropic_service_account_id`). An OAuth token from `claude setup-token` is tied to the subscription of the person who created it | [GitHub Actions](https://code.claude.com/docs/en/github-actions), [action setup guide](https://github.com/anthropics/claude-code-action/blob/main/docs/setup.md) |
| The existing PR watcher drops comments written by the logged-in GitHub user, so a review posted from the developer's account would not reach the comment loop | `watch-pr-reviews/scripts/review-comments.jq` |
| The Atlassian Rovo MCP server lives at `https://mcp.atlassian.com/v2/mcp`, uses OAuth 2.1, and is added with `claude mcp add --transport http atlassian https://mcp.atlassian.com/v2/mcp` | [Atlassian Rovo MCP server](https://support.atlassian.com/atlassian-rovo-mcp-server/docs/getting-started-with-the-atlassian-remote-mcp-server/) |
| That server can search Jira with JQL, read a ticket, comment, list and apply transitions, create a ticket and link two tickets | tool list of the Atlassian server connected in this session (`searchJiraIssuesUsingJql`, `getJiraIssue`, `addCommentToJiraIssue`, `getTransitionsForJiraIssue`, `transitionJiraIssue`, `createJiraIssue`, `createIssueLink`), not Atlassian's documentation |
| GitLab's API creates a thread on a diff line (`POST /projects/:id/merge_requests/:iid/discussions` with a `position` built from the MR's latest version SHAs) and resolves it (`PUT .../discussions/:discussion_id?resolved=true`). Resolving needs the Developer role or above, or being the author of the change | [Discussions API](https://docs.gitlab.com/19.3/api/discussions/) |
| GitLab's API updates an MR note's body (`PUT /projects/:id/merge_requests/:iid/notes/:note_id`) | [Notes API](https://docs.gitlab.com/19.3/api/notes/) |
| GitLab's uploads API (`POST /projects/:id/uploads`) stores a file for use in an MR comment and returns a Markdown link | [Markdown uploads API](https://docs.gitlab.com/19.3/api/project_markdown_uploads/) |
| `glab api` sends `-f` as strings and `-F` with type conversion, reads `@file` values from a file, needs `--form` for uploads, sends `-f` as query parameters on a GET, and rejects bracketed field names in a JSON body (a nested value such as `position` goes as a JSON object with `-F`) | [GitLab CLI docs](https://gitlab.com/gitlab-org/cli/-/tree/main/docs/source) |
| `glab mr create` takes `--draft`, `--target-branch`, `--source-branch`, `--title` and `--description-file`. Its `--attach` flag is experimental. `glab mr update --ready` marks a draft ready. `glab ci get --merge-request <iid> --output json` reads the MR's head pipeline. `glab api` prefixes paths with `/api/v4/` and accepts `:id` and `:fullpath` placeholders | [GitLab CLI docs](https://gitlab.com/gitlab-org/cli/-/tree/main/docs/source) |
| `gh pr create` takes `--draft` and `--attach` (an image or video for the PR body). `gh pr ready` marks a draft ready | [GitHub CLI manual](https://cli.github.com/manual/gh_pr_create) |

## Not verified yet

These need a check during the spike on the test repository.

- Whether a plugin subagent with the `Skill` tool can load superpowers' `test-driven-development` skill.
- How `/code-review` takes its arguments (`medium origin/<base>...HEAD`), and whether the `--allowedTools` list in the review skill is enough for it in a `-p` run.
- The structure of `/code-review`'s JSON output in a `-p` run (file, line, severity, text), which the flow parses to post threads.
- Whether `/code-review` runs on the session's model when the session is started with `--model sonnet`.
- Whether the Claude Code CLI itself (not only the SDKs, the `ant` CLI and `claude-code-action`) accepts the Workload Identity Federation environment variables.
- Whether `gh pr comment` or the GitHub API can attach an image or file to a PR comment. If one can, the assets branch is unnecessary on GitHub.
- These GitLab endpoints used by `forge.sh`: the merge request list with `source_branch`, the `head_pipeline` field of a merge request, a pipeline's jobs with `scope[]=failed`, a job's trace, and adding a note to an existing discussion.
- The GitHub column of the `forge.sh` table, apart from `gh pr create --draft` and `gh pr ready`.
- Whether `createJiraIssue` can create a sub-task under a story (the parent field and the sub-task issue type).
- That a GitLab project or group access token gives a non-human identity for the comments. I know this from GitLab, not from a page read for these documents.
- The format of Bruno request files (`.bru`) and of a Postman collection item, before the handover generates them.
- That curl ships with current Windows versions as `curl.exe`, and that `curl` is an alias for `Invoke-WebRequest` in Windows PowerShell 5.1 but not in PowerShell 7. I know this from experience, not from Microsoft's documentation.
- Whether the Atlassian MCP server can attach files to a ticket. Its tools in this session have no attachment operation.
- Whether a Team or Enterprise seat may be assigned to a shared or non-human account. The documentation I read does not cover it, so the terms need checking with Anthropic.

## Sources

- https://code.claude.com/docs/en/sub-agents
- https://code.claude.com/docs/en/plugins-reference
- https://code.claude.com/docs/en/plugin-marketplaces
- https://code.claude.com/docs/en/plugins/install.md
- https://code.claude.com/docs/en/plugins/dependencies.md
- https://code.claude.com/docs/en/skills.md
- https://code.claude.com/docs/en/hooks
- https://code.claude.com/docs/en/tools-reference.md
- https://code.claude.com/docs/en/workflows.md
- https://code.claude.com/docs/en/github-actions
- https://code.claude.com/docs/en/monitoring-usage
- https://support.atlassian.com/atlassian-rovo-mcp-server/docs/getting-started-with-the-atlassian-remote-mcp-server/
- https://code.claude.com/docs/en/code-review-local
- https://code.claude.com/docs/en/code-review.md
- https://platform.claude.com/docs/en/manage-claude/workload-identity-federation
- https://github.com/anthropics/claude-code-action/blob/main/docs/setup.md
- https://docs.gitlab.com/19.3/api/discussions/
- https://docs.gitlab.com/19.3/api/notes/
- https://docs.gitlab.com/19.3/api/project_markdown_uploads/
- https://gitlab.com/gitlab-org/cli/-/tree/main/docs/source
- https://cli.github.com/manual/gh_pr_create
- https://cli.github.com/manual/gh_pr_ready
