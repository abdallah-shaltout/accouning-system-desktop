/**
 * Drift check (21.02-F, F-4 / 03-domains/03-users.md §2): proves the ts-rs-generated `users` DTOs
 * (`users/types/gen/*`, written by `bun run bindings` from `src-tauri/src/domains/users/dto.rs` and
 * `src-tauri/src/core/auth.rs`'s `Role`/`Area`/`Access`) have exactly the same shape as the
 * hand-written TS types in `./index.ts`.
 *
 * Never imported by app code — this file exists only to be type-checked by `bun run build`.
 */
import type { Equals, Expect } from '@/modules/core/types/contract';
import type { Access, Area, Role, User, UserInput } from './index';

import type { User as GenUser } from './gen/User';
import type { UserInput as GenUserInput } from './gen/UserInput';
import type { Role as GenRole } from './gen/Role';
import type { Area as GenArea } from './gen/Area';
import type { Access as GenAccess } from './gen/Access';

export type _User = Expect<Equals<GenUser, User>>;
export type _UserInput = Expect<Equals<GenUserInput, UserInput>>;
export type _Role = Expect<Equals<GenRole, Role>>;
export type _Area = Expect<Equals<GenArea, Area>>;
export type _Access = Expect<Equals<GenAccess, Access>>;

// `restoreSession` returns `User | null` in TS; ts-rs's `Option<User>` also generates `User | null`
// — already equal via `_User` above, no separate conversion or check needed (§2).
