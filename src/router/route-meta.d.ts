import 'vue-router';
import type { Access, Area } from '@/modules/users/types';

declare module 'vue-router' {
  interface RouteMeta {
    /** Page title (topbar breadcrumb + window title). */
    title?: string;
    /** Parent section shown before the title in the breadcrumb. */
    section?: string;
    /** Permission area gating this route; omitted = any signed-in user. */
    area?: Area;
    /** Minimum access level on `area` (default 'read'). */
    access?: Exclude<Access, 'none'>;
    /** 'blank' = chrome-free full-screen page (POS, print preview, login). */
    layout?: 'default' | 'blank';
    /** Reachable without signing in. */
    public?: boolean;
  }
}
