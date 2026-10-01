# 01 — Pricing and tiers (decided 2026-09-29)

The principle (user, D5), borrowed from ChatGPT and Claude:

> **Free is a real, complete product, not a crippled demo.** Upgrading gives **more** (capacity) and **smarter**
> (analysis, proactive insights, and later AI). It never unlocks basics that were artificially withheld.

## 1. Market prices (Cairo, EGP, USD ≈ 52.0 on 2026-09-29)

| Competitor | Model | Price | Notes |
|---|---|---|---|
| Local cashier programs (K-POS, N-Touch, …) | one-time | 2,500 – 7,500 | No updates and no analytics. The listing is undated, so today's prices are likely higher |
| Local modular accounting program | one-time per module | 1,000 – 7,000 per module, 43,000 for the full set | 2022 data |
| Daftra Basic | cloud SaaS | 489/mo (discounted from 733), 5,874/yr | **Capped at 100 invoices/month**, 1 user |
| Daftra Advanced / Comprehensive | cloud SaaS | 977 / 1,960 per month | |
| Edara Basic | cloud SaaS | $29 base + $5/user ≈ 1,770/mo | |
| Odoo Standard | cloud SaaS | ≈ 377 – 470 per user per month | Plus implementation cost |
| Loyverse | freemium | Free core, advanced inventory $25 per store ≈ 1,300/mo | |

No public price data exists at the level of El Marg or any single neighborhood. Phase G fills that gap with a
Van Westendorp survey (§5).

**Positioning:** the owner's current choices are "buy once for 2.5–5k and it never improves" or "pay 490+/month,
depend on the internet, and live with caps". Equal is free with no invoice cap, works offline, and Pro costs less
than any cloud plan.

## 2. Tiers and prices

| Tier | Monthly | Yearly | Pitch |
|---|---|---|---|
| **Free** | 0 | 0 | «مجاني مدى الحياة» |
| **Pro** ⭐ (marked «الأنسب») | **299** | **2,990** (two months free) | «أقل من 10 جنيه في اليوم» |
| **Business** (up to 3 branches) | **699** | **6,990** | «لسلاسل الفروع» |
| **Max** (when AI ships) | ≈ 1,299 | | Business plus high AI usage |

## 3. Entitlement matrix (mapped to real desktop route names)

The desktop enforces **keys**, never plan names. There are three kinds of entitlement:

- **capacity**: a numeric limit, checked on *create*.
- **mode**: a persistent feature. It is never creditable, because it creates lasting data.
- **action**: a one-shot feature. Free users can spend credits on it.

### Capacity

| Key | Free | Pro | Business |
|---|---|---|---|
| `maxProducts` | 100 | ∞ | ∞ |
| `maxBranches` | 1 | 1 | 3 |
| `maxTerminals` (LAN cashier devices besides the main one) | 0 | 3 | ∞ |
| `maxUsers` | 2 | 5 | ∞ |
| `actionCreditsPerMonth` (for Free users on action features) | 3 | n/a (unlocked) | n/a |

### Always free (never gated)

POS and shifts (`pos`, `pos-shifts`, `pos-shift-report`), invoices, quotations and refunds (`invoice*`, `quotation*`),
purchases and returns (`purchase*`), customers and suppliers, expenses, payments, vouchers, stock movements and
adjustments, categories, basic templates and printing, **backup and export**, and the statements
`report-sales`, `report-purchases`, `report-expenses`, `report-profit-loss`, `report-vat`, `report-vat-detail`,
`report-inventory`, `report-day-book`, `report-trial-balance`, `report-balance-sheet`, `report-ledger`, `report-shifts`.
Legal, compliance and data-safety features are never locked.

### Action features: Pro, and creditable on Free

| Key | Desktop route |
|---|---|
| `report.businessHealth` | `report-business-health` |
| `report.profitLeakage` | `report-profit-leakage` |
| `report.stockHealth` | `report-stock-health` |
| `report.periodComparison` | `report-period-comparison` |
| `report.grossProfit` | `report-gross-profit` |
| `report.discounts` | `report-discounts` |
| `report.returns` | `report-returns` |
| `report.aging` | `report-aging` |
| `report.overdue` | `report-overdue` |
| `report.cashFlow` | `report-cash-flow` |

### Mode features: Pro

| Key | Desktop route / area |
|---|---|
| `priceLists` | `price-lists` |
| `labels` | `labels` |
| `templateDesigner` | `settings-template-designer` |
| `recurringExpenses` | `expenses-recurring` |
| `stocktake` | `counts`, `count-new`, `count`, `report-stocktake-variances` |
| `expiryTracking` | `expiry` |
| `cardSettlements` | `card-settlements` |
| `approvals` | `approvals` |
| `customRoles` | `settings-roles` (editing; the built-in roles stay usable) |
| `proactiveInsights` | `settings-recommendations`, dashboard insights (`core/services/insightEngine.ts`) |

### Mode features: Business

| Key | Desktop route / area |
|---|---|
| `multiBranch` | `settings-branches` beyond `maxBranches`, `transfers`, `report-transfers`, `report-branch-comparison` |
| `costCenters` | `settings-cost-centers`, `report-cost-centers` |
| `budgets` | `report-budget-vs-actual` |
| `fullAuditLog` | `settings-audit-log` |

Pro also includes priority support and early access. Business also includes onboarding help.

## 4. How the free user is invited upward (quiet, never pushy)

1. **Show the result, blur the detail.** For example, profit leakage on Free says «لقينا 3 أماكن بيضيع منك فيها فلوس
   الشهر ده (حوالي 4,200 ج)», with the rows blurred under «اكشفها (متبقي 2 من 3)». This needs the report command to
   return a summary without a credit (a server-side summary, not blurred real data sent to the UI).
