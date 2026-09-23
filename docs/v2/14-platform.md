# 14 — Platform: Command Palette, Appearance, Backup, Attachments, Persistence, Speed

## 1. Mock persistence (prerequisite for most of v2)

v1 re-seeds on every reload, which makes onboarding, restores and attachments pointless.

- **Save:** `src/mocks/persist.ts` snapshots the DB tables to **IndexedDB** 500 ms after every
  mutating service call (services wrap writes in `mutate()`). Attachment blobs go to a separate
  object store.
- **Boot:**
  - Snapshot found → load it. It's run through `migrations[schemaVersion]` if older.
  - No snapshot → the welcome screen ([05 §1](05-onboarding.md)): *fresh* or *demo*.
- **Dev menu:** "إعادة تعيين البيانات" (reset data), "تحميل البيانات التجريبية" (load demo data), and
  a latency switch (0 / realistic / slow) for testing loading states.
- **Unchanged:** services still `clone()` results and throw `ApiError`. Nothing in `modules/` knows
  that persistence exists.

## 2. Command palette (Ctrl+K)

Opened with **Ctrl+K**, or the search pill in the topbar ("ابحث أو نفّذ أمراً… Ctrl K").

- **Groups:** الأوامر (actions) · الصفحات (pages) · العملاء (customers) · الموردون (suppliers) ·
  المنتجات (products) · الفواتير (invoices) · المشتريات (purchases) · السندات (vouchers) · القيود
  (journal) · الحسابات (accounts) · التقارير (reports) · الإعدادات (settings). Each group shows 5
  results, plus a "عرض كل النتائج" (see all) row that opens the list page with the query.
- **Prefixes:** `>` actions only · `@` customers/suppliers · `#` document numbers (`#INV-42`,
  `#JE-310`) · `$` accounts by code or name · `?` help.
  - **Barcode:** an 8–14 digit string is looked up as a barcode first.
- **Actions**, for example:
  - New invoice, POS, new customer, receive payment, new expense, new journal entry, stock-in,
    transfer, print labels.
  - Open/close shift, switch branch, toggle theme, back up now, lock the screen, switch user.
- **Context:** commands can declare `when(route)`. On an invoice page, "طباعة هذه الفاتورة" (print
  this invoice) and "إنشاء إشعار دائن" (create credit note) appear first.
- **Recents:** the last 8 opened items (per user) show when the input is empty.
- **Permissions:** every command and provider declares `area` + `access`; nothing the user can't
  open is listed.
- **Implementation:**
  - `modules/core/controllers/useCommandPalette.ts` holds the registry. Each module adds a
    `commands.ts` exporting `commands[]` and `searchProviders[]`:
    `{ id, group, search(q, signal) → Promise<Result[]> }`. Providers call the existing services
    with `{ search, limit: 5 }`.
  - Searches are debounced (120 ms) and cancelled with `AbortSignal`.
  - Ranking: exact number match > prefix > word prefix > contains; recent items are boosted.
- **Arabic search normalization** (`core/helpers/search.ts`, used by *every* search box in the app):
  - Removes tashkeel and tatweel.
  - Unifies أ/إ/آ→ا, ة→ه, ى→ي, ؤ→و, ئ→ي.
  - Converts Arabic-Indic digits to Latin, and lower-cases Latin text.
  - So "احمد" finds "أحمد" and "٤٢" finds "INV-42".
- **Accessibility:** a combobox/listbox with `aria-activedescendant`; ↑↓ Enter Esc; Tab moves
  between groups.

## 3. Appearance & system preferences (Settings → المظهر، per user)

| Setting | Options | Notes |
|---------|---------|-------|
| الثيم (theme) | فاتح / داكن / النظام (light / dark / system) | exists |
| الأرقام (numerals) | 123 / ١٢٣ | exists |
| نوع الخط (font) | Cairo · IBM Plex Sans Arabic · Tajawal · Noto Naskh Arabic | bundled with `@fontsource` (offline); the CSS for the chosen font is lazy-loaded |
| حجم الخط (text size) | 90% · 100% · 110% · 120% · 130% | text only; see below |
| الكثافة (density) | مريحة / مضغوطة (comfortable / compact) | row height and padding tokens |
| لون التمييز (accent) | 4 presets | each has light and dark tokens checked for contrast |
| التاريخ (date) | dd/mm/yyyy · yyyy-mm-dd; **show Hijri alongside** | Hijri uses `Intl` `islamic-umalqura` |
| بداية الأسبوع (week start) | السبت / الأحد / الإثنين (Sat / Sun / Mon) | date pickers, weekly reports |
| الجداول (tables) | rows per page 25/50/100; zebra rows | |
| الحركة (motion) | reduce motion | also follows the OS setting |
| القائمة الجانبية (sidebar) | collapsed by default | |

**Text size needs a real refactor:**
- **The problem:** there are 167 hard-coded sizes (`text-[13px]` ×105, `text-[11px]` ×42, …) in 55
  files, plus `font-size: 14px` in `design-system.css`.
