# 11 — Journal Redesign, Simpler Home, Analytics & Recommendations

## Part A — Journal entries (القيود اليومية)

### A1. List (`/accounting/journal`)

v1 has: type tabs (all/auto/manual), search, a date range and a flat table.

**v2:**
- **Grouping:** entries are grouped by day with sticky day headers and daily Dr/Cr totals.
- **Inline preview:** each row expands (click or →) to show its lines in place: account, party, cost
  center, debit, credit. Checking many entries no longer means opening each one.
- **Row contents:**
  - Number, date, description.
  - A **source chip**, e.g. "فاتورة INV-00042", which links to the document.
  - Type badge, amount, attachment 📎 count, user.
- **Filters:**
  - Types: system / manual / opening / closing / reversal / recurring / VAT settlement.
  - Also: source document type, **account** (includes children), party, branch, cost center,
    amount range, user, status (draft/posted), *has attachments*, *reversed*.
- **Saved views** (per user) and "مسودات بحاجة لترحيل" (drafts to post) as a default view for
  accountants.
- **Footer:** totals for the filtered entries. Export to Excel (entries, or entries with lines) and a
  printed day book (دفتر اليومية) as a PDF.
- **Keyboard:** J/K moves between rows, Enter opens, N creates an entry.

### A2. Entry form (`/accounting/journal/new`)

- **Header:** date (checks the period lock live), description, reference, branch.
  - **Template:** load one (rent, salaries, depreciation…).
  - **Currency** (optional): FC amounts, with the base amounts calculated.
- **Grid** (spreadsheet behavior):
  - Columns: account (search by code or name; leaf and `allowManual` accounts only), description
    (defaults to the header), debit, credit, **party** (required and shown when the account is AR/AP),
    cost center, branch.
  - Enter moves to the next cell. Typing in debit clears credit.
  - **"=" in an amount cell fills the remaining difference**, the classic balancing shortcut.
  - Ctrl+D duplicates a line. Paste from Excel.
- **Balance bar** (sticky): Σ debit, Σ credit, difference. The post button is disabled until the
  difference is 0.
- **Attachments:** a drop zone plus paste from the clipboard (e.g. a screenshot of a bank transfer).
  Thumbnails with a preview.
- **Actions:** save draft, post, post & new, save as template (optionally recurring), duplicate from
  an existing entry.
- **Validation:** Arabic errors per line (review B1/B2), e.g. "حساب العملاء يتطلب اختيار عميل"
  (the receivables account needs a customer).

### A3. Entry detail

- **Header:** number, type and status badges, date, branch, source-document link.
- **Lines:** each account with its path, party, cost center and FC amounts.
- **Attachments:** an inline viewer (images, PDF).
- **Audit trail:** created, posted and reversed, with who, when and why.
- **Actions:** print voucher (PDF), duplicate, **reverse** (a dialog with date + reason), edit (drafts
  only).
- **Neighbors:** "القيود المرتبطة" (related entries) lists the reversal pair and the other entries
  from the same document, e.g. an invoice and its credit note.

### A4. Other accounting screens touched

- **Fiscal years:** the close-year wizard (review B2) and the lock date setting.
- **VAT settlement:** "تسوية ضريبة القيمة المضافة" (VAT settlement) for a period. It shows output,
  input and the net, posts the settlement entry, then offers "سداد" (pay), which creates a payment to
  the tax authority.
- **Recurring entries:** a list of templates with the next run date. Due entries show as insights.

---

## Part B — Home (نظرة عامة): simpler

**Principle:** answer three questions in this order: *what needs me → how are we doing → what's
the trend*. Everything else moves to التحليلات (analytics) or to its own module.

**Owner/manager home** (top to bottom):
1. **Header row:** greeting and date; branch switcher; period (اليوم / هذا الأسبوع / هذا الشهر).
2. **"يحتاج انتباهك"** (needs your attention): the top 5 recommendations (Part D), as cards with a
   severity icon, one sentence, a metric and **one action button**. "عرض الكل" (see all) opens the
   insights drawer.
   - **Nothing to show:** a positive empty state: "كل شيء على ما يرام ✓" (everything is fine).
3. **4 KPIs**, each with the change vs the previous period (▲ 12%) and a tiny sparkline:
   - المبيعات (net sales)
   - مجمل الربح (gross profit and margin %)
   - السيولة (cash + banks, including clearing)
   - مستحق من العملاء (receivables, with the overdue part in red text)
4. **One chart:** sales per day for the period, with the **previous period as a light line** for
   comparison. It follows the dataviz rules from v1 (single hue, hover tooltip, sr-only table).
5. **Two compact lists side by side:** top 5 products (by gross profit, not revenue) · top 5 customers.

**Removed from home:**
- **Activity feed:** moves to the bell / notifications drawer.
- **Recent invoices:** already on the invoices page.
- **Low-stock table:** becomes an insight card with an action.

**Role homes** (same components, different content):

| Role | Home |
|------|------|
| Cashier | The POS itself; the **shift panel** (open / X-report / close) is in the POS header |
| Storekeeper | Insight cards (receiving to do, incoming transfers, low stock, expiring soon, open counts) + a stock value KPI + quick actions (stock-in, count, labels) |
| Accountant | Insight cards + cash & banks (per account) + receivables/payables aging bars + VAT period box (output, input, net, days to deadline) + drafts count |
| Owner/manager | As above |

