/**
 * v2 phase 9 (docs/v2/07-products-and-inventory.md §4 "Branch stock & transfers", deferred from
 * phase 6): thin service wrapper around `src/mocks/backend/transfers.ts` — pages never import the
 * mock backend directly.
 */
import { clone, session } from '@/mocks';
import * as backend from '@/mocks/backend/transfers';
import type { ReceiveTransferInput, StockTransfer, StockTransferInput } from '../types';

import { wrap } from '@/modules/diagnostics/services/defineService';

export const getTransfers = wrap('products.getTransfers', async function getTransfers(): Promise<StockTransfer[]> {
  return clone(backend.listTransfers());
});

export const getTransfer = wrap('products.getTransfer', async function getTransfer(id: string): Promise<StockTransfer> {
  return clone(backend.transferById(id));
});

export const createTransfer = wrap('products.createTransfer', async function createTransfer(input: StockTransferInput): Promise<StockTransfer> {
  return clone(backend.draftTransfer(input, session.userId));
});

export const sendTransfer = wrap('products.sendTransfer', async function sendTransfer(id: string): Promise<StockTransfer> {
  return clone(backend.sendTransfer(id, session.userId));
});

export const receiveTransfer = wrap('products.receiveTransfer', async function receiveTransfer(id: string, input: ReceiveTransferInput): Promise<StockTransfer> {
  return clone(backend.receiveTransfer(id, input, session.userId));
});

export const rejectTransfer = wrap('products.rejectTransfer', async function rejectTransfer(id: string, reason: string): Promise<StockTransfer> {
  return clone(backend.rejectTransfer(id, reason, session.userId));
});

export const branchStockQty = wrap('products.branchStockQty', function branchStockQty(productId: string, branchId: string): number {
  return backend.branchStockQty(productId, branchId);
});
