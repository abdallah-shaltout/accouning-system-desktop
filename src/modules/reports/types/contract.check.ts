/**
 * Drift check (21.02-F, F-4 / plans/pending/21-rust-backend/03-domains/13-reports.md §2,
 * 13b-reports-operational.md §2): proves the ts-rs-generated `reports` DTOs
 * (`reports/types/gen/*`, written by `bun run bindings` from
 * `src-tauri/src/domains/reports/dto.rs`) have exactly the same shape as the hand-written TS
 * types in `./index.ts` / `../services/reportService.ts`.
 *
 * `ReportRangeFilter` is compared as `Equals<Gen.ReportRangeFilter, ReportRangeFilter>` — the
 * Rust side's flat-field struct and the TS `DateRangeInput & DimensionFilter` intersection have
 * the exact same set of optional fields, so `Simplify<T>` (`core/types/contract.ts`) flattens the
 * intersection before comparing (G-38). `AccountLedger.rowLinks` is `// contract-ok: AppRoute is
 * imported, not generated` — the Rust field's `#[ts(type = "...")]` override already imports the
 * real `AppRoute` type (`core/types/route.ts`), so no separate generated shim exists to compare.
 *
 * Never imported by app code — this file exists only to be type-checked by `bun run build`.
 */
import type { Equals, Expect, Simplify } from '@/modules/core/types/contract';
import type {
  AccountLedger,
  AgingReportRow,
  BalanceSheet,
  BranchComparisonRow,
  BusinessHealthReport,
  BusinessHealthScore,
  CashFlowStatement,
  CostCenterBudgetRow,
  CostCenterPnl,
  CostCenterPnlColumn,
  DateRangeInput,
  DayBookEntry,
  DeadStockRow,
  DimensionFilter,
  DiscountReportRow,
  ExpensesReport,
  GrossProfitRow,
  InventoryReportRow,
  LowStockRow,
  OverdueRow,
  PeriodComparisonLine,
  ProfitAndLoss,
  ProfitLeakageReport,
  PurchasesReport,
  ReportRangeFilter,
  ReturnsReport,
  ReturnsReportRow,
  SalesReport,
  ShiftReportRow,
  StatementLine,
  StocktakeVarianceRow,
  TransferReportRow,
  TrialBalanceRow,
  VatCategoryBox,
  VatDetailRow,
  VatReport,
} from './index';

import { getDimensionOptions, getLedgerTargets, getProfitAndLossComparison } from '../services/reportService';

import type { TrialBalanceRow as GenTrialBalanceRow } from './gen/TrialBalanceRow';
import type { StatementLine as GenStatementLine } from './gen/StatementLine';
import type { ProfitAndLoss as GenProfitAndLoss } from './gen/ProfitAndLoss';
import type { PnlComparison as GenPnlComparison } from './gen/PnlComparison';
import type { CostCenterPnlColumn as GenCostCenterPnlColumn } from './gen/CostCenterPnlColumn';
import type { CostCenterPnl as GenCostCenterPnl } from './gen/CostCenterPnl';
import type { CostCenterBudgetRow as GenCostCenterBudgetRow } from './gen/CostCenterBudgetRow';
import type { BalanceSheet as GenBalanceSheet } from './gen/BalanceSheet';
import type { AccountLedger as GenAccountLedger } from './gen/AccountLedger';
import type { VatCategoryBox as GenVatCategoryBox } from './gen/VatCategoryBox';
import type { VatReport as GenVatReport } from './gen/VatReport';
import type { VatDetailRow as GenVatDetailRow } from './gen/VatDetailRow';
import type { CashFlowStatement as GenCashFlowStatement } from './gen/CashFlowStatement';
import type { DayBookEntry as GenDayBookEntry } from './gen/DayBookEntry';
import type { PeriodComparisonLine as GenPeriodComparisonLine } from './gen/PeriodComparisonLine';
import type { BusinessHealthScore as GenBusinessHealthScore } from './gen/BusinessHealthScore';
import type { BusinessHealthReport as GenBusinessHealthReport } from './gen/BusinessHealthReport';
import type { LedgerTargets as GenLedgerTargets } from './gen/LedgerTargets';
import type { DimensionOptions as GenDimensionOptions } from './gen/DimensionOptions';
import type { ReportRangeFilter as GenReportRangeFilter } from './gen/ReportRangeFilter';
import type { AgingReportRow as GenAgingReportRow } from './gen/AgingReportRow';
import type { OverdueRow as GenOverdueRow } from './gen/OverdueRow';

import type { SalesReport as GenSalesReport } from './gen/SalesReport';
import type { InventoryReportRow as GenInventoryReportRow } from './gen/InventoryReportRow';
import type { DiscountReportRow as GenDiscountReportRow } from './gen/DiscountReportRow';
import type { GrossProfitRow as GenGrossProfitRow } from './gen/GrossProfitRow';
import type { ReturnsReportRow as GenReturnsReportRow } from './gen/ReturnsReportRow';
import type { ReturnsReport as GenReturnsReport } from './gen/ReturnsReport';
import type { ExpensesReport as GenExpensesReport } from './gen/ExpensesReport';
import type { ShiftReportRow as GenShiftReportRow } from './gen/ShiftReportRow';
import type { LowStockRow as GenLowStockRow } from './gen/LowStockRow';
import type { DeadStockRow as GenDeadStockRow } from './gen/DeadStockRow';
import type { StocktakeVarianceRow as GenStocktakeVarianceRow } from './gen/StocktakeVarianceRow';
import type { TransferReportRow as GenTransferReportRow } from './gen/TransferReportRow';
import type { PurchasesReport as GenPurchasesReport } from './gen/PurchasesReport';
import type { BranchComparisonRow as GenBranchComparisonRow } from './gen/BranchComparisonRow';
import type { ProfitLeakageReport as GenProfitLeakageReport } from './gen/ProfitLeakageReport';
import type { DateRangeInput as GenDateRangeInput } from './gen/DateRangeInput';
import type { DimensionFilter as GenDimensionFilter } from './gen/DimensionFilter';

