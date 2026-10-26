# Handover examples

Worked examples for a fictional ticket. Match their structure and tone, not their content.

## MR comment

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

## Client test steps (French)


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