- **Tokens:** add text tokens to `@theme` with a scale variable, for example:
  ```css
  --text-caption: calc(11px * var(--font-scale));  /* was text-[11px] */
  --text-body:    calc(13px * var(--font-scale));  /* was text-[13px] */
  --text-ui:      calc(14px * var(--font-scale));  /* body default */
  --text-lead:    calc(15px * var(--font-scale));
  ```
- **Codemod:** replace the px utilities with the token classes (`text-body`, …), and add a lint rule
  (a grep check in `bun run check`) against new `text-[Npx]`.
- **Result:** spacing stays in `rem`, so text grows without the layout zooming. Density handles
  spacing separately.

**Company-level settings** (admin) stay in their own sections: company, taxes, branches, currencies,
payment methods, numbering, templates, printers, features, recommendations thresholds, backup.

## 4. Backup & restore (Settings → النسخ الاحتياطي)

**File:** `backup-<company>-YYYYMMDD-HHmm.zip`, built with `fflate`. It contains:
- `manifest.json` (see `BackupManifest` in [04](04-domain-model.md)).
- `data.json`: every table plus settings, templates and user preferences.
- `attachments/`: every attachment blob.

With a password, everything except the manifest is encrypted with AES-GCM (WebCrypto, PBKDF2).

| Feature | Details |
|---------|---------|
| **نسخة الآن** (back up now) | Save dialog (Tauri) or a download (browser). Shows size and counts |
| **نسخ تلقائي** (automatic backup) | Daily at a set time **and** on app close. The folder is chosen once; retention keeps the last N (default 14). In the browser: the last 5 snapshots in IndexedDB |
| **السجل** (history) | A list of backups in the folder: date, size, kind (manual / auto / pre-restore), and a verify button (checksum) |
| **الاستعادة** (restore) | Admin with `settings.restoreBackup`. Pick a file → the manifest preview (company, date, counts, version) → a compatibility check (schema version + migrations) → **an automatic pre-restore backup** → typed confirmation ("استعادة") → replace the data → reload |
| **الحالة** (status) | Last successful backup is shown in Settings and the user menu; the backup insight appears after 7 days without one ([11 D2](11-journal-dashboard-insights.md)) |

**Tauri capabilities:** binary write/read, read dir and remove, scoped to the chosen folder.

## 5. Attachments & media

- **`AttachmentField` component:**
  - **Adding files:** a drop zone, a file picker and **paste from the clipboard**.
  - **Types and size:** images, PDF and Office files; max size 10 MB (setting).
  - **Images:** resized in the browser to 2000 px and saved as WebP at 0.85 quality, which keeps
    backups small. A thumbnail is made too.
  - **Viewer:** a lightbox (zoom, rotate) for images; PDFs inline (a blob iframe), otherwise opened
    in the system viewer.
- **Where it appears:**
  - Accounting and payments: journal entries, expenses, vouchers, payments.
  - Purchasing and parties: purchases (the supplier invoice scan), customers and suppliers.
  - Stock: products (the image gallery), stock adjustments, transfers, counts.
  - Company settings: logo, stamp, signature.
- **Audit rule:** on posted accounting documents, attachments can be **added** but only **removed by
  an admin**, and the removal is logged.
- **Media library `/files`:** a grid or list of all files; filters for type, owner kind, date and
  user; search by name; a link to the owner; bulk download as a .zip.

## 6. Notifications & approvals

- **Bell drawer:** insights (by severity) + events:
  - A transfer arrived.
  - An approval is requested.
  - An automatic backup failed.
  - A recurring entry is due.
- **Approvals page (`/approvals`)** for managers:
  - Discount-over-limit requests (when approval by PIN isn't used), write-offs above the threshold,
    price-below-cost sales.
  - Each can be approved or rejected with a comment. Everything is logged.

## 7. Speed checklist

- **Paged lists:** big lists (invoices, journal, movements, payments) use **paged service queries**
  `{ page, pageSize, sort, filters }` → `{ rows, total, totals }`. The mock implements this now, so
  the UI is ready for years of data. Tables with more than 200 rows on screen are virtualized.
- **Lazy loading:** heavy libraries are lazy-loaded (`exceljs`, `bwip-js`, `libphonenumber` metadata,
  extra fonts, the report pages). Only the POS and the dashboard are in the initial bundle.
- **Caching:** master-data caches in Pinia (products + barcode index, parties, accounts, taxes,
  payment methods) are invalidated by `catalog:changed` / `ledger:changed` events. No page refetches
  master data on every visit.
- **Measurement:** a dev overlay shows `performance.mark` timings for scan→cart, checkout→receipt and
  route→first data. The targets are in [06 §1](06-sales-and-pos.md).
- **Keyboard help:** **F1 / ?** opens the keyboard-shortcuts sheet for the current page.
