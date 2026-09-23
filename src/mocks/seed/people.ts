import { db } from '../db';
import { credentialsFixture, customersFixture, partyGroupsFixture, suppliersFixture, usersFixture } from '../fixtures/people';
import { clone } from '../utils';

/** People area: users, login credentials, customers and suppliers. */
export function seedPeople(): void {
  db.users = clone(usersFixture);
  db.credentials = { ...credentialsFixture };
  db.customers = clone(customersFixture);
  db.suppliers = clone(suppliersFixture);
  db.partyGroups = clone(partyGroupsFixture);
  db.partyHistory = [];
}
