# pr-review-watch

Watches the review comments on your open GitHub PRs, so Claude Code can fix them as they arrive
instead of you checking the PR every few minutes.

The script polls GitHub once a minute and prints one line per new comment. It is meant to run
under Claude Code's Monitor tool, which wakes Claude only when a line is printed. A quiet hour
costs no Claude tokens and 60 GraphQL queries of 13 points each, about 780 of the 5,000 points
GitHub gives an account per hour, shared with everything else that uses your `gh` login.

The `watch-pr-reviews` skill (`.claude/skills/watch-pr-reviews/`) is the other half: it starts the
watch and says what Claude does with each batch. Claude fixes and commits locally, then tells you
what to push. It never pushes, and only replies to or resolves a thread on GitHub when you ask.

- [Prerequisites](#prerequisites)
- [Using it from Claude Code](#using-it-from-claude-code)
- [Running the script by hand](#running-the-script-by-hand)
- [Output](#output)
- [Settings](#settings)
- [How it works](#how-it-works)
- [Limits](#limits)
- [Testing](#testing)

## Prerequisites

- `gh`, logged in (`gh auth login`) with read access to the repo
- `jq`
- bash 3.2 or later, so the stock macOS bash works

## Using it from Claude Code

In a Claude Code session started in this repo, type `/watch-pr-reviews`. Claude asks whether to
handle the comments already open or only the new ones, then starts the watch.

A Monitor watch lasts 30 minutes at most. Claude restarts it each time until you say stop. It
ends with the session: closing Claude Code stops the watch, and `/watch-pr-reviews` in a new
session picks up where it left off, since the seen comments are on disk.

What Claude does with a batch:

1. Reads each comment and its whole thread, and weighs it before applying it. A comment that is
   wrong or out of scope is not applied, and the report says why.
2. Works in a git worktree next to the repo (`wt-pr-<number>`), so your working tree is not
   touched. If the PR branch is the one you have checked out, it asks first.
3. Runs the tests and type checks that cover the change, then makes one commit per PR for the
   whole batch.
4. Reports what it did per comment, the commit, the test result and the push command.

## Running the script by hand

```bash
scripts/watch-pr-reviews.sh                 # watch the repo of the current directory
scripts/watch-pr-reviews.sh owner/a owner/b # watch several repos
scripts/watch-pr-reviews.sh --once          # print what is new now and exit
scripts/watch-pr-reviews.sh --mark-seen     # skip the current backlog and exit
scripts/watch-pr-reviews.sh --help
```

Writes nothing to the repo. It only keeps state under `~/.local/state/pr-review-watch/`.

## Output

One event per line on stdout. Everything else goes to stderr.

| Line                                                              | When                                                                                                                                 |
| ----------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| `NEW_REVIEW_COMMENT repo= pr= branch= kind= thread= at= by= url=` | A comment not seen before                                                                                                            |
| `BATCH_END count=<n>`                                             | After each group of `NEW_REVIEW_COMMENT` lines                                                                                       |
| `WATCH_ERROR repo= <message>`                                     | GitHub could not be read. Once per failure streak                                                                                    |
| `WATCH_RECOVERED repo=`                                           | GitHub answers again after a `WATCH_ERROR`                                                                                           |
| `WATCH_WARNING repo= <message>`                                   | Results were cut at a page size, see [Limits](#limits), or fewer than 10% of the hourly GraphQL points are left. Once while it holds |

`kind` is `thread` for an inline comment, or `review-<state>` (like `review-changes_requested`)
for the body of a review. `at` is `path:line`, `path:outdated` when the line no longer exists, or
`-` for a review body.

## Settings

All optional, as environment variables.

| Variable                    | Default                          | What it does                                                                     |
| --------------------------- | -------------------------------- | -------------------------------------------------------------------------------- |
| `POLL_SECONDS`              | `60`                             | Seconds between two polls                                                        |
| `QUIET_SECONDS`             | `180`                            | How long no new comment must arrive before a batch is printed                    |
| `MAX_BACKOFF_SECONDS`       | `900`                            | Longest wait between polls while GitHub keeps failing                            |
| `IGNORE_AUTHORS`            | empty                            | Comma-separated logins to ignore, like a bot that comments                       |
| `ONLY_AUTHORS`              | empty                            | Comma-separated logins to act on. When set, everyone else is ignored             |
| `PR_REVIEW_WATCH_STATE_DIR` | `~/.local/state/pr-review-watch` | Where seen ids, locks and error markers live. Follows `$XDG_STATE_HOME` when set |

## How it works

Each poll is one GraphQL query per repo: your open PRs, most recently updated first
(`is:pr is:open author:@me sort:updated-desc`), their
reviews, and their review threads with the comments.

- Kept: comments in unresolved threads, and review bodies that are not empty.
- Dropped: resolved threads, your own comments and replies, and the logins in `IGNORE_AUTHORS`.
  A reviewer answering in a thread after your reply counts as a new comment.

A reviewer who submits a review posts all its comments at once. One who comments line by line
does not, so new comments are held until none has arrived for `QUIET_SECONDS`, then printed
together. Claude then handles the review in one go rather than one comment at a time.

A comment id is recorded as seen only once printed. A watch stopped during the quiet period finds
those comments again next time.

When GitHub fails, the script reports it once, doubles its wait up to `MAX_BACKOFF_SECONDS`, and
reports when it recovers. It does not exit, because a Monitor watch that exits is not restarted.

Only one watcher per repo can run, through a lock in the state directory. Two Claude sessions
reacting to the same comment would fix it twice. A lock left by a process that died is taken
over. `--once` and `--mark-seen` take the same lock, so they refuse to run while a watch is live
on that repo. Stop the watch first.

Review comments drive code changes, so who can post them matters. In a private repo everyone who
can comment already has access to the code. Elsewhere, set `ONLY_AUTHORS` to your reviewers. The
skill also tells Claude to treat a comment as a request for a code change, never as instructions
to run a command or touch anything else.

## Limits

- Polling, not webhooks. A comment shows up within `POLL_SECONDS + QUIET_SECONDS`, so 4 minutes
  by default. Webhooks would need a public endpoint or a relay to reach a laptop.
- Page sizes: 25 open PRs, 50 reviews and 50 threads per PR, 20 comments per thread. They keep a
  poll at 13 GraphQL points, measured with GitHub's `rateLimit(dryRun: true)`, and far under its
  500,000-node limit. Going past one prints a `WATCH_WARNING`, once while it lasts, and again if
  it comes back later.
- An edited comment is not reported again, since its id does not change.
- A watch only lives as long as the Claude Code session that started it.

## Testing

```bash
scripts/test.sh
```

Needs `jq` only, no network, and takes about 15 seconds. It checks the comment filter
(`review-comments.jq`) against a recorded GitHub answer (`fixtures/graphql-response.json`,
expected rows in `fixtures/expected.tsv`): a resolved thread, your own reply, an empty review
body, an ignored bot, the `ONLY_AUTHORS` allowlist, an outdated line and a deleted account.

It then runs the script itself against a stubbed `gh` that serves that answer: `--once` and
`--mark-seen`, the lock and the takeover of a stale one, comments held during the quiet window
and not recorded as seen, an outage reported once then its recovery, and a truncation warning
that is not repeated while it holds but comes back after it cleared.

Nothing in `.github/` runs it. Run it after changing the script.
