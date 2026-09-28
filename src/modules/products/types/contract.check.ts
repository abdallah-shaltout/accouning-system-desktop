/**
 * Drift check (21.02-F, F-4 / plans/pending/21-rust-backend/03-domains/06-products.md §2 +
 * 06b-inventory.md §2): proves the ts-rs-generated `products` DTOs (`products/types/gen/*`,
 * written by `bun run bindings` from `src-tauri/src/domains/products/dto/{catalog,inventory}.rs`)
 * have exactly the same shape as the hand-written TS types in `./index.ts` /
 * `../services/{catalogService,inventoryService}.ts`.
 *
 * `Simplify<T>` (not a file-local `Flat<T>` — the plan named one, but this codebase already has
 * `Simplify` in `core/types/contract.ts` doing exactly that; CLAUDE.md's "never duplicate" rule
 * applies) flattens an intersection (`Category & { productCount }`) into a single object type so
 * `Equals` (identity, not mutual assignability) can compare it against a ts-rs-generated flat
 * struct — see `Simplify`'s own doc comment.
 *
 * Never imported by app code — this file exists only to be type-checked by `bun run build`. The
 * `./gen/*` imports do not exist until `bun run bindings` runs against a compiled `src-tauri`
 * (manager's wave build) — this file itself is not type-checkable until then.
 */
import type { Equals, Expect, Simplify } from '@/modules/core/types/contract';
import type {
  Category,
  CustomFieldDef,
  CustomFieldType,
  Product,
  ProductFilter,
  ProductInput,
  ProductUnit,
  ProductUnitPrice,
  PriceList,
  Unit,
  UnitPresetKind,
} from './index';

import type { AdjustmentFilter, MovementFilter, ExpiryBucket, ExpiryRow, StockMovementRow } from '../services/inventoryService';
import type { CustomFieldDefInput } from '../services/catalogService';

import type {
  StockAdjustment,
  StockAdjustmentInput,
  StockAdjustmentLine,
  StockAdjustmentType,
  StockCount,
  StockCountInput,
  StockCountScope,
  StockCountStatus,
  StockInReason,
  StockMovementReason,
  StockTransfer,
  StockTransferInput,
  StockTransferStatus,
  ReceiveTransferInput,
} from './index';

// --- 06-products.md catalog DTOs ------------------------------------------------------------------

import type { Product as GenProduct } from './gen/Product';
import type { ProductUnit as GenProductUnit } from './gen/ProductUnit';
import type { ProductUnitPrice as GenProductUnitPrice } from './gen/ProductUnitPrice';
import type { ProductInput as GenProductInput } from './gen/ProductInput';
import type { ProductFilter as GenProductFilter } from './gen/ProductFilter';
import type { Category as GenCategory } from './gen/Category';
import type { Unit as GenUnit } from './gen/Unit';
import type { UnitPresetKind as GenUnitPresetKind } from './gen/UnitPresetKind';
import type { PriceList as GenPriceList } from './gen/PriceList';
import type { CustomFieldDef as GenCustomFieldDef } from './gen/CustomFieldDef';
import type { CustomFieldType as GenCustomFieldType } from './gen/CustomFieldType';
import type { CustomFieldDefInput as GenCustomFieldDefInput } from './gen/CustomFieldDefInput';
import type { CategoryWithCount as GenCategoryWithCount } from './gen/CategoryWithCount';
import type { UnitWithCount as GenUnitWithCount } from './gen/UnitWithCount';

export type _Product = Expect<Equals<GenProduct, Product>>;
export type _ProductUnit = Expect<Equals<GenProductUnit, ProductUnit>>;
export type _ProductUnitPrice = Expect<Equals<GenProductUnitPrice, ProductUnitPrice>>;
export type _ProductInput = Expect<Equals<GenProductInput, Simplify<ProductInput>>>;
export type _ProductFilter = Expect<Equals<GenProductFilter, ProductFilter>>;
export type _Category = Expect<Equals<GenCategory, Category>>;
export type _Unit = Expect<Equals<GenUnit, Unit>>;
export type _UnitPresetKind = Expect<Equals<GenUnitPresetKind, UnitPresetKind>>;
export type _PriceList = Expect<Equals<GenPriceList, PriceList>>;
export type _CustomFieldDef = Expect<Equals<GenCustomFieldDef, CustomFieldDef>>;
export type _CustomFieldType = Expect<Equals<GenCustomFieldType, CustomFieldType>>;
export type _CustomFieldDefInput = Expect<Equals<GenCustomFieldDefInput, CustomFieldDefInput>>;

export type _CategoryWithCount = Expect<Equals<Simplify<GenCategoryWithCount>, Simplify<Category & { productCount: number }>>>;
export type _UnitWithCount = Expect<Equals<Simplify<GenUnitWithCount>, Simplify<Unit & { productCount: number }>>>;

// --- 06b-inventory.md inventory DTOs --------------------------------------------------------------

