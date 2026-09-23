import { ApiError, clone, db, delay, includesText, localDateKey, session, uid } from '@/mocks';
import { customerBalance, customerStatement, supplierBalance, supplierStatement } from '@/mocks/backend/balances';
import { logActivity } from '@/mocks/backend/core';
import { getOpenDocumentsFor, unallocatedCreditFor } from '@/mocks/backend/payments';
import { emit } from '@/mocks/events';
import { mutate } from '@/mocks/persist';
import type { AgingBucket, Customer, CustomerInput, PartyGroup, PartyHistoryEntry, PartyStatementRow, Supplier, SupplierInput } from '../types';

export interface PartyFilter {
  search?: string;
  includeInactive?: boolean;
  withBalanceOnly?: boolean;
  groupId?: string;
  overLimitOnly?: boolean;
}

export interface DuplicateWarning {
  field: 'phone' | 'vatNumber';
  existingId: string;
  existingName: string;
}

function validateCommon(input: { name: string; phone?: string; vatNumber?: string }) {
  if (!input.name.trim()) throw new ApiError('الاسم مطلوب');
  if (input.vatNumber && !/^3\d{13}3$/.test(input.vatNumber)) throw new ApiError('الرقم الضريبي يجب أن يكون 15 رقماً يبدأ وينتهي بالرقم 3');
}

function clean<T extends Record<string, any>>(input: T): T {
  return {
    ...input,
    name: input.name?.trim(),
    nameEn: input.nameEn?.trim() || undefined,
    phone: input.phone?.trim() || undefined,
    email: input.email?.trim() || undefined,
    address: input.address?.trim() || undefined,
    vatNumber: input.vatNumber?.trim() || undefined,
    crNumber: input.crNumber?.trim() || undefined,
    nationalId: input.nationalId?.trim() || undefined,
    notes: input.notes?.trim() || undefined,
  };
}

function nextCode(kind: 'customer' | 'supplier'): string {
  const list = kind === 'customer' ? db.customers : db.suppliers;
  const prefix = kind === 'customer' ? 'C-' : 'S-';
  const max = list.reduce((m, p) => {
    const n = Number(p.code?.replace(prefix, ''));
    return Number.isFinite(n) && n > m ? n : m;
  }, 0);
  return `${prefix}${String(max + 1).padStart(4, '0')}`;
}

/** Duplicate detection (docs/v2/08-customers-and-suppliers.md §1 "Checks: Duplicates"). */
export function findDuplicates(input: { phone?: string; vatNumber?: string }, excludeId?: string): DuplicateWarning[] {
  const all: (Customer | Supplier)[] = [...db.customers, ...db.suppliers];
  const warnings: DuplicateWarning[] = [];
  for (const p of all) {
    if (p.id === excludeId) continue;
    if (input.phone && (p.phone === input.phone || p.phones?.some((ph) => ph.number === input.phone))) {
      warnings.push({ field: 'phone', existingId: p.id, existingName: p.name });
    }
    if (input.vatNumber && p.vatNumber === input.vatNumber) {
      warnings.push({ field: 'vatNumber', existingId: p.id, existingName: p.name });
    }
  }
  return warnings;
}

export async function checkDuplicates(input: { phone?: string; vatNumber?: string }, excludeId?: string): Promise<DuplicateWarning[]> {
  await delay(150);
  return findDuplicates(input, excludeId);
}

// --- Groups ------------------------------------------------------------------------------------

export async function getPartyGroups(kind: 'customer' | 'supplier'): Promise<PartyGroup[]> {
  await delay(100);
  return clone(db.partyGroups.filter((g) => g.kind === kind));
}

// --- Customers -------------------------------------------------------------------------------

function withComputed(c: Customer): Customer {
  return { ...clone(c), balance: customerBalance(c.id), unallocatedCredit: unallocatedCreditFor('customer', c.id) };
}

export async function getCustomers(filter: PartyFilter = {}): Promise<Customer[]> {
  await delay();
  return db.customers
    .map(withComputed)
    .filter(
      (c) =>
        (filter.includeInactive || c.active) &&
        (!filter.withBalanceOnly || c.balance > 0) &&
        (!filter.groupId || c.groupId === filter.groupId) &&
        (!filter.overLimitOnly || ((c.creditLimit ?? 0) > 0 && c.balance > (c.creditLimit ?? 0))) &&
        includesText([c.name, c.nameEn, c.code, c.phone, c.vatNumber], filter.search),
    );
}

