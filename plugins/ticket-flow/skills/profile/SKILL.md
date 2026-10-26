---
name: profile
description: Use when setting up ticket-flow in a repository for the first time, or when the repository's commands, branches or tools changed and .claude/ticket-flow.md is out of date.
disable-model-invocation: true
---

# Repository profile

Writes `.claude/ticket-flow.md`, the file every other ticket-flow skill reads instead of exploring the repository. A human reviews and commits it.

Read `${CLAUDE_SKILL_DIR}/../../reference/conventions.md` first.

## Find out

Look, do not guess. When something cannot be found, write `TODO` and list it for the user.

| Field | Where to look |
|-------|---------------|
| `forge` | `git remote get-url origin`: `github` for github.com, `gitlab` otherwise |
| `base_branch` | `develop` if `origin/develop` exists, else the branch `origin/HEAD` points to |
| `branch_pattern` | recent branch names (`git branch -r`), else `{type}/{key}-{slug}` |
| `protected_branches` | the default list below, plus long-lived branches seen on the remote |
| `commands` | `package.json` scripts, `Makefile`, `composer.json`, `pyproject.toml`, the CI file (`.gitlab-ci.yml`, `.github/workflows/`), the README |
| `run_app`, `app_url` | the dev script and its port, `docker-compose.yml` |
| `seed_accounts` | seed files, fixtures, `.env.example`, the README. Point to the file, never copy a password |
| `api_client` | `bruno` if there is a `bruno.json` or `.bru` files, else `postman` if there is a `*.postman_collection.json`, else `curl` |
| `api_collection` | the folder or file of that collection, empty for curl |
| `risk_paths` | folders for migrations, authentication, permissions, payment |
| module map | top-level source folders, one line each on what they hold |

Ask the user, in one message, for what the repository cannot tell: `jira_project`, `test_env_url`, `client_language`, `team_language`, and `review_bot` if the team has one.

## Write

```markdown
---
forge: gitlab
jira_project: SHOP
base_branch: develop
branch_pattern: "{type}/{key}-{slug}"
protected_branches: [main, master, develop, staging, prod, production, "release/*"]
risk_paths: [migrations/, src/auth/]
review_bot: ""
commands:
  install: npm ci
  lint: npm run lint
  typecheck: npm run typecheck
  test: npm test
  build: npm run build
run_app: npm run dev
app_url: http://localhost:3000
seed_accounts: "see docs/seed.md"
api_client: bruno
api_collection: bruno/
test_env_url: https://recette.example.com
client_language: fr
team_language: en
---

## Module map

- src/cart: cart, pricing, discount codes

## Conventions

Commit style, test placement, anything an agent must follow.
```

Keep every frontmatter value on one line: the scripts read it with a simple parser. A command that must not run gets an empty value.

Then add `.ticket-flow/` to `.git/info/exclude` if it is not there.

## Finish

Run `bash ${CLAUDE_SKILL_DIR}/../../scripts/checks.sh` once and report each check's result. A failing check is fine to report, it means the base branch is red, which the user must know before running the flow.

Show the user the file and the `TODO` list. Do not commit it.
