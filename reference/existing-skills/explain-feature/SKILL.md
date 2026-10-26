---
name: explain-feature
description: Act as a patient teacher/reviewer who explains a feature, PR, branch, commit or diff in plain language (French or English) to someone who no longer writes the code themselves. Covers the problem solved, how it works with analogies, what changed, how to test it manually, how to pitch it to a client, produces an HTML report (rendered diagrams, built-in "listen" button) and optionally Playwright screenshots, a video or an offline audio narration. Use when the user asks to explain, understand, walk through, teach, or "re-expliquer" a feature, PR, commit or change — e.g. "explique-moi cette PR", "qu'est-ce qui a été fait sur cette branche", "explain this feature like I'm a PM", "comment je présente ça au client".
argument-hint: "[PR# | branch | commit | 'last' | feature name] [fr|en] [--screens] [--video] [--client] [--audio] [--no-html]"
---

# Explain Feature — le prof qui t'explique le code qu'il vient d'écrire

The user used to learn by coding and testing. Now the AI writes the code, so they lose that understanding. Your job is to **rebuild that understanding** — like a senior dev sitting next to them, walking through the work, not a code dump.

Audience: technically literate (knows what a frontend, API, database are) but does **not** want to read code. They must come out able to (1) explain it to a client, (2) test it themselves, (3) spot if something looks wrong.

## 1. Parse arguments

- **Target** (first arg): PR number (`123` or URL) → `gh pr view` + `gh pr diff`; branch name → `git diff <base>...<branch>`; commit SHA → `git show`; `last` / nothing → current branch vs base (`main`/`master`/`develop`, detect with `git symbolic-ref refs/remotes/origin/HEAD`), plus uncommitted changes if any; free text (feature name) → search the codebase (grep, git log --grep) to locate the relevant code.
- **Language**: `fr` or `en` if given; otherwise use the language the user wrote in. Write the WHOLE explanation in that language (code identifiers stay as-is).
- **Flags**: `--screens` → take screenshots; `--video` → record a Playwright video; `--client` → also produce the client-ready version (§3, section 7) prominently; `--audio` → also generate an offline audio file of the narration (§6); `--no-html` → terminal only, skip the report. If no flags, still include the client pitch briefly, and *offer* screenshots/video at the end if the feature has a UI.

Everything is written **outside the repository**, so git never sees it: `<out-dir>` below means `/tmp/explain-feature/<repo-name>/<feature-slug>/` (`<repo-name>` = basename of `git rev-parse --show-toplevel`). Create it with `mkdir -p`. Never write any output (report, screenshots, video, audio, throwaway scripts) inside the project. `<skill-dir>` below means the directory containing this SKILL.md.

## 2. Investigate before explaining (do this silently, don't narrate)

1. Get the diff and the stats (`--stat`). Read PR title/description, linked issue, commit messages — they hold the "why".
2. Read the changed files **and enough surrounding code** to understand how the change plugs into the existing app. A diff alone is misleading.
3. Identify: entry point (what a user does / what triggers it), data flow (where data comes from, where it goes, what is stored), external services touched, config/env vars/migrations added, tests added.
4. Separate **the meaningful change** from noise (formatting, renames, lockfiles, generated files). Never explain noise in detail — just mention it in one line.
5. Note anything risky or surprising: missing tests, hardcoded values, TODOs, security-sensitive areas (auth, payments, permissions, personal data), DB migrations, breaking changes. You are also a reviewer.

If the diff is huge, group by sub-feature and explain each one; don't go file by file.

## 3. Explanation content → `explanation.md`