2. A 🔒 badge appears only where the feature lives. No popups, no persistent banners, no countdowns.
3. Nothing interrupts work. Limits apply on *create*, never mid-sale.
4. At the limit, one calm line in the relevant page only: «استخدمت الـ 3 مرات الشهر ده — العداد يرجع يوم 1» next to «رقّي لـ Pro».
5. At most one monthly value recap: «Pro كان هينبهك إن الصنف X خلص قبل ما يخلص فعلًا».

**For paid users:** the app is proactive (morning summary, alerts before problems), there are no in-app payment
reminders (renewal happens by email or portal), and yearly subscribers get a monthly value recap (see §6, point 7).

## 5. Validating the price in El Marg / Cairo (phase G)

Use the **Van Westendorp Price Sensitivity Meter** (1976), which is also recommended in Ramanujam & Tacke,
*Monetizing Innovation* (2016). Interview 15–20 shop owners in El Marg and the surrounding areas, after a 5-minute
demo. Ask four questions, per month:

1. «بكام تحس إنه رخيص لدرجة إنك تشك في جودته؟»
2. «بكام تحس إنه لقطة؟»
3. «بكام تحس إنه غالي بس ممكن تدفع؟»
4. «بكام تحس إنه غالي لدرجة إنك مش هتشتريه؟»

Plot the cumulative curves. If 299 falls inside the acceptable range, keep it. Otherwise, publish a new `plan_version`.

## 6. Scientific basis

1. **Zero-price effect.** Shampanier, Mazar & Ariely (2007), *Zero as a Special Price*, *Marketing Science*.
   «مجاني» pulls disproportionately more adoption than «رخيص», so Free is kept forever rather than offered as a discount.
2. **Freemium economics.** Typical free-to-paid conversion is 2–5% (industry benchmarks). Kumar (2014),
   *Making "Freemium" Work*, *HBR*, warns that freemium fails when serving free users is costly. Equal is offline,
   so a free user costs almost nothing. This advantage is structural and cloud competitors can't copy it.
3. **A premium tier above the target raises its sales.** Gu, Kannan & Ma (2018), *Selling the Premium in Freemium*,
   *Journal of Marketing* (a randomized field experiment). This works through the compromise effect (Simonson 1989,
   *JCR*) and the decoy effect (Huber, Payne & Puto 1982, *JCR*). So Pro sits in the middle and is marked «الأنسب»,
   and Business and Max exist even if they sell little.
4. **Left-digit effect.** Thomas & Morwitz (2005), *JCR*. That's why the price is 299, not 300.
5. **Temporal reframing.** Gourville (1998), *Pennies-a-Day*, *JCR*. That's why the copy says «أقل من 10 جنيه في اليوم».
6. **Flat-rate bias.** Lambrecht & Skiera (2006), *JMR*. Users prefer, and are happier with, unmetered plans, so Pro
   is "unlimited" and only AI is metered.
7. **Payment depreciation.** Gourville & Soman (1998), *JCR*, and their 2002 *HBR* article. People who pay up front
   use the product less over time and renew less, so yearly plans get a monthly value recap.
8. **Anchoring and value-based pricing.** Tversky & Kahneman (1974), plus Nagle & Müller, *The Strategy and Tactics of
   Pricing*. The reference value is the ~3,000 one-time local program, and the yearly Pro price (2,990) sits under it
   while adding updates, support and analytics.
9. **Penetration pricing.** Nagle & Müller: it suits a large, price-sensitive market with low marginal cost and an
   unknown brand. All four conditions hold here.
10. **Loss aversion.** Kahneman & Tversky (1979). On a price increase, existing subscribers keep their `plan_version`
    for 12 months. Optionally, add a «سعر المؤسسين» for sign-ups in the first 3 months.

## Sources

- Daftra plans (EGP): https://www.daftra.com/plans
- Edara pricing: https://getedara.com/pricing.html
- Odoo in Egypt: https://oec.sh/odoo-pricing/egypt
- Loyverse pricing: https://loyverse.com/pricing
- Cashier program prices in Egypt (BlackCat): http://kkctgroup.com/site/index.html%3Fp=4019.html
- Deltawy cashier prices (2022): https://deltawy.com/article/403/%D8%A3%D8%B3%D8%B9%D8%A7%D8%B1-%D8%A8%D8%B1%D9%86%D8%A7%D9%85%D8%AC-%D8%A7%D9%84%D9%83%D8%A7%D8%B4%D9%8A%D8%B1
- Exchange rate, Youm7, 2026-09-29: https://www.youm7.com/story/2026/9/29/%D8%A7%D8%B3%D8%AA%D9%82%D8%B1%D8%A7%D8%B1-%D8%B3%D8%B9%D8%B1-%D8%A7%D9%84%D8%AF%D9%88%D9%84%D8%A7%D8%B1-%D9%85%D9%82%D8%A7%D8%A8%D9%84-%D8%A7%D9%84%D8%AC%D9%86%D9%8A%D9%87-%D9%81%D9%89-%D8%A8%D8%AF%D8%A7%D9%8A%D8%A9-%D8%AA%D8%B9%D8%A7%D9%85%D9%84%D8%A7%D8%AA-%D8%A7%D9%84%D9%8A%D9%88%D9%85/7561263
- Gu, Kannan & Ma (2018): https://journals.sagepub.com/doi/10.1177/0022242918807170
- Kumar (2014), HBR: https://hbr.org/2014/05/making-freemium-work
