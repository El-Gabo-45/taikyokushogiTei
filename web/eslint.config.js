/**
 * ESLint flat config for the Taikyoku Shogi web GUI.
 *
 * Scope: `src/**` only — `dist/` holds the committed build output and
 * `tests/` use vitest globals, so neither is linted (the `lint` script
 * also targets `src` explicitly).
 */

import js from '@eslint/js';
import tseslint from 'typescript-eslint';

export default tseslint.config(
  { ignores: ['dist/**', 'node_modules/**'] },
  js.configs.recommended,
  tseslint.configs.recommended,
  {
    files: ['src/**/*.ts'],
    rules: {
      // The codebase is fully typed; `any` would defeat the strict TS flags.
      '@typescript-eslint/no-explicit-any': 'error',
    },
  },
);
