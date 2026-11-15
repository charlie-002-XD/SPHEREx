import js from "@eslint/js";
import tseslint from "typescript-eslint";

export default tseslint.config(
  // Generated folders: build output and Playwright artifacts/reports are
  // never linted.
  {
    ignores: [
      "node_modules",
      "dist/**",
      "test-results/**",
      "playwright-report/**",
    ],
  },
  js.configs.recommended,
  ...tseslint.configs.recommended,
);
