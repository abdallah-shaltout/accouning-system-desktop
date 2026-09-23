/**
 * Party (customers/suppliers) invariants — docs/v2/02-accounting-review.md §4 items 3, 6.
 */
import { db } from '../../src/mocks/db';
import { customerBalance, customerStatement, supplierBalance, supplierStatement } from '../../src/mocks/backend/balances';
import { check, closeEnough, glBalance, ok, round2, type Result } from './shared';

export function run(): Result[] {
  const results: Result[] = [];

  // 3. GL(receivable) = Σ customer sub-ledgers; GL(payable) = Σ supplier sub-ledgers. Holds
  // structurally now (C2): every AR/AP posting tags partyKind/partyId, and balances are computed
  // from those same ledger lines (src/mocks/backend/balances.ts), not from summing documents.
  // (Per-currency split is Phase 9 territory — single base currency for now.)
  const arGl = glBalance('receivable');
  const arSum = round2(db.customers.reduce((a, c) => a + customerBalance(c.id), 0));
  results.push(check(closeEnough(arGl, arSum), `AR GL (${arGl}) matches Σ customer balances (${arSum})`));

  const apGl = -glBalance('payable');
  const apSum = round2(db.suppliers.reduce((a, s) => a + supplierBalance(s.id), 0));
  results.push(check(closeEnough(apGl, apSum), `AP GL (${apGl}) matches Σ supplier balances (${apSum})`));

  // 6. For every party: Σ document outstanding − unallocated credit ± opening balance = sub-ledger
  // balance. Unallocated credit / opening balance aren't modeled until Phase 4/5 — verified here as
  // "statement's last running balance equals the party's balance function" (what already applies).
  const custMismatch = db.customers.filter((c) => {
    const st = customerStatement(c.id);
    const last = st.at(-1)?.balance ?? 0;
    return !closeEnough(last, customerBalance(c.id));
  });
  results.push(check(custMismatch.length === 0, `every customer statement's running balance matches customerBalance() (${custMismatch.length} mismatched)`));

  const supMismatch = db.suppliers.filter((s) => {
    const st = supplierStatement(s.id);
    const last = st.at(-1)?.balance ?? 0;
    return !closeEnough(last, supplierBalance(s.id));
  });
  results.push(check(supMismatch.length === 0, `every supplier statement's running balance matches supplierBalance() (${supMismatch.length} mismatched)`));

  results.push(ok(`${db.customers.length} customers, ${db.suppliers.length} suppliers, ${db.payments.length} payments`));

  return results;
}
