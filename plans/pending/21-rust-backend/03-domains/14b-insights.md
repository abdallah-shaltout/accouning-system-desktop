# 21 · 03.14b — `dashboard` insights (rule engine, product inline hints, thresholds)

> **Status:** code complete 2026-09-28 (this implementer): `service/insights/{engine,common}.rs`,
> `rules/{stock,receivables,cash,accounting,sales}.rs` (all 21 rules) and `hints.rs`; the 2 commands
> wired into `domains/dashboard/{commands,mod,dto}.rs` alongside 14's 13. `cargo check` not run
> (hard rule); tests written in `domain_analytics.rs` (shared file with 14) but not executed.
> Frontend: `InsightIconKey` (`insightTypes.ts`), `INSIGHT_ICONS` (`insightRules.ts`), the
> `computeAll(role?)`/`getProductInlineHints` Rust branches + `toInsight()` + mirror clears
> (`insightEngine.ts`), thresholds routed through `useSettingsStore` (I-2), `setThresholds` made
> async (I-3) with the one-word `RecommendationsSettingsPage.vue:53` edit
> (`await setThresholds(patch)`). `contract.check.ts` entry for `InsightDto` added (with the
> `contract-ok` note for `icon`/`roles`). **Needs from manager**: same as 14's note (domain
> registration hooks + G-39's `transfer_dto`). Wave W6 (entry file §4). Depends on:
> [`14-analytics.md`](14-analytics.md) (same `domains/dashboard/`, gaps G-36–G-38), 13's G-32, G-33, G-13, G-1, and
> 01-settings (thresholds are a `StoreSettings` field it already serves).
> **Split:** second half of the entry file's row 14. [`14-analytics.md`](14-analytics.md) holds
> analytics and the dashboard KPIs; this file holds the insight engine. Same implementer, after 14.

**Goal.** Move the 21-rule insight catalogue (`insightRules.ts`) and the product inline hints to
Rust, so every terminal computes the same hints from the branch DB, while the per-device parts
(dismiss/snooze, the role/rule/limit filters over the list) stay in TypeScript. Pages, components and
controllers don't change.

**Read first.** [`01-frontend-analysis/core.md`](../01-frontend-analysis/core.md) §1b, §2, §5, §7, §9
(`AC§n`) · `src/modules/core/services/insightEngine.ts` (`ie:<line>`) ·
`src/modules/core/services/insightRules.ts` (`ir:<line>`) · `src/modules/core/services/insightTypes.ts`
(`it:<line>`) · `src/modules/core/controllers/useInsights.ts` (callers, all inside `computed`) ·
`src/modules/settings/pages/RecommendationsSettingsPage.vue:24,44,53` (thresholds caller) ·
`src-tauri/src/entities/values.rs:259-265` (`InsightThresholds` = sparse `BTreeMap<String, JsonDecimal>`).

## 1. Commands and dispositions

