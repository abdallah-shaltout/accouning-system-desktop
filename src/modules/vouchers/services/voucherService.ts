import { clone, db, delay, inDateRange, includesText, session } from '@/mocks';
import { wrap } from '@/modules/diagnostics/services/defineService';

import {
  getVoucherById,
  recordOwnerVoucher,
  recordPaymentVoucher,
  recordReceiptVoucher,
  recordTransferVoucher,
} from '@/mocks/backend/vouchers';
import {
  estimatedFeeFor,
  getCardSettlementById,
  recordCardSettlement,
  unsettledTenderGroups,
} from '@/mocks/backend/settlements';
import type {
  CardSettlement,
  CardSettlementInput,
  OwnerVoucherInput,
  PaymentVoucherInput,
  ReceiptVoucherInput,
  TransferVoucherInput,
  UnsettledTenderGroup,
  Voucher,
  VoucherFilter,
} from '../types';

export const createReceiptVoucher = wrap('vouchers.createReceiptVoucher', async function createReceiptVoucher(input: ReceiptVoucherInput): Promise<Voucher> {
  await delay(300);
  return clone(recordReceiptVoucher(input, session.userId));
});

export const createPaymentVoucher = wrap('vouchers.createPaymentVoucher', async function createPaymentVoucher(input: PaymentVoucherInput): Promise<Voucher> {
  await delay(300);
  return clone(recordPaymentVoucher(input, session.userId));
});

export const createTransferVoucher = wrap('vouchers.createTransferVoucher', async function createTransferVoucher(input: TransferVoucherInput): Promise<Voucher> {
  await delay(300);
  return clone(recordTransferVoucher(input, session.userId));
});

export const createOwnerVoucher = wrap('vouchers.createOwnerVoucher', async function createOwnerVoucher(input: OwnerVoucherInput): Promise<Voucher> {
  await delay(300);
  return clone(recordOwnerVoucher(input, session.userId));
});

export const getVouchers = wrap('vouchers.getVouchers', async function getVouchers(filter: VoucherFilter = {}): Promise<Voucher[]> {
  await delay();
  return db.vouchers
    .filter((v) => (!filter.kind || v.kind === filter.kind) && inDateRange(v.date, filter.from, filter.to))
    .filter((v) => includesText([v.number, v.description, v.note], filter.search))
    .map(clone)
    .sort((a, b) => b.date.localeCompare(a.date));
});

export const getVoucher = wrap('vouchers.getVoucher', async function getVoucher(id: string): Promise<Voucher> {
  await delay();
  return clone(getVoucherById(id));
});

// --- Card/wallet settlement (docs/v2/09 §2) -----------------------------------------------------

export const getUnsettledTenderGroups = wrap('vouchers.getUnsettledTenderGroups', async function getUnsettledTenderGroups(): Promise<UnsettledTenderGroup[]> {
  await delay();
  return clone(unsettledTenderGroups());
});

export const estimateSettlementFee = wrap('vouchers.estimateSettlementFee', async function estimateSettlementFee(groups: UnsettledTenderGroup[]): Promise<number> {
  await delay(0);
  return estimatedFeeFor(groups);
});

export const createCardSettlement = wrap('vouchers.createCardSettlement', async function createCardSettlement(input: CardSettlementInput): Promise<CardSettlement> {
  await delay(300);
  return clone(recordCardSettlement(input, session.userId));
});

export const getCardSettlements = wrap('vouchers.getCardSettlements', async function getCardSettlements(): Promise<CardSettlement[]> {
  await delay();
  return clone(db.cardSettlements).sort((a, b) => b.date.localeCompare(a.date));
});

export const getCardSettlement = wrap('vouchers.getCardSettlement', async function getCardSettlement(id: string): Promise<CardSettlement> {
  await delay();
  return clone(getCardSettlementById(id));
});
