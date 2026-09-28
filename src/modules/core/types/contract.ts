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