## Part C — Analytics (`/analytics`, التحليلات)

Depth lives here. The page has tabs, and every chart has a **one-sentence plain-Arabic insight**
under its title, e.g. "الخميس هو أعلى يوم مبيعاً بمتوسط 4,210 ر.س — أعلى بـ 32% من بقية الأيام"
(Thursday is the best sales day, averaging 4,210 SAR, 32% above the other days).

| Tab | Charts |
|-----|--------|
| المبيعات (sales) | Trend with comparison; sales by weekday × hour heatmap; payment-method mix; average invoice & items per invoice; returns rate |
| المنتجات (products) | Top/bottom by profit; category mix; margin by category; slow movers (no sale in N days) with stock value |
| العملاء (customers) | New vs returning; top customers; revenue concentration (top 10 = x%); average days to pay |
| الفروع (branches) | Side-by-side KPIs; sales per branch trend; stock value per branch |
| الربحية (profitability) | Gross → net waterfall (sales, returns, COGS, variance/write-offs, expenses by group, net); expense mix by category |

**Filters:** period with comparison, branch, category. Every chart can be exported as PNG or its
data as Excel.

## Part D — Recommendations engine (التوصيات)

### D1. Design

- **Rules:** each rule is a pure function in the mock backend (`src/mocks/insights/rules/*.ts`):
  `(db, ctx: { userRole, branchId, today, settings }) → Insight[]`.
- **Service:** `insightService.getInsights({ branchId })` runs the rules the user's role can see,
  sorts them by severity then value at stake, and applies the user's dismissals and snoozes.
- **Text:** each insight has a stable key (rule + entity), so dismissing it hides that instance only.
  Text is rule-generated Arabic with real numbers, not generic advice.
- **Action:** every insight has a **primary action** that deep-links to a *pre-filled* screen, e.g.
  the purchase draft with the low-stock items of supplier X already added.
- **Thresholds:** configurable in Settings → Recommendations (days overdue, dead-stock days,
  cash-in-drawer limit…), with sensible defaults.
- **Where insights appear:**
  - Home "يحتاج انتباهك" and the insights drawer (bell).
  - A **report insights box** on reports, following the reference `ReportInsights` shape:
    headline, metrics, recommendations.
  - **Inline hints** on entity pages, e.g. on a product: "لم يُبع منذ 64 يوماً — قيمة المخزون 1,840
    ر.س" (not sold for 64 days; stock value 1,840 SAR).

### D2. Rule catalogue (first set)

| Rule | Who sees it | Trigger → message → action |
|------|-------------|----------------------------|
| Reorder | storekeeper, manager | Qty ≤ reorder point → "7 أصناف عند حد الطلب لدى مورد ‹X›" (7 items at reorder point with supplier X) → **create purchase draft** grouped by preferred supplier |
| Dead stock | manager | No sales in 60 days and value > threshold → "بضاعة راكدة بقيمة 12,400" (slow stock worth 12,400) → **see list / create discount price list** |
| Expiring | storekeeper, manager | Batch expiring ≤ alert days → "5 تشغيلات تنتهي خلال 30 يوماً" (5 batches expire within 30 days) → **return to supplier / write off / discount** |
| Overdue customers | accountant, manager | Invoices past due → "‹عميل› متأخر 45 يوماً بمبلغ 3,200" (customer 45 days overdue, 3,200) → **statement + WhatsApp** / **receive payment** |
| Credit limit | cashier (at POS), accountant | Balance > limit → blocks with an override prompt; home card lists customers over their limit |
| Supplier dues | accountant, manager | Supplier invoices due in ≤ 7 days vs available cash → "مستحقات 18,000 هذا الأسبوع — السيولة 11,500" (18,000 due this week; cash 11,500) → **plan payments** |
| VAT deadline | accountant, manager | ≤ 10 days before the return is due (end of the month after the period) → "إقرار الربع الثالث مستحق خلال 8 أيام" (Q3 return due in 8 days) → **open VAT report** |
| Cash in drawer | manager, cashier | Drawer cash > limit → "النقدية في الصندوق 14,300 — أودِعها في البنك" (drawer cash 14,300; deposit it) → **transfer voucher** |
| Shift not closed | manager | A shift has been open > 14 h or since yesterday → **force close** |
| Unsettled cards | accountant | Card clearing older than 3 days → **card settlement** |
| Below-cost prices | manager | Price ≤ average cost, or margin < 5% → **edit prices** |
| Discount leak | manager | A cashier's discount rate is > 2× the average this week → **see sales by cashier** |
| Refund spike | manager | Returns this week > 2× the average → **returns report** |
| Recurring due | accountant | A recurring expense or entry is due → **post now** |
| Budget | manager | A cost center passes 90% of its budget → **budget report** |
| Opening balance equity | accountant | 3900 ≠ 0 after go-live → **close to capital** |
| Backup | admin | Last backup > 7 days ago, or never → **back up now** |
| Setup | admin | Checklist incomplete → **continue setup** |
| Year end | accountant | ≤ 30 days to year end, or the year ended and isn't closed → **close year wizard** |
| Good news (positive) | manager | Best sales day or week on record, or a margin improvement → no action, just the card |

**Performance:** the rules run on the mock data in < 30 ms for the demo seed. Results are cached per
(user, branch) and recomputed after any posting (the services emit a `ledger:changed` event).
