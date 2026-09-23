import type { z } from 'zod';

/**
 * Validate form state with a Zod schema and return `{ field: firstMessage }` (empty = valid).
 * Nested paths are joined with dots, e.g. `lines.2.qty`.
 */
export function validate<S extends z.ZodType>(schema: S, data: unknown): Record<string, string> {
  const result = schema.safeParse(data);
  if (result.success) return {};
  const errors: Record<string, string> = {};
  for (const issue of result.error.issues) {
    const key = issue.path.map(String).join('.');
    if (!errors[key]) errors[key] = issue.message;
  }
  return errors;
}
