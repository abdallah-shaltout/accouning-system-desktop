/**
 * A navigation target: a named route object, never a path string (CLAUDE.md "Navigation").
 * Kept intentionally loose (`Record<string, unknown>` for params/query) since this project has no
 * generated route-name map yet — tighten this once the real route set exists across D2/D3.
 */
export interface AppRoute {
  name: string;
  params?: Record<string, string | number>;
  query?: Record<string, string | number | (string | number)[] | null | undefined>;
  hash?: string;
}
