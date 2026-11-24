# Ticket flow

Design notes for a Claude Code plugin that takes a Jira ticket on a web project and carries it to a merge request that a human can test and merge. GitLab comes first, GitHub second.

Status: a first draft of the plugin is in `plugins/ticket-flow/`. Its scripts pass their offline tests and it loads in Claude Code, but it has not run on a real repository yet: there is no test repository. Version 1 runs locally in Claude Code.

## The idea in one paragraph

Haiku sorts the ticket (bug, feature, technical change, or a story to split) and stops it if it is vague. Sonnet or Opus, depending on size and risk, reads the code once and writes a plan that a human approves. Sonnet or Haiku implements it test first. Lint, type checks and tests run before anything is pushed, a draft merge request opens, and the forge's CI has to go green. Then Claude Code's `/code-review` reviews the MR in a separate session, the flow fixes what is actionable and leaves questions and disagreements open for a human. It ends with a handover report that tells the developer how to test the change, and client test steps published on Jira once a human has read them.

## Documents

- [architecture.md](architecture.md) walks through the flow stage by stage, says which model does what, and explains where the tokens go.
- [plugin-design.md](plugin-design.md) describes the plugin layout, the skills, agents and hooks to write, how a repository installs it, and the build order.
- [handover-report.md](handover-report.md) describes the report left for the humans at the end: the MR comment for the developer and the test steps for the client.
- [decisions.md](decisions.md) lists what is settled, the calls I made that need checking, and the questions still open.
- [comparison.md](comparison.md) compares the first design with a second, service-oriented plan. The merge it proposed is now the design in the other documents.
- [review-controls.md](review-controls.md) maps the ideas of Florian Bruniaux's talk "Du code au merge" (Dev With AI, October 2026) to what the plugin does about each one, and what it leaves out.
- [verification.md](verification.md) lists every technical claim these documents rely on, with its source. It also records two corrections to earlier statements.

## Starting points

Copies of the personal skills the plugin draws on are in `reference/existing-skills/`. The originals are never edited. Two of them already cover part of the flow. `parallel-tickets` does the analysis, the plan and the review rules. `watch-pr-reviews` watches review comments with the Monitor tool and fixes them, but today it works on GitHub only, never pushes and never replies.

## Try it

```bash
bash plugins/ticket-flow/tests/test-guard.sh
bash plugins/ticket-flow/tests/test-forge.sh
bash plugins/ticket-flow/tests/test-checks.sh
claude --plugin-dir /path/to/ai-automatic-flow/plugins/ticket-flow   # from the repository to work on
```

## License

Released under the [PolyForm Noncommercial License 1.0.0](LICENSE). You may read, run, modify and share the code for any noncommercial purpose. Any commercial use requires the author's prior agreement.\n