export async function getCustomer(id: string): Promise<Customer> {
  await delay();
  const c = db.customers.find((x) => x.id === id);
  if (!c) throw new ApiError('العميل غير موجود', 'NOT_FOUND');
  return withComputed(c);
}

export async function saveCustomer(input: CustomerInput, id?: string): Promise<Customer> {
  await delay();
  validateCommon(input);
  const data = clean(input);
  let customer: Customer;
  if (id) {
    const found = db.customers.find((c) => c.id === id);
    if (!found) throw new ApiError('العميل غير موجود', 'NOT_FOUND');
    if (data.active === false && customerBalance(id) > 0) throw new ApiError('لا يمكن إيقاف عميل عليه رصيد مستحق');
    mutate(() => Object.assign(found, data, { updatedAt: new Date().toISOString() }));
    customer = found;
    logActivity('party', `تعديل العميل ${customer.name}`, session.userId, new Date().toISOString(), `/customers/${customer.id}`);
    mutate(() =>
      db.partyHistory.push({ id: uid('phist'), partyId: customer.id, partyKind: 'customer', date: new Date().toISOString(), message: 'تعديل بيانات العميل', userId: session.userId }),
    );
  } else {
    customer = { id: uid('cus'), code: nextCode('customer'), balance: 0, createdAt: new Date().toISOString(), ...data } as Customer;
    mutate(() => db.customers.push(customer));
    logActivity('party', `إضافة العميل ${customer.name}`, session.userId, new Date().toISOString(), `/customers/${customer.id}`);
    mutate(() =>
      db.partyHistory.push({ id: uid('phist'), partyId: customer.id, partyKind: 'customer', date: new Date().toISOString(), message: 'إنشاء بطاقة العميل', userId: session.userId }),
    );
  }
  emit('parties:changed');
  return withComputed(customer);
}

export async function getCustomerStatement(id: string): Promise<PartyStatementRow[]> {
  await delay();
  return customerStatement(id);
}

// --- Suppliers -------------------------------------------------------------------------------

function withComputedSupplier(s: Supplier): Supplier {
  return { ...clone(s), balance: supplierBalance(s.id), unallocatedCredit: unallocatedCreditFor('supplier', s.id) };
}

export async function getSuppliers(filter: PartyFilter = {}): Promise<Supplier[]> {
  await delay();
  return db.suppliers
    .map(withComputedSupplier)
    .filter(
      (s) =>
        (filter.includeInactive || s.active) &&
        (!filter.withBalanceOnly || s.balance > 0) &&
        (!filter.groupId || s.groupId === filter.groupId) &&
        includesText([s.name, s.nameEn, s.code, s.phone, s.contactPerson, s.vatNumber], filter.search),
    );
}

export async function getSupplier(id: string): Promise<Supplier> {
  await delay();
  const s = db.suppliers.find((x) => x.id === id);
  if (!s) throw new ApiError('المورد غير موجود', 'NOT_FOUND');
  return withComputedSupplier(s);
}

export async function saveSupplier(input: SupplierInput, id?: string): Promise<Supplier> {
  await delay();
  validateCommon(input);
  const data = { ...clean(input), contactPerson: input.contactPerson?.trim() || undefined };
  let supplier: Supplier;
  if (id) {
    const found = db.suppliers.find((s) => s.id === id);
    if (!found) throw new ApiError('المورد غير موجود', 'NOT_FOUND');
    mutate(() => Object.assign(found, data, { updatedAt: new Date().toISOString() }));
    supplier = found;
    logActivity('party', `تعديل المورد ${supplier.name}`, session.userId, new Date().toISOString(), `/suppliers/${supplier.id}`);
    mutate(() =>
      db.partyHistory.push({ id: uid('phist'), partyId: supplier.id, partyKind: 'supplier', date: new Date().toISOString(), message: 'تعديل بيانات المورد', userId: session.userId }),
    );
  } else {
    supplier = { id: uid('sup'), code: nextCode('supplier'), balance: 0, createdAt: new Date().toISOString(), ...data } as Supplier;
    mutate(() => db.suppliers.push(supplier));
    logActivity('party', `إضافة المورد ${supplier.name}`, session.userId, new Date().toISOString(), `/suppliers/${supplier.id}`);
    mutate(() =>
      db.partyHistory.push({ id: uid('phist'), partyId: supplier.id, partyKind: 'supplier', date: new Date().toISOString(), message: 'إنشاء بطاقة المورد', userId: session.userId }),
    );
  }
  emit('parties:changed');
  return withComputedSupplier(supplier);
}

