import type { AppRoute } from '@/modules/core/types/route';

export type PartyKind = 'customer' | 'supplier';

/** Named route for a party screen — the single place that maps kind → route name. */
export function partyRoute(kind: PartyKind, action: 'list' | 'detail' | 'new' | 'edit', id?: string): AppRoute {
  if (kind === 'customer') {
    if (action === 'list') return { name: 'customers' };
    if (action === 'new') return { name: 'customer-new' };
    if (action === 'edit') return { name: 'customer-edit', params: { id: id! } };
    return { name: 'customer', params: { id: id! } };
  }
  if (action === 'list') return { name: 'suppliers' };
  if (action === 'new') return { name: 'supplier-new' };
  if (action === 'edit') return { name: 'supplier-edit', params: { id: id! } };
  return { name: 'supplier', params: { id: id! } };
}

/** A party id's own prefix says which kind it is (used for the duplicate-name warning link). */
export function partyRouteById(id: string): AppRoute {
  return id.startsWith('sup') ? partyRoute('supplier', 'detail', id) : partyRoute('customer', 'detail', id);
}
