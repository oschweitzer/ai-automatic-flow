# Ticket Flow Dashboard design system

The visual rules for the dashboard. The browsable version, with live component previews, is the private artifact https://claude.ai/artifact/Hd9J6Z5ZPWdVFEoTvVF1vV. These files are the copy the app builds from. When the two differ, change `tokens.json` here first, then republish the artifact.

| File | What |
|------|------|
| `tokens.json` | Colours, type styles, spacing, radii and shadow, each with a usage note. The source of every value. |
| `tokens.css` | The same tokens as CSS custom properties and one class per type style. Import it once at the app root. |
| `components/components.css` | Styles for the nine components, built on the tokens only. Classes are prefixed `tf-`. |
| `components/index.d.ts` | The props of each component. The React components still have to be written in the app from these types. |
| `components/<Name>.md` | When and how to use each component. |

Fonts are Geist and Geist Mono. `components.css` loads them from Google Fonts for now. The desktop app should ship the font files instead, so it works offline.

Ticket Flow Dashboard is a desktop app where each developer follows the tickets the ticket-flow plugin carries from Jira to a merge request. It is a working screen, read many times a day: what waits on me, where each AI run stands, what the client owes us. The system is dark only, quiet, with one lavender accent and colour reserved for state.

## Content fundamentals

- Write in the viewer's language. The UI ships in French and English, chosen per developer. Examples here are French because the team works in French.
- Address the developer as "toi" in French, "you" in English. Short, plain sentences. No exclamation marks, no emoji.
- Keep the flow's own terms as they are: stage names (`triage`, `plan`, `execute`, `open-mr`, `ci-gate`, `review`, `mr-loop`, `handover`) and end states (`ready_for_qa`, `parked_needs_info`) stay in English and lower case, in `label` or `small` style.
- Ticket keys are always written in full (`SHOP-142`), followed by the Jira summary as written in Jira.
- Status text says who, then what: "toi · plan à valider", "client · précision demandée", "CI rouge, job test:unit".
- Times are relative under a day ("il y a 4 min", "depuis 12 min"), then a date ("hier", "7 nov.").
- A colleague's run is never guessed: write "Run sur le poste de AL", never an empty progress track.

## Visual foundations

**Grounds.** The page sits on `bg`. The sidebar uses `rail`, one step darker, so the shell reads before the content. Sections, side cards and the detail panel use `surface`. Hover, selection and the active nav item use `surface-2`. Hairlines are `line` between rows and `line-2` for stronger dividers. Never put text on `line` or `line-2`.

**Text.** `text` for titles and ticket summaries, `text-2` for secondary copy, `text-3` for headers, sub-lines and timestamps. All three pass 4.5:1 on every ground.

**Accent.** `accent` (lavender) is the only accent: ticket keys, the primary button, the selected sprint chip, done stage dots, links, the focus ring. Text on a lavender fill uses `on-accent`. `accent-soft` grounds count badges and the `run` pill. One primary button per area.

**State colour.** State colours carry meaning and never decorate. `you` (amber) means waiting on the developer. `client` (sky) means waiting on the client. `ok` (green) and `bad` (red) are CI and run outcomes. Each sits on its `-soft` ground inside a `StatusPill`, always with a word, so state never relies on hue alone. Amber and sky differ in hue and lightness, and green and red are never the only cue.

**The violet surface.** `ActionCard` is the only place the violet gradient (`violet`, `violet-deep`, `violet-night`) appears. It marks what waits on the developer. Secondary text inside it uses `on-violet`. Do not reuse the gradient for headers, charts or decoration.

**Type.** Geist for everything, Geist Mono only for file paths and identifiers in the plan (`code`). Big text is light, small text is regular or medium:
- `page-title` (40px, 300) once per screen, for the project name.
- `panel-title` (26px, 300) for the detail panel key and the action card title.
- `count` (22px, 300) for counts in the all-projects table.
- `section` (15px) for section titles, `body` (14px) by default, `body-strong` (14px, 500) for ticket keys.
- `small` (12.5px) for sub-lines, `label` (12px, 500) for table headers and pill text.
Digits use tabular figures everywhere, so counts and times line up.

**Spacing.** `space-28` pads the main area, `space-22` separates the list from the side column, `space-20` pads cards and the panel, `space-18` pads table cells inline and `space-12` in block, `space-14` stacks sections. Use `gap`, never margins between siblings.

**Shape.** `radius-lg` (18px) for sections, cards and the panel, `radius-md` (12px) for buttons and action rows, `radius-nav` (10px) for nav items and the search field, `radius-pill` for chips, pills and avatars. Borders are 1px.

**Depth.** Borders separate, shadows do not. The only shadow is `shadow-panel`, on the detail panel. No glows, no coloured halos. The current stage dot gets a solid ring in `accent-soft` (or `bad-soft`), not a blur.

**Focus and controls.** Focus is a 2px solid `accent` ring, offset 2px, at least 10:1 on every ground. Control borders (chips, ghost buttons, the search field) use `control-border`, at least 3:1 on every ground. Browser selection uses `accent` with `on-accent`. Scrollbars are thin, with a `line-2` thumb.

**Motion.** Small and functional. Section chevrons rotate in 200ms with an ease-out curve, hovers fade in 150ms. Nothing animates on load. Respect `prefers-reduced-motion`.

**Layout.** A 248px sidebar (`rail`): brand, "Tous les projets", the project list as `NavItem`s, then sync state and settings at the bottom. The top bar holds the person filter (`Avatar`s), the sprint filter (`Chip`s) and search. A project screen is a list column of four `Section`s (En cours, En attente, À faire, Terminé) and a 340px side column with the `ActionCard`. Clicking a ticket swaps the side column for the detail panel, closed with Escape.

## The signature

`StageTrack` shows an AI run as eight dots, one per stage of the flow, filled in lavender up to the running stage. A blocked stage turns its ring red. The same stages appear as a `StepList` in the detail panel, with outcomes and times. Both read the run's `log.jsonl`. They are the reason the app exists, so give them room and never replace them with a percentage bar.

## Iconography

Icons follow Lucide (ISC licence): 24px grid, drawn at 16px, stroke 1.6, round caps and joins, `currentColor`. The bundle carries the few the components need (`chevron`, `arrow`, `folder`, `grid`, `check`, `alert`) through `TicketFlow.Icon`. The app should use `lucide-react` with the same stroke. No emoji, no Unicode glyphs as icons. There is no logo yet: the brand is the name set in Geist 500.
