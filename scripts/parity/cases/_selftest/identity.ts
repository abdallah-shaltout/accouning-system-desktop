/**
 * Harness self-test (plan 21 Part 04, B-13): the smallest round trip — import the demo base, log in,
 * read two lists. Green on `--mock-only` proves the runner; green against Rust proves the host,
 * the importer's id pairs and the login path.
 */
import { defineCase } from '../../case';
import * as userService from '../../../../src/modules/users/services/userService';
import * as settingsService from '../../../../src/modules/settings/services/settingsService';

export default defineCase({
  name: '_selftest/identity',
  source: 'plans/pending/21-rust-backend/04-integration-and-audit/phase-b-parity-harness.md B-13',
  base: 'demo-sa',
  user: 'admin',
  async run(s) {
    await s.step('users', () => userService.getUsers());
    await s.step('settings', () => settingsService.getSettings());
  },
});
