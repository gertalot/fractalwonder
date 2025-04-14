import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react-swc";
import path from "path";
import { defineConfig as defineViteConfig } from "vite";
import { defineConfig, mergeConfig } from "vitest/config";

// Export a function so Vite can inject the mode
export default defineViteConfig(({ mode }) => {
  const baseViteConfig = {
    plugins: [react(), tailwindcss()],
    base: "/fractalwonder/",
    resolve: {
      alias: {
        // eslint-disable-next-line no-undef
        "@": path.resolve(__dirname, "./src"),
      },
    },
    // Remove console.log in production by treating it as
    // a pure function that will be tree-shaken out
    esbuild: {
      pure: mode === "production" ? ["console.log"] : [],
    },
  };

  const vitestConfig = defineConfig({
    test: {
      globals: true,
      environment: "jsdom",
      setupFiles: "./src/setupTests.ts",
      coverage: {
        provider: "v8",
        reporter: ["text", "json", "html"],
      },
    },
  });

  return mergeConfig(baseViteConfig, vitestConfig);
});
