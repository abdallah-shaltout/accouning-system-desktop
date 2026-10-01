// vitest.config.ts sets `test.globals: true` (describe/it/expect/vi available without importing
// them), but the root tsconfig's `typeRoots` is restricted to `node_modules/@types` and this
// directory, so plain `tsc --noEmit` (bun run lint) never picks up vitest's own `vitest/globals`
// ambient types. This re-exports them so test files under `src/**/__tests__` type-check.
/// <reference types="vitest/globals" />
