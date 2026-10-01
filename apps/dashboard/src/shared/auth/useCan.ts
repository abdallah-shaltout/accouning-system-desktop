import { computed, type ComputedRef } from "vue";
import { useAdminSession } from "./useSession";

/**
 * `useCan('owner', 'finance')` — true when the current admin session's role is one of the given
 * roles. Portal has no role gating yet (all portal users share the same permissions today), so this
 * only reads the admin session; widen it if/when portal roles are introduced.
 */
export function useCan(...roles: string[]): ComputedRef<boolean> {
  const session = useAdminSession();
  return computed(() => {
    const role = session.user?.role as string | undefined;
    if (!role) return false;
    return roles.includes(role);
  });
}
