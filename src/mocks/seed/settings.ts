import { db } from '../db';
import { settingsFixture, taxesFixture } from '../fixtures/settings';
import { clone } from '../utils';

/** Settings area: store settings and taxes. */
export function seedSettings(): void {
  db.taxes = clone(taxesFixture);
  db.settings = clone(settingsFixture);
}
