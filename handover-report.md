# Handover report

The last stage of the flow writes two documents for the humans who take over:

- a comment on the merge request (MR), for the developer who tests, validates and merges,
- a draft of test steps for the client, posted on the Jira ticket only after a human has read it.

The developer should be able to test the change, explain it to a colleague or the client, and know what could go wrong, without reading the code.

## Where the content comes from

The report is written from what the flow already recorded, not from a fresh exploration of the repository:

| Input | Gives |
|-------|-------|
| `ticket.md` and the triage output | the problem, the acceptance criteria |
| `plan.json` | what was meant to change, files, behaviour |
| the diff | what actually changed, new env vars, migrations, dependencies |
| the executor's report | tests run, what was not tested and why, steps left to a human, known limits |
| review findings, comments and their triage | what was fixed, threads left open and why |
| CI status | the gates already passed |
| `triage.json` | the ticket type and the risk flags, which become critical points |
| the repository profile | how to run the app, test accounts, API client, test environment, client language |

To make this possible, the executor's report carries two fields: `not_tested` (with the reason) and `known_limits` (each with why it was accepted). The comment triage records the reason for every thread it left open.

## The MR comment

One comment per MR, carrying a hidden marker (`<!-- ticket-flow:handover SHOP-142 -->`). When the flow runs again after a fix, it updates that comment instead of posting a new one. It is written in the team's language.

Template, with example content for a fictional ticket:

````markdown
<!-- ticket-flow:handover SHOP-142 -->
## SHOP-142 handover

_Posted by ticket-flow from the developer's account._

CI green. AI review: 3 findings fixed and resolved, 1 thread left open. Plan approved by the developer.

### What this MR does

Customers can now enter a discount code at checkout. A percentage code lowers the cart total, and the total can never go below zero. Before this, discount codes existed in the back office but nothing applied them.

How it works, in one picture: the checkout asks the pricing service "what is this cart worth with this code?", the way a cashier checks a coupon against the list behind the counter before ringing it up. The cart itself stores nothing new, only the order keeps the code that was used.

In one sentence for a client: "Your customers can now use your promotion codes directly at checkout."

### What changed

| Area | Change |
|------|--------|
| Screen | A "Discount code" field and an "Apply" button on the checkout page |
| API | `POST /api/cart/discount` checks a code and returns the new total |
| Database | Migration adding `discount_code` to `orders` |
| Config | New optional env var `DISCOUNT_MAX_PERCENT` (default 50), added to `.env.example` |
| Tests | 6 unit tests on the discount rules, 1 API test |

### How to test

Prerequisites: check out `feat/SHOP-142-cart-discount`, run `npm ci`, `npm run db:migrate`, `npm run dev`, then log in as `buyer@test.local` (seed account).

1. Add two products to the cart and open the checkout. You should see the new "Discount code" field.
2. Enter `WELCOME10` and click "Apply". The total drops by 10 % and the code shows under the total. [Screenshot](link)
3. Enter `FAKE`. A message says the code is not valid, the total does not change.
4. Remove all products, then apply `WELCOME10`. The button is disabled.

Backend only: the repository has a Bruno collection, so the call is provided as [apply-discount.bru](link). Expected response: `{"total": 54.00, "discount": 6.00, "code": "WELCOME10"}`.

It is broken if: the total changes with an invalid code, or the code disappears after a page refresh.

### Test files

| File | Used in | Where |
|------|---------|-------|
| `discount-codes.xlsx` | importing codes in the back office, step 0 if the seed has none | [download](link) |

### Points of attention

Critical:
- The migration adds a nullable column, safe on existing orders. It must run before deployment.

Known limits, accepted on purpose:
- Two discount codes cannot be combined. The ticket does not ask for it.
- If a code expires while the customer is on the checkout page, it still applies to that order. Codes expire at midnight and checkout sessions last 30 minutes, so the window is tiny. The order keeps the code, so it can be traced.

To watch later:
- The pricing service is now called on every "Apply" click. Fine at current traffic, worth caching if checkout traffic grows a lot.

Not tested by the flow:
- The payment step with a discounted total, because it needs the payment sandbox.

