import { clone, db, delay, inDateRange, includesText, session } from '@/mocks';
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

export async function createReceiptVoucher(input: ReceiptVoucherInput): Promise<Voucher> {
  await delay(300);
  return clone(recordReceiptVoucher(input, session.userId));
}

export async function createPaymentVoucher(input: PaymentVoucherInput): Promise<Voucher> {
  await delay(300);
  return clone(recordPaymentVoucher(input, session.userId));
}

export async function createTransferVoucher(input: TransferVoucherInput): Promise<Voucher> {
  await delay(300);
  return clone(recordTransferVoucher(input, session.userId));
}

export async function createOwnerVoucher(input: OwnerVoucherInput): Promise<Voucher> {
  await delay(300);
  return clone(recordOwnerVoucher(input, session.userId));
}

export async function getVouchers(filter: VoucherFilter = {}): Promise<Voucher[]> {
  await delay();
  return db.vouchers
    .filter((v) => (!filter.kind || v.kind === filter.kind) && inDateRange(v.date, filter.from, filter.to))
    .filter((v) => includesText([v.number, v.description, v.note], filter.search))
    .map(clone)
    .sort((a, b) => b.date.localeCompare(a.date));
}

export async function getVoucher(id: string): Promise<Voucher> {
  await delay();
  return clone(getVoucherById(id));
}

// --- Card/wallet settlement (docs/v2/09 §2) -----------------------------------------------------

export async function getUnsettledTenderGroups(): Promise<UnsettledTenderGroup[]> {
  await delay();
  return clone(unsettledTenderGroups());
}

export async function estimateSettlementFee(groups: UnsettledTenderGroup[]): Promise<number> {
  await delay(0);
  return estimatedFeeFor(groups);
}

export async function createCardSettlement(input: CardSettlementInput): Promise<CardSettlement> {
  await delay(300);
  return clone(recordCardSettlement(input, session.userId));
}

export async function getCardSettlements(): Promise<CardSettlement[]> {
  await delay();
  return clone(db.cardSettlements).sort((a, b) => b.date.localeCompare(a.date));
}

export async function getCardSettlement(id: string): Promise<CardSettlement> {
  await delay();
  return clone(getCardSettlementById(id));
}
