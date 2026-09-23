import { db } from '../db';
import { paymentMethodsFixture, settingsFixture, taxesFixture } from '../fixtures/settings';
import { clone } from '../utils';

/** Settings area: store settings, taxes and payment methods. */
export function seedSettings(): void {
  db.taxes = clone(taxesFixture);
  db.paymentMethods = clone(paymentMethodsFixture);
  db.settings = clone(settingsFixture);
}