// --- 13 — statements / ledgers / VAT / cash flow / lookups (21 entries) -------------------------

export type _TrialBalanceRow = Expect<Equals<GenTrialBalanceRow, TrialBalanceRow>>;
export type _StatementLine = Expect<Equals<GenStatementLine, StatementLine>>;
export type _ProfitAndLoss = Expect<Equals<GenProfitAndLoss, ProfitAndLoss>>;
export type _PnlComparison = Expect<Equals<GenPnlComparison, Awaited<ReturnType<typeof getProfitAndLossComparison>>>>;
export type _CostCenterPnlColumn = Expect<Equals<GenCostCenterPnlColumn, CostCenterPnlColumn>>;
export type _CostCenterPnl = Expect<Equals<GenCostCenterPnl, CostCenterPnl>>;
export type _CostCenterBudgetRow = Expect<Equals<GenCostCenterBudgetRow, CostCenterBudgetRow>>;
export type _BalanceSheet = Expect<Equals<GenBalanceSheet, BalanceSheet>>;
// contract-ok: AppRoute is imported, not generated — `rowLinks`' Rust field has its own
// `#[ts(type = "...")]` override importing the real `AppRoute` type.
export type _AccountLedger = Expect<Equals<GenAccountLedger, AccountLedger>>;
export type _VatCategoryBox = Expect<Equals<GenVatCategoryBox, VatCategoryBox>>;
export type _VatReport = Expect<Equals<GenVatReport, VatReport>>;
export type _VatDetailRow = Expect<Equals<GenVatDetailRow, VatDetailRow>>;
export type _CashFlowStatement = Expect<Equals<GenCashFlowStatement, CashFlowStatement>>;
export type _DayBookEntry = Expect<Equals<GenDayBookEntry, DayBookEntry>>;
export type _PeriodComparisonLine = Expect<Equals<GenPeriodComparisonLine, PeriodComparisonLine>>;
export type _BusinessHealthScore = Expect<Equals<GenBusinessHealthScore, BusinessHealthScore>>;
export type _BusinessHealthReport = Expect<Equals<GenBusinessHealthReport, BusinessHealthReport>>;
export type _LedgerTargets = Expect<Equals<GenLedgerTargets, Awaited<ReturnType<typeof getLedgerTargets>>>>;
export type _DimensionOptions = Expect<Equals<GenDimensionOptions, Awaited<ReturnType<typeof getDimensionOptions>>>>;
// `ReportRangeFilter` = `DateRangeInput & DimensionFilter` on the TS side (an intersection type);
// the Rust struct is flat with the same optional fields — `Simplify` normalizes both before
// comparing (G-38).
export type _ReportRangeFilter = Expect<Equals<Simplify<GenReportRangeFilter>, Simplify<ReportRangeFilter>>>;
export type _AgingReportRow = Expect<Equals<GenAgingReportRow, AgingReportRow>>;
export type _OverdueRow = Expect<Equals<GenOverdueRow, OverdueRow>>;

// --- 13b — operational reports (20 entries) ------------------------------------------------------

export type _SalesReport = Expect<Equals<GenSalesReport, SalesReport>>;
export type _InventoryReportRow = Expect<Equals<GenInventoryReportRow, InventoryReportRow>>;
export type _DiscountReportRow = Expect<Equals<GenDiscountReportRow, DiscountReportRow>>;
export type _GrossProfitRow = Expect<Equals<GenGrossProfitRow, GrossProfitRow>>;
export type _ReturnsReportRow = Expect<Equals<GenReturnsReportRow, ReturnsReportRow>>;
export type _ReturnsReport = Expect<Equals<GenReturnsReport, ReturnsReport>>;
export type _ExpensesReport = Expect<Equals<GenExpensesReport, ExpensesReport>>;
export type _ShiftReportRow = Expect<Equals<GenShiftReportRow, ShiftReportRow>>;
export type _LowStockRow = Expect<Equals<GenLowStockRow, LowStockRow>>;
export type _DeadStockRow = Expect<Equals<GenDeadStockRow, DeadStockRow>>;
export type _StocktakeVarianceRow = Expect<Equals<GenStocktakeVarianceRow, StocktakeVarianceRow>>;
export type _TransferReportRow = Expect<Equals<GenTransferReportRow, TransferReportRow>>;
export type _PurchasesReport = Expect<Equals<GenPurchasesReport, PurchasesReport>>;
export type _BranchComparisonRow = Expect<Equals<GenBranchComparisonRow, BranchComparisonRow>>;
export type _ProfitLeakageReport = Expect<Equals<GenProfitLeakageReport, ProfitLeakageReport>>;

// `DateRangeInput`/`DimensionFilter` are checked once here (both feed `ReportRangeFilter` above and
// are used bare by several 13b commands) rather than once per call site.
export type _DateRangeInput = Expect<Equals<GenDateRangeInput, DateRangeInput>>;
export type _DimensionFilter = Expect<Equals<GenDimensionFilter, DimensionFilter>>;
