import type { RouteLocationAsRelative } from 'vue-router';
import type { RouteNamedMap } from '@/router/route-map.gen';

export type RouteName = keyof RouteNamedMap;

/** A navigation target: a named route object. `name` is required; params are checked per route. */
export type AppRoute<N extends RouteName = RouteName> = { [K in N]: RouteLocationAsRelative<K> & { name: K } }[N];
