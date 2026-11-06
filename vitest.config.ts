// SPDX-License-Identifier: AGPL-3.0-only
import { defineConfig } from 'vitest/config';
import { fileURLToPath } from 'node:url';

export default defineConfig({
  resolve: {
    alias: {
      '@kallamo/umwelt': fileURLToPath(
        new URL('./packages/umwelt/src/index.ts', import.meta.url),
      ),
    },
  },
  test: {
    include: ['conformance/**/*.test.ts', 'packages/**/tests/**/*.test.ts'],
  },
});
