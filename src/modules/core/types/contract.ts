/**
 * Exact-identity type equality (21.02-F, F-4) — used by `contract.check.ts` files across modules
 * to prove a ts-rs-generated Rust DTO has exactly the same shape as the hand-written TS type it
 * mirrors (master plan rule 1). Mutual assignability (`A extends B ? B extends A ? …`) is not
 * enough: it would pass for structurally-equivalent-but-differently-optional fields, which is
 * exactly the kind of drift this check exists to catch. The conditional-type identity trick below
 * (comparing two generic instantiations of a dummy function type) does distinguish those cases.
 */

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type Equals<A, B> = (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2 ? true : false;

/** Fails to compile (a type error, not a runtime one) unless `T` is exactly `true`. */
export type Expect<T extends true> = T;

/**
 * G-38: flattens an intersection type into a single object type with the same members, so `Equals`
 * (an identity check, not mutual assignability) can compare it against a ts-rs-generated flat
 * struct. Without this, a hand-written type built as an intersection — `HomeKpi & { marginPct:
 * number }`, `Invoice & { customerName?: string }` — is never *identical* to the Rust side's single
 * flat struct even when every member matches exactly, because `Equals` distinguishes an
 * intersection type from the flattened object type it's assignable to/from. `contract.check.ts`
 * files compare `Simplify<Gen.X>` against `Simplify<X>` for exactly this reason.
 */
export type Simplify<T> = { [K in keyof T]: T[K] };
