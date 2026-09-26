# AGENT_MEMORY

> **Generated** by `bun run memory` (scripts/memory). Do not edit by hand — re-run after structural changes
> (new module, service, route, Rust command, mock file, or moved folders). `bun run memory:check` fails when stale.

Indexed: **721 files / 74,511 lines** (json 3, md 47, rust 22, ts 220, vue 429).

**Lookup order:** Where-to-find → Open diagnostics → Domain map → Service API → Routes → IPC → Mock map. Only grep when this file has no answer.

## Where to find X

| Concern | Path |
|---|---|
| App bootstrap | `src/main.ts` |
| Router (module route aggregation, guards) | `src/router/index.ts` |
| Route meta typing | `src/router/route-meta.d.ts` |
| Typed route names (generated) | `src/router/route-map.gen.d.ts` |
| Sidebar navigation groups | `src/modules/core/helpers/navigation.ts` |
| Brand / product name | `src/modules/core/helpers/brand.ts` |
| Formatting (money, dates, numbers) | `src/modules/core/helpers/format.ts` |
| Design tokens | `src/assets/styles/design-system.css` |
| Theme / appearance controllers | `src/modules/core/controllers/useAppearance.ts` |
| Native save dialog helper | `src/modules/core/services/saveFile.ts` |
| PDF service (→ Rust render_pdf) | `src/modules/core/services/pdfService.ts` |
| Print service (→ Rust thermal print) | `src/modules/core/services/printService.ts` |
| Command palette | `src/modules/core/commandPalette` |
| Dev UI gallery | `src/modules/core/pages/DevUiPage.vue` |
| Mock DB + persistence | `src/mocks/db.ts` |
| Mock posting engine | `src/mocks/backend` |
| Mock seed data | `src/mocks/seed` |
| Rust command registry | `src-tauri/src/lib.rs` |
| Typst templates | `src-tauri/templates` |
| Accounting invariants (verify:mocks) | `scripts/verify/run.ts` |
| e2e runner | `scripts/e2e/run.py` |
| UI guard scripts | `scripts/check-rtl.js` |
| Design system doc | `docs/design_system.md` |
| Posting rules | `docs/v2/02-accounting-review.md` |

## Open diagnostics (docs/diagnostics — 18.G)

Known failures not yet fixed — check before starting work in an affected area. Full ledger: `docs/diagnostics/ISSUES.md` (regenerate with `bun run diag`; `bun run diag:check` is part of the definition of done).

**By kind:** | Kind | Open count |
|---|---|
| خلل | 6 |

**By area:** `branches-currencies (1)`, `full-persona-pass (1)`, `onboarding (1)`, `purchases (1)`, `reports-v2 (1)`, `setup-wizard-eg (1)`



| ID | Kind | Area | Status | Occurrences | Last seen | Debug namespace | File |
|---|---|---|---|---|---|---|---|
| `BUG-0001` | خلل | onboarding | مفتوح | 2 | 2026-09-26 |  | `docs/diagnostics/issues/BUG-0001-onboarding-flow-crashed-unhandled-except.md` |
| `BUG-0002` | خلل | setup-wizard-eg | مفتوح | 1 | 2026-09-26 |  | `docs/diagnostics/issues/BUG-0002-setup-wizard-eg-flow-crashed-unhandled-e.md` |
| `BUG-0004` | خلل | purchases | مفتوح | 2 | 2026-09-26 |  | `docs/diagnostics/issues/BUG-0004-purchases-flow-crashed-unhandled-excepti.md` |
| `BUG-0005` | خلل | branches-currencies | مفتوح | 2 | 2026-09-26 |  | `docs/diagnostics/issues/BUG-0005-branches-currencies-flow-failed-an-asser.md` |
| `BUG-0006` | خلل | reports-v2 | مفتوح | 1 | 2026-09-26 |  | `docs/diagnostics/issues/BUG-0006-reports-v2-flow-crashed-unhandled-except.md` |
| `BUG-0007` | خلل | full-persona-pass | مفتوح | 1 | 2026-09-26 |  | `docs/diagnostics/issues/BUG-0007-full-persona-pass-flow-failed-an-asserti.md` |

## Architecture (layers & data flow)

```text
pages (112) / components (353) / controllers (23)   src/modules/<domain>/…
        │  may call ONLY ▼                  (seam rule — see Boundary report)
services (39)   src/modules/<domain>/services/*   ← swap point for a real backend
   │                                   │
   ▼                                   ▼
mock backend  src/mocks/     Tauri IPC invoke('<cmd>') → src-tauri/src/lib.rs
(db, persist, events, seed,           (PDF/Typst render, thermal printing)
 backend/* = accounting engine)       + plugins fs / dialog / opener / sql
```

Module anatomy: `pages` (routed screens) · `components` (feature-only UI) · `controllers` (composables / Pinia stores) · `services` (async API, the seam) · `helpers` (pure logic) · `routes` (route records + meta) · `types` · `validators` (Zod) · `commands.ts` (command-palette registration).

## Domain map

| Module | Files / lines | Layers (file count) | Routes | Palette |
|---|---|---|---|---|
| **accounting** | 12 / 3370 | commands 1, components 1, pages 7, routes 1, services 1, types 1 | 8 | yes |
| **analytics** | 8 / 467 | components 5, pages 1, routes 1, services 1 | 1 |  |
| **approvals** | 5 / 250 | commands 1, pages 1, routes 1, services 1, types 1 | 1 | yes |
| **core** | 348 / 19528 | commandPalette 1, components 295, controllers 14, data 2, helpers 16, pages 4, routes 1, services 10, types 5 | 5 |  |
| **diagnostics** | 20 / 2061 | commands 1, components 7, config 1, controllers 1, pages 1, services 8, types 1 | 0 | yes |
| **expenses** | 8 / 947 | pages 5, routes 1, services 1, types 1 | 5 |  |
| **invoices** | 30 / 5383 | commands 1, components 13, controllers 1, helpers 2, pages 10, routes 1, services 1, types 1 | 11 | yes |
| **parties** | 11 / 1845 | components 1, helpers 3, pages 3, routes 1, services 1, types 1, validators 1 | 8 |  |
| **payments** | 6 / 877 | pages 3, routes 1, services 1, types 1 | 3 |  |
| **products** | 32 / 5090 | components 8, controllers 1, helpers 1, pages 15, routes 1, services 4, types 1, validators 1 | 16 |  |
| **purchases** | 11 / 1712 | commands 1, components 1, pages 6, routes 1, services 1, types 1 | 7 | yes |
| **reports** | 44 / 6208 | commands 1, components 5, controllers 2, helpers 1, pages 28, print 4, routes 1, services 1, types 1 | 28 | yes |
| **settings** | 33 / 4964 | commands 1, components 5, controllers 3, helpers 2, pages 15, routes 1, services 3, types 3 | 18 | yes |
| **setup** | 17 / 1673 | components 12, pages 2, routes 1, services 1, types 1 | 2 |  |
| **templates** | 4 / 919 | pages 2, services 1, types 1 | 0 |  |
| **users** | 11 / 966 | controllers 1, helpers 1, pages 4, routes 1, services 2, types 1, validators 1 | 4 |  |
| **vouchers** | 9 / 810 | commands 1, pages 5, routes 1, services 1, types 1 | 5 | yes |

## Service API (the seam — pages call only these)

