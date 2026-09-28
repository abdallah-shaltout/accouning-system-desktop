/**
 * Drift check (21.03 14-analytics.md §1): proves the ts-rs-generated `analytics` DTOs
 * (`analytics/types/gen/*`, written by `bun run bindings` from
 * `src-tauri/src/domains/analytics/dto.rs`) have exactly the same shape as the hand-written TS
 * types declared in `../services/analyticsService.ts`.
 *
 * Never imported by app code — this file exists only to be type-checked by `bun run build`.
 */
import type { Equals, Expect } from '@/modules/core/types/contract';
import type { CustomerAnalytics, ProductAnalytics, ProductProfitRow, SalesAnalytics, SalesTrendPoint } from '../services/analyticsService';

import type { SalesTrendPoint as GenSalesTrendPoint } from './gen/SalesTrendPoint';
import type { SalesAnalytics as GenSalesAnalytics } from './gen/SalesAnalytics';
import type { ProductProfitRow as GenProductProfitRow } from './gen/ProductProfitRow';
import type { ProductAnalytics as GenProductAnalytics } from './gen/ProductAnalytics';
import type { CustomerAnalytics as GenCustomerAnalytics } from './gen/CustomerAnalytics';
import type { CustomerShare as GenCustomerShare } from './gen/CustomerShare';

export type _SalesTrendPoint = Expect<Equals<GenSalesTrendPoint, SalesTrendPoint>>;
export type _SalesAnalytics = Expect<Equals<GenSalesAnalytics, SalesAnalytics>>;
export type _ProductProfitRow = Expect<Equals<GenProductProfitRow, ProductProfitRow>>;
export type _ProductAnalytics = Expect<Equals<GenProductAnalytics, ProductAnalytics>>;
export type _CustomerAnalytics = Expect<Equals<GenCustomerAnalytics, CustomerAnalytics>>;

// `CustomerAnalytics.topCustomers` has no hand-written named type (`analyticsService.ts:175`) —
// checked directly against the array element.
export type _CustomerShare = Expect<Equals<GenCustomerShare, CustomerAnalytics['topCustomers'][number]>>;
