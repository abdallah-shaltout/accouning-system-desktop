# AGENT_MEMORY

> **Generated** by `bun run memory` (scripts/memory). Do not edit by hand — re-run after structural changes
> (new module, service, route, Rust command, mock file, or moved folders). `bun run memory:check` fails when stale.

Indexed: **1227 files / 213,178 lines** (json 4, md 137, rust 377, ts 252, vue 457).

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
| Rounding rule (round2/round4, half away from zero) | `src/modules/core/helpers/numbers.ts` |
| Backend contract inventory (generated, `bun run contract`) | `docs/backend/contract/README.md` |

## Open diagnostics (docs/diagnostics — 18.G)

Known failures not yet fixed — check before starting work in an affected area. Full ledger: `docs/diagnostics/ISSUES.md` (regenerate with `bun run diag`; `bun run diag:check` is part of the definition of done).

**By kind:** | Kind | Open count |
|---|---|
| خلل | 18 |
| أداء | 2 |

**By area:** `onboarding (6)`, `setup-wizard-eg (5)`, `accounting (2)`, `branches-currencies (1)`, `full-persona-pass (1)`, `purchases (1)`, `reports-v2 (1)`, `settings (1)`, `ui (1)`, `window (1)`



| ID | Kind | Area | Status | Occurrences | Last seen | Debug namespace | File |
|---|---|---|---|---|---|---|---|
| `BUG-0001` | خلل | onboarding | مفتوح | 2 | 2026-09-26 |  | `docs/diagnostics/issues/BUG-0001-onboarding-flow-crashed-unhandled-except.md` |
| `BUG-0002` | خلل | setup-wizard-eg | مفتوح | 1 | 2026-09-26 |  | `docs/diagnostics/issues/BUG-0002-setup-wizard-eg-flow-crashed-unhandled-e.md` |
| `BUG-0004` | خلل | purchases | مفتوح | 3 | 2026-09-26 |  | `docs/diagnostics/issues/BUG-0004-purchases-flow-crashed-unhandled-excepti.md` |
| `BUG-0005` | خلل | branches-currencies | مفتوح | 3 | 2026-09-26 |  | `docs/diagnostics/issues/BUG-0005-branches-currencies-flow-failed-an-asser.md` |
| `BUG-0006` | خلل | reports-v2 | مفتوح | 1 | 2026-09-26 |  | `docs/diagnostics/issues/BUG-0006-reports-v2-flow-crashed-unhandled-except.md` |
| `BUG-0007` | خلل | full-persona-pass | مفتوح | 2 | 2026-09-26 |  | `docs/diagnostics/issues/BUG-0007-full-persona-pass-flow-failed-an-asserti.md` |
| `BUG-0008` | خلل | onboarding | مفتوح | 3 | 2026-09-26 |  | `docs/diagnostics/issues/BUG-0008-onboarding-flow-failed-an-assertion.md` |
| `BUG-0009` | خلل | onboarding | مفتوح | 1 | 2026-09-26 |  | `docs/diagnostics/issues/BUG-0009-onboarding-flow-crashed-unhandled-except.md` |
| `BUG-0010` | خلل | setup-wizard-eg | مفتوح | 1 | 2026-09-26 |  | `docs/diagnostics/issues/BUG-0010-setup-wizard-eg-flow-crashed-unhandled-e.md` |
| `BUG-0011` | خلل | onboarding | مفتوح | 1 | 2026-09-26 |  | `docs/diagnostics/issues/BUG-0011-onboarding-flow-crashed-unhandled-except.md` |
| `BUG-0012` | خلل | setup-wizard-eg | مفتوح | 3 | 2026-09-26 |  | `docs/diagnostics/issues/BUG-0012-setup-wizard-eg-flow-failed-an-assertion.md` |
| `BUG-0013` | خلل | setup-wizard-eg | مفتوح | 1 | 2026-09-26 |  | `docs/diagnostics/issues/BUG-0013-setup-wizard-eg-flow-crashed-unhandled-e.md` |
| `BUG-0014` | خلل | onboarding | مفتوح | 1 | 2026-09-26 |  | `docs/diagnostics/issues/BUG-0014-onboarding-flow-crashed-unhandled-except.md` |
| `BUG-0015` | خلل | accounting | مفتوح | 1 | 2026-09-26 |  | `docs/diagnostics/issues/BUG-0015-maximum-call-stack-size-exceeded.md` |
| `BUG-0016` | خلل | accounting | مفتوح | 1 | 2026-09-26 |  | `docs/diagnostics/issues/BUG-0016-maximum-call-stack-size-exceeded.md` |
| `BUG-0017` | خلل | ui | مفتوح | 1 | 2026-09-26 |  | `docs/diagnostics/issues/BUG-0017-failed-to-fetch-dynamically-imported-mod.md` |
| `BUG-0018` | خلل | settings | مفتوح | 1 | 2026-09-26 |  | `docs/diagnostics/issues/BUG-0018-failed-to-execute-structuredclone-on-win.md` |
| `BUG-0019` | خلل | window | مفتوح | 1 | 2026-09-26 |  | `docs/diagnostics/issues/BUG-0019-failed-to-fetch-dynamically-imported-mod.md` |
| `PERF-0001` | أداء | onboarding | مفتوح | 9 | 2026-09-26 |  | `docs/diagnostics/issues/PERF-0001-onboarding-flow-ran-218-slower-than-base.md` |
| `PERF-0002` | أداء | setup-wizard-eg | مفتوح | 8 | 2026-09-26 |  | `docs/diagnostics/issues/PERF-0002-setup-wizard-eg-flow-ran-567-slower-than.md` |

## Architecture (layers & data flow)