| Module | Service | Exports |
|---|---|---|
| accounting | `accountingService` | `signedBalance`, `getAccounts`, `accountPath`, `rolledBalance`, `saveAccount`, `deleteAccount`, `reparentAccount`, `getJournalEntries`, `getJournalEntriesForSource`, `getJournalEntriesPaged`, `getJournalEntry`, `createJournalEntry`, `updateJournalDraft`, `postJournalDraft`, `deleteJournalDraft`, `reverseJournalEntry`, `getFiscalYears`, `getCurrentFiscalYear`, `saveFiscalYear`, `getLockDate`, `saveLockDate`, `getCloseYearPreChecks`, `closeYear`, `reopenYear`, `getJournalTemplates`, `getJournalTemplate`, `createOrUpdateJournalTemplate`, `removeJournalTemplate`, `loadTemplateIntoEntry`, `postRecurringTemplate`, `getVatPeriodTotals`, `submitVatSettlement`, `payVatSettlementNow` |
| analytics | `analyticsService` | `getSalesAnalytics`, `getProductAnalytics`, `getCustomerAnalytics` |
| approvals | `approvalService` | `submitApprovalRequest`, `getApprovalRequests`, `getPendingApprovalCount`, `approveRequest`, `rejectRequest` |
| core | `attachmentService` | `fetchAttachments`, `fetchAttachment`, `saveAttachment`, `removeAttachment`, `uid` |
| core | `dashboardService` | `getInTransitTransfers`, `getPendingApprovalRequests`, `getLastBackupFailedAt`, `getJournalDraftCount`, `getStockValueSnapshot`, `hasAnyProducts`, `onLedgerChanged`, `getDashboardSummary`, `getLowStockProducts`, `getRecentInvoices`, `getRecentActivity`, `getHomeKpis`, `getTopProducts`, `getTopCustomers` |
| core | `devToolsService` | `resetToEmpty`, `reloadDemoData` |
| core | `geoService` | `getRegions`, `getCities`, `getDistricts`, `getLabels`, `searchPlaces` |
| core | `insightEngine` | `getThresholds`, `setThresholds`, `dismissInsight`, `snoozeInsight`, `clearDismissal`, `getInsights`, `getInsightsFor`, `getInsightsForEntity`, `getProductInlineHints`, `forceRefresh` |
| core | `insightRules` | `INSIGHT_RULES` |
| core | `insightTypes` | `DEFAULT_THRESHOLDS` |
| core | `pdfService` | `render`, `renderAndSave`, `renderPreview`, `sampleInvoicePayload`, `buildLabelItems`, `renderLabels`, `renderLabelsAndSave`, `renderGenericReport`, `renderGenericReportAndSave`, `renderReportPdf`, `saveReportPdf`, `renderLabelsPreview` |
| core | `printService` | `printReceipt`, `testPrint`, `initPrintResultListener` |
| core | `saveFile` | `saveFile` |
| diagnostics | `accountingDebugService` | `listRecentDocuments`, `getPostingTrace`, `getJournalEntryRaw`, `getBalancesAround`, `getInvariantResults`, `getDriftReport`, `explainAccountBalance`, `startReproRecording`, `stopReproRecording`, `isReproRecording`, `exportReproBundle` |
| diagnostics | `actionJournal` | `isRecording`, `startRecording`, `stopRecording`, `recordServiceCall`, `currentJournal`, `buildReproBundle`, `debugRecordingAvailable` |
| diagnostics | `auditService` | `getAuditEntries`, `getAuditEntities`, `getAuditEntityKinds` |
| diagnostics | `defineService` | `serviceRegistry`, `wrap` |
| diagnostics | `diagnosticsReadService` | `loadChannel`, `groupByFingerprint`, `computePerfStats`, `slowestLongTasks` |
| diagnostics | `diagnosticsService` | `readLogs`, `clearLogs`, `openLogFolder`, `rotateLogs`, `exportAll`, `initDiagnostics` |
| diagnostics | `logService` | `fingerprintOf`, `setLogContext`, `newCorrelationId`, `withCorrelation`, `registerSink`, `debugEnabled`, `log` |
| diagnostics | `supportBundleService` | `exportSupportBundle` |
| expenses | `expenseService` | `getExpenseCategories`, `saveExpenseCategory`, `deleteExpenseCategory`, `getExpenses`, `getExpense`, `createExpense`, `getRecurringExpenses`, `saveRecurringExpense`, `deleteRecurringExpense`, `getDueRecurringExpenses`, `postDueRecurringExpense` |
| invoices | `invoiceService` | `isOverdue`, `getInvoices`, `getInvoicesPaged`, `getInvoice`, `previewSale`, `createSale`, `createRefund`, `getRefund`, `getInvoicePrintData`, `getCurrentShift`, `getShifts`, `getShift`, `openPosShift`, `getXReport`, `closePosShift`, `forceClosePosShift`, `recordCashInOut`, `getHeldSales`, `holdSale`, `resumeHeldSale`, `discardHeldSale`, `getQuotations`, `getQuotation`, `saveQuotation`, `setQuotationStatus`, `convertQuotationToInvoice` |
| parties | `partyService` | `findDuplicates`, `checkDuplicates`, `getPartyGroups`, `getCustomers`, `getCustomer`, `saveCustomer`, `getCustomerStatement`, `getSuppliers`, `getSupplier`, `saveSupplier`, `getSupplierStatement`, `linkPartyRecords`, `unlinkPartyRecord`, `getLinkedNetBalance`, `getPartyHistory`, `getPartyAging`, `ApiError`, `uid` |
| payments | `paymentService` | `getPayments`, `getPaymentsPaged`, `getPayment`, `createPayment`, `allocateExistingPayment`, `removeAllocation`, `getOpenDocuments` |
| products | `catalogService` | `onCatalogChanged`, `getCategories`, `saveCategory`, `deleteCategory`, `getUnits`, `saveUnit`, `applyUnitPreset`, `deleteUnit`, `getPriceLists`, `savePriceList`, `deletePriceList`, `setPriceListValues`, `getCustomFieldDefs`, `saveCustomFieldDef`, `deleteCustomFieldDef`, `reorderCustomFieldDefs` |
| products | `inventoryService` | `adjustmentValue`, `getStockAdjustments`, `getStockAdjustment`, `createStockAdjustment`, `completeAdjustment`, `deleteDraftAdjustment`, `getStockMovements`, `getStockMovementsPaged`, `getBatches`, `getExpiryReport`, `batchAlertTone`, `writeOffExpiredBatches`, `returnBatchesToSupplier`, `getDebitNoteDrafts`, `getStockCounts`, `getStockCount`, `createStockCount`, `updateStockCountLine`, `submitCountForReview`, `resumeCounting`, `completeStockCount` |
| products | `productService` | `isLowStock`, `getProducts`, `getProduct`, `findByCode`, `generateEan13`, `createProduct`, `updateProduct`, `suggestSku` |
| products | `transferService` | `getTransfers`, `getTransfer`, `createTransfer`, `sendTransfer`, `receiveTransfer`, `rejectTransfer`, `branchStockQty` |
| purchases | `purchaseService` | `getPurchaseOrders`, `getPurchaseOrder`, `savePurchaseOrder`, `sendPurchaseOrderToSupplier`, `receivePurchaseOrder`, `confirmPurchaseOrder`, `cancelPurchaseOrder`, `createPurchaseReturn`, `getPurchaseReturn`, `getActiveBatches`, `getDebitNoteDrafts`, `postDebitNoteDraft`, `computePurchaseTotals` |
| reports | `reportService` | `getTrialBalance`, `getProfitAndLoss`, `getProfitAndLossComparison`, `getCostCenterProfitAndLoss`, `getCostCenterBudgetVsActual`, `getBalanceSheet`, `getAccountLedger`, `getPartyLedger`, `getSalesReport`, `getInventoryReport`, `getVatReport`, `getVatDetail`, `getLedgerTargets`, `getCashFlowStatement`, `getDayBook`, `getAgingReport`, `getOverdueReport`, `getGrossProfitReport`, `getReturnsReport`, `getDiscountsReport`, `getShiftsReport`, `getLowStockReport`, `getDeadStockReport`, `getStocktakeVariances`, `getTransfersReport`, `getPurchasesReport`, `getExpensesReport`, `getPeriodComparison`, `getBranchComparison`, `getBusinessHealthReport`, `getProfitLeakageReport`, `getDimensionOptions` |
| settings | `backupService` | `backupSettings`, `saveBackupSettings`, `isTauriMode`, `previewBackupCounts`, `backupNow`, `listHistory`, `deleteHistoryEntry`, `verifyHistoryEntry`, `pickRestoreFile`, `previewRestore`, `restoreFromArchive`, `isClosingWithBackup`, `initAutoBackup`, `stopAutoBackup`, `pickBackupFolder` |
| settings | `branchesService` | `getBranches`, `createBranch`, `updateBranch`, `deactivateBranch`, `reactivateBranch`, `getCostCenters`, `createCostCenter`, `updateCostCenter`, `deleteCostCenter`, `getCurrencies`, `getExchangeRates`, `createCurrency`, `updateCurrency`, `saveExchangeRate`, `isBaseCurrencyLocked`, `setBaseCurrency`, `getRevaluationPreview`, `getDefaultRevaluationRates`, `postRevaluation` |
| settings | `settingsService` | `getSettings`, `updateSettings`, `getTaxes`, `saveTax`, `deleteTax`, `getPaymentMethods`, `savePaymentMethod`, `reorderPaymentMethods`, `deletePaymentMethod` |
| setup | `setupService` | `ensureEmptyCompanyShell`, `persistProgress`, `getOnboardingProgress`, `saveOnboardingProgress`, `markStepDone`, `markStepSkipped`, `applyBusinessTypeDefaults`, `isBaseCurrencyLocked`, `applyCountryTax`, `applyFiscalYear`, `applyBranches`, `previewCoaTemplate`, `applyCoaTemplate`, `applyPaymentMethods`, `getOpeningBalanceEquityNet`, `isFirstUsePosted`, `postOpeningBalances`, `postOpeningStock`, `recloseOpeningBalanceEquity`, `postPartyOpening`, `reversePartyOpening`, `finishOnboarding`, `uid` |
| templates | `templateService` | `listTemplates`, `getTemplate`, `getDefaultTemplate`, `saveTemplate`, `setAsDefault`, `duplicateTemplate`, `deleteTemplate`, `resetTemplateToDefaults`, `exportTemplate`, `importTemplate`, `createTemplate` |
| users | `authService` | `isFreshInstall`, `login`, `restoreSession`, `logout`, `verifyManagerPin`, `getDemoAccounts` |
| users | `userService` | `getUsers`, `getUser`, `createUser`, `updateUser` |
| vouchers | `voucherService` | `createReceiptVoucher`, `createPaymentVoucher`, `createTransferVoucher`, `createOwnerVoucher`, `getVouchers`, `getVoucher`, `getUnsettledTenderGroups`, `estimateSettlementFee`, `createCardSettlement`, `getCardSettlements`, `getCardSettlement` |