Write the full explanation to `<out-dir>/explanation.md`, starting with a `# <Feature name>` title and a one-line meta (source: PR #/branch/commit, date). It is rendered as an HTML page (§5), so Mermaid diagrams, tables, `<details>` blocks and images (`screens/01-....png`, relative paths) all display properly. Do NOT dump this whole document in the terminal (see §5).

Use these sections, in this order. Short paragraphs, bullet points, bold for key ideas. No jargon without an immediate plain-language translation in parentheses. Max ~1 code snippet total, and only if it truly helps (e.g. a 3-line rule that IS the business logic).

### 1. En une phrase / In one sentence
What this does, as you'd say it to a non-technical client.

### 2. Le problème résolu / The problem it solves
The situation before: what was painful, broken or missing, and for whom (end user, admin, dev team). If the why isn't documented, infer it and say it's an inference.

### 3. Comment ça marche / How it works
- Start with an **analogy from everyday life** (restaurant, post office, library, building a house, etc.) that maps to the real mechanism — and say explicitly which piece of the analogy is which piece of the system. Pick one analogy and stick to it; a bad or stretched analogy is worse than none.
- Then a **step-by-step story** of what happens when the feature is used: "1. The user clicks X → 2. The app asks the server Y → 3. …". Name the real components (e.g. "the `InvoiceService`") once, alongside the plain explanation, so the user can connect it to what they'll see in the code or in a conversation with a dev.
- If the flow has 3+ actors or branches, add a small **Mermaid diagram** (sequence or flowchart) — it renders in the HTML report. Keep node labels short and in plain language; in flowcharts, quote labels containing parentheses or special characters (`A["Client (navigateur)"]`); in sequence diagrams, never quote participant aliases (`participant A as Application (serveur)`).

### 4. Ce qui a changé concrètement / What actually changed
A short table or list grouped by area (UI, server/API, database, config, tests). For each: what, in plain words. Include file paths as clickable links for those who want to dig, as absolute `file://` paths into the repo (the report lives outside it, so relative links would break). Mention new env vars, migrations or dependencies explicitly — these matter for deployment.

### 5. Comment le tester à la main / How to test it manually
A numbered checklist someone non-technical can follow: how to launch the app (exact commands from package.json/Makefile/README), where to go, what to click/type, **what they should see** (expected result). Include:
- the happy path,
- 2–3 edge cases worth trying (empty input, wrong permissions, very long text, network error…),
- how to tell it's broken.
If it can't be tested via a UI (pure backend, cron job…), give the simplest alternative (a curl command, a test command, a log line to look for) and explain what the output means.

### 6. Points d'attention / Watch-outs (reviewer hat)
Honest, prioritized: risks, limits, what's not handled, things to verify before shipping, questions to ask the dev/AI. If everything looks solid, say so in one line — don't invent problems.

### 7. Pour l'expliquer au client / How to explain it to a client
3–5 sentences, zero jargon, benefit-oriented ("vos utilisateurs peuvent désormais…"). Plus a one-liner version for an email/changelog. With `--client`, make this a polished, copy-pasteable block.

### 8. Vérifie ta compréhension / Check your understanding
2–3 short questions the user should be able to answer now (with answers hidden in a `<details>` block). This replaces the learning they used to get from coding it themselves.

## 4. Screenshots & video (when requested, or accepted after offering)

Only for features with a UI. Steps:
1. Find how to run the app (README, package.json scripts, docker-compose). Start the dev server in the background; wait until it responds.
2. Check whether Playwright is available (`npx playwright --version`). If not, ask before installing anything (`npm i -D @playwright/test && npx playwright install chromium` — or use a throwaway dir outside the project to avoid touching package.json).
3. Write a small throwaway script in the scratchpad (not in the repo) that walks the happy path from §5:
   - screenshots: `page.screenshot({ path, fullPage: true })` at each meaningful step, numbered (`01-login.png`, `02-form-filled.png`…);
   - video: `browser.newContext({ recordVideo: { dir, size: { width: 1280, height: 720 } } })`, slow the actions down a bit (`slowMo: 500`) so it's watchable, close the context to flush the video.
4. If the flow needs login, use only test credentials from the project's seed/fixtures/.env.example; never real credentials. If none exist, stop and ask.
5. Save screenshots in `<out-dir>/screens/` and the video in `<out-dir>/video/`. Reference each screenshot in `explanation.md` section 5 next to the matching step (`![Étape 2](screens/02-form-filled.png)`), and the video with `<video src="video/xxx.webm" controls></video>`.
6. Stop the dev server when done.

If the built-in browser pane tools are available, they're a fine alternative for screenshots of a running dev server.

Do this step BEFORE building the HTML report, so the images are referenced in it.

## 5. HTML report + terminal summary

The terminal does not render Mermaid or images, so the full explanation lives in an HTML page.

1. Write `<out-dir>/narration.txt` (rules in §6).
2. If `--audio`: run `bash <skill-dir>/scripts/tts.sh <out-dir>/narration.txt <out-dir>/narration <lang>`. On success it prints the audio file path; pass its basename to `--audio` below. Exit code 2 means no offline engine is available: skip silently (the page still has a 🔊 button using the browser's voice) and mention once how to get one (§6).
3. Build and open the page:
   ```bash
   python3 <skill-dir>/scripts/build_report.py --md <out-dir>/explanation.md \
     --narration <out-dir>/narration.txt [--audio narration.m4a] \
     --lang <fr|en> --title "<Feature name>" --out <out-dir>/report.html --open
   ```
   The page renders the markdown and Mermaid diagrams, follows light/dark mode, and has a sticky 🔊 listen bar (voice picker + speed). It loads marked and mermaid from a CDN, so it needs internet to render.
4. In the terminal, print only a **short summary** (≈15–25 lines): the one-sentence version, the problem solved, a small **ASCII diagram** of the flow (boxes and arrows with plain characters — never Mermaid in the terminal), the top watch-outs, and the path to `report.html` as a clickable link. If `--no-html`, print the full explanation instead, with ASCII diagrams only.

## 6. Narration script (for listening)

`narration.txt` is a **separate text written to be heard**, not a copy of the explanation. Think of a colleague explaining the feature over coffee, 2–5 minutes spoken (≈300–700 words; scale with the change).

- Plain prose, short sentences, in the chosen language. No markdown, no bullets, no tables, no emojis, no file paths, no code, no URLs.
- Spell out what would sound odd: "API" is fine, but say "le fichier de configuration" rather than `config.yml`; write numbers and symbols as words when ambiguous.
- Structure it orally: announce what's coming ("Trois choses à retenir…"), use the same analogy as the written version, recap at the end.
- Cover: the problem, the analogy and the flow, what to test, the main watch-out, and the client one-liner. Skip the file-by-file details and the quiz.
- Separate paragraphs with a blank line (natural pauses).

Offline audio engines used by `scripts/tts.sh` (all free):
- **macOS**: built-in `say`; it picks the best installed voice for the language. Better voices: System Settings → Accessibility → Spoken Content → System voice → Manage Voices → download a "Premium"/"Enhanced" French or English voice. Force one with `EXPLAIN_TTS_VOICE="<name>"`.
- **Linux**: [Piper](https://github.com/rhasspy/piper) (`pipx install piper-tts`) plus a voice model (`.onnx` + `.onnx.json`, e.g. `fr_FR-siwis-medium`, `en_US-lessac-medium` from the rhasspy/piper-voices repo on Hugging Face) placed in `~/.local/share/piper/`. `ffmpeg` optional (mp3 instead of wav). Never install or download these without asking the user first.

## 7. Tone & style rules

- You are a **teacher**, not a documentation generator: explain *why* things are done this way, not just *what*. When a design choice is non-obvious ("why a queue instead of doing it directly?"), explain the trade-off in one or two sentences.
- Introduce at most a handful of technical terms per explanation, each with a plain definition the first time. These are the words the user will learn and reuse with clients and devs — choose them deliberately.
- Concrete over abstract: use real examples from this app (actual screen names, actual field names, realistic sample data).
- No filler, no praise of the code, no "great question".
- Length scales with the change: a 20-line fix gets a short explanation (sections can be one line or merged); a big feature gets the full treatment.
- End with one line offering follow-ups: go deeper on a section, generate screenshots/video or audio (if not done), or produce the client version as a document.
