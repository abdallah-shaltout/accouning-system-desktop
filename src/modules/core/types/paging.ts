/**
 * The paged query contract used by big list services (invoices, journal, stock movements,
 * payments) and `DataTable`'s server mode. Defined once here so every service that adopts it
 * speaks the same shape — see docs/v2/14-platform.md §7.
 */

export interface PageSort {
  key: string;
  dir: 'asc' | 'desc';
}

export interface PagedQuery<F = Record<string, unknown>> {
  page: number;
  pageSize: number;
  sort?: PageSort;
  filters?: F;
}

export interface PagedResult<R> {
  rows: R[];
  total: number;
  /** Optional aggregate totals over the *filtered* set (not just the current page), e.g. sums for a footer row. */
  totals?: Record<string, number>;
}
