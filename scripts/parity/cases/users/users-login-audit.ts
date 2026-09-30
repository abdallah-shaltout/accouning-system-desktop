/**
 * L1 platform lane. Login writes an activity row and the audit path (03-users.md §3 `login`):
 * success, wrong password, unknown user, inactive user. Logging out and back in as a different
 * seeded user proves the session is replaced, not merged.
 */
import { defineCase } from '../../case';
import * as authService from '../../../../src/modules/users/services/authService';

export default defineCase({
  name: 'users/users-login-audit',
  source: '03-domains/03-users.md §8(b)',
  base: 'demo-sa',
  // This case drives its own logins from a signed-out start.
  user: null,
  // 03-users.md D-1 (a written security decision): Rust's `users_restore_session` only re-attaches
  // this process's *existing* session — a client-held id alone never authenticates. The mock's
  // `restoreSession(id)` signs in any active user by id. So restoring another user's id while signed
  // in as someone else, or with no session at all, is `User` on the mock and `null` on Rust.
  allow: [
    { path: 'steps.restore-session-wrong-id.value', reason: '03-users D-1: restore only re-attaches the same process session; another user id returns null on Rust' },
    { path: 'steps.restore-session-none.value', reason: '03-users D-1: restore with no live session returns null on Rust (a client-held id never authenticates)' },
  ],
  async run(s) {
    await s.login('admin');
    await s.step('restore-session-self', () => authService.restoreSession(s.baseId('usr-1')));
    await s.logout();

    await s.expectError('unknown-user', () => authService.login('nobody', 'whatever'));
    await s.expectError('wrong-password', () => authService.login('admin', 'wrong-password'));
    // `fahad` (usr-6) is seeded inactive.
    await s.expectError('inactive-user', () => authService.login('fahad', 'fahad123'));

    await s.login('manager');
    await s.step('restore-session-wrong-id', () => authService.restoreSession(s.baseId('usr-1')));
    // Through the harness so it knows the session ended (no signed-in books read afterwards).
    await s.logout();
    await s.step('restore-session-none', () => authService.restoreSession(s.baseId('usr-1')));
  },
});