export async function getSupplierStatement(id: string): Promise<PartyStatementRow[]> {
  await delay();
  return supplierStatement(id);
}

// --- "Both roles" linking (docs/v2/08 §1 "Both roles") -----------------------------------------

/** Links a customer record and a supplier record as the same real-world party (net balance shown on each). */
export async function linkPartyRecords(customerId: string, supplierId: string): Promise<void> {
  await delay();
  const customer = db.customers.find((c) => c.id === customerId);
  const supplier = db.suppliers.find((s) => s.id === supplierId);
  if (!customer || !supplier) throw new ApiError('الطرف غير موجود', 'NOT_FOUND');
  mutate(() => {
    customer.linkedPartyId = supplier.id;
    supplier.linkedPartyId = customer.id;
  });
  emit('parties:changed');
}

export async function unlinkPartyRecord(partyId: string, kind: 'customer' | 'supplier'): Promise<void> {
  await delay();
  const list = kind === 'customer' ? db.customers : db.suppliers;
  const other = kind === 'customer' ? db.suppliers : db.customers;
  const p = list.find((x) => x.id === partyId);
  if (!p?.linkedPartyId) return;
  const counterpart = other.find((x) => x.id === p.linkedPartyId);
  mutate(() => {
    p.linkedPartyId = undefined;
    if (counterpart) counterpart.linkedPartyId = undefined;
  });
  emit('parties:changed');
}

/** Net balance across a linked customer+supplier pair (docs/v2/08 §1 "a net balance is shown on each"). */
export async function getLinkedNetBalance(customerId?: string, supplierId?: string): Promise<number | undefined> {
  await delay(80);
  if (!customerId || !supplierId) return undefined;
  return customerBalance(customerId) - supplierBalance(supplierId);
}

// --- History (docs/v2/08 §3 "السجل") ------------------------------------------------------------

export async function getPartyHistory(partyId: string): Promise<PartyHistoryEntry[]> {
  await delay(100);
  return clone(db.partyHistory.filter((h) => h.partyId === partyId)).sort((a, b) => b.date.localeCompare(a.date));
}

// --- Aging (docs/v2/08 §3 "الأعمار") ------------------------------------------------------------

const AGING_BUCKETS: { key: AgingBucket['key']; label: string }[] = [
  { key: 'current', label: 'حتى تاريخ الاستحقاق' },
  { key: '30', label: '1–30 يوم' },
  { key: '60', label: '31–60 يوم' },
  { key: '90plus', label: '90+ يوم' },
];

/**
 * Aging buckets by days overdue (docs/v2/08 §3): uses each document's `dueDate` when set (credit
 * sales — docs/v2/02-accounting-review.md D3), falling back to the document date for documents
 * without one (cash/no-terms docs rarely carry a balance, but the bucket still needs a date to
 * compare against "today").
 */
export async function getPartyAging(kind: 'customer' | 'supplier', partyId: string): Promise<AgingBucket[]> {
  await delay(120);
  const today = localDateKey(new Date());
  const docs = getOpenDocumentsFor(kind === 'customer' ? 'customer' : 'supplier', partyId);
  const buckets: AgingBucket[] = AGING_BUCKETS.map((b) => ({ ...b, total: 0, documents: [] }));
  for (const doc of docs) {
    const refDate = doc.dueDate ?? doc.date;
    const daysOverdue = Math.floor((new Date(today).getTime() - new Date(localDateKey(refDate)).getTime()) / 86_400_000);
    const bucketIndex = daysOverdue <= 0 ? 0 : daysOverdue <= 30 ? 1 : daysOverdue <= 60 ? 2 : 3;
    buckets[bucketIndex].total = Math.round((buckets[bucketIndex].total + doc.outstanding) * 100) / 100;
    buckets[bucketIndex].documents.push({ id: doc.id, number: doc.number, date: doc.date, dueDate: doc.dueDate, outstanding: doc.outstanding });
  }
  return buckets;
}
