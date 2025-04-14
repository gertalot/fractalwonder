import js from "@eslint/js";
import pluginReact from "eslint-plugin-react";
import pluginReactHooks from "eslint-plugin-react-hooks";
import { defineConfig } from "eslint/config";
import globals from "globals";
import tseslint from "typescript-eslint";

const __dirname = import.meta.dirname;

export default defineConfig([
  { files: ["**/*.{js,mjs,cjs,ts,jsx,tsx}"], plugins: { js }, extends: ["js/recommended"] },
  { files: ["**/*.{js,mjs,cjs,ts,jsx,tsx}"], languageOptions: { globals: globals.browser } },
  // Apply TypeScript specific rules
  {
    files: ["**/*.{ts,tsx}"], // Target only TypeScript/TSX files
    plugins: {
      "@typescript-eslint": tseslint.plugin, // Make sure the plugin is explicitly available
    },
    languageOptions: {
      parser: tseslint.parser, // Use the TypeScript parser
      parserOptions: {
        project: ["./tsconfig.json", "./tsconfig.app.json", "./tsconfig.node.json"], // Or specify your tsconfig path if needed
        tsconfigRootDir: __dirname,
      },
    },
    rules: {
      // Use the recommended rules from typescript-eslint
      ...tseslint.configs.recommended.rules,
      // Override the no-unused-vars rule specifically for TS/TSX
      "@typescript-eslint/no-unused-vars": [
        "warn", // Keep it as a warning
        {
          argsIgnorePattern: "^_", // Ignore arguments starting with _
          varsIgnorePattern: "^_", // Ignore variables starting with _
          caughtErrorsIgnorePattern: "^_", // Ignore caught error variables starting with _
        },
      ],
      // Disable the base ESLint rule, as @typescript-eslint/no-unused-vars handles it better for TS
      "no-unused-vars": "off",
    },
  },
  // Apply React specific rules
  {
    files: ["**/*.{js,jsx,ts,tsx}"], // Target files where React might be used
    plugins: {
      "react": pluginReact,
      "react-hooks": pluginReactHooks,
    },
    settings: {
      react: {
        version: "detect", // Automatically detect React version
      },
    },
    rules: {
      ...pluginReact.configs.flat.recommended.rules,
      ...pluginReactHooks.configs["recommended-latest"].rules,
      "react/react-in-jsx-scope": "off", // Already off, keeping it
    },
  },
  // Global overrides or rules for all files (JS, TS, etc.)
  {
    files: ["**/*.{js,mjs,cjs,ts,jsx,tsx}"],
    rules: {
      // Keep other general rules here if needed, like no-undef
      "no-undef": "warn",
      // Add any other project-wide rules here
    },
  },
]);
