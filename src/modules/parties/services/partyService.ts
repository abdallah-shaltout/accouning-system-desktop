import { ApiError, clone, db, delay, includesText, session, uid } from '@/mocks';
import { customerBalance, customerStatement, supplierBalance, supplierStatement } from '@/mocks/backend/balances';
import { logActivity } from '@/mocks/backend/core';
import type { Customer, CustomerInput, PartyStatementRow, Supplier, SupplierInput } from '../types';

export interface PartyFilter {
  search?: string;
  includeInactive?: boolean;
  withBalanceOnly?: boolean;
}

function validateCommon(input: { name: string; phone?: string; vatNumber?: string }) {
  if (!input.name.trim()) throw new ApiError('الاسم مطلوب');
  if (input.vatNumber && !/^3\d{13}3$/.test(input.vatNumber)) throw new ApiError('الرقم الضريبي يجب أن يكون 15 رقماً يبدأ وينتهي بالرقم 3');
}

function clean<T extends { name: string; phone?: string; address?: string; vatNumber?: string }>(input: T): T {
  return {
    ...input,
    name: input.name.trim(),
    phone: input.phone?.trim() || undefined,
    address: input.address?.trim() || undefined,
    vatNumber: input.vatNumber?.trim() || undefined,
  };
}

// --- Customers -------------------------------------------------------------------------------

export async function getCustomers(filter: PartyFilter = {}): Promise<Customer[]> {
  await delay();
  return db.customers
    .map((c) => ({ ...clone(c), balance: customerBalance(c.id) }))
    .filter(
      (c) =>
        (filter.includeInactive || c.active) &&
        (!filter.withBalanceOnly || c.balance > 0) &&
        includesText([c.name, c.phone, c.vatNumber], filter.search),
    );
}

export async function getCustomer(id: string): Promise<Customer> {
  await delay();
  const c = db.customers.find((x) => x.id === id);
  if (!c) throw new ApiError('العميل غير موجود', 'NOT_FOUND');
  return { ...clone(c), balance: customerBalance(id) };
}

export async function saveCustomer(input: CustomerInput, id?: string): Promise<Customer> {
  await delay();
  validateCommon(input);
  const data = clean(input);
  let customer: Customer;
  if (id) {
    const found = db.customers.find((c) => c.id === id);
    if (!found) throw new ApiError('العميل غير موجود', 'NOT_FOUND');
    if (!data.active && customerBalance(id) > 0) throw new ApiError('لا يمكن إيقاف عميل عليه رصيد مستحق');
    Object.assign(found, data);
    customer = found;
  } else {
    customer = { id: uid('cus'), ...data, balance: 0 };
    db.customers.push(customer);
  }
  logActivity('party', `${id ? 'تعديل' : 'إضافة'} العميل ${customer.name}`, session.userId, new Date().toISOString(), `/customers/${customer.id}`);
  return { ...clone(customer), balance: customerBalance(customer.id) };
}

export async function getCustomerStatement(id: string): Promise<PartyStatementRow[]> {
  await delay();
  return customerStatement(id);
}

// --- Suppliers -------------------------------------------------------------------------------

export async function getSuppliers(filter: PartyFilter = {}): Promise<Supplier[]> {
  await delay();
  return db.suppliers
    .map((s) => ({ ...clone(s), balance: supplierBalance(s.id) }))
    .filter(
      (s) =>
        (filter.includeInactive || s.active) &&
        (!filter.withBalanceOnly || s.balance > 0) &&
        includesText([s.name, s.phone, s.contactPerson, s.vatNumber], filter.search),
    );
}

export async function getSupplier(id: string): Promise<Supplier> {
  await delay();
  const s = db.suppliers.find((x) => x.id === id);
  if (!s) throw new ApiError('المورد غير موجود', 'NOT_FOUND');
  return { ...clone(s), balance: supplierBalance(id) };
}

export async function saveSupplier(input: SupplierInput, id?: string): Promise<Supplier> {
  await delay();
  validateCommon(input);
  const data = { ...clean(input), contactPerson: input.contactPerson?.trim() || undefined };
  let supplier: Supplier;
  if (id) {
    const found = db.suppliers.find((s) => s.id === id);
    if (!found) throw new ApiError('المورد غير موجود', 'NOT_FOUND');
    Object.assign(found, data);
    supplier = found;
  } else {
    supplier = { id: uid('sup'), ...data, balance: 0 };
    db.suppliers.push(supplier);
  }
  logActivity('party', `${id ? 'تعديل' : 'إضافة'} المورد ${supplier.name}`, session.userId, new Date().toISOString(), `/suppliers/${supplier.id}`);
  return { ...clone(supplier), balance: supplierBalance(supplier.id) };
}

export async function getSupplierStatement(id: string): Promise<PartyStatementRow[]> {
  await delay();
  return supplierStatement(id);
}