## Routes

| Module | Path | Name | Title | Area | Page |
|---|---|---|---|---|---|
| accounting | `/accounting/accounts` | accounts | شجرة الحسابات | accounting | ChartOfAccountsPage.vue |
| accounting | `/accounting/journal` | journal | القيود اليومية | accounting | JournalListPage.vue |
| accounting | `/accounting/journal/new` | journal-new | قيد يدوي | accounting | JournalEntryFormPage.vue |
| accounting | `/accounting/journal/:id` | journal-entry | تفاصيل القيد | accounting | JournalDetailPage.vue |
| accounting | `/accounting/journal-templates` | journal-templates | قوالب القيود والقيود المتكررة | accounting | JournalTemplatesPage.vue |
| accounting | `/accounting/fiscal-years` | fiscal-years | السنة المالية | accounting | FiscalYearsPage.vue |
| accounting | `/accounting/vat-settlement` | vat-settlement | تسوية ضريبة القيمة المضافة | accounting | VatSettlementPage.vue |
| accounting | `/accounting/day-book` | day-book |  |  | (redirect) |
| analytics | `/analytics` | analytics | التحليلات | analytics | AnalyticsPage.vue |
| approvals | `/approvals` | approvals | طلبات الاعتماد | approvals | ApprovalsPage.vue |
| core | `/` | home | الرئيسية | dashboard | DashboardPage.vue |
| core | `/forbidden` | forbidden | غير مصرح |  | ForbiddenPage.vue |
| core | `/dev/ui` | dev-ui | معرض المكوّنات |  | DevUiPage.vue |
| core | `/dev/diagnostics` | dev-diagnostics | التشخيص |  | DevDiagnosticsPage.vue |
| core | `/:pathMatch(.*)*` | not-found | غير موجود |  | NotFoundPage.vue |
| expenses | `/expenses` | expenses | المصروفات | expenses | ExpenseListPage.vue |
| expenses | `/expenses/new` | expense-new | مصروف جديد | expenses | ExpenseFormPage.vue |
| expenses | `/expenses/recurring` | expenses-recurring | المصروفات المتكررة | expenses | RecurringExpensesPage.vue |
| expenses | `/expenses/:id` | expense-detail | تفاصيل المصروف | expenses | ExpenseDetailPage.vue |
| expenses | `/settings/expenses` | settings-expenses | تصنيفات المصروفات | settings | ExpenseCategoriesSettingsPage.vue |
| invoices | `/pos` | pos | نقطة البيع | pos | PosPage.vue |
| invoices | `/pos/shifts` | pos-shifts | إدارة الورديات | pos | ShiftsManagerPage.vue |
| invoices | `/pos/shifts/:id` | pos-shift-report | تقرير Z | pos | ShiftReportPage.vue |
| invoices | `/print/invoices/:id` | invoice-print | معاينة الطباعة |  | InvoicePrintPage.vue |
| invoices | `/invoices` | invoices | الفواتير | sales | InvoiceListPage.vue |
| invoices | `/invoices/:id` | invoice | تفاصيل الفاتورة | sales | InvoiceDetailPage.vue |
| invoices | `/invoices/:id/refund` | invoice-refund | مرتجع مبيعات | sales | RefundPage.vue |
| invoices | `/sales/invoices/new` | invoice-new | فاتورة مبيعات جديدة | sales | InvoiceFormPage.vue |
| invoices | `/sales/quotations` | quotations | عروض الأسعار | sales | QuotationListPage.vue |
| invoices | `/sales/quotations/new` | quotation-new | عرض سعر جديد | sales | InvoiceFormPage.vue |
| invoices | `/sales/quotations/:id` | quotation | تفاصيل عرض السعر | sales | QuotationDetailPage.vue |
| parties | `/customers` | customers | العملاء | parties | PartyListPage.vue |
| parties | `/customers/new` | customer-new | عميل جديد | parties | PartyFormPage.vue |
| parties | `/customers/:id/edit` | customer-edit | تعديل عميل | parties | PartyFormPage.vue |
| parties | `/customers/:id` | customer | بطاقة عميل | parties | PartyDetailPage.vue |
| parties | `/suppliers` | suppliers | الموردين | parties | PartyListPage.vue |
| parties | `/suppliers/new` | supplier-new | مورد جديد | parties | PartyFormPage.vue |
| parties | `/suppliers/:id/edit` | supplier-edit | تعديل مورد | parties | PartyFormPage.vue |
| parties | `/suppliers/:id` | supplier | بطاقة مورد | parties | PartyDetailPage.vue |
| payments | `/payments` | payments | سندات القبض والصرف | payments | PaymentListPage.vue |
| payments | `/payments/new` | payment-new | سند جديد | payments | PaymentFormPage.vue |
| payments | `/payments/:id` | payment-detail | تفاصيل السند | payments | PaymentDetailPage.vue |
| products | `/products` | products | المنتجات | inventory | ProductListPage.vue |
| products | `/products/new` | product-new | منتج جديد | inventory | ProductFormPage.vue |
| products | `/products/:id` | product | بطاقة منتج | inventory | ProductDetailPage.vue |
| products | `/products/:id/edit` | product-edit | تعديل منتج | inventory | ProductFormPage.vue |
| products | `/catalog/categories` | categories | التصنيفات والوحدات | inventory | CategoriesUnitsPage.vue |
| products | `/catalog/price-lists` | price-lists | قوائم الأسعار | inventory | PriceListsPage.vue |
| products | `/inventory/adjustments` | adjustments | تسويات المخزون | inventory | StockAdjustmentListPage.vue |
| products | `/inventory/adjustments/new` | adjustment-new | تسوية جديدة | inventory | StockAdjustmentFormPage.vue |
| products | `/inventory/adjustments/:id` | adjustment | تفاصيل التسوية | inventory | StockAdjustmentDetailPage.vue |
| products | `/inventory/movements` | movements | حركة المخزون | inventory | StockMovementsPage.vue |
| products | `/inventory/counts` | counts | الجرد | inventory | StockCountListPage.vue |
| products | `/inventory/counts/new` | count-new | جرد جديد | inventory | StockCountNewPage.vue |
| products | `/inventory/counts/:id` | count | تفاصيل الجرد | inventory | StockCountDetailPage.vue |
| products | `/inventory/expiry` | expiry | تقرير الصلاحية | inventory | ExpiryReportPage.vue |
| products | `/inventory/transfers` | transfers | تحويلات الفروع | inventory | StockTransferListPage.vue |
| products | `/catalog/labels` | labels | منشئ الملصقات | inventory | LabelBuilderPage.vue |
| purchases | `/purchases` | purchases | أوامر الشراء | purchases | PurchaseListPage.vue |
| purchases | `/purchases/new` | purchase-new | أمر شراء جديد | purchases | PurchaseFormPage.vue |
| purchases | `/purchases/:id` | purchase | تفاصيل أمر الشراء | purchases | PurchaseDetailPage.vue |
| purchases | `/purchases/:id/edit` | purchase-edit | تعديل أمر شراء | purchases | PurchaseFormPage.vue |
| purchases | `/purchases/:id/receive` | purchase-receive | استلام أمر شراء | purchases | PurchaseReceivePage.vue |
| purchases | `/purchases/:id/return` | purchase-return | مرتجع مشتريات | purchases | PurchaseReturnPage.vue |
| purchases | `/print/purchases/:id` | purchase-print | طباعة أمر الشراء |  | PurchasePrintPage.vue |
| reports | `/reports` | reports | التقارير | reports | ReportsHubPage.vue |
| reports | `/reports/trial-balance` | report-trial-balance | ميزان المراجعة | reports | TrialBalancePage.vue |
| reports | `/reports/profit-loss` | report-profit-loss | قائمة الدخل | reports | ProfitLossPage.vue |
| reports | `/reports/balance-sheet` | report-balance-sheet | الميزانية العمومية | reports | BalanceSheetPage.vue |
| reports | `/reports/cash-flow` | report-cash-flow | قائمة التدفقات النقدية | reports | CashFlowPage.vue |
| reports | `/reports/ledger` | report-ledger | كشف حساب | reports | LedgerPage.vue |
| reports | `/reports/day-book` | report-day-book | دفتر اليومية | reports | DayBookPage.vue |
| reports | `/reports/cost-centers` | report-cost-centers | الأرباح حسب مركز التكلفة | reports | CostCenterPnlPage.vue |
| reports | `/reports/aging` | report-aging | أعمار الديون | reports | AgingReportPage.vue |
| reports | `/reports/overdue` | report-overdue | المستندات المتأخرة | reports | OverdueReportPage.vue |
| reports | `/reports/sales` | report-sales | تقرير المبيعات | reports | SalesReportPage.vue |
| reports | `/reports/gross-profit` | report-gross-profit | تقرير مجمل الربح | reports | GrossProfitPage.vue |
| reports | `/reports/returns` | report-returns | تحليل المرتجعات | reports | ReturnsReportPage.vue |
| reports | `/reports/discounts` | report-discounts | الخصومات وتجاوزات السعر | reports | DiscountsReportPage.vue |
| reports | `/reports/shifts` | report-shifts | سجل الورديات | reports | ShiftsReportPage.vue |
| reports | `/reports/inventory` | report-inventory | تقرير المخزون | reports | InventoryReportPage.vue |
| reports | `/reports/stock-health` | report-stock-health | المخزون المنخفض والراكد | reports | StockHealthReportPage.vue |
| reports | `/reports/stocktake-variances` | report-stocktake-variances | فروقات الجرد | reports | StocktakeVariancesPage.vue |
| reports | `/reports/transfers` | report-transfers | تقرير التحويلات | reports | TransfersReportPage.vue |
| reports | `/reports/purchases` | report-purchases | تقرير المشتريات | reports | PurchasesReportPage.vue |
| reports | `/reports/expenses` | report-expenses | تقرير المصروفات | reports | ExpensesReportPage.vue |
| reports | `/reports/budget-vs-actual` | report-budget-vs-actual | الميزانية مقابل الفعلي | reports | BudgetVsActualPage.vue |
| reports | `/reports/vat` | report-vat | ملخص الضريبة | reports | VatReportPage.vue |
| reports | `/reports/vat-detail` | report-vat-detail | التفصيل الضريبي | reports | VatDetailPage.vue |
| reports | `/reports/period-comparison` | report-period-comparison | مقارنة الفترات | reports | PeriodComparisonPage.vue |
| reports | `/reports/branch-comparison` | report-branch-comparison | مقارنة الفروع | reports | BranchComparisonPage.vue |
| reports | `/reports/business-health` | report-business-health | الصحة المالية | reports | BusinessHealthPage.vue |
| reports | `/reports/profit-leakage` | report-profit-leakage | تسرب الربح | reports | ProfitLeakagePage.vue |
| settings | `/settings` | settings |  |  | (redirect) |
| settings | `/settings/general` | settings-general | عام | settings | GeneralSettingsPage.vue |
| settings | `/settings/taxes` | settings-taxes | الضرائب | settings | TaxesSettingsPage.vue |
| settings | `/settings/payment-methods` | settings-payment-methods | طرق الدفع | settings | PaymentMethodsSettingsPage.vue |
| settings | `/settings/printing` | settings-printing | الطباعة | settings | PrintingSettingsPage.vue |
| settings | `/settings/products` | settings-products | المنتجات | inventory | ProductsSettingsPage.vue |
| settings | `/settings/branches` | settings-branches | الفروع | settings | BranchesSettingsPage.vue |
| settings | `/settings/cost-centers` | settings-cost-centers | مراكز التكلفة | settings | CostCentersSettingsPage.vue |
| settings | `/settings/currencies` | settings-currencies | العملات | settings | CurrenciesSettingsPage.vue |
| settings | `/settings/recommendations` | settings-recommendations | التوصيات | settings | RecommendationsSettingsPage.vue |
| settings | `/settings/roles` | settings-roles | المستخدمون والأدوار | users | RoleMatrixSettingsPage.vue |
| settings | `/settings/audit-log` | settings-audit-log | سجل التدقيق | users | AuditLogSettingsPage.vue |
| settings | `/settings/appearance` | settings-appearance | المظهر |  | AppearanceSettingsPage.vue |
| settings | `/settings/keyboard-shortcuts` | settings-keyboard-shortcuts | اختصارات لوحة المفاتيح |  | KeyboardShortcutsSettingsPage.vue |
| settings | `/settings/backup` | settings-backup | النسخ الاحتياطي | settings | BackupSettingsPage.vue |
| settings | `/settings/templates` | settings-templates | قوالب الطباعة | settings | TemplateListPage.vue |
| settings | `/settings/templates/:id` | settings-template-designer | قالب الطباعة | settings | TemplateDesignerPage.vue |
| settings | `/settings/about` | settings-about | حول / الدعم |  | AboutSettingsPage.vue |
| setup | `/setup` | setup-wizard | إعداد الشركة |  | SetupWizardPage.vue |
| setup | `/setup/opening` | setup-opening | الأرصدة الافتتاحية | accounting | OpeningBalancesPage.vue |
| users | `/welcome` | welcome | مرحباً بك |  | WelcomePage.vue |
| users | `/login` | login | تسجيل الدخول |  | LoginPage.vue |
| users | `/users` | users | المستخدمين | users | UserListPage.vue |
| users | `/users/:id` | user-editor | بيانات المستخدم | users | UserEditorPage.vue |
| vouchers | `/vouchers` | vouchers | السندات العامة | payments | VoucherListPage.vue |
| vouchers | `/vouchers/new` | voucher-new | سند عام جديد | payments | VoucherFormPage.vue |
| vouchers | `/vouchers/:id` | voucher-detail | تفاصيل السند | payments | VoucherDetailPage.vue |
| vouchers | `/print/vouchers/:id` | voucher-print | طباعة السند |  | VoucherPrintPage.vue |
| vouchers | `/payments/settlements` | card-settlements | تسوية البطاقات | payments | CardSettlementPage.vue |

