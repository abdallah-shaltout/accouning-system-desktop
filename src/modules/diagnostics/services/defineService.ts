/**
 * Wraps a service function so every call is timed (perf channel) and its failures are logged
 * (error channel) with a correlation id, without changing its public API (18.B2 — pages must not
 * need edits).
 *
 * Applied per function, at its declaration (by `scripts/diagnostics/wrapServices.ts`'s codemod):
 *
 *   export const getSuppliers = wrap('parties.getSuppliers', function getSuppliers(filter) { ... });
 *
 * `wrap()` takes a *named* function expression on purpose: `scripts/memory/parse/tsImports.ts`'s
 * `export const NAME = ...` pattern already covers this shape (no parser change needed here,
 * unlike the destructured-object form this file used to expose), and keeping the function's own
 * name means any call to it elsewhere in the same file — including recursive/sibling calls between
 * service functions — still resolves through the function expression's own name binding.
 */
import { log, newCorrelationId, withCorrelation } from './logService';
import { PERF_BUDGET_MS } from '../config';

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type AnyFn = (...args: any[]) => any;

export function wrap<F extends AnyFn>(source: string, fn: F): F {
  const wrapped = (...args: Parameters<F>): ReturnType<F> => {
    const correlationId = newCorrelationId();
    const start = performance.now();

    const finish = (ok: boolean, error?: unknown) => {
      const durationMs = performance.now() - start;
      log.perf(source, `${durationMs.toFixed(1)}ms`, { durationMs, ok });
      if (durationMs > PERF_BUDGET_MS.service) {
        log.perf(source, `budget breach: ${durationMs.toFixed(1)}ms > ${PERF_BUDGET_MS.service}ms`, { durationMs, budget: PERF_BUDGET_MS.service });
      }
      if (!ok) {
        const err = error instanceof Error ? error : new Error(String(error));
        log.error(source, err.message, err, { args: redactArgs(args) });
      }
    };

    return withCorrelation(correlationId, () => {
      try {
        const result = fn(...args);
        if (result instanceof Promise) {
          return result.then(
            (v) => {
              finish(true);
              return v;
            },
            (e) => {
              finish(false, e);
              throw e;
            },
          ) as ReturnType<F>;
        }
        finish(true);
        return result;
      } catch (e) {
        finish(false, e);
        throw e;
      }
    });
  };
  return wrapped as F;
}

/** Args often carry full form objects (name, phone, notes…) — logService's own redaction handles
 * key-based secrets, but args are logged as a single opaque blob here to avoid ever writing a
 * customer's full record into a log file over a validation error. */
function redactArgs(args: unknown[]): string {
  return `${args.length} arg(s)`;
}
