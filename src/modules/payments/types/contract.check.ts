/**
 * Drift check (21.02-F, F-4 / plans/pending/21-rust-backend/03-domains/09-payments.md §2): proves
 * the ts-rs-generated `payments` DTOs (`payments/types/gen/*`, written by `bun run bindings` from
 * `src-tauri/src/domains/payments/dto.rs`) have exactly the same shape as the hand-written TS
 * types in `./index.ts` / `../services/paymentService.ts`.
 *
 * Never imported by app code — this file exists only to be type-checked by `bun run build`.
 */
import type { Equals, Expect, Simplify } from '@/modules/core/types/contract';
import type { AllocationStatus, OpenDocument, Payment, PaymentAllocation, PaymentAllocationInput, PaymentFilter, PaymentInput } from './index';

import type { PaymentRow } from '../services/paymentService';

import type { AllocationStatus as GenAllocationStatus } from './gen/AllocationStatus';
import type { OpenDocument as GenOpenDocument } from './gen/OpenDocument';
import type { Payment as GenPayment } from './gen/Payment';
import type { PaymentAllocation as GenPaymentAllocation } from './gen/PaymentAllocation';
import type { PaymentAllocationInput as GenPaymentAllocationInput } from './gen/PaymentAllocationInput';
import type { PaymentFilter as GenPaymentFilter } from './gen/PaymentFilter';
import type { PaymentInput as GenPaymentInput } from './gen/PaymentInput';
import type { PaymentRow as GenPaymentRow } from './gen/PaymentRow';

export type _Payment = Expect<Equals<GenPayment, Payment>>;
export type _PaymentAllocation = Expect<Equals<GenPaymentAllocation, PaymentAllocation>>;
export type _PaymentAllocationInput = Expect<Equals<GenPaymentAllocationInput, PaymentAllocationInput>>;
export type _PaymentInput = Expect<Equals<GenPaymentInput, PaymentInput>>;
export type _PaymentFilter = Expect<Equals<GenPaymentFilter, PaymentFilter>>;
export type _OpenDocument = Expect<Equals<GenOpenDocument, OpenDocument>>;
export type _AllocationStatus = Expect<Equals<GenAllocationStatus, AllocationStatus>>;
// contract-ok: `PaymentRow = Payment & { partyName, allocated, unallocated, allocationStatus }`
// (`paymentService.ts`) is an intersection type; ts-rs emits a single flat object literal for
// `Gen`. `Simplify<>` (see its doc comment in `core/types/contract.ts`, written for exactly this
// "hand-written intersection vs Rust flat struct" shape) flattens the intersection before the
// exact-identity check — every field matches 1:1 once flattened.
export type _PaymentRow = Expect<Equals<Simplify<GenPaymentRow>, Simplify<PaymentRow>>>;