import type { StockAdjustmentType as GenStockAdjustmentType } from './gen/StockAdjustmentType';
import type { StockInReason as GenStockInReason } from './gen/StockInReason';
import type { StockAdjustmentLine as GenStockAdjustmentLine } from './gen/StockAdjustmentLine';
import type { StockAdjustment as GenStockAdjustment } from './gen/StockAdjustment';
import type { StockAdjustmentInput as GenStockAdjustmentInput } from './gen/StockAdjustmentInput';
import type { StockMovementReason as GenStockMovementReason } from './gen/StockMovementReason';
import type { StockMovement as GenStockMovement } from './gen/StockMovement';
import type { ProductBatch as GenProductBatch } from './gen/ProductBatch';
import type { DebitNoteDraft as GenDebitNoteDraft } from './gen/DebitNoteDraft';
import type { StockCountScope as GenStockCountScope } from './gen/StockCountScope';
import type { StockCountStatus as GenStockCountStatus } from './gen/StockCountStatus';
import type { StockCountLine as GenStockCountLine } from './gen/StockCountLine';
import type { StockCount as GenStockCount } from './gen/StockCount';
import type { StockCountInput as GenStockCountInput } from './gen/StockCountInput';
import type { StockTransferStatus as GenStockTransferStatus } from './gen/StockTransferStatus';
import type { StockTransferLine as GenStockTransferLine } from './gen/StockTransferLine';
import type { StockTransfer as GenStockTransfer } from './gen/StockTransfer';
import type { StockTransferInput as GenStockTransferInput } from './gen/StockTransferInput';
import type { ReceiveTransferInput as GenReceiveTransferInput } from './gen/ReceiveTransferInput';
import type { ExpiryBucket as GenExpiryBucket } from './gen/ExpiryBucket';
import type { StockMovementRow as GenStockMovementRow } from './gen/StockMovementRow';
import type { ExpiryRow as GenExpiryRow } from './gen/ExpiryRow';

import type { StockCountLine, StockTransferLine } from './index';

export type _StockAdjustmentType = Expect<Equals<GenStockAdjustmentType, StockAdjustmentType>>;
export type _StockInReason = Expect<Equals<GenStockInReason, StockInReason>>;
export type _StockAdjustmentLine = Expect<Equals<GenStockAdjustmentLine, StockAdjustmentLine>>;
export type _StockAdjustment = Expect<Equals<GenStockAdjustment, StockAdjustment>>;
export type _StockAdjustmentInput = Expect<Equals<GenStockAdjustmentInput, StockAdjustmentInput>>;
export type _StockMovementReason = Expect<Equals<GenStockMovementReason, StockMovementReason>>;
export type _StockMovement = Expect<Equals<GenStockMovement, import('./index').StockMovement>>;
export type _ProductBatch = Expect<Equals<GenProductBatch, import('./index').ProductBatch>>;
export type _DebitNoteDraft = Expect<Equals<GenDebitNoteDraft, import('./index').DebitNoteDraft>>;
export type _StockCountScope = Expect<Equals<GenStockCountScope, StockCountScope>>;
export type _StockCountStatus = Expect<Equals<GenStockCountStatus, StockCountStatus>>;
export type _StockCountLine = Expect<Equals<GenStockCountLine, StockCountLine>>;
export type _StockCount = Expect<Equals<GenStockCount, StockCount>>;
export type _StockCountInput = Expect<Equals<GenStockCountInput, StockCountInput>>;
export type _StockTransferStatus = Expect<Equals<GenStockTransferStatus, StockTransferStatus>>;
export type _StockTransferLine = Expect<Equals<GenStockTransferLine, StockTransferLine>>;
export type _StockTransfer = Expect<Equals<GenStockTransfer, StockTransfer>>;
export type _StockTransferInput = Expect<Equals<GenStockTransferInput, StockTransferInput>>;
export type _ReceiveTransferInput = Expect<Equals<GenReceiveTransferInput, ReceiveTransferInput>>;
export type _ExpiryBucket = Expect<Equals<GenExpiryBucket, ExpiryBucket>>;

export type _StockMovementRow = Expect<Equals<Simplify<GenStockMovementRow>, Simplify<StockMovementRow>>>;
export type _ExpiryRow = Expect<Equals<Simplify<GenExpiryRow>, Simplify<ExpiryRow>>>;

import type { StockAdjustmentDetail as GenStockAdjustmentDetail } from './gen/StockAdjustmentDetail';
export type _StockAdjustmentDetail = Expect<Equals<Simplify<GenStockAdjustmentDetail>, Simplify<StockAdjustment & { journalEntryId?: string }>>>;

// Filters are frontend-only shapes (not exported as ts-rs DTOs with the exact same field set as the
// Rust `Args` structs' `filter` field) — checked structurally against the Rust filter DTOs directly.
import type { AdjustmentFilter as GenAdjustmentFilter } from './gen/AdjustmentFilter';
import type { MovementFilter as GenMovementFilter } from './gen/MovementFilter';
export type _AdjustmentFilter = Expect<Equals<GenAdjustmentFilter, AdjustmentFilter>>;
export type _MovementFilter = Expect<Equals<GenMovementFilter, MovementFilter>>;