| Mock fn | Disposition (AC§1b → here) | Rust command | Args → Return | Area / Access | tx | events |
|---|---|---|---|---|---|---|
| `computeAll` (`ie:103`, internal) | **port** (new command, decision I-1) | `dashboard_compute_insights` | `{ numerals: Numerals }` → `Vec<InsightDto>` | Dashboard / Read | `with_read_ctx` | none |
| `getProductInlineHints` (`ie:164`) | port | `dashboard_get_product_inline_hints` | `{ productId: Id }` → `Vec<InsightDto>` | Inventory / Read | `with_read_ctx` | none |
| `getInsights` (`ie:129`) | port → **stay-frontend filter** over the mirrored `computeAll` (I-1) | — | — | — | — | — |
| `getInsightsFor` (`ie:148`) | port → stay-frontend (predicate can't cross IPC, AC§9) | — | — | — | — | — |
| `getInsightsForEntity` (`ie:153`) | port → stay-frontend (built on `getInsightsFor`) | — | — | — | — | — |
| `getThresholds` (`ie:24`) | port → **served by 01-settings** (I-2) | (`settings_get_settings`) | — | — | — | — |
| `setThresholds` (`ie:28`) | port → **served by 01-settings** (I-2); becomes `async` (contract fix I-3) | (`settings_update_settings`) | — | — | — | — |
| `dismissInsight`, `snoozeInsight`, `clearDismissal` | frontend (AC§1b, D9 note) | — | — | — | — | — |
| `forceRefresh` (`ie:227`) | frontend; also clears the insight mirrors | — | — | — | — | — |

Both commands filter the result to insights whose `roles` contain the **session actor's** role
(server authority; the TS filter by `auth.role` then keeps the same list, `useInsights.ts:17`).

## 2. DTOs (`domains/dashboard/dto.rs`, `export_to = "core/types/gen/"`)

| Rust DTO | TS type | Notes |
|---|---|---|
| `InsightSeverity { Critical, Warning, Info, Positive }` lowercase | `it:11` | |
| `InsightIcon` | new `InsightIconKey` in `insightTypes.ts` | The 18 icon names used (`ir:1-20`, `ie:1`): `AlertTriangle, Banknote, CalendarRange, Clock, CreditCard, DatabaseBackup, FileWarning, Landmark, PackageX, Percent, PiggyBank, Repeat, RotateCcw, ScrollText, Sparkles, TimerOff, TrendingDown, UserX`, serialized verbatim. |
| `InsightDto` | `Insight` (`it:13-31`) minus the Vue `icon` | `id`, `ruleKey`, `severity`, `message`, `metric?`, `actionLabel`, `actionTo: RouteRef` (with `query`, 13 G-13), `icon: InsightIcon`, `roles: Vec<Role>` (`#[ts(type = "import('../../../users/types').Role[]")]`), `value: Decimal`, `createdAt: String` (today's key). |
| `Thresholds` (Rust-only, not exported) | `InsightThresholds` (`it:47-76`) | All 14 fields `Decimal`; defaults `it:78-93`; `Thresholds::load(conn)` = defaults overlaid by `settings.insight_thresholds` (unknown keys ignored). |

`contract.check.ts` (core): `Expect<Equals<Simplify<GenInsightDto>, Simplify<Omit<Insight, 'icon' |
'roles'> & { icon: InsightIconKey; roles: Role[] }>>>` with `// contract-ok: icon is a Vue component;
the service maps InsightIconKey → Component; roles is readonly in TS`.

## 3. Service logic (`domains/dashboard/service/insights/`)

### 3.0 Engine (`engine.rs`, port of `ie:103-116`)

1. `ctx = { today: clock.today(), now: clock.now, thresholds: Thresholds::load(txn), numerals }`.
2. Run the 21 rules **in catalogue order** (`ir:635-657`); each returns `Vec<InsightDto>`; a rule that
   returns an error is skipped and logged `log::warn!(target: "insights", "rule <key> failed: <err>")`
   ("a single bad rule shouldn't break the whole home screen", `ie:108-112`).
3. Keep insights whose `roles` contain the actor's role; return in that order (the TS side sorts by
   severity then value, `ie:134`, stable, so catalogue order breaks ties exactly as in the mock).

Helpers (`common.rs`): `days_between` and `day_of_key` (13 §3.0); `trunc_days(t)` = `t.trunc()` as
`i64` (JS `setDate` truncates a fractional day count); `acct_balance(role)` (14 §3.0);
`fmt_money`/`fmt_num(d, max)` = G-36 with `ctx.numerals`; `js_num` for raw `${}` numbers; `NEG_ZERO`
= the text the mock prints for `formatNumber(-0)`: `"-0"` (Latn) / `"-٠"` (Arab) (G-36 covers it).
Common fields: `createdAt = today key`.

### 3.1 The 21 rules (`rules/*.rs`, one fn each, exact strings)

1. **reorder** (`ir:52-80`): live products `active && type = product && stock_mode ≠ none && qty ≤ min ?? 0`,
   grouped by `preferred_supplier_id ?? "__none__"` (IndexMap). Per group: supplier = live supplier
   party or none; `label = supplier ? "لدى مورد \"{name}\"" : "بلا مورد مفضل"`; id `reorder:{key}`; warning;
   message `"{n} {n == 1 ? "صنف" : "أصناف"} عند حد الطلب {label}"`; metric `"{fmt_num(n)} صنف"`; action
   `"إنشاء أمر شراء"` → supplier ? `{ purchase-new, query { supplier: key } }` : `{ products, query { stock: "low" } }`;
   `PackageX`; roles storekeeper, manager, admin; value `n × 100`.
2. **dead-stock** (`ir:83-118`): `cutoff = today − trunc(deadStockDays)`; last sale per product =
   **max raw date key** over non-draft invoices' lines (`MAX(inv.date_key COLLATE utf8mb4_bin)`);
   stale = live products active, product, `stock_mode ≠ none`, `qty > 0`, `stock_value ≥ deadStockValue`,
   and (no sale or `day_of_key(last) < cutoff`); none → `[]`. `value = round2(sum2(stock_value))`; id
   `dead-stock:all`; warning; message `"{n} صنف بلا مبيعات منذ {js_num(deadStockDays)} يوماً — بضاعة راكدة
   بقيمة {fmt_money(value)}"`; metric `fmt_money(value)`; `"عرض القائمة"` → `{ products }`; `TrendingDown`;
   manager, admin; value.
3. **expiring** (`ir:121-143`): batches `qty > 0.0001` with an expiry date and `0 ≤ days_between(expiry, today)
   ≤ expiryAlertDays`; none → `[]`; id `expiring:all`; warning; message `"{n} {n == 1 ? "تشغيلة تنتهي" :
   "تشغيلات تنتهي"} خلال {js_num(expiryAlertDays)} يوماً"`; metric `"{fmt_num(n)} تشغيلة"`; `"تقرير الصلاحية"`
   → `{ expiry }`; `TimerOff`; storekeeper, manager, admin; value `n × 80`.
4. **overdue-customers** (`ir:146-177`): invoices where `isOverdue` (`invoiceService.ts:56-59`: status ≠
   REFUNDED — **drafts not excluded**, quirk Q-3 — due date set, outstanding > 0, due key < ISO(now));
   per customer (skip none): `days = max(0, days_between(today, due))`, `total = round2(total + outstanding)`,
   `maxDays = max`. For each (first-appearance order): live customer else skip; `total ≤ 0` skip; id
   `overdue-customer:{id}`, ruleKey `overdue-customers`; critical; message `"\"{name}\" متأخر {fmt_num(maxDays)}
   يوماً بمبلغ {fmt_money(total)}"`; metric `fmt_money(total)`; `"كشف الحساب"` → `{ customer, params { id } }`;
   `UserX`; accountant, manager, admin; value total.
5. **credit-limit** (`ir:180-204`): live customers with `credit_limit > 0`; `balance =
   shared::balances::customer_balance` (P2-26, one copy); skip `balance < limit × 0.9`; `over = balance >
   limit`; id `credit-limit:{id}`; severity over ? critical : warning; message over ? `"\"{name}\" تجاوز الحد
   الائتماني: الرصيد {fmt_money(balance)} من أصل {fmt_money(limit)}"` : `"\"{name}\" قريب من الحد الائتماني:
   {fmt_money(balance)} من أصل {fmt_money(limit)}"`; metric `fmt_money(balance)`; `"عرض العميل"` → `{ customer,
   params { id } }`; `CreditCard`; cashier, accountant, manager, admin; value balance.
6. **supplier-dues** (`ir:207-237`): `cutoff = today + trunc(supplierDueDays)`; over live suppliers' open POs
   (13 G-1), `day_of_key(due_date_key ?? date_key) ≤ cutoff` → `due = round2(due + outstanding)`; `due ≤ 0` →
   `[]`; `cash = round2(acct_balance(Cash) + acct_balance(Bank))`; `due < cash` → `[]`; id `supplier-dues:all`;
   warning; message `"مستحقات {fmt_money(due)} خلال {js_num(supplierDueDays)} أيام — السيولة المتاحة
   {fmt_money(cash)}"`; metric `fmt_money(due)`; `"التخطيط للسداد"` → `{ payments }`; `Landmark`; accountant,
   manager, admin; value due.
7. **vat-deadline** (`ir:240-263`): `periodEnd` = last day of the previous month; `deadline` = last day of
   the month after `periodEnd` (= the current month's last day); `daysLeft = −days_between(deadline, today)`
   (≤ 0). The mock fires only when `daysLeft` is `-0` (today is the month's last day), since any other value
   is `< 0` (quirk Q-1). Then: `quarter = floor(periodEnd.month0 / 3) + 1`; id `vat-deadline:{periodEnd.year}-q{quarter}`;
   warning; message `"إقرار الربع {quarter} مستحق خلال {NEG_ZERO} أيام"` (the `daysLeft === 1` branch is
   unreachable); metric `"{NEG_ZERO} يوم"`; `"تسوية الضريبة"` → `{ vat-settlement }`; `FileWarning`; accountant,
   manager, admin; value `(vatDeadlineDays + 1) × 1000`. Also returns `[]` when `vatDeadlineDays < 0`.
8. **cash-drawer** (`ir:266-284`): `cash = acct_balance(Cash)`; `cash ≤ cashDrawerLimit` → `[]`; id
   `cash-drawer:all`; warning; message `"النقدية في الصندوق {fmt_money(cash)} — أودِعها في البنك"`; metric;
   `"سند تحويل"` → `{ vouchers }`; `Banknote`; manager, admin, cashier; value cash.
9. **shift-open** (`ir:287-310`): `now` = **today at 00:00 UTC** (`new Date(ctx.today)`, quirk Q-2); per OPEN
   shift, `opened` = `opened_at_instant` or `opened_at_day` at 00:00 UTC; `hours = (now − opened) ms /
   3_600_000`; keep `hours ≥ shiftOpenHours`; `h = floor(hours)`; id `shift-open:{id}`; warning; message
   `"وردية {number} مفتوحة منذ {fmt_num(h)} ساعة"`; metric `"{fmt_num(h)} ساعة"`; `"إغلاق قسري"` →
   `{ pos-shifts }`; `Clock`; manager, admin; value `h`.
10. **unsettled-cards** (`ir:313-343`): `cutoff = today − trunc(unsettledClearingDays)`; `total =
    round2(SUM(debit))` of lines with `debit > 0` on live accounts whose `system_role ∈ {cardClearing,
    walletClearing}` and entry `date_day ≤ cutoff` (settlement credits are **not** netted, quirk Q-4);
    no such lines → `[]`; id `unsettled-cards:all`; info; message `"تحصيلات بطاقات/محافظ غير مسواة منذ أكثر
    من {js_num(unsettledClearingDays)} أيام بقيمة {fmt_money(total)}"`; metric; `"تسوية البطاقات"` →
    `{ card-settlements }`; `CreditCard`; accountant, admin; value total.
11. **below-cost** (`ir:346-369`): live products active, product, `price > 0`, and (`price ≤ cost` or
    `(price − cost)/price × 100 < minMarginPct`); none → `[]`; id `below-cost:all`; warning; message `"{n} صنف
    بسعر أقل من التكلفة أو بهامش ربح ضعيف"`; metric `"{fmt_num(n)} صنف"`; `"مراجعة الأسعار"` → `{ products }`;
    `TrendingDown`; manager, admin; value `n × 50`.
12. **discount-leak** (`ir:372-409`): `weekAgo = today − 7`; `week` = non-draft invoices with `day ≥ weekAgo`
    and `grand > 0`; `< 4` → `[]`; rates `discount_rate` grouped by cashier (skip empty); `avg = sum2(all
    rates) / count` (raw division); `avg ≤ 0` → `[]`; per cashier with ≥ 3 rates: `cAvg = sum2(rates) / n`;
    skip `cAvg < avg × discountLeakMultiplier`; id `discount-leak:{cashierId}`; warning; message
    `"متوسط خصم \"{user name or id}\" {fmt_num(cAvg, 1)}% مقابل {fmt_num(avg, 1)}% لبقية الكاشيرين هذا الأسبوع"`;
    metric `"{fmt_num(cAvg, 1)}%"`; `"تقرير المبيعات"` → `{ report-sales }`; `Percent`; manager, admin; value `cAvg` (raw).
13. **refund-spike** (`ir:412-441`): `weekAgo = today − 7`, `fourWeeksAgo = today − 28`; `this` = refunds
    `day ≥ weekAgo`; `prior` = `fourWeeksAgo ≤ day < weekAgo`; `this` empty → `[]`; `t = sum2(this grand)`;
    `p = prior non-empty ? sum2(prior grand) / 3 : 0`; `p ≤ 0 || t < p × refundSpikeMultiplier` → `[]`; id
    `refund-spike:all`; warning; message `"المرتجعات هذا الأسبوع {fmt_money(t)} — أعلى من المتوسط
    ({fmt_money(round2(p))})"`; metric `fmt_money(t)`; `"تقرير المبيعات"` → `{ report-sales }`; `RotateCcw`;
    manager, admin; value t.
14. **recurring-journal-due** (`ir:444-458`): live journal templates with a recurrence and `next_date ≤ today`;
    id `recurring-journal:{id}`; info; message `"قيد متكرر \"{name}\" مستحق الترحيل"`; no metric; `"ترحيل الآن"`
    → `{ journal-templates }`; `Repeat`; accountant, admin; value 200.
15. **recurring-expense-due** (`ir:460-475`): live recurring expenses `active && next_date ≤ today`; id
    `recurring-expense:{id}`; info; message `"مصروف متكرر \"{name}\" بمبلغ {fmt_money(amount)} مستحق التسجيل"`;
    metric `fmt_money(amount)`; `"تسجيل الآن"` → `{ expenses }`; `Repeat`; accountant, admin; value amount.
16. **budget** (`ir:478-509`): `fy` = first fiscal year (array order) not closed with `start ≤ today ≤ end`,
    else the first not closed; none → `[]`. Per live cost center with a budget row for `fy` and `amount > 0`:
    `actual = sum2(debit − credit)` of its lines on live EXPENSE accounts whose entry **key string** is
    `≥ start` and `≤ end` (computed in Rust from `DocDate::key()`; an instant on the last day is excluded,
    quirk Q-5); `pct = actual / amount × 100` raw; skip `pct < budgetNearPct`; id `budget:{id}`; severity
    `pct ≥ 100` critical else warning; message `"مركز التكلفة \"{name}\" تجاوز {fmt_num(pct, 0)}% من ميزانيته
    ({fmt_money(actual)} من {fmt_money(amount)})"`; metric `"{fmt_num(pct, 0)}%"`; `"تقرير مراكز التكلفة"` →
    `{ report-cost-centers }`; `PiggyBank`; manager, admin; value actual.
17. **opening-balance-equity** (`ir:512-530`): `b = acct_balance(OpeningBalanceEquity)`; `|b| < 0.01` → `[]`;
    id `opening-balance-equity:all`; info; message `"رصيد حساب \"أرصدة افتتاحية\" {fmt_money(b)} — يحتاج إقفال
    إلى رأس المال"`; metric; `"قيد يومية"` → `{ journal-new }`; `ScrollText`; accountant, admin; value `|b|`.
18. **backup-overdue** (`ir:533-553`): `last = settings.backup.last_backup_at` (decision I-4); `since = last ?
    days_between(today, local_date_key(last)) : ∞`; `since < backupOverdueDays` → `[]`; id `backup-overdue:all`;
    critical; message `last ? "لم يتم أخذ نسخة احتياطية منذ {fmt_num(since)} يوماً" : "لم يتم أخذ أي نسخة احتياطية بعد"`;
    metric `last ? "{fmt_num(since)} يوم" : omitted`; `"أخذ نسخة الآن"` → `{ settings-backup }`; `DatabaseBackup`;
    admin; value `last ? since : 9999`.
19. **year-end** (`ir:556-578`): per fiscal year (array order) not closed: `d = −days_between(end, today)`
    (= `today − end`, sign inverted — quirk Q-1); skip `d < −365 || d > yearEndDays`; `ended = d < 0`
    (a `-0` is not `< 0`); id `year-end:{id}`; severity ended ? critical : warning; message ended ?
    `"السنة المالية \"{name}\" انتهت ولم تُقفل بعد"` : `"السنة المالية \"{name}\" تنتهي خلال {fmt_num(d)} يوماً"`
    (`d == 0` prints `NEG_ZERO`); metric ended ? omitted : `"{fmt_num(d)} يوم"`; `"إغلاق السنة"` →
    `{ fiscal-years }`; `CalendarRange`; accountant, admin; value ended ? 5000 : `yearEndDays − d`.
20. **good-news** (`ir:581-609`): per-day `round2(total + grand)` over all non-draft invoices (IndexMap by day
    key); no entry for today or `today ≤ 0` → `[]`; `best = max(0, other days)`; `today ≤ best` → `[]`; id
    `good-news:{today}`; positive; message `"اليوم أفضل يوم مبيعات مسجل بقيمة {fmt_money(today)} 🎉"`;
    metric; `"عرض المبيعات"` → `{ invoices }`; `Sparkles`; manager, admin; value.
21. **missing-supplier-invoice** (`ir:615-633`): POs `RECEIVED` with `supplier_invoice_no` NULL or `""`; none →
    `[]`; id `missing-supplier-invoice:all`; info; message `"{n} أمر شراء مستلم بلا رقم فاتورة مورد"`; metric
    `"{fmt_num(n)} أمر"`; `"عرض المشتريات"` → `{ purchases }`; `AlertTriangle`; accountant, manager, admin; value `n × 20`.

### 3.2 `dashboard_get_product_inline_hints` (`ie:164-225`)

1. Live product by id; missing, inactive or not `type = product` → `[]` (no error, `ie:166`).
2. `reorder-item` when `stock_mode ≠ none && qty ≤ min ?? 0`: warning, `"هذا المنتج منخفض المخزون — عند حد
   الطلب أو أقل"`, `"إنشاء أمر شراء"` → `{ purchase-new }`, `PackageX`, storekeeper/manager/admin, value 1.
3. `below-cost-item` when `price > 0 && stock_mode ≠ none`: `margin = price ≤ cost ? −1 : (price − cost)/price
   × 100`; if `margin < minMarginPct`: warning, message `margin < 0 ? "سعر البيع أقل من التكلفة" : "هامش الربح
   ضعيف ({round0(margin)}%)"` (`Math.round`, margin ≥ 0 here, half up), `"تعديل السعر"` → `{ product-edit,
   params { id } }`, `TrendingDown`, manager/admin, value 1.
4. `dead-stock-item`: `last` = max raw invoice date key over non-draft invoices with a line for this
   product; `cutoff = today − trunc(deadStockDays)`; when `qty > 0 && stock_value ≥ deadStockValue && (no last
   || day_of_key(last) < cutoff)`: info, message `"لم يُبع منذ {last ? floor((now − instant(last)) / 1 day) :
   "+" + js_num(deadStockDays)} يوماً — قيمة المخزون {round0(stock_value)} ر.س"` (`instant(key)` = the instant,
   or the day at 00:00 UTC), `"عرض المخزون"` → `{ movements }`, `TrendingDown`, manager/admin, value 1.
5. Role filter (actor). No formatter here, so no `numerals` arg.

## 4. Concurrency (D8)

Read-only snapshot per call (AC§5). Thresholds are written by 01-settings' `settings_update_settings`
(its own lock rules); a threshold change takes effect at the next recompute. Insight freshness
across terminals comes from the mirror clearing on `backend:changed` (G-37), as the mock's cache clears on
the three events (`ie:101`).

## 5. Undo

Not applicable: no writes in this file (the thresholds write belongs to 01-settings and is not
undoable, AC§4).

## 6. Frontend switch lines

`src/modules/core/services/insightEngine.ts`:

- `computeAll()` gets a Rust branch used by `getInsights` and `getInsightsFor`:
  `if (usesRust('dashboard')) return mirrored(\`insights:${role ?? ''}:${numeralSystem.value}\`, async () => (await backendCall('dashboard_compute_insights', { numerals: numeralSystem.value })).map(toInsight), []);`
  — `computeAll(role?)` takes the caller's role only to key the mirror; `getInsights` passes `opts.role`,
  `getInsightsFor` passes `role`. Everything after it (role/ruleKey filter, `isHidden`, sort, `limit`,
  predicates) is unchanged TS.
- `getProductInlineHints(role, productId)`: `if (usesRust('dashboard')) return mirrored(\`productHints:${role ?? ''}:${productId}\`, async () => (await backendCall('dashboard_get_product_inline_hints', { productId })).map(toInsight), []);`
- `toInsight(dto) = { ...dto, icon: INSIGHT_ICONS[dto.icon] }`; `INSIGHT_ICONS: Record<InsightIconKey, Component>`
  is exported from `insightRules.ts` (it already imports those icons) and `InsightIconKey` from `insightTypes.ts`.
- `forceRefresh()` and `setThresholds()` also call `clearMirrors('insights:')` and `clearMirrors('productHints:')`.
- Thresholds (I-2), switched on `usesRust('settings')`: `getThresholds()` → `{ ...DEFAULT_THRESHOLDS,
  ...(useSettingsStore().settings?.insightThresholds ?? {}) }` (the store is loaded by the router guard
  before any page, `src/router/index.ts:75`); `setThresholds(patch)` → `const next = { ...getThresholds(),
  ...patch }; await useSettingsStore().update({ insightThresholds: next }); clearMirrors(…); return getThresholds();`.
- **Contract fix I-3 (mock too):** `setThresholds` becomes `async function … : Promise<InsightThresholds>`
  and `RecommendationsSettingsPage.vue:53` becomes `await setThresholds(patch);` — the only page edit in
  this file (one word, so a failed save reaches the page's existing `catch` instead of showing
  "تم الحفظ"). Coordinate with the 01-settings implementer (same module).

## 7. Known mock quirks (kept) and decisions

**Quirks:**
- **Q-1** Sign inversion in two rules: `vat-deadline` (`ir:245`) fires only on the last day of each month
  and prints `-0`; `year-end` (`ir:560`) calls a **future** year end "ended" and a past one "ends in N days".
  Kept literally; a later fix should drop the `* -1` in both.
- **Q-2** `shift-open` measures hours from today's **UTC midnight**, not the current time (`ir:288`).
- **Q-3** `overdue-customers` does not exclude draft invoices (`isOverdue`, `invoiceService.ts:57`).
- **Q-4** `unsettled-cards` sums every old clearing debit, settled or not (`ir:317-327`).
- **Q-5** `budget` compares raw date keys, so an entry stamped with an instant on the fiscal year's last
  day is left out (`ir:486`).
- **Q-6** Messages say `ر.س` whatever the currency (`ie:214`).

**Decisions (strictest option, logged):**
- **I-1** Only the **rule evaluation** (`computeAll`) is a Rust command. Dismiss/snooze is per-device
  `localStorage` (D9, AC§1b) and `getInsightsFor` takes a function (AC§9), so filtering stays in TS over
  the Rust list. Sorting/filtering a list is not business logic; the numbers are all computed in Rust.
- **I-2** Thresholds have no command of their own: they are `StoreSettings.insightThresholds`, a branch
  field (cross-cutting §3) that 01-settings already reads and writes. A second path would be a second copy.
  Consequence for Part 04: `dashboard` may flip only **after** `settings`.
- **I-3** `setThresholds` becomes async (a write must be awaitable to surface failures; zero data loss).
- **I-4** `backup-overdue` reads `settings.backup.last_backup_at` from the DB, not the UI's backup store:
  the mock's value is empty until the backup page has been opened once (`useBackupStore.load`), which is UI
  state, not data. Parity cases load the backup store first.
- **I-5** Rust filters by the **session** role (server authority); the TS role arg only keys the mirror.

## 8. Tests

**(a) `src-tauri/tests/domain_dashboard.rs`** (same file as 14):
- one test per rule: firing and non-firing fixture, exact `id`/`message`/`metric`/`actionTo`/`value`
  for both numeral systems where the rule formats numbers;
- `vat-deadline` fires on the month's last day with `-0` text, not the day before (Q-1);
- `year-end` future/past cases as in Q-1;
- a rule that fails (no `openingBalanceEquity` account) does not stop the other rules;
- role filtering: a cashier session gets only rules listing `cashier`;
- thresholds: sparse `insight_thresholds` overlay, unknown key ignored, fractional day threshold truncated;
- product hints: each of the three hints; inactive product → `[]`; role filter.

**(b) Parity cases:** seeded demo data, clock pinned, both numeral systems, for `dashboard_compute_insights`
vs the mock's `computeAll()` (roles filtered the same way on both sides); product hints for three
products (low stock, below cost, dead stock). Epsilon `1e-9` on `InsightDto.value` for `discount-leak`
(raw division). Backup store loaded before the mock run (I-4).

## 9. Checklist

- [x] G-36, G-37, G-38 and G-13/G-1 merged; 01-settings round-trips `insightThresholds` (a `JsonDecimal` map).
- [x] `service/insights/{engine,common}.rs` + `rules/{stock,receivables,cash,accounting,sales}.rs` (21 fns) + `hints.rs`.
- [x] `InsightDto`, `InsightSeverity`, `InsightIcon` in `dto.rs` (+ `export_bindings`); 2 commands + `ipc_sig!`.
- [x] `InsightIconKey` + `INSIGHT_ICONS`; `computeAll(role?)` Rust branch; `getProductInlineHints` branch; mirror clears.
- [x] Thresholds routed through the settings store (I-2) and `setThresholds` async + the one-word page edit (I-3).
- [x] `contract.check.ts` entry for `InsightDto`.
- [x] Tests §8(a) written (in `domain_analytics.rs`, shared with 14) — not run; parity cases §8(b) handed to Part 04.
- [x] Status notes on 14 and 14b.

## Gate

`cargo check` clean (manager's run after W6); tests and parity cases written (deferred run); switch
lines present; `contract.check.ts` compiles; `bun run build` green with the async `setThresholds`;
`bun run memory:check`: both commands invoked + registered, 0 contract gaps.
