import path from "path";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react-swc";
import { defineConfig, mergeConfig } from "vitest/config";
import { defineConfig as defineViteConfig } from "vite";

// Base Vite config (without test section)
const viteConfig = defineViteConfig({
  plugins: [react(), tailwindcss()],
  base: "/fractalwonder/",
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
});

// Merge in Vitest-specific config
export default mergeConfig(
  viteConfig,
  defineConfig({
    test: {
      globals: true,
      environment: "jsdom",
      setupFiles: "./src/setupTests.ts",
      coverage: {
        provider: "v8",
        reporter: ["text", "json", "html"],
      },
    },
  })
);