```text
pages (114) / components (380) / controllers (26)   src/modules/<domain>/…
        │  may call ONLY ▼                  (seam rule — see Boundary report)
services (45)   src/modules/<domain>/services/*   ← swap point for a real backend
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
| **accounting** | 13 / 3496 | commands 1, components 1, pages 7, routes 1, services 1, types 2 | 8 | yes |
| **analytics** | 9 / 499 | components 5, pages 1, routes 1, services 1, types 1 | 1 |  |
| **approvals** | 6 / 286 | commands 1, pages 1, routes 1, services 1, types 2 | 1 | yes |
| **core** | 353 / 20197 | commandPalette 1, components 295, controllers 14, data 2, helpers 16, pages 4, routes 1, services 12, types 8 | 5 |  |
| **diagnostics** | 21 / 2193 | commands 1, components 7, config 1, controllers 1, pages 1, services 8, types 2 | 0 | yes |
| **expenses** | 9 / 991 | pages 5, routes 1, services 1, types 2 | 5 |  |
| **invoices** | 58 / 7718 | commands 1, components 36, controllers 3, helpers 3, pages 10, routes 1, services 2, types 2 | 11 | yes |
| **parties** | 12 / 1921 | components 1, helpers 3, pages 3, routes 1, services 1, types 2, validators 1 | 8 |  |
| **payments** | 7 / 921 | pages 3, routes 1, services 1, types 2 | 3 |  |
| **products** | 33 / 5352 | components 8, controllers 1, helpers 1, pages 15, routes 1, services 4, types 2, validators 1 | 16 |  |
| **purchases** | 12 / 1803 | commands 1, components 1, pages 6, routes 1, services 1, types 2 | 7 | yes |
| **reports** | 45 / 6391 | commands 1, components 5, controllers 2, helpers 1, pages 28, print 4, routes 1, services 1, types 2 | 28 | yes |
| **settings** | 39 / 5667 | commands 1, components 6, controllers 4, helpers 2, pages 16, routes 1, services 4, types 5 | 19 | yes |
| **setup** | 25 / 2551 | components 15, pages 3, routes 1, services 3, types 2, validators 1 | 3 |  |
| **templates** | 5 / 969 | pages 2, services 1, types 2 | 0 |  |
| **users** | 12 / 1013 | controllers 1, helpers 1, pages 4, routes 1, services 2, types 2, validators 1 | 4 |  |
| **vouchers** | 10 / 879 | commands 1, pages 5, routes 1, services 1, types 2 | 5 | yes |

## Service API (the seam — pages call only these)

| Module | Service | Exports |
|---|---|---|
| accounting | `accountingService` | `signedBalance`, `getAccounts`, `accountPath`, `rolledBalance`, `saveAccount`, `deleteAccount`, `reparentAccount`, `getJournalEntries`, `getJournalEntriesForSource`, `getJournalEntriesPaged`, `getJournalEntry`, `createJournalEntry`, `updateJournalDraft`, `postJournalDraft`, `deleteJournalDraft`, `reverseJournalEntry`, `getFiscalYears`, `getCurrentFiscalYear`, `saveFiscalYear`, `getLockDate`, `saveLockDate`, `getCloseYearPreChecks`, `closeYear`, `reopenYear`, `getJournalTemplates`, `getJournalTemplate`, `createOrUpdateJournalTemplate`, `removeJournalTemplate`, `loadTemplateIntoEntry`, `postRecurringTemplate`, `getVatPeriodTotals`, `submitVatSettlement`, `payVatSettlementNow` |
| analytics | `analyticsService` | `getSalesAnalytics`, `getProductAnalytics`, `getCustomerAnalytics` |
| approvals | `approvalService` | `submitApprovalRequest`, `getApprovalRequests`, `getPendingApprovalCount`, `approveRequest`, `rejectRequest` |
| core | `attachmentService` | `fetchAttachments`, `fetchAttachment`, `saveAttachment`, `removeAttachment`, `uid` |
| core | `backend` | `usesRust`, `backendCall`, `initBackendBridge`, `getBackendStatus`, `ApiError` |
| core | `backendMirror` | `mirrored`, `clearMirrors` |
| core | `dashboardService` | `getInTransitTransfers`, `getPendingApprovalRequests`, `getLastBackupFailedAt`, `getJournalDraftCount`, `getStockValueSnapshot`, `hasAnyProducts`, `onLedgerChanged`, `getDashboardSummary`, `getLowStockProducts`, `getRecentInvoices`, `getRecentActivity`, `getHomeKpis`, `getTopProducts`, `getTopCustomers` |
| core | `devToolsService` | `resetToEmpty`, `reloadDemoData` |
| core | `geoService` | `getRegions`, `getCities`, `getDistricts`, `getLabels`, `searchPlaces` |
| core | `insightEngine` | `getThresholds`, `setThresholds`, `dismissInsight`, `snoozeInsight`, `clearDismissal`, `getInsights`, `getInsightsFor`, `getInsightsForEntity`, `getProductInlineHints`, `forceRefresh` |
| core | `insightRules` | `INSIGHT_ICONS`, `INSIGHT_RULES` |
| core | `insightTypes` | `DEFAULT_THRESHOLDS` |
| core | `pdfService` | `render`, `renderAndSave`, `renderPreview`, `sampleInvoicePayload`, `buildLabelItems`, `renderLabels`, `renderLabelsAndSave`, `renderGenericReport`, `renderGenericReportAndSave`, `renderReportPdf`, `saveReportPdf`, `renderLabelsPreview` |
| core | `printService` | `listPrinters`, `printReceipt`, `testPrint`, `initPrintResultListener` |
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
| invoices | `invoiceImageService` | `saveInvoiceImage`, `copyInvoiceImage` |
| invoices | `invoiceService` | `isOverdue`, `getInvoices`, `getInvoicesPaged`, `getInvoice`, `previewSale`, `createSale`, `createRefund`, `getRefund`, `getInvoicePrintData`, `getCurrentShift`, `getShifts`, `getShift`, `openPosShift`, `getXReport`, `closePosShift`, `forceClosePosShift`, `recordCashInOut`, `getHeldSales`, `holdSale`, `resumeHeldSale`, `discardHeldSale`, `getQuotations`, `getQuotation`, `saveQuotation`, `setQuotationStatus`, `convertQuotationToInvoice` |
| parties | `partyService` | `findDuplicates`, `checkDuplicates`, `getPartyGroups`, `getCustomers`, `getCustomer`, `saveCustomer`, `getCustomerStatement`, `getSuppliers`, `getSupplier`, `saveSupplier`, `getSupplierStatement`, `linkPartyRecords`, `unlinkPartyRecord`, `getLinkedNetBalance`, `getPartyHistory`, `getPartyAging`, `ApiError`, `uid` |
| payments | `paymentService` | `getPayments`, `getPaymentsPaged`, `getPayment`, `createPayment`, `allocateExistingPayment`, `removeAllocation`, `getOpenDocuments` |
| products | `catalogService` | `onCatalogChanged`, `getCategories`, `saveCategory`, `deleteCategory`, `getUnits`, `saveUnit`, `applyUnitPreset`, `deleteUnit`, `getPriceLists`, `savePriceList`, `deletePriceList`, `setPriceListValues`, `getCustomFieldDefs`, `saveCustomFieldDef`, `deleteCustomFieldDef`, `reorderCustomFieldDefs` |
| products | `inventoryService` | `adjustmentValue`, `getStockAdjustments`, `getStockAdjustment`, `createStockAdjustment`, `completeAdjustment`, `deleteDraftAdjustment`, `getStockMovements`, `getStockMovementsPaged`, `getBatches`, `getExpiryReport`, `batchAlertTone`, `writeOffExpiredBatches`, `returnBatchesToSupplier`, `getDebitNoteDrafts`, `getStockCounts`, `getStockCount`, `createStockCount`, `updateStockCountLine`, `submitCountForReview`, `resumeCounting`, `completeStockCount` |
| products | `productService` | `rememberBranchStock`, `branchStockFromCache`, `isLowStock`, `getProducts`, `getProduct`, `findByCode`, `generateEan13`, `createProduct`, `updateProduct`, `suggestSku` |
| products | `transferService` | `getTransfers`, `getTransfer`, `createTransfer`, `sendTransfer`, `receiveTransfer`, `rejectTransfer`, `branchStockQty` |
| purchases | `purchaseService` | `getPurchaseOrders`, `getPurchaseOrder`, `savePurchaseOrder`, `sendPurchaseOrderToSupplier`, `receivePurchaseOrder`, `confirmPurchaseOrder`, `cancelPurchaseOrder`, `createPurchaseReturn`, `getPurchaseReturn`, `getActiveBatches`, `getDebitNoteDrafts`, `postDebitNoteDraft`, `computePurchaseTotals` |
| reports | `reportService` | `getTrialBalance`, `getProfitAndLoss`, `getProfitAndLossComparison`, `getCostCenterProfitAndLoss`, `getCostCenterBudgetVsActual`, `getBalanceSheet`, `getAccountLedger`, `getPartyLedger`, `getSalesReport`, `getInventoryReport`, `getVatReport`, `getVatDetail`, `getLedgerTargets`, `getCashFlowStatement`, `getDayBook`, `getAgingReport`, `getOverdueReport`, `getGrossProfitReport`, `getReturnsReport`, `getDiscountsReport`, `getShiftsReport`, `getLowStockReport`, `getDeadStockReport`, `getStocktakeVariances`, `getTransfersReport`, `getPurchasesReport`, `getExpensesReport`, `getPeriodComparison`, `getBranchComparison`, `getBusinessHealthReport`, `getProfitLeakageReport`, `getDimensionOptions` |
| settings | `backupService` | `backupSettings`, `saveBackupSettings`, `isTauriMode`, `previewBackupCounts`, `backupNow`, `listHistory`, `deleteHistoryEntry`, `verifyHistoryEntry`, `pickRestoreFile`, `previewRestore`, `restoreFromArchive`, `isClosingWithBackup`, `initAutoBackup`, `stopAutoBackup`, `pickBackupFolder` |
| settings | `branchesService` | `getBranches`, `createBranch`, `updateBranch`, `deactivateBranch`, `reactivateBranch`, `getCostCenters`, `createCostCenter`, `updateCostCenter`, `deleteCostCenter`, `getCurrencies`, `getExchangeRates`, `createCurrency`, `updateCurrency`, `saveExchangeRate`, `isBaseCurrencyLocked`, `setBaseCurrency`, `getRevaluationPreview`, `getDefaultRevaluationRates`, `postRevaluation` |
| settings | `networkService` | `getLanSharingStatus`, `enableLanSharing`, `disableLanSharing`, `rotatePairingCode`, `reconnectBackend` |
| settings | `settingsService` | `getSettings`, `updateSettings`, `getTaxes`, `saveTax`, `deleteTax`, `getPaymentMethods`, `savePaymentMethod`, `reorderPaymentMethods`, `deletePaymentMethod` |
| setup | `deviceService` | `refreshDeviceSetupState`, `ensureDeviceSetupState`, `isFreshInstallCached`, `getDeviceSetupState`, `provisionMainDevice`, `pairTerminalDevice` |
| setup | `legacyImportService` | `hasLegacySnapshot`, `inspectLegacySnapshot`, `importLegacySnapshot` |
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
| settings | `/settings/network` | settings-network | الشبكة وقاعدة البيانات | settings | NetworkSettingsPage.vue |
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
| setup | `/device-setup` | device-setup | إعداد الجهاز |  | DeviceSetupPage.vue |
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
| **accounting** | core (108), users (6), mocks (4), parties (4), reports (3), settings (2), diagnostics (1), invoices (1) | 10 |
| **analytics** | core (18), diagnostics (1), mocks (1) | 1 |
| **app** | core (16), settings (7), diagnostics (4), users (4), accounting (2), approvals (2), invoices (2), purchases (2), reports (2), setup (2), vouchers (2), analytics (1), expenses (1), mocks (1), parties (1), payments (1), products (1) | 1 |
| **approvals** | core (14), mocks (2), diagnostics (1), users (1) | 5 |
| **core** | mocks (20), users (20), products (14), invoices (13), settings (12), diagnostics (10), parties (4), purchases (3), accounting (2), templates (2), vouchers (2), app (1), approvals (1), payments (1), reports (1), setup (1) | 18 |
| **diagnostics** | core (31), mocks (5), accounting (1) | 18 |
| **expenses** | core (61), accounting (3), users (3), mocks (2), parties (2), settings (2), diagnostics (1) | 2 |
| **invoices** | core (222), products (11), settings (11), mocks (9), users (9), parties (8), reports (6), approvals (2), diagnostics (2), accounting (1), payments (1) | 10 |
| **mocks** | core (14), invoices (9), accounting (8), products (8), settings (6), diagnostics (4), vouchers (3), approvals (2), expenses (2), parties (2), payments (2), purchases (2), setup (2), users (1) | 18 |
| **parties** | core (63), mocks (6), payments (3), users (2), diagnostics (1), invoices (1), purchases (1), settings (1), setup (1) | 10 |
| **payments** | core (44), invoices (2), mocks (2), parties (2), users (2), diagnostics (1) | 6 |
| **products** | core (211), mocks (14), users (13), settings (8), diagnostics (4), accounting (3), purchases (2), approvals (1), parties (1), templates (1) | 8 |
| **purchases** | core (81), products (5), users (4), invoices (3), mocks (3), parties (3), settings (2), diagnostics (1), payments (1) | 5 |
| **reports** | core (165), settings (7), accounting (5), mocks (4), diagnostics (1), invoices (1), users (1) | 4 |
| **settings** | core (196), users (19), mocks (15), diagnostics (9), invoices (6), products (3), templates (2) | 12 |
| **setup** | core (74), mocks (9), settings (4), diagnostics (3), accounting (2), products (2), parties (1), users (1) | 5 |
| **templates** | core (18), diagnostics (1), mocks (1) | 3 |
| **users** | core (40), mocks (5), products (3), diagnostics (2), setup (1) | 15 |
| **vouchers** | core (50), mocks (3), accounting (2), settings (2), users (2), diagnostics (1), invoices (1) | 3 |

**Most-used npm packages** (files importing): `vue (425)`, `@lucide/vue (174)`, `reka-ui (118)`, `vue-router (100)`, `@vueuse/core (79)`, `@tauri-apps/api (18)`, `pinia (10)`, `class-variance-authority (9)`, `zod (6)`, `@tauri-apps/plugin-dialog (4)`, `@tauri-apps/plugin-fs (4)`, `fflate (4)`, `uqr (3)`, `@fontsource-variable/cairo (2)`, `@fontsource/ibm-plex-sans-arabic (2)`, `@internationalized/date (2)`, `@tauri-apps/plugin-opener (2)`, `exceljs (2)`, `@fontsource/noto-naskh-arabic (1)`, `@fontsource/tajawal (1)`, `bwip-js (1)`, `clsx (1)`, `libphonenumber-js (1)`, `modern-screenshot (1)`, `tailwind-merge (1)` … +1 more

## Rust ↔ Vue IPC contract

| Command | Rust path | Defined in | Registered | Invoked from |
|---|---|---|---|---|
| `diag_append` | `core::diag::diag_append` | `src-tauri/src/core/diag/mod.rs` | yes | `src/modules/diagnostics/services/diagnosticsService.ts` |
| `diag_read` | `core::diag::diag_read` | `src-tauri/src/core/diag/mod.rs` | yes | `src/modules/diagnostics/services/diagnosticsService.ts` |
| `diag_clear` | `core::diag::diag_clear` | `src-tauri/src/core/diag/mod.rs` | yes | `src/modules/diagnostics/services/diagnosticsService.ts` |
| `diag_open_folder` | `core::diag::diag_open_folder` | `src-tauri/src/core/diag/mod.rs` | yes | `src/modules/diagnostics/services/diagnosticsService.ts` |
| `diag_rotate` | `core::diag::diag_rotate` | `src-tauri/src/core/diag/mod.rs` | yes | `src/modules/diagnostics/services/diagnosticsService.ts` |
| `core_backend_status` | `core::status::core_backend_status` | `src-tauri/src/core/status.rs` | yes | `src/modules/core/services/backend.ts` |
| `accounting_get_accounts` | `domains::accounting::commands::accounts::accounting_get_accounts` | `src-tauri/src/domains/accounting/commands/accounts.rs` | yes | `src/modules/accounting/services/accountingService.ts` |
| `accounting_save_account` | `domains::accounting::commands::accounts::accounting_save_account` | `src-tauri/src/domains/accounting/commands/accounts.rs` | yes | `src/modules/accounting/services/accountingService.ts` |
| `accounting_delete_account` | `domains::accounting::commands::accounts::accounting_delete_account` | `src-tauri/src/domains/accounting/commands/accounts.rs` | yes | `src/modules/accounting/services/accountingService.ts` |
| `accounting_reparent_account` | `domains::accounting::commands::accounts::accounting_reparent_account` | `src-tauri/src/domains/accounting/commands/accounts.rs` | yes | `src/modules/accounting/services/accountingService.ts` |
| `accounting_get_journal_entries` | `domains::accounting::commands::journal::accounting_get_journal_entries` | `src-tauri/src/domains/accounting/commands/journal.rs` | yes | `src/modules/accounting/services/accountingService.ts` |
| `accounting_get_journal_entries_for_source` | `domains::accounting::commands::journal::accounting_get_journal_entries_for_source` | `src-tauri/src/domains/accounting/commands/journal.rs` | yes | `src/modules/accounting/services/accountingService.ts` |
| `accounting_get_journal_entries_paged` | `domains::accounting::commands::journal::accounting_get_journal_entries_paged` | `src-tauri/src/domains/accounting/commands/journal.rs` | yes | `src/modules/accounting/services/accountingService.ts` |
| `accounting_get_journal_entry` | `domains::accounting::commands::journal::accounting_get_journal_entry` | `src-tauri/src/domains/accounting/commands/journal.rs` | yes | `src/modules/accounting/services/accountingService.ts` |
| `accounting_create_journal_entry` | `domains::accounting::commands::journal::accounting_create_journal_entry` | `src-tauri/src/domains/accounting/commands/journal.rs` | yes | `src/modules/accounting/services/accountingService.ts` |
| `accounting_update_journal_draft` | `domains::accounting::commands::journal::accounting_update_journal_draft` | `src-tauri/src/domains/accounting/commands/journal.rs` | yes | `src/modules/accounting/services/accountingService.ts` |
| `accounting_post_journal_draft` | `domains::accounting::commands::journal::accounting_post_journal_draft` | `src-tauri/src/domains/accounting/commands/journal.rs` | yes | `src/modules/accounting/services/accountingService.ts` |
| `accounting_delete_journal_draft` | `domains::accounting::commands::journal::accounting_delete_journal_draft` | `src-tauri/src/domains/accounting/commands/journal.rs` | yes | `src/modules/accounting/services/accountingService.ts` |
| `accounting_reverse_journal_entry` | `domains::accounting::commands::journal::accounting_reverse_journal_entry` | `src-tauri/src/domains/accounting/commands/journal.rs` | yes | `src/modules/accounting/services/accountingService.ts` |
| `accounting_get_fiscal_years` | `domains::accounting::commands::period::accounting_get_fiscal_years` | `src-tauri/src/domains/accounting/commands/period.rs` | yes | `src/modules/accounting/services/accountingService.ts` |
| `accounting_get_current_fiscal_year` | `domains::accounting::commands::period::accounting_get_current_fiscal_year` | `src-tauri/src/domains/accounting/commands/period.rs` | yes | `src/modules/accounting/services/accountingService.ts` |
| `accounting_save_fiscal_year` | `domains::accounting::commands::period::accounting_save_fiscal_year` | `src-tauri/src/domains/accounting/commands/period.rs` | yes | `src/modules/accounting/services/accountingService.ts` |
| `accounting_get_lock_date` | `domains::accounting::commands::period::accounting_get_lock_date` | `src-tauri/src/domains/accounting/commands/period.rs` | yes | `src/modules/accounting/services/accountingService.ts` |
| `accounting_save_lock_date` | `domains::accounting::commands::period::accounting_save_lock_date` | `src-tauri/src/domains/accounting/commands/period.rs` | yes | `src/modules/accounting/services/accountingService.ts` |
| `accounting_get_close_year_pre_checks` | `domains::accounting::commands::period::accounting_get_close_year_pre_checks` | `src-tauri/src/domains/accounting/commands/period.rs` | yes | `src/modules/accounting/services/accountingService.ts` |
| `accounting_close_year` | `domains::accounting::commands::period::accounting_close_year` | `src-tauri/src/domains/accounting/commands/period.rs` | yes | `src/modules/accounting/services/accountingService.ts` |
| `accounting_reopen_year` | `domains::accounting::commands::period::accounting_reopen_year` | `src-tauri/src/domains/accounting/commands/period.rs` | yes | `src/modules/accounting/services/accountingService.ts` |
| `accounting_get_vat_period_totals` | `domains::accounting::commands::period::accounting_get_vat_period_totals` | `src-tauri/src/domains/accounting/commands/period.rs` | yes | `src/modules/accounting/services/accountingService.ts` |
| `accounting_submit_vat_settlement` | `domains::accounting::commands::period::accounting_submit_vat_settlement` | `src-tauri/src/domains/accounting/commands/period.rs` | yes | `src/modules/accounting/services/accountingService.ts` |
| `accounting_pay_vat_settlement_now` | `domains::accounting::commands::period::accounting_pay_vat_settlement_now` | `src-tauri/src/domains/accounting/commands/period.rs` | yes | `src/modules/accounting/services/accountingService.ts` |
| `accounting_get_journal_templates` | `domains::accounting::commands::templates::accounting_get_journal_templates` | `src-tauri/src/domains/accounting/commands/templates.rs` | yes | `src/modules/accounting/services/accountingService.ts` |
| `accounting_get_journal_template` | `domains::accounting::commands::templates::accounting_get_journal_template` | `src-tauri/src/domains/accounting/commands/templates.rs` | yes | `src/modules/accounting/services/accountingService.ts` |
| `accounting_create_or_update_journal_template` | `domains::accounting::commands::templates::accounting_create_or_update_journal_template` | `src-tauri/src/domains/accounting/commands/templates.rs` | yes | `src/modules/accounting/services/accountingService.ts` |
| `accounting_remove_journal_template` | `domains::accounting::commands::templates::accounting_remove_journal_template` | `src-tauri/src/domains/accounting/commands/templates.rs` | yes | `src/modules/accounting/services/accountingService.ts` |
| `accounting_load_template_into_entry` | `domains::accounting::commands::templates::accounting_load_template_into_entry` | `src-tauri/src/domains/accounting/commands/templates.rs` | yes | `src/modules/accounting/services/accountingService.ts` |
| `accounting_post_recurring_template` | `domains::accounting::commands::templates::accounting_post_recurring_template` | `src-tauri/src/domains/accounting/commands/templates.rs` | yes | `src/modules/accounting/services/accountingService.ts` |
| `analytics_get_sales_analytics` | `domains::analytics::commands::analytics_get_sales_analytics` | `src-tauri/src/domains/analytics/commands.rs` | yes | `src/modules/analytics/services/analyticsService.ts` |
| `analytics_get_product_analytics` | `domains::analytics::commands::analytics_get_product_analytics` | `src-tauri/src/domains/analytics/commands.rs` | yes | `src/modules/analytics/services/analyticsService.ts` |
| `analytics_get_customer_analytics` | `domains::analytics::commands::analytics_get_customer_analytics` | `src-tauri/src/domains/analytics/commands.rs` | yes | `src/modules/analytics/services/analyticsService.ts` |
| `approvals_submit_approval_request` | `domains::approvals::commands::approvals_submit_approval_request` | `src-tauri/src/domains/approvals/commands.rs` | yes | `src/modules/approvals/services/approvalService.ts` |
| `approvals_get_approval_requests` | `domains::approvals::commands::approvals_get_approval_requests` | `src-tauri/src/domains/approvals/commands.rs` | yes | `src/modules/approvals/services/approvalService.ts` |
| `approvals_get_pending_approval_count` | `domains::approvals::commands::approvals_get_pending_approval_count` | `src-tauri/src/domains/approvals/commands.rs` | yes | `src/modules/approvals/services/approvalService.ts` |
| `approvals_approve_request` | `domains::approvals::commands::approvals_approve_request` | `src-tauri/src/domains/approvals/commands.rs` | yes | `src/modules/approvals/services/approvalService.ts` |
| `approvals_reject_request` | `domains::approvals::commands::approvals_reject_request` | `src-tauri/src/domains/approvals/commands.rs` | yes | `src/modules/approvals/services/approvalService.ts` |
| `dashboard_get_dashboard_summary` | `domains::dashboard::commands::dashboard_get_dashboard_summary` | `src-tauri/src/domains/dashboard/commands.rs` | yes | `src/modules/core/services/dashboardService.ts` |
| `dashboard_get_home_kpis` | `domains::dashboard::commands::dashboard_get_home_kpis` | `src-tauri/src/domains/dashboard/commands.rs` | yes | `src/modules/core/services/dashboardService.ts` |
| `dashboard_get_low_stock_products` | `domains::dashboard::commands::dashboard_get_low_stock_products` | `src-tauri/src/domains/dashboard/commands.rs` | yes | `src/modules/core/services/dashboardService.ts` |
| `dashboard_get_recent_invoices` | `domains::dashboard::commands::dashboard_get_recent_invoices` | `src-tauri/src/domains/dashboard/commands.rs` | yes | `src/modules/core/services/dashboardService.ts` |
| `dashboard_get_recent_activity` | `domains::dashboard::commands::dashboard_get_recent_activity` | `src-tauri/src/domains/dashboard/commands.rs` | yes | `src/modules/core/services/dashboardService.ts` |
| `dashboard_get_top_products` | `domains::dashboard::commands::dashboard_get_top_products` | `src-tauri/src/domains/dashboard/commands.rs` | yes | `src/modules/core/services/dashboardService.ts` |
| `dashboard_get_top_customers` | `domains::dashboard::commands::dashboard_get_top_customers` | `src-tauri/src/domains/dashboard/commands.rs` | yes | `src/modules/core/services/dashboardService.ts` |
| `dashboard_get_in_transit_transfers` | `domains::dashboard::commands::dashboard_get_in_transit_transfers` | `src-tauri/src/domains/dashboard/commands.rs` | yes | `src/modules/core/services/dashboardService.ts` |
| `dashboard_get_pending_approval_requests` | `domains::dashboard::commands::dashboard_get_pending_approval_requests` | `src-tauri/src/domains/dashboard/commands.rs` | yes | `src/modules/core/services/dashboardService.ts` |
| `dashboard_get_last_backup_failed_at` | `domains::dashboard::commands::dashboard_get_last_backup_failed_at` | `src-tauri/src/domains/dashboard/commands.rs` | yes | `src/modules/core/services/dashboardService.ts` |
| `dashboard_get_journal_draft_count` | `domains::dashboard::commands::dashboard_get_journal_draft_count` | `src-tauri/src/domains/dashboard/commands.rs` | yes | `src/modules/core/services/dashboardService.ts` |
| `dashboard_get_stock_value_snapshot` | `domains::dashboard::commands::dashboard_get_stock_value_snapshot` | `src-tauri/src/domains/dashboard/commands.rs` | yes | `src/modules/core/services/dashboardService.ts` |
| `dashboard_has_any_products` | `domains::dashboard::commands::dashboard_has_any_products` | `src-tauri/src/domains/dashboard/commands.rs` | yes | `src/modules/core/services/dashboardService.ts` |
| `dashboard_compute_insights` | `domains::dashboard::commands::dashboard_compute_insights` | `src-tauri/src/domains/dashboard/commands.rs` | yes | `src/modules/core/services/insightEngine.ts` |
| `dashboard_get_product_inline_hints` | `domains::dashboard::commands::dashboard_get_product_inline_hints` | `src-tauri/src/domains/dashboard/commands.rs` | yes | `src/modules/core/services/insightEngine.ts` |
| `diagnostics_get_audit_entries` | `domains::diagnostics::commands::diagnostics_get_audit_entries` | `src-tauri/src/domains/diagnostics/commands.rs` | yes | `src/modules/diagnostics/services/auditService.ts` |
| `diagnostics_get_audit_entities` | `domains::diagnostics::commands::diagnostics_get_audit_entities` | `src-tauri/src/domains/diagnostics/commands.rs` | yes | `src/modules/diagnostics/services/auditService.ts` |
| `diagnostics_export_support_bundle` | `domains::diagnostics::commands::diagnostics_export_support_bundle` | `src-tauri/src/domains/diagnostics/commands.rs` | yes | `src/modules/diagnostics/services/supportBundleService.ts` |
| `diagnostics_list_recent_documents` | `domains::diagnostics::commands::diagnostics_list_recent_documents` | `src-tauri/src/domains/diagnostics/commands.rs` | yes | `src/modules/diagnostics/services/accountingDebugService.ts` |
| `diagnostics_get_posting_trace` | `domains::diagnostics::commands::diagnostics_get_posting_trace` | `src-tauri/src/domains/diagnostics/commands.rs` | yes | `src/modules/diagnostics/services/accountingDebugService.ts` |
| `diagnostics_get_journal_entry_raw` | `domains::diagnostics::commands::diagnostics_get_journal_entry_raw` | `src-tauri/src/domains/diagnostics/commands.rs` | yes | `src/modules/diagnostics/services/accountingDebugService.ts` |
| `diagnostics_get_balances_around` | `domains::diagnostics::commands::diagnostics_get_balances_around` | `src-tauri/src/domains/diagnostics/commands.rs` | yes | `src/modules/diagnostics/services/accountingDebugService.ts` |
| `diagnostics_get_invariant_results` | `domains::diagnostics::commands::diagnostics_get_invariant_results` | `src-tauri/src/domains/diagnostics/commands.rs` | yes | `src/modules/diagnostics/services/accountingDebugService.ts` |
| `diagnostics_get_drift_report` | `domains::diagnostics::commands::diagnostics_get_drift_report` | `src-tauri/src/domains/diagnostics/commands.rs` | yes | `src/modules/diagnostics/services/accountingDebugService.ts` |
| `diagnostics_explain_account_balance` | `domains::diagnostics::commands::diagnostics_explain_account_balance` | `src-tauri/src/domains/diagnostics/commands.rs` | yes | `src/modules/diagnostics/services/accountingDebugService.ts` |
| `expenses_get_expense_categories` | `domains::expenses::commands::expenses_get_expense_categories` | `src-tauri/src/domains/expenses/commands.rs` | yes | `src/modules/expenses/services/expenseService.ts` |
| `expenses_save_expense_category` | `domains::expenses::commands::expenses_save_expense_category` | `src-tauri/src/domains/expenses/commands.rs` | yes | `src/modules/expenses/services/expenseService.ts` |
| `expenses_delete_expense_category` | `domains::expenses::commands::expenses_delete_expense_category` | `src-tauri/src/domains/expenses/commands.rs` | yes | `src/modules/expenses/services/expenseService.ts` |
| `expenses_get_expenses` | `domains::expenses::commands::expenses_get_expenses` | `src-tauri/src/domains/expenses/commands.rs` | yes | `src/modules/expenses/services/expenseService.ts` |
| `expenses_get_expense` | `domains::expenses::commands::expenses_get_expense` | `src-tauri/src/domains/expenses/commands.rs` | yes | `src/modules/expenses/services/expenseService.ts` |
| `expenses_create_expense` | `domains::expenses::commands::expenses_create_expense` | `src-tauri/src/domains/expenses/commands.rs` | yes | `src/modules/expenses/services/expenseService.ts` |
| `expenses_get_recurring_expenses` | `domains::expenses::commands::expenses_get_recurring_expenses` | `src-tauri/src/domains/expenses/commands.rs` | yes | `src/modules/expenses/services/expenseService.ts` |
| `expenses_save_recurring_expense` | `domains::expenses::commands::expenses_save_recurring_expense` | `src-tauri/src/domains/expenses/commands.rs` | yes | `src/modules/expenses/services/expenseService.ts` |
| `expenses_delete_recurring_expense` | `domains::expenses::commands::expenses_delete_recurring_expense` | `src-tauri/src/domains/expenses/commands.rs` | yes | `src/modules/expenses/services/expenseService.ts` |
| `expenses_get_due_recurring_expenses` | `domains::expenses::commands::expenses_get_due_recurring_expenses` | `src-tauri/src/domains/expenses/commands.rs` | yes | `src/modules/expenses/services/expenseService.ts` |
| `expenses_post_due_recurring_expense` | `domains::expenses::commands::expenses_post_due_recurring_expense` | `src-tauri/src/domains/expenses/commands.rs` | yes | `src/modules/expenses/services/expenseService.ts` |
| `invoices_get_invoices` | `domains::invoices::commands::invoices_get_invoices` | `src-tauri/src/domains/invoices/commands.rs` | yes | `src/modules/invoices/services/invoiceService.ts` |
| `invoices_get_invoices_paged` | `domains::invoices::commands::invoices_get_invoices_paged` | `src-tauri/src/domains/invoices/commands.rs` | yes | `src/modules/invoices/services/invoiceService.ts` |
| `invoices_get_invoice` | `domains::invoices::commands::invoices_get_invoice` | `src-tauri/src/domains/invoices/commands.rs` | yes | `src/modules/invoices/services/invoiceService.ts` |
| `invoices_preview_sale` | `domains::invoices::commands::invoices_preview_sale` | `src-tauri/src/domains/invoices/commands.rs` | yes | `src/modules/invoices/services/invoiceService.ts` |
| `invoices_create_sale` | `domains::invoices::commands::invoices_create_sale` | `src-tauri/src/domains/invoices/commands.rs` | yes | `src/modules/invoices/services/invoiceService.ts` |
| `invoices_create_refund` | `domains::invoices::commands::invoices_create_refund` | `src-tauri/src/domains/invoices/commands.rs` | yes | `src/modules/invoices/services/invoiceService.ts` |
| `invoices_get_refund` | `domains::invoices::commands::invoices_get_refund` | `src-tauri/src/domains/invoices/commands.rs` | yes | `src/modules/invoices/services/invoiceService.ts` |
| `invoices_get_invoice_print_data` | `domains::invoices::commands::invoices_get_invoice_print_data` | `src-tauri/src/domains/invoices/commands.rs` | yes | `src/modules/invoices/services/invoiceService.ts` |
| `invoices_get_quotations` | `domains::invoices::commands::invoices_get_quotations` | `src-tauri/src/domains/invoices/commands.rs` | yes | `src/modules/invoices/services/invoiceService.ts` |
| `invoices_get_quotation` | `domains::invoices::commands::invoices_get_quotation` | `src-tauri/src/domains/invoices/commands.rs` | yes | `src/modules/invoices/services/invoiceService.ts` |
| `invoices_save_quotation` | `domains::invoices::commands::invoices_save_quotation` | `src-tauri/src/domains/invoices/commands.rs` | yes | `src/modules/invoices/services/invoiceService.ts` |
| `invoices_set_quotation_status` | `domains::invoices::commands::invoices_set_quotation_status` | `src-tauri/src/domains/invoices/commands.rs` | yes | `src/modules/invoices/services/invoiceService.ts` |
| `invoices_convert_quotation_to_invoice` | `domains::invoices::commands::invoices_convert_quotation_to_invoice` | `src-tauri/src/domains/invoices/commands.rs` | yes | `src/modules/invoices/services/invoiceService.ts` |
| `invoices_get_current_shift` | `domains::invoices::commands::invoices_get_current_shift` | `src-tauri/src/domains/invoices/commands.rs` | yes | `src/modules/invoices/services/invoiceService.ts` |
| `invoices_get_shifts` | `domains::invoices::commands::invoices_get_shifts` | `src-tauri/src/domains/invoices/commands.rs` | yes | `src/modules/invoices/services/invoiceService.ts` |
| `invoices_get_shift` | `domains::invoices::commands::invoices_get_shift` | `src-tauri/src/domains/invoices/commands.rs` | yes | `src/modules/invoices/services/invoiceService.ts` |
| `invoices_open_pos_shift` | `domains::invoices::commands::invoices_open_pos_shift` | `src-tauri/src/domains/invoices/commands.rs` | yes | `src/modules/invoices/services/invoiceService.ts` |
| `invoices_get_x_report` | `domains::invoices::commands::invoices_get_x_report` | `src-tauri/src/domains/invoices/commands.rs` | yes | `src/modules/invoices/services/invoiceService.ts` |
| `invoices_close_pos_shift` | `domains::invoices::commands::invoices_close_pos_shift` | `src-tauri/src/domains/invoices/commands.rs` | yes | `src/modules/invoices/services/invoiceService.ts` |
| `invoices_force_close_pos_shift` | `domains::invoices::commands::invoices_force_close_pos_shift` | `src-tauri/src/domains/invoices/commands.rs` | yes | `src/modules/invoices/services/invoiceService.ts` |
| `invoices_record_cash_in_out` | `domains::invoices::commands::invoices_record_cash_in_out` | `src-tauri/src/domains/invoices/commands.rs` | yes | `src/modules/invoices/services/invoiceService.ts` |
| `invoices_get_held_sales` | `domains::invoices::commands::invoices_get_held_sales` | `src-tauri/src/domains/invoices/commands.rs` | yes | `src/modules/invoices/services/invoiceService.ts` |
| `invoices_hold_sale` | `domains::invoices::commands::invoices_hold_sale` | `src-tauri/src/domains/invoices/commands.rs` | yes | `src/modules/invoices/services/invoiceService.ts` |
| `invoices_resume_held_sale` | `domains::invoices::commands::invoices_resume_held_sale` | `src-tauri/src/domains/invoices/commands.rs` | yes | `src/modules/invoices/services/invoiceService.ts` |
| `invoices_discard_held_sale` | `domains::invoices::commands::invoices_discard_held_sale` | `src-tauri/src/domains/invoices/commands.rs` | yes | `src/modules/invoices/services/invoiceService.ts` |
| `parties_check_duplicates` | `domains::parties::commands::parties_check_duplicates` | `src-tauri/src/domains/parties/commands.rs` | yes | `src/modules/parties/services/partyService.ts` |
| `parties_get_party_groups` | `domains::parties::commands::parties_get_party_groups` | `src-tauri/src/domains/parties/commands.rs` | yes | `src/modules/parties/services/partyService.ts` |
| `parties_get_customers` | `domains::parties::commands::parties_get_customers` | `src-tauri/src/domains/parties/commands.rs` | yes | `src/modules/parties/services/partyService.ts` |
| `parties_get_customer` | `domains::parties::commands::parties_get_customer` | `src-tauri/src/domains/parties/commands.rs` | yes | `src/modules/parties/services/partyService.ts` |
| `parties_save_customer` | `domains::parties::commands::parties_save_customer` | `src-tauri/src/domains/parties/commands.rs` | yes | `src/modules/parties/services/partyService.ts` |
| `parties_get_customer_statement` | `domains::parties::commands::parties_get_customer_statement` | `src-tauri/src/domains/parties/commands.rs` | yes | `src/modules/parties/services/partyService.ts` |
| `parties_get_suppliers` | `domains::parties::commands::parties_get_suppliers` | `src-tauri/src/domains/parties/commands.rs` | yes | `src/modules/parties/services/partyService.ts` |
| `parties_get_supplier` | `domains::parties::commands::parties_get_supplier` | `src-tauri/src/domains/parties/commands.rs` | yes | `src/modules/parties/services/partyService.ts` |
| `parties_save_supplier` | `domains::parties::commands::parties_save_supplier` | `src-tauri/src/domains/parties/commands.rs` | yes | `src/modules/parties/services/partyService.ts` |
| `parties_get_supplier_statement` | `domains::parties::commands::parties_get_supplier_statement` | `src-tauri/src/domains/parties/commands.rs` | yes | `src/modules/parties/services/partyService.ts` |
| `parties_link_party_records` | `domains::parties::commands::parties_link_party_records` | `src-tauri/src/domains/parties/commands.rs` | yes | `src/modules/parties/services/partyService.ts` |
| `parties_unlink_party_record` | `domains::parties::commands::parties_unlink_party_record` | `src-tauri/src/domains/parties/commands.rs` | yes | `src/modules/parties/services/partyService.ts` |
| `parties_get_linked_net_balance` | `domains::parties::commands::parties_get_linked_net_balance` | `src-tauri/src/domains/parties/commands.rs` | yes | `src/modules/parties/services/partyService.ts` |
| `parties_get_party_history` | `domains::parties::commands::parties_get_party_history` | `src-tauri/src/domains/parties/commands.rs` | yes | `src/modules/parties/services/partyService.ts` |
| `parties_get_party_aging` | `domains::parties::commands::parties_get_party_aging` | `src-tauri/src/domains/parties/commands.rs` | yes | `src/modules/parties/services/partyService.ts` |
| `payments_get_payments` | `domains::payments::commands::payments_get_payments` | `src-tauri/src/domains/payments/commands.rs` | yes | `src/modules/payments/services/paymentService.ts` |
| `payments_get_payments_paged` | `domains::payments::commands::payments_get_payments_paged` | `src-tauri/src/domains/payments/commands.rs` | yes | `src/modules/payments/services/paymentService.ts` |
| `payments_get_payment` | `domains::payments::commands::payments_get_payment` | `src-tauri/src/domains/payments/commands.rs` | yes | `src/modules/payments/services/paymentService.ts` |
| `payments_create_payment` | `domains::payments::commands::payments_create_payment` | `src-tauri/src/domains/payments/commands.rs` | yes | `src/modules/payments/services/paymentService.ts` |
| `payments_allocate_existing_payment` | `domains::payments::commands::payments_allocate_existing_payment` | `src-tauri/src/domains/payments/commands.rs` | yes | `src/modules/payments/services/paymentService.ts` |
| `payments_remove_allocation` | `domains::payments::commands::payments_remove_allocation` | `src-tauri/src/domains/payments/commands.rs` | yes | `src/modules/payments/services/paymentService.ts` |
| `payments_get_open_documents` | `domains::payments::commands::payments_get_open_documents` | `src-tauri/src/domains/payments/commands.rs` | yes | `src/modules/payments/services/paymentService.ts` |
| `products_get_products` | `domains::products::commands::catalog::products_get_products` | `src-tauri/src/domains/products/commands/catalog.rs` | yes | `src/modules/products/services/productService.ts` |
| `products_get_product` | `domains::products::commands::catalog::products_get_product` | `src-tauri/src/domains/products/commands/catalog.rs` | yes | `src/modules/core/services/backend.ts`, `src/modules/products/services/productService.ts` |
| `products_find_by_code` | `domains::products::commands::catalog::products_find_by_code` | `src-tauri/src/domains/products/commands/catalog.rs` | yes | `src/modules/products/services/productService.ts` |
| `products_generate_ean13` | `domains::products::commands::catalog::products_generate_ean13` | `src-tauri/src/domains/products/commands/catalog.rs` | yes | `src/modules/products/services/productService.ts` |
| `products_create_product` | `domains::products::commands::catalog::products_create_product` | `src-tauri/src/domains/products/commands/catalog.rs` | yes | `src/modules/products/services/productService.ts` |
| `products_update_product` | `domains::products::commands::catalog::products_update_product` | `src-tauri/src/domains/products/commands/catalog.rs` | yes | `src/modules/products/services/productService.ts` |
| `products_suggest_sku` | `domains::products::commands::catalog::products_suggest_sku` | `src-tauri/src/domains/products/commands/catalog.rs` | yes | `src/modules/products/services/productService.ts` |
| `products_get_categories` | `domains::products::commands::catalog::products_get_categories` | `src-tauri/src/domains/products/commands/catalog.rs` | yes | `src/modules/products/services/catalogService.ts` |
| `products_save_category` | `domains::products::commands::catalog::products_save_category` | `src-tauri/src/domains/products/commands/catalog.rs` | yes | `src/modules/products/services/catalogService.ts` |
| `products_delete_category` | `domains::products::commands::catalog::products_delete_category` | `src-tauri/src/domains/products/commands/catalog.rs` | yes | `src/modules/products/services/catalogService.ts` |
| `products_get_units` | `domains::products::commands::catalog::products_get_units` | `src-tauri/src/domains/products/commands/catalog.rs` | yes | `src/modules/products/services/catalogService.ts` |
| `products_save_unit` | `domains::products::commands::catalog::products_save_unit` | `src-tauri/src/domains/products/commands/catalog.rs` | yes | `src/modules/products/services/catalogService.ts` |
| `products_apply_unit_preset` | `domains::products::commands::catalog::products_apply_unit_preset` | `src-tauri/src/domains/products/commands/catalog.rs` | yes | `src/modules/products/services/catalogService.ts` |
| `products_delete_unit` | `domains::products::commands::catalog::products_delete_unit` | `src-tauri/src/domains/products/commands/catalog.rs` | yes | `src/modules/products/services/catalogService.ts` |
| `products_get_price_lists` | `domains::products::commands::catalog::products_get_price_lists` | `src-tauri/src/domains/products/commands/catalog.rs` | yes | `src/modules/products/services/catalogService.ts` |
| `products_save_price_list` | `domains::products::commands::catalog::products_save_price_list` | `src-tauri/src/domains/products/commands/catalog.rs` | yes | `src/modules/products/services/catalogService.ts` |
| `products_delete_price_list` | `domains::products::commands::catalog::products_delete_price_list` | `src-tauri/src/domains/products/commands/catalog.rs` | yes | `src/modules/products/services/catalogService.ts` |
| `products_set_price_list_values` | `domains::products::commands::catalog::products_set_price_list_values` | `src-tauri/src/domains/products/commands/catalog.rs` | yes | `src/modules/products/services/catalogService.ts` |
| `products_get_custom_field_defs` | `domains::products::commands::catalog::products_get_custom_field_defs` | `src-tauri/src/domains/products/commands/catalog.rs` | yes | `src/modules/products/services/catalogService.ts` |
| `products_save_custom_field_def` | `domains::products::commands::catalog::products_save_custom_field_def` | `src-tauri/src/domains/products/commands/catalog.rs` | yes | `src/modules/products/services/catalogService.ts` |
| `products_delete_custom_field_def` | `domains::products::commands::catalog::products_delete_custom_field_def` | `src-tauri/src/domains/products/commands/catalog.rs` | yes | `src/modules/products/services/catalogService.ts` |
| `products_reorder_custom_field_defs` | `domains::products::commands::catalog::products_reorder_custom_field_defs` | `src-tauri/src/domains/products/commands/catalog.rs` | yes | `src/modules/products/services/catalogService.ts` |
| `products_get_stock_adjustments` | `domains::products::commands::inventory::products_get_stock_adjustments` | `src-tauri/src/domains/products/commands/inventory.rs` | yes | `src/modules/products/services/inventoryService.ts` |
| `products_get_stock_adjustment` | `domains::products::commands::inventory::products_get_stock_adjustment` | `src-tauri/src/domains/products/commands/inventory.rs` | yes | `src/modules/products/services/inventoryService.ts` |
| `products_create_stock_adjustment` | `domains::products::commands::inventory::products_create_stock_adjustment` | `src-tauri/src/domains/products/commands/inventory.rs` | yes | `src/modules/products/services/inventoryService.ts` |
| `products_complete_adjustment` | `domains::products::commands::inventory::products_complete_adjustment` | `src-tauri/src/domains/products/commands/inventory.rs` | yes | `src/modules/products/services/inventoryService.ts` |
| `products_delete_draft_adjustment` | `domains::products::commands::inventory::products_delete_draft_adjustment` | `src-tauri/src/domains/products/commands/inventory.rs` | yes | `src/modules/products/services/inventoryService.ts` |
| `products_get_stock_movements` | `domains::products::commands::inventory::products_get_stock_movements` | `src-tauri/src/domains/products/commands/inventory.rs` | yes | `src/modules/products/services/inventoryService.ts` |
| `products_get_stock_movements_paged` | `domains::products::commands::inventory::products_get_stock_movements_paged` | `src-tauri/src/domains/products/commands/inventory.rs` | yes | `src/modules/products/services/inventoryService.ts` |
| `products_get_batches` | `domains::products::commands::inventory::products_get_batches` | `src-tauri/src/domains/products/commands/inventory.rs` | yes | `src/modules/products/services/inventoryService.ts` |
| `products_get_expiry_report` | `domains::products::commands::inventory::products_get_expiry_report` | `src-tauri/src/domains/products/commands/inventory.rs` | yes | `src/modules/products/services/inventoryService.ts` |
| `products_write_off_expired_batches` | `domains::products::commands::inventory::products_write_off_expired_batches` | `src-tauri/src/domains/products/commands/inventory.rs` | yes | `src/modules/products/services/inventoryService.ts` |
| `products_return_batches_to_supplier` | `domains::products::commands::inventory::products_return_batches_to_supplier` | `src-tauri/src/domains/products/commands/inventory.rs` | yes | `src/modules/products/services/inventoryService.ts` |
| `products_get_debit_note_drafts` | `domains::products::commands::inventory::products_get_debit_note_drafts` | `src-tauri/src/domains/products/commands/inventory.rs` | yes | `src/modules/products/services/inventoryService.ts` |
| `products_get_stock_counts` | `domains::products::commands::inventory::products_get_stock_counts` | `src-tauri/src/domains/products/commands/inventory.rs` | yes | `src/modules/products/services/inventoryService.ts` |
| `products_get_stock_count` | `domains::products::commands::inventory::products_get_stock_count` | `src-tauri/src/domains/products/commands/inventory.rs` | yes | `src/modules/products/services/inventoryService.ts` |
| `products_create_stock_count` | `domains::products::commands::inventory::products_create_stock_count` | `src-tauri/src/domains/products/commands/inventory.rs` | yes | `src/modules/products/services/inventoryService.ts` |
| `products_update_stock_count_line` | `domains::products::commands::inventory::products_update_stock_count_line` | `src-tauri/src/domains/products/commands/inventory.rs` | yes | `src/modules/products/services/inventoryService.ts` |
| `products_submit_count_for_review` | `domains::products::commands::inventory::products_submit_count_for_review` | `src-tauri/src/domains/products/commands/inventory.rs` | yes | `src/modules/products/services/inventoryService.ts` |
| `products_resume_counting` | `domains::products::commands::inventory::products_resume_counting` | `src-tauri/src/domains/products/commands/inventory.rs` | yes | `src/modules/products/services/inventoryService.ts` |
| `products_complete_stock_count` | `domains::products::commands::inventory::products_complete_stock_count` | `src-tauri/src/domains/products/commands/inventory.rs` | yes | `src/modules/products/services/inventoryService.ts` |
| `products_get_transfers` | `domains::products::commands::transfers::products_get_transfers` | `src-tauri/src/domains/products/commands/transfers.rs` | yes | `src/modules/products/services/transferService.ts` |
| `products_get_transfer` | `domains::products::commands::transfers::products_get_transfer` | `src-tauri/src/domains/products/commands/transfers.rs` | yes | `src/modules/products/services/transferService.ts` |
| `products_create_transfer` | `domains::products::commands::transfers::products_create_transfer` | `src-tauri/src/domains/products/commands/transfers.rs` | yes | `src/modules/products/services/transferService.ts` |
| `products_send_transfer` | `domains::products::commands::transfers::products_send_transfer` | `src-tauri/src/domains/products/commands/transfers.rs` | yes | `src/modules/products/services/transferService.ts` |
| `products_receive_transfer` | `domains::products::commands::transfers::products_receive_transfer` | `src-tauri/src/domains/products/commands/transfers.rs` | yes | `src/modules/products/services/transferService.ts` |
| `products_reject_transfer` | `domains::products::commands::transfers::products_reject_transfer` | `src-tauri/src/domains/products/commands/transfers.rs` | yes | `src/modules/products/services/transferService.ts` |
| `purchases_get_purchase_orders` | `domains::purchases::commands::purchases_get_purchase_orders` | `src-tauri/src/domains/purchases/commands.rs` | yes | `src/modules/purchases/services/purchaseService.ts` |
| `purchases_get_purchase_order` | `domains::purchases::commands::purchases_get_purchase_order` | `src-tauri/src/domains/purchases/commands.rs` | yes | `src/modules/purchases/services/purchaseService.ts` |
| `purchases_get_purchase_return` | `domains::purchases::commands::purchases_get_purchase_return` | `src-tauri/src/domains/purchases/commands.rs` | yes | `src/modules/purchases/services/purchaseService.ts` |
| `purchases_get_active_batches` | `domains::purchases::commands::purchases_get_active_batches` | `src-tauri/src/domains/purchases/commands.rs` | yes | `src/modules/purchases/services/purchaseService.ts` |
| `purchases_get_debit_note_drafts` | `domains::purchases::commands::purchases_get_debit_note_drafts` | `src-tauri/src/domains/purchases/commands.rs` | yes | `src/modules/purchases/services/purchaseService.ts` |
| `purchases_save_purchase_order` | `domains::purchases::commands::purchases_save_purchase_order` | `src-tauri/src/domains/purchases/commands.rs` | yes | `src/modules/purchases/services/purchaseService.ts` |
| `purchases_send_purchase_order_to_supplier` | `domains::purchases::commands::purchases_send_purchase_order_to_supplier` | `src-tauri/src/domains/purchases/commands.rs` | yes | `src/modules/purchases/services/purchaseService.ts` |
| `purchases_receive_purchase_order` | `domains::purchases::commands::purchases_receive_purchase_order` | `src-tauri/src/domains/purchases/commands.rs` | yes | `src/modules/purchases/services/purchaseService.ts` |
| `purchases_confirm_purchase_order` | `domains::purchases::commands::purchases_confirm_purchase_order` | `src-tauri/src/domains/purchases/commands.rs` | yes | `src/modules/purchases/services/purchaseService.ts` |
| `purchases_cancel_purchase_order` | `domains::purchases::commands::purchases_cancel_purchase_order` | `src-tauri/src/domains/purchases/commands.rs` | yes | `src/modules/purchases/services/purchaseService.ts` |
| `purchases_create_purchase_return` | `domains::purchases::commands::purchases_create_purchase_return` | `src-tauri/src/domains/purchases/commands.rs` | yes | `src/modules/purchases/services/purchaseService.ts` |
| `purchases_post_debit_note_draft` | `domains::purchases::commands::purchases_post_debit_note_draft` | `src-tauri/src/domains/purchases/commands.rs` | yes | `src/modules/purchases/services/purchaseService.ts` |
| `reports_get_trial_balance` | `domains::reports::commands::reports_get_trial_balance` | `src-tauri/src/domains/reports/commands.rs` | yes | `src/modules/reports/services/reportService.ts` |
| `reports_get_profit_and_loss` | `domains::reports::commands::reports_get_profit_and_loss` | `src-tauri/src/domains/reports/commands.rs` | yes | `src/modules/reports/services/reportService.ts` |
| `reports_get_profit_and_loss_comparison` | `domains::reports::commands::reports_get_profit_and_loss_comparison` | `src-tauri/src/domains/reports/commands.rs` | yes | `src/modules/reports/services/reportService.ts` |
| `reports_get_cost_center_profit_and_loss` | `domains::reports::commands::reports_get_cost_center_profit_and_loss` | `src-tauri/src/domains/reports/commands.rs` | yes | `src/modules/reports/services/reportService.ts` |
| `reports_get_cost_center_budget_vs_actual` | `domains::reports::commands::reports_get_cost_center_budget_vs_actual` | `src-tauri/src/domains/reports/commands.rs` | yes | `src/modules/reports/services/reportService.ts` |
| `reports_get_balance_sheet` | `domains::reports::commands::reports_get_balance_sheet` | `src-tauri/src/domains/reports/commands.rs` | yes | `src/modules/reports/services/reportService.ts` |
| `reports_get_account_ledger` | `domains::reports::commands::reports_get_account_ledger` | `src-tauri/src/domains/reports/commands.rs` | yes | `src/modules/reports/services/reportService.ts` |
| `reports_get_party_ledger` | `domains::reports::commands::reports_get_party_ledger` | `src-tauri/src/domains/reports/commands.rs` | yes | `src/modules/reports/services/reportService.ts` |
| `reports_get_vat_report` | `domains::reports::commands::reports_get_vat_report` | `src-tauri/src/domains/reports/commands.rs` | yes | `src/modules/reports/services/reportService.ts` |
| `reports_get_vat_detail` | `domains::reports::commands::reports_get_vat_detail` | `src-tauri/src/domains/reports/commands.rs` | yes | `src/modules/reports/services/reportService.ts` |
| `reports_get_cash_flow_statement` | `domains::reports::commands::reports_get_cash_flow_statement` | `src-tauri/src/domains/reports/commands.rs` | yes | `src/modules/reports/services/reportService.ts` |
| `reports_get_day_book` | `domains::reports::commands::reports_get_day_book` | `src-tauri/src/domains/reports/commands.rs` | yes | `src/modules/reports/services/reportService.ts` |
| `reports_get_period_comparison` | `domains::reports::commands::reports_get_period_comparison` | `src-tauri/src/domains/reports/commands.rs` | yes | `src/modules/reports/services/reportService.ts` |
| `reports_get_business_health_report` | `domains::reports::commands::reports_get_business_health_report` | `src-tauri/src/domains/reports/commands.rs` | yes | `src/modules/reports/services/reportService.ts` |
| `reports_get_ledger_targets` | `domains::reports::commands::reports_get_ledger_targets` | `src-tauri/src/domains/reports/commands.rs` | yes | `src/modules/reports/services/reportService.ts` |
| `reports_get_dimension_options` | `domains::reports::commands::reports_get_dimension_options` | `src-tauri/src/domains/reports/commands.rs` | yes | `src/modules/reports/services/reportService.ts` |
| `reports_get_sales_report` | `domains::reports::commands::reports_get_sales_report` | `src-tauri/src/domains/reports/commands.rs` | yes | `src/modules/reports/services/reportService.ts` |
| `reports_get_inventory_report` | `domains::reports::commands::reports_get_inventory_report` | `src-tauri/src/domains/reports/commands.rs` | yes | `src/modules/reports/services/reportService.ts` |
| `reports_get_discounts_report` | `domains::reports::commands::reports_get_discounts_report` | `src-tauri/src/domains/reports/commands.rs` | yes | `src/modules/reports/services/reportService.ts` |
| `reports_get_gross_profit_report` | `domains::reports::commands::reports_get_gross_profit_report` | `src-tauri/src/domains/reports/commands.rs` | yes | `src/modules/reports/services/reportService.ts` |
| `reports_get_returns_report` | `domains::reports::commands::reports_get_returns_report` | `src-tauri/src/domains/reports/commands.rs` | yes | `src/modules/reports/services/reportService.ts` |
| `reports_get_expenses_report` | `domains::reports::commands::reports_get_expenses_report` | `src-tauri/src/domains/reports/commands.rs` | yes | `src/modules/reports/services/reportService.ts` |
| `reports_get_shifts_report` | `domains::reports::commands::reports_get_shifts_report` | `src-tauri/src/domains/reports/commands.rs` | yes | `src/modules/reports/services/reportService.ts` |
| `reports_get_low_stock_report` | `domains::reports::commands::reports_get_low_stock_report` | `src-tauri/src/domains/reports/commands.rs` | yes | `src/modules/reports/services/reportService.ts` |
| `reports_get_dead_stock_report` | `domains::reports::commands::reports_get_dead_stock_report` | `src-tauri/src/domains/reports/commands.rs` | yes | `src/modules/reports/services/reportService.ts` |
| `reports_get_stocktake_variances` | `domains::reports::commands::reports_get_stocktake_variances` | `src-tauri/src/domains/reports/commands.rs` | yes | `src/modules/reports/services/reportService.ts` |
| `reports_get_transfers_report` | `domains::reports::commands::reports_get_transfers_report` | `src-tauri/src/domains/reports/commands.rs` | yes | `src/modules/reports/services/reportService.ts` |
| `reports_get_purchases_report` | `domains::reports::commands::reports_get_purchases_report` | `src-tauri/src/domains/reports/commands.rs` | yes | `src/modules/reports/services/reportService.ts` |
| `reports_get_branch_comparison` | `domains::reports::commands::reports_get_branch_comparison` | `src-tauri/src/domains/reports/commands.rs` | yes | `src/modules/reports/services/reportService.ts` |
| `reports_get_profit_leakage_report` | `domains::reports::commands::reports_get_profit_leakage_report` | `src-tauri/src/domains/reports/commands.rs` | yes | `src/modules/reports/services/reportService.ts` |
| `reports_get_aging_report` | `domains::reports::commands::reports_get_aging_report` | `src-tauri/src/domains/reports/commands.rs` | yes | `src/modules/reports/services/reportService.ts` |
| `reports_get_overdue_report` | `domains::reports::commands::reports_get_overdue_report` | `src-tauri/src/domains/reports/commands.rs` | yes | `src/modules/reports/services/reportService.ts` |
| `settings_get_settings` | `domains::settings::commands::settings_get_settings` | `src-tauri/src/domains/settings/commands.rs` | yes | `src/modules/settings/services/settingsService.ts` |
| `settings_update_settings` | `domains::settings::commands::settings_update_settings` | `src-tauri/src/domains/settings/commands.rs` | yes | `src/modules/settings/services/settingsService.ts` |
| `settings_get_taxes` | `domains::settings::commands::settings_get_taxes` | `src-tauri/src/domains/settings/commands.rs` | yes | `src/modules/settings/services/settingsService.ts` |
| `settings_save_tax` | `domains::settings::commands::settings_save_tax` | `src-tauri/src/domains/settings/commands.rs` | yes | `src/modules/settings/services/settingsService.ts` |
| `settings_delete_tax` | `domains::settings::commands::settings_delete_tax` | `src-tauri/src/domains/settings/commands.rs` | yes | `src/modules/settings/services/settingsService.ts` |
| `settings_get_payment_methods` | `domains::settings::commands::settings_get_payment_methods` | `src-tauri/src/domains/settings/commands.rs` | yes | `src/modules/settings/services/settingsService.ts` |
| `settings_save_payment_method` | `domains::settings::commands::settings_save_payment_method` | `src-tauri/src/domains/settings/commands.rs` | yes | `src/modules/settings/services/settingsService.ts` |
| `settings_reorder_payment_methods` | `domains::settings::commands::settings_reorder_payment_methods` | `src-tauri/src/domains/settings/commands.rs` | yes | `src/modules/settings/services/settingsService.ts` |
| `settings_delete_payment_method` | `domains::settings::commands::settings_delete_payment_method` | `src-tauri/src/domains/settings/commands.rs` | yes | `src/modules/settings/services/settingsService.ts` |
| `settings_get_branches` | `domains::settings::commands::settings_get_branches` | `src-tauri/src/domains/settings/commands.rs` | yes | `src/modules/settings/services/branchesService.ts` |
| `settings_create_branch` | `domains::settings::commands::settings_create_branch` | `src-tauri/src/domains/settings/commands.rs` | yes | `src/modules/settings/services/branchesService.ts` |
| `settings_update_branch` | `domains::settings::commands::settings_update_branch` | `src-tauri/src/domains/settings/commands.rs` | yes | `src/modules/settings/services/branchesService.ts` |
| `settings_deactivate_branch` | `domains::settings::commands::settings_deactivate_branch` | `src-tauri/src/domains/settings/commands.rs` | yes | `src/modules/settings/services/branchesService.ts` |
| `settings_reactivate_branch` | `domains::settings::commands::settings_reactivate_branch` | `src-tauri/src/domains/settings/commands.rs` | yes | `src/modules/settings/services/branchesService.ts` |
| `settings_get_cost_centers` | `domains::settings::commands::settings_get_cost_centers` | `src-tauri/src/domains/settings/commands.rs` | yes | `src/modules/settings/services/branchesService.ts` |
| `settings_create_cost_center` | `domains::settings::commands::settings_create_cost_center` | `src-tauri/src/domains/settings/commands.rs` | yes | `src/modules/settings/services/branchesService.ts` |
| `settings_update_cost_center` | `domains::settings::commands::settings_update_cost_center` | `src-tauri/src/domains/settings/commands.rs` | yes | `src/modules/settings/services/branchesService.ts` |
| `settings_delete_cost_center` | `domains::settings::commands::settings_delete_cost_center` | `src-tauri/src/domains/settings/commands.rs` | yes | `src/modules/settings/services/branchesService.ts` |
| `settings_get_currencies` | `domains::settings::commands::settings_get_currencies` | `src-tauri/src/domains/settings/commands.rs` | yes | `src/modules/settings/services/branchesService.ts` |
| `settings_get_exchange_rates` | `domains::settings::commands::settings_get_exchange_rates` | `src-tauri/src/domains/settings/commands.rs` | yes | `src/modules/settings/services/branchesService.ts` |
| `settings_create_currency` | `domains::settings::commands::settings_create_currency` | `src-tauri/src/domains/settings/commands.rs` | yes | `src/modules/settings/services/branchesService.ts` |
| `settings_update_currency` | `domains::settings::commands::settings_update_currency` | `src-tauri/src/domains/settings/commands.rs` | yes | `src/modules/settings/services/branchesService.ts` |
| `settings_save_exchange_rate` | `domains::settings::commands::settings_save_exchange_rate` | `src-tauri/src/domains/settings/commands.rs` | yes | `src/modules/settings/services/branchesService.ts` |
| `settings_is_base_currency_locked` | `domains::settings::commands::settings_is_base_currency_locked` | `src-tauri/src/domains/settings/commands.rs` | yes | `src/modules/settings/services/branchesService.ts` |
| `settings_set_base_currency` | `domains::settings::commands::settings_set_base_currency` | `src-tauri/src/domains/settings/commands.rs` | yes | `src/modules/settings/services/branchesService.ts` |
| `settings_get_revaluation_preview` | `domains::settings::commands::settings_get_revaluation_preview` | `src-tauri/src/domains/settings/commands.rs` | yes | `src/modules/settings/services/branchesService.ts` |
| `settings_get_default_revaluation_rates` | `domains::settings::commands::settings_get_default_revaluation_rates` | `src-tauri/src/domains/settings/commands.rs` | yes | `src/modules/settings/services/branchesService.ts` |
| `settings_post_revaluation` | `domains::settings::commands::settings_post_revaluation` | `src-tauri/src/domains/settings/commands.rs` | yes | `src/modules/settings/services/branchesService.ts` |
| `settings_get_lan_sharing_status` | `domains::settings::commands::settings_get_lan_sharing_status` | `src-tauri/src/domains/settings/commands.rs` | yes | `src/modules/settings/services/networkService.ts` |
| `settings_enable_lan_sharing` | `domains::settings::commands::settings_enable_lan_sharing` | `src-tauri/src/domains/settings/commands.rs` | yes | `src/modules/settings/services/networkService.ts` |
| `settings_disable_lan_sharing` | `domains::settings::commands::settings_disable_lan_sharing` | `src-tauri/src/domains/settings/commands.rs` | yes | `src/modules/settings/services/networkService.ts` |
| `settings_rotate_pairing_code` | `domains::settings::commands::settings_rotate_pairing_code` | `src-tauri/src/domains/settings/commands.rs` | yes | `src/modules/settings/services/networkService.ts` |
| `settings_reconnect_backend` | `domains::settings::commands::settings_reconnect_backend` | `src-tauri/src/domains/settings/commands.rs` | yes | `src/modules/settings/services/networkService.ts` |
| `setup_get_device_setup_state` | `domains::setup::commands::setup_get_device_setup_state` | `src-tauri/src/domains/setup/commands.rs` | yes | `src/modules/setup/services/deviceService.ts` |
| `setup_provision_main` | `domains::setup::commands::setup_provision_main` | `src-tauri/src/domains/setup/commands.rs` | yes | `src/modules/setup/services/deviceService.ts` |
| `setup_pair_terminal` | `domains::setup::commands::setup_pair_terminal` | `src-tauri/src/domains/setup/commands.rs` | yes | `src/modules/setup/services/deviceService.ts` |
| `setup_get_onboarding_progress` | `domains::setup::commands::setup_get_onboarding_progress` | `src-tauri/src/domains/setup/commands.rs` | yes | `src/modules/setup/services/setupService.ts` |
| `setup_save_onboarding_progress` | `domains::setup::commands::setup_save_onboarding_progress` | `src-tauri/src/domains/setup/commands.rs` | yes | `src/modules/setup/services/setupService.ts` |
| `setup_mark_step_done` | `domains::setup::commands::setup_mark_step_done` | `src-tauri/src/domains/setup/commands.rs` | yes | `src/modules/setup/services/setupService.ts` |
| `setup_mark_step_skipped` | `domains::setup::commands::setup_mark_step_skipped` | `src-tauri/src/domains/setup/commands.rs` | yes | `src/modules/setup/services/setupService.ts` |
| `setup_apply_business_type_defaults` | `domains::setup::commands::setup_apply_business_type_defaults` | `src-tauri/src/domains/setup/commands.rs` | yes | `src/modules/setup/services/setupService.ts` |
| `setup_is_base_currency_locked` | `domains::setup::commands::setup_is_base_currency_locked` | `src-tauri/src/domains/setup/commands.rs` | yes | `src/modules/setup/services/setupService.ts` |
| `setup_apply_country_tax` | `domains::setup::commands::setup_apply_country_tax` | `src-tauri/src/domains/setup/commands.rs` | yes | `src/modules/setup/services/setupService.ts` |
| `setup_apply_fiscal_year` | `domains::setup::commands::setup_apply_fiscal_year` | `src-tauri/src/domains/setup/commands.rs` | yes | `src/modules/setup/services/setupService.ts` |
| `setup_apply_branches` | `domains::setup::commands::setup_apply_branches` | `src-tauri/src/domains/setup/commands.rs` | yes | `src/modules/setup/services/setupService.ts` |
| `setup_apply_coa_template` | `domains::setup::commands::setup_apply_coa_template` | `src-tauri/src/domains/setup/commands.rs` | yes | `src/modules/setup/services/setupService.ts` |
| `setup_apply_payment_methods` | `domains::setup::commands::setup_apply_payment_methods` | `src-tauri/src/domains/setup/commands.rs` | yes | `src/modules/setup/services/setupService.ts` |
| `setup_get_opening_balance_equity_net` | `domains::setup::commands::setup_get_opening_balance_equity_net` | `src-tauri/src/domains/setup/commands.rs` | yes | `src/modules/setup/services/setupService.ts` |
| `setup_is_first_use_posted` | `domains::setup::commands::setup_is_first_use_posted` | `src-tauri/src/domains/setup/commands.rs` | yes | `src/modules/setup/services/setupService.ts` |
| `setup_post_opening_balances` | `domains::setup::commands::setup_post_opening_balances` | `src-tauri/src/domains/setup/commands.rs` | yes | `src/modules/setup/services/setupService.ts` |
| `setup_post_opening_stock` | `domains::setup::commands::setup_post_opening_stock` | `src-tauri/src/domains/setup/commands.rs` | yes | `src/modules/setup/services/setupService.ts` |
| `setup_reclose_opening_balance_equity` | `domains::setup::commands::setup_reclose_opening_balance_equity` | `src-tauri/src/domains/setup/commands.rs` | yes | `src/modules/setup/services/setupService.ts` |
| `setup_post_party_opening` | `domains::setup::commands::setup_post_party_opening` | `src-tauri/src/domains/setup/commands.rs` | yes | `src/modules/setup/services/setupService.ts` |
| `setup_reverse_party_opening` | `domains::setup::commands::setup_reverse_party_opening` | `src-tauri/src/domains/setup/commands.rs` | yes | `src/modules/setup/services/setupService.ts` |
| `setup_finish_onboarding` | `domains::setup::commands::setup_finish_onboarding` | `src-tauri/src/domains/setup/commands.rs` | yes | `src/modules/setup/services/setupService.ts` |
| `templates_list_templates` | `domains::templates::commands::templates_list_templates` | `src-tauri/src/domains/templates/commands.rs` | yes | `src/modules/templates/services/templateService.ts` |
| `templates_get_template` | `domains::templates::commands::templates_get_template` | `src-tauri/src/domains/templates/commands.rs` | yes | `src/modules/templates/services/templateService.ts` |
| `templates_get_default_template` | `domains::templates::commands::templates_get_default_template` | `src-tauri/src/domains/templates/commands.rs` | yes | `src/modules/templates/services/templateService.ts` |
| `templates_save_template` | `domains::templates::commands::templates_save_template` | `src-tauri/src/domains/templates/commands.rs` | yes | `src/modules/templates/services/templateService.ts` |
| `templates_set_as_default` | `domains::templates::commands::templates_set_as_default` | `src-tauri/src/domains/templates/commands.rs` | yes | `src/modules/templates/services/templateService.ts` |
| `templates_duplicate_template` | `domains::templates::commands::templates_duplicate_template` | `src-tauri/src/domains/templates/commands.rs` | yes | `src/modules/templates/services/templateService.ts` |
| `templates_delete_template` | `domains::templates::commands::templates_delete_template` | `src-tauri/src/domains/templates/commands.rs` | yes | `src/modules/templates/services/templateService.ts` |
| `templates_reset_template_to_defaults` | `domains::templates::commands::templates_reset_template_to_defaults` | `src-tauri/src/domains/templates/commands.rs` | yes | `src/modules/templates/services/templateService.ts` |
| `templates_import_template` | `domains::templates::commands::templates_import_template` | `src-tauri/src/domains/templates/commands.rs` | yes | `src/modules/templates/services/templateService.ts` |
| `templates_create_template` | `domains::templates::commands::templates_create_template` | `src-tauri/src/domains/templates/commands.rs` | yes | `src/modules/templates/services/templateService.ts` |
| `users_get_users` | `domains::users::commands::users_get_users` | `src-tauri/src/domains/users/commands.rs` | yes | `src/modules/users/services/userService.ts` |
| `users_get_user` | `domains::users::commands::users_get_user` | `src-tauri/src/domains/users/commands.rs` | yes | `src/modules/users/services/userService.ts` |
| `users_create_user` | `domains::users::commands::users_create_user` | `src-tauri/src/domains/users/commands.rs` | yes | `src/modules/users/services/userService.ts` |
| `users_update_user` | `domains::users::commands::users_update_user` | `src-tauri/src/domains/users/commands.rs` | yes | `src/modules/users/services/userService.ts` |
| `users_login` | `domains::users::commands::users_login` | `src-tauri/src/domains/users/commands.rs` | yes | `src/modules/users/services/authService.ts` |
| `users_logout` | `domains::users::commands::users_logout` | `src-tauri/src/domains/users/commands.rs` | yes | `src/modules/users/services/authService.ts` |
| `users_restore_session` | `domains::users::commands::users_restore_session` | `src-tauri/src/domains/users/commands.rs` | yes | `src/modules/users/services/authService.ts` |
| `users_verify_manager_pin` | `domains::users::commands::users_verify_manager_pin` | `src-tauri/src/domains/users/commands.rs` | yes | `src/modules/users/services/authService.ts` |
| `vouchers_create_receipt_voucher` | `domains::vouchers::commands::vouchers_create_receipt_voucher` | `src-tauri/src/domains/vouchers/commands.rs` | yes | `src/modules/vouchers/services/voucherService.ts` |
| `vouchers_create_payment_voucher` | `domains::vouchers::commands::vouchers_create_payment_voucher` | `src-tauri/src/domains/vouchers/commands.rs` | yes | `src/modules/vouchers/services/voucherService.ts` |
| `vouchers_create_transfer_voucher` | `domains::vouchers::commands::vouchers_create_transfer_voucher` | `src-tauri/src/domains/vouchers/commands.rs` | yes | `src/modules/vouchers/services/voucherService.ts` |
| `vouchers_create_owner_voucher` | `domains::vouchers::commands::vouchers_create_owner_voucher` | `src-tauri/src/domains/vouchers/commands.rs` | yes | `src/modules/vouchers/services/voucherService.ts` |
| `vouchers_get_vouchers` | `domains::vouchers::commands::vouchers_get_vouchers` | `src-tauri/src/domains/vouchers/commands.rs` | yes | `src/modules/vouchers/services/voucherService.ts` |
| `vouchers_get_voucher` | `domains::vouchers::commands::vouchers_get_voucher` | `src-tauri/src/domains/vouchers/commands.rs` | yes | `src/modules/vouchers/services/voucherService.ts` |
| `vouchers_get_unsettled_tender_groups` | `domains::vouchers::commands::vouchers_get_unsettled_tender_groups` | `src-tauri/src/domains/vouchers/commands.rs` | yes | `src/modules/vouchers/services/voucherService.ts` |
| `vouchers_estimate_settlement_fee` | `domains::vouchers::commands::vouchers_estimate_settlement_fee` | `src-tauri/src/domains/vouchers/commands.rs` | yes | `src/modules/vouchers/services/voucherService.ts` |
| `vouchers_create_card_settlement` | `domains::vouchers::commands::vouchers_create_card_settlement` | `src-tauri/src/domains/vouchers/commands.rs` | yes | `src/modules/vouchers/services/voucherService.ts` |
| `vouchers_get_card_settlements` | `domains::vouchers::commands::vouchers_get_card_settlements` | `src-tauri/src/domains/vouchers/commands.rs` | yes | `src/modules/vouchers/services/voucherService.ts` |
| `vouchers_get_card_settlement` | `domains::vouchers::commands::vouchers_get_card_settlement` | `src-tauri/src/domains/vouchers/commands.rs` | yes | `src/modules/vouchers/services/voucherService.ts` |
| `settings_backup_settings` | `infrastructure::backup::commands::settings_backup_settings` | `src-tauri/src/infrastructure/backup/commands.rs` | yes | `src/modules/settings/services/backupService.ts` |
| `settings_save_backup_settings` | `infrastructure::backup::commands::settings_save_backup_settings` | `src-tauri/src/infrastructure/backup/commands.rs` | yes | `src/modules/settings/services/backupService.ts` |
| `settings_preview_backup_counts` | `infrastructure::backup::commands::settings_preview_backup_counts` | `src-tauri/src/infrastructure/backup/commands.rs` | yes | `src/modules/settings/services/backupService.ts` |
| `settings_build_backup_archive` | `infrastructure::backup::commands::settings_build_backup_archive` | `src-tauri/src/infrastructure/backup/commands.rs` | yes | `src/modules/settings/services/backupService.ts` |
| `settings_record_backup_saved` | `infrastructure::backup::commands::settings_record_backup_saved` | `src-tauri/src/infrastructure/backup/commands.rs` | yes | `src/modules/settings/services/backupService.ts` |
| `settings_run_auto_backup_if_due` | `infrastructure::backup::commands::settings_run_auto_backup_if_due` | `src-tauri/src/infrastructure/backup/commands.rs` | yes | `src/modules/settings/services/backupService.ts` |
| `settings_preview_restore` | `infrastructure::backup::commands::settings_preview_restore` | `src-tauri/src/infrastructure/backup/commands.rs` | yes | `src/modules/settings/services/backupService.ts` |
| `settings_restore_from_archive` | `infrastructure::backup::commands::settings_restore_from_archive` | `src-tauri/src/infrastructure/backup/commands.rs` | yes | `src/modules/settings/services/backupService.ts` |
| `setup_inspect_legacy_snapshot` | `infrastructure::import::commands::setup_inspect_legacy_snapshot` | `src-tauri/src/infrastructure/import/commands.rs` | yes | `src/modules/setup/services/legacyImportService.ts` |
| `setup_import_snapshot` | `infrastructure::import::commands::setup_import_snapshot` | `src-tauri/src/infrastructure/import/commands.rs` | yes | `src/modules/core/services/devToolsService.ts`, `src/modules/setup/services/legacyImportService.ts` |
| `render_pdf` | `infrastructure::pdf::render::render_pdf` | `src-tauri/src/infrastructure/pdf/render.rs` | yes | `src/modules/core/services/pdfService.ts` |
| `render_preview` | `infrastructure::pdf::render::render_preview` | `src-tauri/src/infrastructure/pdf/render.rs` | yes | `src/modules/core/services/pdfService.ts` |
| `list_printers` | `infrastructure::print::commands::list_printers` | `src-tauri/src/infrastructure/print/commands.rs` | yes | `src/modules/core/services/printService.ts` |
| `print_thermal_receipt` | `infrastructure::print::commands::print_thermal_receipt` | `src-tauri/src/infrastructure/print/commands.rs` | yes | `src/modules/core/services/printService.ts` |
| `print_test_receipt` | `infrastructure::print::commands::print_test_receipt` | `src-tauri/src/infrastructure/print/commands.rs` | yes | `src/modules/core/services/printService.ts` |

**Plugins:** `single_instance`, `opener`, `fs`, `dialog`, `log`. Frontend plugin use: `@tauri-apps/api (18)`, `@tauri-apps/plugin-dialog (4)`, `@tauri-apps/plugin-fs (4)`, `@tauri-apps/plugin-opener (2)`

**Rust module tree:** `core/mod.rs → pub mod auth`, `core/mod.rs → pub mod db`, `core/mod.rs → pub mod device`, `core/mod.rs → pub mod diag`, `core/mod.rs → pub mod dto`, `core/mod.rs → pub mod error`, `core/mod.rs → pub mod events`, `core/mod.rs → pub mod grants`, `core/mod.rs → pub mod ipc`, `core/mod.rs → pub mod lock`, `core/mod.rs → pub mod poller`, `core/mod.rs → pub mod settings`, `core/mod.rs → pub mod state`, `core/mod.rs → pub mod status`, `core/mod.rs → pub mod terminal`, `core/mod.rs → pub mod tx`, `domains/accounting/commands/mod.rs → pub mod accounts`, `domains/accounting/commands/mod.rs → pub mod journal`, `domains/accounting/commands/mod.rs → pub mod period`, `domains/accounting/commands/mod.rs → pub mod templates`, `domains/accounting/dto/mod.rs → pub mod accounts`, `domains/accounting/dto/mod.rs → pub mod journal`, `domains/accounting/dto/mod.rs → pub mod period`, `domains/accounting/dto/mod.rs → pub mod templates`, `domains/accounting/mod.rs → pub mod commands`, `domains/accounting/mod.rs → pub mod dto`, `domains/accounting/mod.rs → pub mod service`, `domains/accounting/mod.rs → pub mod undo`, `domains/accounting/mod.rs → pub mod undo_period`, `domains/accounting/service/mod.rs → pub mod accounts`, `domains/accounting/service/mod.rs → pub mod journal`, `domains/accounting/service/mod.rs → pub mod journal_reads`, `domains/accounting/service/mod.rs → pub mod period`, `domains/accounting/service/mod.rs → pub mod rows`, `domains/accounting/service/mod.rs → pub mod templates`, `domains/accounting/service/mod.rs → pub mod vat`, `domains/analytics/mod.rs → pub mod commands`, `domains/analytics/mod.rs → pub mod dto`, `domains/analytics/mod.rs → pub mod service`, `domains/analytics/service/mod.rs → pub mod common`, `domains/analytics/service/mod.rs → pub mod customers`, `domains/analytics/service/mod.rs → pub mod products`, `domains/analytics/service/mod.rs → pub mod sales`, `domains/approvals/mod.rs → pub mod commands`, `domains/approvals/mod.rs → pub mod dto`, `domains/approvals/mod.rs → pub mod service`, `domains/dashboard/mod.rs → pub mod commands`, `domains/dashboard/mod.rs → pub mod dto`, `domains/dashboard/mod.rs → pub mod service`, `domains/dashboard/service/insights/mod.rs → pub mod common`, `domains/dashboard/service/insights/mod.rs → pub mod engine`, `domains/dashboard/service/insights/mod.rs → pub mod hints`, `domains/dashboard/service/insights/mod.rs → pub mod rules`, `domains/dashboard/service/insights/rules/mod.rs → pub mod accounting`, `domains/dashboard/service/insights/rules/mod.rs → pub mod cash`, `domains/dashboard/service/insights/rules/mod.rs → pub mod receivables`, `domains/dashboard/service/insights/rules/mod.rs → pub mod sales`, `domains/dashboard/service/insights/rules/mod.rs → pub mod stock`, `domains/dashboard/service/mod.rs → pub mod common`, `domains/dashboard/service/mod.rs → pub mod feed` … +309 more

**Contract gaps:** invoked-but-unregistered — · registered-but-never-invoked — · defined-but-unregistered — · registered-but-undefined —

## Mock backend map (src/mocks — accounting engine)

Read `docs/v2/02-accounting-review.md` before touching posting, VAT, cost or account resolution; keep `bun run verify:mocks` green.

| File | Exported functions | Used by |
|---|---|---|
| `attachments.ts` | `putAttachment`, `getAttachment`, `deleteAttachment`, `listAttachments`, `getAllAttachmentRecords`, `replaceAllAttachments` | core, settings |
| `backend/accounts.ts` | `accountFor`, `accountById`, `settlementAccountFor`, `revenueAccountFor`, `cogsAccountFor`, `purchaseAccountFor`, `saleTaxIdFor`, `purchaseTaxIdFor` | core, invoices, reports |
| `backend/approvals.ts` | `requestApproval`, `decideApproval`, `listApprovalRequests`, `pendingApprovalCount` | approvals |
| `backend/balances.ts` | `customerBalance`, `supplierBalance`, `customerBalanceFc`, `supplierBalanceFc`, `customerStatement`, `supplierStatement` | core, invoices, parties, reports |
| `backend/branches.ts` | `branchById`, `activeBranches`, `branchesEnabled`, `currenciesEnabled`, `costCentersEnabled`, `listBranches`, `createBranch`, `updateBranch`, `deactivateBranch`, `reactivateBranch`, `branchPrefix`, `listCostCenters`, `costCenterById`, `createCostCenter`, `updateCostCenter`, `deleteCostCenter`, `defaultCostCenterFor` | settings |
| `backend/core.ts` | `resolvePosting`, `assertOpenPeriod`, `postJournal`, `draftJournal`, `updateDraftJournal`, `deleteDraftJournal`, `postDraftJournal`, `productById`, `applyStockChange`, `logActivity`, `logAudit`, `diffFields`, `salesTaxRate`, `purchaseTaxRate`, `userById`, `closeYearPreChecks`, `closeFiscalYear`, `reopenFiscalYear` | accounting, invoices, parties, products, settings, users |
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
| `index.ts` | `bootMockDb`, `isBooted` | accounting, analytics, approvals, core, diagnostics, expenses, invoices, main, parties, payments, products, purchases, reports, settings, setup, templates, users, vouchers |
| `persist.ts` | `flushSnapshot`, `mutate`, `loadSnapshot`, `readPersistedSnapshot`, `clearSnapshot` | accounting, core, invoices, parties, products, settings, setup, users |
| `seed/accounts.ts` | `seedAccounts`, `postOpeningCapital` | — |
| `seed/branches9.ts` | `seedBranches9` | — |
| `seed/catalog.ts` | `seedCatalog`, `postOpeningStock` | — |
| `seed/history.ts` | `seedHistory` | — |
| `seed/index.ts` | `seedDatabase`, `seedEmptyCompany` | core, setup |
| `seed/people.ts` | `seedPeople` | — |
| `seed/purchases8.ts` | `seedPurchases8` | — |
| `seed/settings.ts` | `seedSettings` | — |
| `seed/shifts.ts` | `seedShifts` | — |
| `utils.ts` | `setLatencyMode`, `getLatencyMode`, `delay`, `clone`, `sum`, `uid`, `bumpIdCounter`, `padNumber`, `localDateKey`, `inDateRange`, `includesText`, `createRandom` | core, settings |

## Shared UI kit (reuse before building — src/modules/core)

| Group | Names |
|---|---|
| App*/ui components | `AiOrb`, `AppButton`, `AppCard`, `AppCombobox`, `AppDatePicker`, `AppInput`, `AppModal`, `AppPhoneInput`, `AppSelect`, `AppSwitch`, `AppTextarea`, `AttachmentField`, `AttachmentViewer`, `BrandLogo`, `ConfirmDialog`, `CountryFlag`, `DataTable`, `DateRangeFilter`, `DirIcon`, `EmptyState`, `ErrorState`, `MoneyText`, `PageHeader`, `PdfPreview`, `RiyalIcon`, `ScrollFade`, `SearchInput`, `SegmentedControl`, `SkeletonBlock`, `StatusBadge` |
| Page blocks | `AddressFields`, `DetailHeader`, `FilterBar`, `FormActions`, `FormField`, `FormSection`, `LineItemsEditor`, `StatCards`, `TotalsPanel` |
| Page layouts | `DetailPage`, `FormPage`, `ListPage`, `SettingsPage` |
| App shell | `AppSidebar`, `AppTopbar`, `BrandBranchSwitcher`, `DefaultLayout`, `KeyboardShortcutsSheet`, `NavMain`, `NavQuickActions`, `NavUser`, `NotificationsDrawer` |
| Core controllers (composables/stores) | `useAppearance`, `useAsync`, `useCommandPalette`, `useConfirm`, `useForm`, `useGridTab`, `useHotkeys`, `useInsights`, `useKeybindings`, `useKeyboardShortcutsSheet`, `useNotificationStore`, `useNotifications`, `useTheme`, `useToast` |
| Core helpers | `attachments`, `brand`, `countries`, `countryProfiles`, `dirIcon`, `exportXlsx`, `format`, `keyCode`, `keyboardShortcuts`, `labels`, `navigation`, `numbers`, `search`, `tafqit`, `utils`, `validation` |
| Core services | `attachmentService`, `backend`, `backendMirror`, `dashboardService`, `devToolsService`, `geoService`, `insightEngine`, `insightRules`, `insightTypes`, `pdfService`, `printService`, `saveFile` |
| shadcn primitives | `alert-dialog`, `avatar`, `badge`, `breadcrumb`, `button`, `calendar`, `card`, `collapsible`, `combobox`, `command`, `dialog`, `dropdown-menu`, `empty`, `field`, `input`, `input-group`, `kbd`, `label`, `native-select`, `popover`, `range-calendar`, `separator`, `sheet`, `sidebar`, `skeleton`, `sonner`, `switch`, `table`, `tabs`, `textarea`, `toggle`, `toggle-group`, `tooltip` |

## Boundary report

### Seam violations — value imports of `src/mocks` outside `services/` (0 new, 0 known)

_none_

### Pages over 250 lines (CLAUDE.md rule 12)

| Page | Lines |
|---|---|
| `src/modules/invoices/pages/PosPage.vue` | 866 |
| `src/modules/accounting/pages/JournalEntryFormPage.vue` | 593 |
| `src/modules/core/pages/DevUiPage.vue` | 590 |
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
| Rust crates | `tauri`, `tauri-plugin-opener`, `serde`, `serde_json`, `tauri-plugin-fs`, `tauri-plugin-dialog`, `migration`, `sea-orm`, `rust_decimal`, `rust_decimal_macros`, `uuid`, `chrono`, `chrono-tz`, `argon2`, `keyring`, `thiserror`, `serde_with`, `async-trait`, `log`, `tokio`, `tauri-plugin-log`, `typst`, `typst-pdf`, `typst-library`, `typst-layout`, `typst-syntax`, `typst-utils`, `typst-svg`, `qrcode`, `image`, `ecow`, `time`, `lopdf`, `resvg`, `usvg`, `tiny-skia`, `tauri-plugin-single-instance`, `tauri-plugin-autostart`, `getrandom`, `ts-rs` … +9 more |
| Rust extra binaries | `typst_spike`, `pdf_smoke`, `report_smoke`, `thermal_smoke` |
| Frontend deps | `@fontsource-variable/cairo`, `@fontsource/ibm-plex-sans-arabic`, `@fontsource/noto-naskh-arabic`, `@fontsource/tajawal`, `@lucide/vue`, `@tailwindcss/vite`, `@tauri-apps/api`, `@tauri-apps/plugin-dialog`, `@tauri-apps/plugin-fs`, `@tauri-apps/plugin-opener`, `@vueuse/core`, `bwip-js`, `class-variance-authority`, `clsx`, `exceljs`, `fflate`, `libphonenumber-js`, `modern-screenshot`, `pinia`, `reka-ui`, `tailwind-merge`, `tailwindcss`, `tw-animate-css`, `uqr`, `vue`, `vue-router`, `vue-sonner`, `zod` |
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
| `bun run verify:export-snapshot` | `bun run scripts/verify/export-snapshot.ts` |
| `bun run check` | `node scripts/check-text-tokens.js && node scripts/check-rtl.js && bun run scripts/check-contrast.ts && node scripts/check-ui-rules.js && node scripts/check-routes.js` |
| `bun run memory` | `bun run scripts/memory/run.ts` |
| `bun run memory:check` | `bun run scripts/memory/run.ts --check` |
| `bun run contract` | `bun run scripts/contract/run.ts` |
| `bun run contract:check` | `bun run scripts/contract/run.ts --check` |
| `bun run diag` | `bun run scripts/diagnostics/run.ts` |
| `bun run diag:check` | `bun run scripts/diagnostics/run.ts --check` |
| `bun run geo:build` | `bun run scripts/geo/build.ts` |
| `bun run fetch:mariadb` | `node scripts/fetch-mariadb.js` |
| `bun run db:dev` | `node scripts/fetch-mariadb.js && cargo run --manifest-path src-tauri/Cargo.toml --bin db_dev_server` |
| `bun run bindings` | `cargo test --manifest-path src-tauri/Cargo.toml --lib export_bindings` |
| `bun run bindings:check` | `bun run bindings && git diff --exit-code -- "src/modules/*/types/gen"` |

## Docs index

| Doc | Title |
|---|---|
| `docs/action_plan.md` | Execution Plan — Frontend-Only Rebuild |
| `docs/backend/contract/accounting.md` | Contract — `accounting` |
| `docs/backend/contract/analytics.md` | Contract — `analytics` |
| `docs/backend/contract/approvals.md` | Contract — `approvals` |
| `docs/backend/contract/core.md` | Contract — `core` |
| `docs/backend/contract/diagnostics.md` | Contract — `diagnostics` |
| `docs/backend/contract/expenses.md` | Contract — `expenses` |
| `docs/backend/contract/invoices.md` | Contract — `invoices` |
| `docs/backend/contract/mocks.md` | Mock engine functions (`src/mocks/backend` + `db.ts`) |
| `docs/backend/contract/parties.md` | Contract — `parties` |
| `docs/backend/contract/payments.md` | Contract — `payments` |
| `docs/backend/contract/products.md` | Contract — `products` |
| `docs/backend/contract/purchases.md` | Contract — `purchases` |
| `docs/backend/contract/README.md` | Backend contract inventory |
| `docs/backend/contract/reports.md` | Contract — `reports` |
| `docs/backend/contract/settings.md` | Contract — `settings` |
| `docs/backend/contract/setup.md` | Contract — `setup` |
| `docs/backend/contract/templates.md` | Contract — `templates` |
| `docs/backend/contract/users.md` | Contract — `users` |
| `docs/backend/contract/vouchers.md` | Contract — `vouchers` |
| `docs/design_system.md` | Design System — "Linear-style" Light/Dark |
| `docs/diagnostics/ISSUES.md` | سجل المشاكل (Issues) — ملف مُولَّد |
| `docs/diagnostics/issues/BUG-0001-onboarding-flow-crashed-unhandled-except.md` |  |
| `docs/diagnostics/issues/BUG-0002-setup-wizard-eg-flow-crashed-unhandled-e.md` |  |
| `docs/diagnostics/issues/BUG-0004-purchases-flow-crashed-unhandled-excepti.md` |  |
| `docs/diagnostics/issues/BUG-0005-branches-currencies-flow-failed-an-asser.md` |  |
| `docs/diagnostics/issues/BUG-0006-reports-v2-flow-crashed-unhandled-except.md` |  |
| `docs/diagnostics/issues/BUG-0007-full-persona-pass-flow-failed-an-asserti.md` |  |
| `docs/diagnostics/issues/BUG-0008-onboarding-flow-failed-an-assertion.md` |  |
| `docs/diagnostics/issues/BUG-0009-onboarding-flow-crashed-unhandled-except.md` |  |
| `docs/diagnostics/issues/BUG-0010-setup-wizard-eg-flow-crashed-unhandled-e.md` |  |
| `docs/diagnostics/issues/BUG-0011-onboarding-flow-crashed-unhandled-except.md` |  |
| `docs/diagnostics/issues/BUG-0012-setup-wizard-eg-flow-failed-an-assertion.md` |  |
| `docs/diagnostics/issues/BUG-0013-setup-wizard-eg-flow-crashed-unhandled-e.md` |  |
| `docs/diagnostics/issues/BUG-0014-onboarding-flow-crashed-unhandled-except.md` |  |
| `docs/diagnostics/issues/BUG-0015-maximum-call-stack-size-exceeded.md` |  |
| `docs/diagnostics/issues/BUG-0016-maximum-call-stack-size-exceeded.md` |  |
| `docs/diagnostics/issues/BUG-0017-failed-to-fetch-dynamically-imported-mod.md` |  |
| `docs/diagnostics/issues/BUG-0018-failed-to-execute-structuredclone-on-win.md` |  |
| `docs/diagnostics/issues/BUG-0019-failed-to-fetch-dynamically-imported-mod.md` |  |
| `docs/diagnostics/issues/DBG-0001-example-entry.md` |  |
| `docs/diagnostics/issues/PERF-0001-onboarding-flow-ran-218-slower-than-base.md` |  |
| `docs/diagnostics/issues/PERF-0002-setup-wizard-eg-flow-ran-567-slower-than.md` |  |
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
| `plans/pending/21-rust-backend/00-MASTER-PLAN.md` | 21 — Real backend: Tauri + Rust + SeaORM + MariaDB (master plan) |
| `plans/pending/21-rust-backend/01-FRONTEND-ANALYSIS.md` | 21 · Part 01 — Frontend analysis (the contract the Rust backend must honour) |
| `plans/pending/21-rust-backend/01-frontend-analysis/accounting.md` | 21 · 01.B — `accounting` contract |
| `plans/pending/21-rust-backend/01-frontend-analysis/analytics.md` | 21 · 01.B — `analytics` contract |
| `plans/pending/21-rust-backend/01-frontend-analysis/approvals.md` | 21 · 01.B — `approvals` contract |
| `plans/pending/21-rust-backend/01-frontend-analysis/core.md` | 21 · 01.B — `core` contract |
| `plans/pending/21-rust-backend/01-frontend-analysis/cross-cutting.md` | 21 · 01.D — Cross-cutting contract |
| `plans/pending/21-rust-backend/01-frontend-analysis/diagnostics.md` | 21 · 01.B — `diagnostics` contract |
| `plans/pending/21-rust-backend/01-frontend-analysis/expenses.md` | 21 · 01.B — `expenses` contract |
| `plans/pending/21-rust-backend/01-frontend-analysis/invoices.md` | 21 · 01.B — `invoices` contract |
| `plans/pending/21-rust-backend/01-frontend-analysis/parties.md` | 21 · 01.B — `parties` contract |
| `plans/pending/21-rust-backend/01-frontend-analysis/payments.md` | 21 · 01.B — `payments` contract |
| `plans/pending/21-rust-backend/01-frontend-analysis/products.md` | 21 · 01.B — `products` contract |
| `plans/pending/21-rust-backend/01-frontend-analysis/purchases.md` | 21 · 01.B — `purchases` contract |
| `plans/pending/21-rust-backend/01-frontend-analysis/reports.md` | 21 · 01.B — `reports` contract |
| `plans/pending/21-rust-backend/01-frontend-analysis/settings.md` | 21 · 01.B — `settings` contract |
| `plans/pending/21-rust-backend/01-frontend-analysis/setup.md` | 21 · 01.B — `setup` contract |
| `plans/pending/21-rust-backend/01-frontend-analysis/TEMPLATE.md` | 21 · 01.B — `<module>` contract |
| `plans/pending/21-rust-backend/01-frontend-analysis/templates.md` | 21 · 01.B — `templates` contract |
| `plans/pending/21-rust-backend/01-frontend-analysis/users.md` | 21 · 01.B — `users` contract |
| `plans/pending/21-rust-backend/01-frontend-analysis/vouchers.md` | 21 · 01.B — `vouchers` contract |
| `plans/pending/21-rust-backend/02-CORE-AND-SHARED-ARCHITECTURE.md` | 21 · Part 02 — Core and shared architecture (the Rust foundation) |
| `plans/pending/21-rust-backend/02-core-and-shared/phase-a-foundation.md` | 21 · 02.A — Foundation: workspace, DB, `AppError`, `AppState`, `with_tx` |
| `plans/pending/21-rust-backend/02-core-and-shared/phase-a2-bundled-database.md` | 21 · 02.A2 — Bundled MariaDB server (D11): payload, installer, provisioning, supervision |
| `plans/pending/21-rust-backend/02-core-and-shared/phase-a3-main-pc-hosting.md` | 21 · 02.A3 — Main-PC LAN hosting: firewall, pairing, continuity (D11, D8) |
| `plans/pending/21-rust-backend/02-core-and-shared/phase-b-entities.md` | 21 · 02.B — Entities and migrations for the 46 tables (+ `print_templates`) |
| `plans/pending/21-rust-backend/02-core-and-shared/phase-c-ledger.md` | 21 · 02.C — `shared::ledger`, `shared::numbering`, `shared::currency` |
| `plans/pending/21-rust-backend/02-core-and-shared/phase-d-stock.md` | 21 · 02.D — `shared::stock`, `shared::balances`, `shared::invariants` |
| `plans/pending/21-rust-backend/02-core-and-shared/phase-e-activity.md` | 21 · 02.E — `shared::activity` and the undo registry |
| `plans/pending/21-rust-backend/02-core-and-shared/phase-f-ipc-bridge.md` | 21 · 02.F — IPC bridge: typed bindings, the frontend switch, change events |
| `plans/pending/21-rust-backend/03-DOMAINS-IMPLEMENTATION.md` | 21 · Part 03 — Domains implementation (every `port` function on Rust) |
| `plans/pending/21-rust-backend/03-domains/_part02-gaps.md` | 21 · Part 03 — Part 02 gaps found while planning (manager-owned) |
| `plans/pending/21-rust-backend/03-domains/00-import.md` | 21 · 03.00 — `import` (D10 snapshot importer: MockDb snapshot → MariaDB, one transaction) |
| `plans/pending/21-rust-backend/03-domains/01-settings.md` | 21 · 03.01 — `settings` (store settings with the branch/device split, taxes, payment methods, branches, cost centers, currencies, revaluation, LAN sharing + server-failure screen) |
| `plans/pending/21-rust-backend/03-domains/02-setup.md` | 21 · 03.02 — `setup` (first-run device role + provisioning + terminal pairing, the 11-step wizard, opening balances, party openings) |
| `plans/pending/21-rust-backend/03-domains/03-users.md` | 21 · 03.03 — `users` (user records, login/session with argon2 credentials, manager PIN) |
| `plans/pending/21-rust-backend/03-domains/04-approvals.md` | 21 · 03.04 — `approvals` (async manager-approval queue) |
| `plans/pending/21-rust-backend/03-domains/05-parties.md` | 21 · 03.05 — `parties` (customers/suppliers, codes, statements, balances, aging, linking) |
| `plans/pending/21-rust-backend/03-domains/06-products.md` | 21 · 03.06 — `products` (catalog: products, categories, units, price lists, custom fields) |
| `plans/pending/21-rust-backend/03-domains/06b-inventory.md` | 21 · 03.06b — `products` (inventory: adjustments, movements, batches/expiry, stock counts, branch transfers) |
| `plans/pending/21-rust-backend/03-domains/07-purchases.md` | 21 · 03.07 — `purchases` (purchase orders, receiving at cost, landed costs, returns / debit notes) |
| `plans/pending/21-rust-backend/03-domains/08-invoices.md` | 21 · 03.08 — `invoices` part 1 (sales, sales returns, quotations, invoice reads, print data) |
| `plans/pending/21-rust-backend/03-domains/08b-pos-shifts.md` | 21 · 03.08b — `invoices` part 2 (POS shifts, cash in/out, X/Z report, held sales) |
| `plans/pending/21-rust-backend/03-domains/09-payments.md` | 21 · 03.09 — `payments` (customer receipts / supplier payments, sub-ledger allocations, realized FX) |
| `plans/pending/21-rust-backend/03-domains/10-vouchers.md` | 21 · 03.10 — `vouchers` (general receipt/payment/transfer/owner vouchers, card & wallet settlements) |
| `plans/pending/21-rust-backend/03-domains/11-expenses.md` | 21 · 03.11 — `expenses` (expense categories, one-shot expenses, recurring-expense templates) |
| `plans/pending/21-rust-backend/03-domains/12-accounting.md` | 21 · 03.12 — `accounting` (chart of accounts, manual journal, drafts, reversal, templates/recurring, undo) |
| `plans/pending/21-rust-backend/03-domains/12b-period-close.md` | 21 · 03.12b — `accounting` part 2 (fiscal years, lock date, year close/reopen, VAT settlement) |
| `plans/pending/21-rust-backend/03-domains/13-reports.md` | 21 · 03.13 — `reports` (read-only reports engine: financial statements and ledgers) |
| `plans/pending/21-rust-backend/03-domains/13b-reports-operational.md` | 21 · 03.13b — `reports` (read-only reports engine: operational reports) |
| `plans/pending/21-rust-backend/03-domains/14-analytics.md` | 21 · 03.14 — `analytics` + `dashboard` (read-only analytics tabs and home KPIs) |
| `plans/pending/21-rust-backend/03-domains/14b-insights.md` | 21 · 03.14b — `dashboard` insights (rule engine, product inline hints, thresholds) |
| `plans/pending/21-rust-backend/03-domains/15-templates.md` | 21 · 03.15 — `templates` (Typst print-template designer store, D9) |
| `plans/pending/21-rust-backend/03-domains/16-diagnostics.md` | 21 · 03.16 — `diagnostics` (audit-log reads, support-bundle data, debug-build accounting debugger) |
| `plans/pending/21-rust-backend/03-domains/17-backup.md` | 21 · 03.17 — `backup` (backup/restore through SQL in the existing archive format, auto backup on the Main PC, automatic backup before pending migrations) |
| `plans/pending/21-rust-backend/README.md` | 21 — Real backend (Tauri + Rust + SeaORM + MariaDB) |
| `plans/pending/22-invoice-templates/README.md` | 22 — Invoice templates: 10 × A4, 10 × mobile image, thermal unchanged |