Threads left open for a human:
- "Use a decimal library for money" (incorrect, in the flow's view): the project already rounds through `roundCents`, which the change uses.
- "Should a code be case sensitive?" (question): the ticket does not say. The code accepts `welcome10` today.

### Before merging

- [x] CI green
- [x] Unit and API tests added for the discount rules
- [ ] Manual test done (steps above)
- [ ] Open threads answered
- [ ] Payment step tested with a discounted total in the payment sandbox
- [ ] Migration planned for the test environment and production
- [ ] Local setup: add `DISCOUNT_MAX_PERCENT` to your `.env` if you want a value other than 50
- [ ] Infrastructure: set `DISCOUNT_MAX_PERCENT` on the test environment and production, or confirm the default is fine
- [ ] Client test steps below read and corrected
- [ ] After deployment to the test environment: `/ticket-flow:handover SHOP-142 --publish`

<details>
<summary>Client test steps (draft, not on Jira yet)</summary>

(the Jira comment below)

</details>
````

## API requests

For backend changes, the report uses the API tool the team already has. The `profile` skill detects it once and records it in the repository profile:

1. Bruno, if the repository contains a `bruno.json` or `.bru` files. The report provides a `.bru` request.
2. Otherwise Postman, if it contains a Postman collection (`*.postman_collection.json`). The report provides a request to import.
3. Otherwise curl, written so the same command works on Linux, macOS and Windows.

A cross-platform curl command fits on one line (no `\` or backtick line breaks, which differ between shells), uses double quotes only, and reads the JSON body from a file instead of inline, so no shell has to escape the JSON:

```text
curl -s -X POST "http://localhost:3000/api/cart/discount" -H "Content-Type: application/json" --data "@apply-discount.json"
```

The body file goes with the other test files, and the report adds one line for Windows: in Windows PowerShell, type `curl.exe`, because `curl` there is an alias for `Invoke-WebRequest`.

Requests and body files are attached the same way as the screenshots (see below). They are not added to the team's collection in the MR, which would change the diff after the review.

## Configuration in the checklist

When the change needs configuration, the checklist gets a line for each place to update, and only then. The report finds these from the diff and the plan:

- New or changed environment variables: names read in the code but missing from `.env.example`, or added to it.
- Local setup: `.env` files, `docker-compose` services, seed data, scripts a developer must run once.
- Infrastructure per environment: environment variables and secrets, Helm values or Kubernetes manifests, Terraform, CI variables, feature flags, cron jobs, new external services or URLs to allow.

Each line says what to set, where, and on which environments (local, test environment, production). When a default value exists, the line says so, and the item becomes "confirm the default is fine".

## The client test steps

Written in the client's language, with the words the client sees on screen, no technical detail, no internal links. It names the test environment, the data or account to use, each step with its expected result, the files to use, and the known limits in neutral terms.

It stays a draft until a human has read it. `/ticket-flow:handover SHOP-142 --publish` then posts it as a Jira comment and moves the ticket to the client test status. This follows the rule that anything a client sees is reviewed by a person first.

Example in French for the same ticket:

```markdown
Bonjour,

La saisie d'un code promo au moment de la commande est disponible sur l'environnement de recette : https://recette.example.com

Ce qui change : vos clients peuvent saisir un code promotionnel sur la page de paiement. La réduction s'applique immédiatement au total.

Pour tester :
1. Connectez-vous avec le compte de test qui vous a été communiqué.
2. Ajoutez deux produits au panier puis cliquez sur "Commander".
3. Dans le champ "Code promo", saisissez WELCOME10 puis cliquez sur "Appliquer".
   Résultat attendu : le total baisse de 10 % et le code apparaît sous le total.
4. Saisissez un code inexistant, par exemple TEST.
   Résultat attendu : un message indique que le code n'est pas valide.

Fichier utile : le fichier joint "codes-promo.xlsx" permet d'importer vos propres codes depuis l'administration.

À savoir : un seul code promo peut être utilisé par commande.

Si quelque chose ne se passe pas comme décrit, indiquez-nous l'étape et ce que vous avez vu, avec une capture d'écran si possible.
```

## Screenshots and test files

Screenshots are taken only when the change touches a screen and the profile says how to start the app. A throwaway Playwright script walks the happy path from "How to test", as the `explain-feature` skill already does, using seed accounts only, never real credentials.

Test files (images, spreadsheets, PDFs) are generated by a script when they are small and synthetic, for example a 3-row spreadsheet of fake discount codes. They never contain real client data. When a realistic file is needed and cannot be generated, the report says which file is needed and where to find it, and the checklist gets a line for it.

How files reach the comment depends on the forge. On GitLab, `forge.sh upload` sends each file to the project uploads API, which returns a Markdown link to paste into the comment, so nothing is pushed. On GitHub, `gh pr create` has an `--attach` option for the PR body, but I have not checked whether a comment can take one. Until then, the flow pushes screenshots and test files to a dedicated branch, `ticket-flow-assets`, under one folder per ticket, and links them from the comment. That branch is never merged, and the git guard allows pushing to it.

The Atlassian MCP tools listed in this session have no attachment operation, so files meant for the client go on the checklist as "attach to the Jira ticket", done by a human for now.

## Repository profile fields used

```yaml
run_app: npm run dev
app_url: http://localhost:3000
seed_accounts: "see docs/seed.md"
api_client: bruno            # detected: bruno, else postman, else curl
api_collection: bruno/       # where the detected collection lives, empty for curl
test_env_url: https://recette.example.com
client_language: fr
team_language: en
jira_statuses:
  client_test: "Recette client"
```

## Cost

The report is written by Sonnet from the run's own files, so it reads little code. Screenshots cost time more than tokens, which is why they are optional. The `explain-feature` HTML report, audio narration and comprehension quiz are left out of the flow. A developer can still run that skill on the MR when they want the long version.
