/**
 * Drift check (21.02-F, F-4 / plans/pending/21-rust-backend/03-domains/01-settings.md §2): proves
 * the ts-rs-generated `settings` DTOs (`settings/types/gen/*`, written by `bun run bindings` from
 * `src-tauri/src/domains/settings/dto.rs`) have exactly the same shape as the hand-written TS
 * types in `./index.ts` / `./dimensions.ts` / `./network.ts`.
 *
 * Never imported by app code — this file exists only to be type-checked by `bun run build`.
 */
import type { Equals, Expect } from '@/modules/core/types/contract';
import type { Branch, BranchInput, CostCenter, CostCenterBudget, CostCenterInput, Currency, ExchangeRate, ExchangeRateInput, FcBalanceRow, PaymentMethod, PaymentMethodInput, RevaluationResult, StoreSettings, Tax } from './index';
import type { LanSharingStatus, PairingInfo } from './network';
import type { AutoBackupOutcome, AutoBackupTrigger, BackupArchive, BackupKind, BackupManifest, BackupSettings, RestorePreview } from './backup';

import type { StoreSettings as GenStoreSettings } from './gen/StoreSettings';
import type { Tax as GenTax } from './gen/Tax';
import type { PaymentMethod as GenPaymentMethod } from './gen/PaymentMethod';
import type { PaymentMethodInput as GenPaymentMethodInput } from './gen/PaymentMethodInput';
import type { Branch as GenBranch } from './gen/Branch';
import type { BranchInput as GenBranchInput } from './gen/BranchInput';
import type { CostCenter as GenCostCenter } from './gen/CostCenter';
import type { CostCenterInput as GenCostCenterInput } from './gen/CostCenterInput';
import type { CostCenterBudget as GenCostCenterBudget } from './gen/CostCenterBudget';
import type { Currency as GenCurrency } from './gen/Currency';
import type { ExchangeRate as GenExchangeRate } from './gen/ExchangeRate';
import type { ExchangeRateInput as GenExchangeRateInput } from './gen/ExchangeRateInput';
import type { FcBalanceRow as GenFcBalanceRow } from './gen/FcBalanceRow';
import type { RevaluationResult as GenRevaluationResult } from './gen/RevaluationResult';
import type { PairingInfo as GenPairingInfo } from './gen/PairingInfo';
import type { LanSharingStatus as GenLanSharingStatus } from './gen/LanSharingStatus';
import type { BackupManifest as GenBackupManifest } from './gen/BackupManifest';
import type { BackupKind as GenBackupKind } from './gen/BackupKind';
import type { BackupSettings as GenBackupSettings } from './gen/BackupSettings';
import type { RestorePreview as GenRestorePreview } from './gen/RestorePreview';
import type { BackupArchive as GenBackupArchive } from './gen/BackupArchive';
import type { AutoBackupOutcome as GenAutoBackupOutcome } from './gen/AutoBackupOutcome';
import type { AutoBackupTrigger as GenAutoBackupTrigger } from './gen/AutoBackupTrigger';

// `StoreSettingsPatch` vs `Partial<StoreSettings>`: ts-rs cannot express the nested `Partial` of
// `printer` (every inner key optional independently of the outer key), so per the entry file's own
// escape hatch this checks `StoreSettings` itself (below) rather than the patch shape.
// contract-ok: request-only Partial, validated field-by-field in Rust (entry §3.4)
export type _StoreSettings = Expect<Equals<GenStoreSettings, StoreSettings>>;

export type _Tax = Expect<Equals<GenTax, Tax>>;
export type _PaymentMethod = Expect<Equals<GenPaymentMethod, PaymentMethod>>;
export type _PaymentMethodInput = Expect<Equals<GenPaymentMethodInput, PaymentMethodInput>>;
export type _Branch = Expect<Equals<GenBranch, Branch>>;
export type _BranchInput = Expect<Equals<GenBranchInput, BranchInput>>;
export type _CostCenter = Expect<Equals<GenCostCenter, CostCenter>>;
export type _CostCenterInput = Expect<Equals<GenCostCenterInput, CostCenterInput>>;
export type _CostCenterBudget = Expect<Equals<GenCostCenterBudget, CostCenterBudget>>;
export type _Currency = Expect<Equals<GenCurrency, Currency>>;
export type _ExchangeRate = Expect<Equals<GenExchangeRate, ExchangeRate>>;
export type _ExchangeRateInput = Expect<Equals<GenExchangeRateInput, ExchangeRateInput>>;
export type _FcBalanceRow = Expect<Equals<GenFcBalanceRow, FcBalanceRow>>;
export type _RevaluationResult = Expect<Equals<GenRevaluationResult, RevaluationResult>>;
export type _PairingInfo = Expect<Equals<GenPairingInfo, PairingInfo>>;
export type _LanSharingStatus = Expect<Equals<GenLanSharingStatus, LanSharingStatus>>;

export type _BackupManifest = Expect<Equals<GenBackupManifest, BackupManifest>>;
export type _BackupKind = Expect<Equals<GenBackupKind, BackupKind>>;
export type _BackupSettings = Expect<Equals<GenBackupSettings, BackupSettings>>;
export type _RestorePreview = Expect<Equals<GenRestorePreview, RestorePreview>>;
export type _BackupArchive = Expect<Equals<GenBackupArchive, BackupArchive>>;
export type _AutoBackupOutcome = Expect<Equals<GenAutoBackupOutcome, AutoBackupOutcome>>;
export type _AutoBackupTrigger = Expect<Equals<GenAutoBackupTrigger, AutoBackupTrigger>>;