## Module dependencies (who imports whom)

Counts are import statements. `app` = router / main.ts / App.vue; `mocks` = src/mocks.

| From | Imports from | Imported by (# modules) |
|---|---|---|
| **accounting** | core (106), users (6), mocks (4), parties (4), reports (3), settings (2), diagnostics (1), invoices (1) | 9 |
| **analytics** | core (16), diagnostics (1), mocks (1) | 1 |
| **app** | core (14), settings (5), diagnostics (4), users (4), accounting (2), approvals (2), invoices (2), purchases (2), reports (2), vouchers (2), analytics (1), expenses (1), mocks (1), parties (1), payments (1), products (1), setup (1) | 1 |
| **approvals** | core (12), mocks (2), diagnostics (1), users (1) | 5 |
| **core** | users (19), mocks (16), products (14), invoices (13), settings (10), diagnostics (8), parties (4), purchases (3), accounting (2), templates (2), vouchers (2), app (1), approvals (1), payments (1), reports (1), setup (1) | 18 |
| **diagnostics** | core (26), mocks (5) | 18 |
| **expenses** | core (58), accounting (3), users (3), mocks (2), parties (2), settings (2), diagnostics (1) | 2 |
| **invoices** | core (171), products (10), mocks (9), settings (9), parties (8), users (8), reports (6), approvals (2), accounting (1), diagnostics (1), payments (1) | 10 |
| **mocks** | core (10), invoices (9), accounting (8), products (8), settings (5), diagnostics (4), vouchers (3), approvals (2), expenses (2), parties (2), payments (2), purchases (2), users (1) | 17 |
| **parties** | core (59), mocks (6), payments (3), users (2), diagnostics (1), invoices (1), purchases (1), settings (1), setup (1) | 10 |
| **payments** | core (42), invoices (2), mocks (2), parties (2), users (2), diagnostics (1) | 6 |
| **products** | core (204), mocks (13), users (13), settings (8), diagnostics (4), accounting (3), purchases (2), approvals (1), parties (1), templates (1) | 8 |
| **purchases** | core (79), products (5), users (4), invoices (3), mocks (3), parties (3), settings (2), diagnostics (1), payments (1) | 5 |
| **reports** | core (162), settings (7), accounting (5), mocks (4), diagnostics (1), invoices (1), users (1) | 4 |
| **settings** | core (175), users (18), mocks (14), diagnostics (7), invoices (3), products (3), templates (2) | 12 |
| **setup** | core (57), mocks (8), settings (4), accounting (2), products (2), diagnostics (1), parties (1), users (1) | 3 |
| **templates** | core (16), diagnostics (1) | 3 |
| **users** | core (37), mocks (5), products (3), diagnostics (2) | 15 |
| **vouchers** | core (48), mocks (3), accounting (2), settings (2), users (2), diagnostics (1), invoices (1) | 3 |

**Most-used npm packages** (files importing): `vue (392)`, `@lucide/vue (166)`, `reka-ui (118)`, `vue-router (98)`, `@vueuse/core (79)`, `@tauri-apps/api (16)`, `class-variance-authority (9)`, `pinia (9)`, `zod (5)`, `@tauri-apps/plugin-dialog (4)`, `@tauri-apps/plugin-fs (4)`, `fflate (4)`, `uqr (3)`, `@fontsource-variable/cairo (2)`, `@fontsource/ibm-plex-sans-arabic (2)`, `@internationalized/date (2)`, `@tauri-apps/plugin-opener (2)`, `exceljs (2)`, `@fontsource/noto-naskh-arabic (1)`, `@fontsource/tajawal (1)`, `bwip-js (1)`, `clsx (1)`, `libphonenumber-js (1)`, `tailwind-merge (1)`, `vue-sonner (1)`

## Rust ↔ Vue IPC contract

| Command | Rust path | Defined in | Registered | Invoked from |
|---|---|---|---|---|
| `diag_append` | `diag::diag_append` | `src-tauri/src/diag/mod.rs` | yes | `src/modules/diagnostics/services/diagnosticsService.ts` |
| `diag_read` | `diag::diag_read` | `src-tauri/src/diag/mod.rs` | yes | `src/modules/diagnostics/services/diagnosticsService.ts` |
| `diag_clear` | `diag::diag_clear` | `src-tauri/src/diag/mod.rs` | yes | `src/modules/diagnostics/services/diagnosticsService.ts` |
| `diag_open_folder` | `diag::diag_open_folder` | `src-tauri/src/diag/mod.rs` | yes | `src/modules/diagnostics/services/diagnosticsService.ts` |
| `diag_rotate` | `diag::diag_rotate` | `src-tauri/src/diag/mod.rs` | yes | `src/modules/diagnostics/services/diagnosticsService.ts` |
| `greet` | `greet` | `src-tauri/src/lib.rs` | yes | — |
| `render_pdf_spike` | `pdf::render_pdf_spike` | `src-tauri/src/pdf/mod.rs` | yes | — |
| `render_pdf` | `pdf::render::render_pdf` | `src-tauri/src/pdf/render.rs` | yes | `src/modules/core/services/pdfService.ts` |
| `render_preview` | `pdf::render::render_preview` | `src-tauri/src/pdf/render.rs` | yes | `src/modules/core/services/pdfService.ts` |
| `list_printers` | `print::commands::list_printers` | `src-tauri/src/print/commands.rs` | yes | `src/modules/settings/pages/PrintingSettingsPage.vue` |
| `print_thermal_receipt` | `print::commands::print_thermal_receipt` | `src-tauri/src/print/commands.rs` | yes | `src/modules/core/services/printService.ts` |
| `print_test_receipt` | `print::commands::print_test_receipt` | `src-tauri/src/print/commands.rs` | yes | `src/modules/core/services/printService.ts` |

**Plugins:** `sql`, `opener`, `fs`, `dialog`, `log`. Frontend plugin use: `@tauri-apps/api (16)`, `@tauri-apps/plugin-dialog (4)`, `@tauri-apps/plugin-fs (4)`, `@tauri-apps/plugin-opener (2)`

**Rust module tree:** `lib.rs → pub mod diag`, `lib.rs → pub mod pdf`, `lib.rs → pub mod print`, `pdf/mod.rs → pub mod fonts`, `pdf/mod.rs → mod payload`, `pdf/mod.rs → mod qr`, `pdf/mod.rs → pub mod raster`, `pdf/mod.rs → pub mod render`, `pdf/mod.rs → mod world`, `print/mod.rs → pub mod dither`, `print/mod.rs → pub mod escpos`, `print/mod.rs → pub mod payload`, `print/mod.rs → pub mod printers`, `print/mod.rs → pub mod render`, `print/mod.rs → mod transport`, `print/mod.rs → pub mod commands`

**Contract gaps:** invoked-but-unregistered — · registered-but-never-invoked `greet`, `render_pdf_spike` · defined-but-unregistered — · registered-but-undefined —

## Mock backend map (src/mocks — accounting engine)

Read `docs/v2/02-accounting-review.md` before touching posting, VAT, cost or account resolution; keep `bun run verify:mocks` green.

| File | Exported functions | Used by |
|---|---|---|
| `attachments.ts` | `putAttachment`, `getAttachment`, `deleteAttachment`, `listAttachments`, `getAllAttachmentRecords`, `replaceAllAttachments` | core, settings |
| `backend/accounts.ts` | `accountFor`, `accountById`, `settlementAccountFor`, `revenueAccountFor`, `cogsAccountFor`, `purchaseAccountFor`, `saleTaxIdFor`, `purchaseTaxIdFor` | core, invoices, reports |
| `backend/approvals.ts` | `requestApproval`, `decideApproval`, `listApprovalRequests`, `pendingApprovalCount` | approvals |
| `backend/balances.ts` | `customerBalance`, `supplierBalance`, `customerBalanceFc`, `supplierBalanceFc`, `customerStatement`, `supplierStatement` | core, invoices, parties, reports |
| `backend/branches.ts` | `branchById`, `activeBranches`, `branchesEnabled`, `currenciesEnabled`, `costCentersEnabled`, `listBranches`, `createBranch`, `updateBranch`, `deactivateBranch`, `reactivateBranch`, `branchPrefix`, `listCostCenters`, `costCenterById`, `createCostCenter`, `updateCostCenter`, `deleteCostCenter`, `defaultCostCenterFor` | settings |
| `backend/core.ts` | `resolvePosting`, `assertOpenPeriod`, `postJournal`, `draftJournal`, `updateDraftJournal`, `deleteDraftJournal`, `postDraftJournal`, `productById`, `applyStockChange`, `round4`, `logActivity`, `logAudit`, `diffFields`, `salesTaxRate`, `purchaseTaxRate`, `userById`, `closeYearPreChecks`, `closeFiscalYear`, `reopenFiscalYear` | accounting, invoices, parties, products, settings, users |
| `backend/currency.ts` | `baseCurrency`, `isBaseCurrency`, `currencyByCode`, `activeCurrencies`, `createCurrency`, `updateCurrency`, `isBaseCurrencyLocked`, `setBaseCurrency`, `saveExchangeRate`, `latestRate`, `requireRate`, `convertLinesToBase`, `toBase` | settings, setup |
| `backend/expenses.ts` | `saveExpenseCategory`, `deleteExpenseCategory`, `recordExpense`, `getExpenseById`, `saveRecurringExpense`, `deleteRecurringExpense`, `dueRecurringExpenses`, `postRecurringExpense` | expenses |
| `backend/invariants.ts` | `checkBalancedEntries`, `checkTrialBalance`, `checkArApControl`, `checkInventoryGl`, `checkVatControl`, `checkPartyAllocation`, `checkSourceRefIntegrity`, `checkLockDate`, `checkOpeningBalanceEquity`, `checkClearingAccounts`, `checkShiftVariance`, `checkDraftsIsolated`, `checkAllocationsWithinTotal`, `checkFxConversion`, `runAllInvariants` | — |
| `backend/inventory.ts` | `activeBatchesFor`, `isBatchExpired`, `isBatchNearExpiry`, `receiveBatch`, `consumeFefo`, `recordStockAdjustment`, `completeStockAdjustment`, `startStockCount`, `setStockCountLine`, `submitStockCountForReview`, `backToCounting`, `applyStockCount`, `writeOffBatches`, `draftReturnToSupplier` | products, purchases |
| `backend/journal.ts` | `splitLineByCostCenters`, `recordManualJournal`, `editDraftJournal`, `reverseJournal`, `saveJournalTemplate`, `deleteJournalTemplate`, `advanceRecurrence`, `vatTotalsForPeriod`, `postVatSettlement`, `payVatSettlement` | accounting |
| `backend/opening.ts` | `hasFirstUsePosted`, `openingBalanceEquityNet`, `buildOpeningLines`, `postOpeningEntry`, `closeOpeningBalanceEquity`, `postOpeningBalancesAndClose`, `postOpeningStockForBranch`, `postOpeningStockDefault`, `postPartyOpeningBalance`, `reversePartyOpeningBalance` | setup |
| `backend/payments.ts` | `getOpenDocumentsFor`, `allocatedTotal`, `unallocatedAmount`, `unallocatedCreditFor`, `recordPayment`, `allocatePayment`, `unallocatePayment` | core, parties, payments, reports |
| `backend/posting-trace.ts` | `recordPostingTrace`, `recentPostingTraces`, `postingTraceFor`, `clearPostingTraces` | — |
| `backend/purchases.ts` | `computePurchaseTotals`, `purchaseOutstanding`, `missingSupplierInvoice`, `duplicateSupplierInvoice`, `baseQty`, `baseUnitCost`, `savePurchase`, `sendPurchaseToSupplier`, `receivePurchase`, `confirmPurchase`, `cancelPurchase`, `returnedQtyByProduct`, `recordPurchaseReturn`, `supplierOutstandingTotal`, `getDebitNoteDrafts`, `postDebitNoteFromDraft` | purchases |
| `backend/revaluation.ts` | `openFcBalances`, `defaultRevaluationRates`, `postRevaluation` | settings |
| `backend/sales.ts` | `previewSaleJournal`, `recordSale`, `returnedQtyByLine`, `recordRefund` | invoices |
| `backend/settlements.ts` | `unsettledTenderGroups`, `recordCardSettlement`, `getCardSettlementById`, `estimatedFeeFor` | vouchers |
| `backend/setup.ts` | `applyBusinessTypeUnitDefaults`, `applyCoaTemplate`, `previewCoaTemplate`, `setFiscalYear`, `applyBranches`, `applyPaymentMethods` | setup |
| `backend/shifts.ts` | `currentOpenShift`, `openShift`, `recordShiftMovement`, `shiftSummary`, `closeShift`, `forceCloseShift`, `cashAccountName` | invoices |
| `backend/transfers.ts` | `listTransfers`, `transferById`, `branchStockQty`, `draftTransfer`, `sendTransfer`, `receiveTransfer`, `rejectTransfer` | products |
| `backend/vouchers.ts` | `recordReceiptVoucher`, `recordPaymentVoucher`, `recordTransferVoucher`, `recordOwnerVoucher`, `getVoucherById` | vouchers |
| `db.ts` | `resetDb`, `nextNumber` | core, invoices, settings |
| `events.ts` | `on`, `off`, `emit` | core, invoices, parties, products |
| `index.ts` | `bootMockDb`, `isBooted` | accounting, analytics, approvals, core, diagnostics, expenses, invoices, main, parties, payments, products, purchases, reports, settings, setup, users, vouchers |
| `persist.ts` | `flushSnapshot`, `mutate`, `loadSnapshot`, `clearSnapshot` | accounting, core, invoices, parties, products, settings, setup, users |
| `seed/accounts.ts` | `seedAccounts`, `postOpeningCapital` | — |
| `seed/branches9.ts` | `seedBranches9` | — |
| `seed/catalog.ts` | `seedCatalog`, `postOpeningStock` | — |
| `seed/history.ts` | `seedHistory` | — |
| `seed/index.ts` | `seedDatabase`, `seedEmptyCompany` | core, setup |
| `seed/people.ts` | `seedPeople` | — |
| `seed/purchases8.ts` | `seedPurchases8` | — |
| `seed/settings.ts` | `seedSettings` | — |
| `seed/shifts.ts` | `seedShifts` | — |
| `utils.ts` | `setLatencyMode`, `getLatencyMode`, `delay`, `clone`, `round2`, `sum`, `uid`, `bumpIdCounter`, `padNumber`, `localDateKey`, `inDateRange`, `includesText`, `createRandom` | core, settings |

## Shared UI kit (reuse before building — src/modules/core)

| Group | Names |
|---|---|
| App*/ui components | `AiOrb`, `AppButton`, `AppCard`, `AppCombobox`, `AppDatePicker`, `AppInput`, `AppModal`, `AppPhoneInput`, `AppSelect`, `AppSwitch`, `AppTextarea`, `AttachmentField`, `AttachmentViewer`, `BrandLogo`, `ConfirmDialog`, `CountryFlag`, `DataTable`, `DateRangeFilter`, `DirIcon`, `EmptyState`, `ErrorState`, `MoneyText`, `PageHeader`, `PdfPreview`, `RiyalIcon`, `ScrollFade`, `SearchInput`, `SegmentedControl`, `SkeletonBlock`, `StatusBadge` |
| Page blocks | `AddressFields`, `DetailHeader`, `FilterBar`, `FormActions`, `FormField`, `FormSection`, `LineItemsEditor`, `StatCards`, `TotalsPanel` |
| Page layouts | `DetailPage`, `FormPage`, `ListPage`, `SettingsPage` |
| App shell | `AppSidebar`, `AppTopbar`, `BrandBranchSwitcher`, `DefaultLayout`, `KeyboardShortcutsSheet`, `NavMain`, `NavQuickActions`, `NavUser`, `NotificationsDrawer` |
| Core controllers (composables/stores) | `useAppearance`, `useAsync`, `useCommandPalette`, `useConfirm`, `useForm`, `useGridTab`, `useHotkeys`, `useInsights`, `useKeybindings`, `useKeyboardShortcutsSheet`, `useNotificationStore`, `useNotifications`, `useTheme`, `useToast` |
| Core helpers | `attachments`, `brand`, `countries`, `countryProfiles`, `dirIcon`, `exportXlsx`, `format`, `keyCode`, `keyboardShortcuts`, `labels`, `navigation`, `numbers`, `search`, `tafqit`, `utils`, `validation` |
| Core services | `attachmentService`, `dashboardService`, `devToolsService`, `geoService`, `insightEngine`, `insightRules`, `insightTypes`, `pdfService`, `printService`, `saveFile` |
| shadcn primitives | `alert-dialog`, `avatar`, `badge`, `breadcrumb`, `button`, `calendar`, `card`, `collapsible`, `combobox`, `command`, `dialog`, `dropdown-menu`, `empty`, `field`, `input`, `input-group`, `kbd`, `label`, `native-select`, `popover`, `range-calendar`, `separator`, `sheet`, `sidebar`, `skeleton`, `sonner`, `switch`, `table`, `tabs`, `textarea`, `toggle`, `toggle-group`, `tooltip` |

## Boundary report

### Seam violations — value imports of `src/mocks` outside `services/` (0 new, 0 known)

_none_

### Pages over 250 lines (CLAUDE.md rule 12)

| Page | Lines |
|---|---|
| `src/modules/invoices/pages/PosPage.vue` | 866 |
| `src/modules/accounting/pages/JournalEntryFormPage.vue` | 593 |
| `src/modules/core/pages/DevUiPage.vue` | 578 |
| `src/modules/parties/pages/PartyFormPage.vue` | 525 |
| `src/modules/accounting/pages/JournalListPage.vue` | 482 |
| `src/modules/products/pages/ProductFormPage.vue` | 469 |
| `src/modules/templates/pages/TemplateDesignerPage.vue` | 446 |
| `src/modules/parties/pages/PartyDetailPage.vue` | 409 |
| `src/modules/products/pages/StockAdjustmentFormPage.vue` | 394 |
| `src/modules/accounting/pages/ChartOfAccountsPage.vue` | 392 |
| `src/modules/purchases/pages/PurchaseFormPage.vue` | 355 |
| `src/modules/reports/pages/LedgerPage.vue` | 350 |
| `src/modules/invoices/pages/InvoiceFormPage.vue` | 348 |
| `src/modules/accounting/pages/JournalDetailPage.vue` | 337 |
| `src/modules/invoices/pages/InvoiceListPage.vue` | 309 |
| `src/modules/settings/pages/PaymentMethodsSettingsPage.vue` | 278 |
| `src/modules/payments/pages/PaymentFormPage.vue` | 273 |
| `src/modules/products/pages/LabelBuilderPage.vue` | 273 |
| `src/modules/settings/pages/BackupSettingsPage.vue` | 268 |
| `src/modules/accounting/pages/FiscalYearsPage.vue` | 261 |
| `src/modules/products/pages/PriceListsPage.vue` | 255 |

### Cross-module page imports

| From | Imports page |
|---|---|
| `src/modules/core/routes/index.ts` | `src/modules/diagnostics/pages/DevDiagnosticsPage.vue` |
| `src/modules/settings/routes/index.ts` | `src/modules/templates/pages/TemplateListPage.vue` |
| `src/modules/settings/routes/index.ts` | `src/modules/templates/pages/TemplateDesignerPage.vue` |

## Stack & commands

| Layer | Facts |
|---|---|
| Desktop shell | Tauri v2 — product `Equal`, identifier `com.abdallah.accounting-app` (never change) |
| Rust crates | `tauri`, `tauri-plugin-opener`, `serde`, `serde_json`, `tauri-plugin-sql`, `tauri-plugin-fs`, `tauri-plugin-dialog`, `tauri-plugin-log`, `typst`, `typst-pdf`, `typst-library`, `typst-layout`, `typst-syntax`, `typst-utils`, `typst-svg`, `qrcode`, `image`, `ecow`, `time`, `lopdf`, `resvg`, `usvg`, `tiny-skia`, `windows` |
| Rust extra binaries | `typst_spike`, `pdf_smoke`, `report_smoke`, `thermal_smoke` |
| Frontend deps | `@fontsource-variable/cairo`, `@fontsource/ibm-plex-sans-arabic`, `@fontsource/noto-naskh-arabic`, `@fontsource/tajawal`, `@lucide/vue`, `@tailwindcss/vite`, `@tauri-apps/api`, `@tauri-apps/plugin-dialog`, `@tauri-apps/plugin-fs`, `@tauri-apps/plugin-opener`, `@vueuse/core`, `bwip-js`, `class-variance-authority`, `clsx`, `exceljs`, `fflate`, `libphonenumber-js`, `pinia`, `reka-ui`, `tailwind-merge`, `tailwindcss`, `tw-animate-css`, `uqr`, `vue`, `vue-router`, `vue-sonner`, `zod` |
| Dev deps | `@tauri-apps/cli`, `@types/node`, `@vitejs/plugin-vue`, `typescript`, `vite`, `vue-tsc` |

| Command | Runs |
|---|---|
| `bun run dev` | `vite` |
| `bun run build` | `vue-tsc --noEmit && vite build` |
| `bun run preview` | `vite preview` |
| `bun run tauri` | `tauri` |
| `bun run stop` | `node scripts/stop-dev.js` |
| `bun run desktop` | `node scripts/stop-dev.js && tauri dev` |
| `bun run scaffold` | `node scripts/scaffold.js` |
| `bun run build:android` | `cd src-tauri/gen/android && ./gradlew.bat assembleArm64Debug` |
| `bun run verify:mocks` | `bun run scripts/verify/run.ts` |
| `bun run verify:replay` | `bun run scripts/verify/replay.ts` |
| `bun run check` | `node scripts/check-text-tokens.js && node scripts/check-rtl.js && bun run scripts/check-contrast.ts && node scripts/check-ui-rules.js && node scripts/check-routes.js` |
| `bun run memory` | `bun run scripts/memory/run.ts` |
| `bun run memory:check` | `bun run scripts/memory/run.ts --check` |
| `bun run diag` | `bun run scripts/diagnostics/run.ts` |
| `bun run diag:check` | `bun run scripts/diagnostics/run.ts --check` |
| `bun run geo:build` | `bun run scripts/geo/build.ts` |

## Docs index

| Doc | Title |
|---|---|
| `docs/action_plan.md` | Execution Plan — Frontend-Only Rebuild |
| `docs/design_system.md` | Design System — "Linear-style" Light/Dark |
| `docs/diagnostics/ISSUES.md` | سجل المشاكل (Issues) — ملف مُولَّد |
| `docs/diagnostics/issues/BUG-0001-onboarding-flow-crashed-unhandled-except.md` |  |
| `docs/diagnostics/issues/BUG-0002-setup-wizard-eg-flow-crashed-unhandled-e.md` |  |
| `docs/diagnostics/issues/BUG-0004-purchases-flow-crashed-unhandled-excepti.md` |  |
| `docs/diagnostics/issues/BUG-0005-branches-currencies-flow-failed-an-asser.md` |  |
| `docs/diagnostics/issues/BUG-0006-reports-v2-flow-crashed-unhandled-except.md` |  |
| `docs/diagnostics/issues/BUG-0007-full-persona-pass-flow-failed-an-asserti.md` |  |
| `docs/diagnostics/issues/DBG-0001-example-entry.md` |  |
| `docs/domain_model.md` | Domain Model (Frontend Mock Data Reference) |
| `docs/project_specs.md` | Desktop Accounting & POS UI — Product Spec (Frontend-Only) |
| `docs/v2/01-personas.md` | 01 — Personas: Using the App as Each Role |
| `docs/v2/02-accounting-review.md` | 02 — Accounting Review of v1 + Corrected Posting Rules |
| `docs/v2/03-chart-of-accounts.md` | 03 — Chart of Accounts v2 (شجرة الحسابات) |
| `docs/v2/04-domain-model.md` | 04 — Domain Model v2 |
| `docs/v2/05-onboarding.md` | 05 — Onboarding, Opening Entry & Moving From an Old System |
| `docs/v2/06-sales-and-pos.md` | 06 — Sales: POS, Full Invoice Form, Pricing & VAT Math, Returns, Shifts |
| `docs/v2/07-products-and-inventory.md` | 07 — Products & Inventory |
| `docs/v2/08-customers-and-suppliers.md` | 08 — Customers & Suppliers |
| `docs/v2/09-purchases-payments-expenses.md` | 09 — Purchases, Payment Methods, Payments & Expenses |
| `docs/v2/10-branches-currencies-cost-centers.md` | 10 — Branches, Currencies & Cost Centers |
| `docs/v2/11-journal-dashboard-insights.md` | 11 — Journal Redesign, Simpler Home, Analytics & Recommendations |
| `docs/v2/12-documents-pdf-excel.md` | 12 — Documents: Real PDF Engine, Template Designer, Labels, Thermal, Excel |
| `docs/v2/13-reports.md` | 13 — Reports v2 |
| `docs/v2/14-platform.md` | 14 — Platform: Command Palette, Appearance, Backup, Attachments, Persistence, Speed |
| `docs/v2/15-action-plan.md` | 15 — Action Plan v2 (execute from here) |
| `docs/v2/16-equal-rebrand-and-ui-kit.md` | 16 — Equal: close fix, rebrand, shadcn-vue UI kit & sidebar-07 layout |
| `docs/v2/17-ui-system-rtl-themes.md` | 17 — Real RTL, full motion, themes, a simpler sidebar, and one shared UI system |
| `docs/v2/README.md` | Plan v2 — From Demo Shop to Real Business Tool |
| `plans/completed/19-shadcn-date-picker/phase-a-component.md` | Phase A — Build `AppDatePicker` |
| `plans/completed/19-shadcn-date-picker/phase-b-migrate-appinput.md` | Phase B — Migrate all `AppInput type="date"` call sites |
| `plans/completed/19-shadcn-date-picker/phase-c-migrate-raw-inputs.md` | Phase C — Migrate remaining raw `<input type="date">` sites |
| `plans/completed/19-shadcn-date-picker/README.md` | 19 — Shared shadcn date picker (`AppDatePicker`), replacing native `<input type="date">` |
| `plans/completed/20-named-route-objects/phase-a-typed-map.md` | 20.A — Rule, typed route map, `AppRoute`, guard (report mode) |
| `plans/completed/20-named-route-objects/phase-b-shared-layer.md` | 20.B — Shared layer: navigation config, palette, services, guards |
| `plans/completed/20-named-route-objects/phase-c-document-pages.md` | 20.C — Pages: sales/POS, purchases, payments, vouchers, expenses, parties |
| `plans/completed/20-named-route-objects/phase-d-remaining-pages-lock.md` | 20.D — Remaining pages, then lock the rule in |
| `plans/completed/20-named-route-objects/README.md` | 20 — Named route objects everywhere (no path strings) |
| `plans/pending/18-countries-a11y-diagnostics/phase-a-phone-switch.md` | 18.A — Quick fixes: phone input and switch |
| `plans/pending/18-countries-a11y-diagnostics/phase-b-diagnostics.md` | 18.B — Diagnostics foundation (error · perf · debug · audit · accounting) |
| `plans/pending/18-countries-a11y-diagnostics/phase-c-contrast.md` | 18.C — Contrast & accessibility |
| `plans/pending/18-countries-a11y-diagnostics/phase-d-country-profiles.md` | 18.D — Country profiles: Egypt first, then Saudi |
| `plans/pending/18-countries-a11y-diagnostics/phase-e-address-picker.md` | 18.E — Address picker (`eg.json`, `sa.json`) |
| `plans/pending/18-countries-a11y-diagnostics/phase-f-accounting-debugger.md` | 18.F — Accounting debugger |
| `plans/pending/18-countries-a11y-diagnostics/phase-g-dev-loop.md` | 18.G — Close the dev loop |
| `plans/pending/18-countries-a11y-diagnostics/README.md` | 18 — Egypt + Saudi as real countries, address picker, phone/switch fixes, contrast, and a diagnostics system |
