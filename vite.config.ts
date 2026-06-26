import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  build: {
    rolldownOptions: {
      output: {
        codeSplitting: true,
        manualChunks(id) {
          if (!id.includes("node_modules")) return undefined;
          if (id.includes("recharts")) return "charts";
          if (id.includes("@sentry") || id.includes("@tauri-apps")) return "platform";
          if (id.includes("react") || id.includes("scheduler")) return "react-vendor";
          return "vendor";
        }
      }
    }
  },
  test: {
    environment: "jsdom",
    setupFiles: ["src/test/setup.ts"],
    coverage: {
      provider: "v8",
      include: ["src/components/**/*.tsx", "src/lib/**/*.ts"],
      exclude: [
        "src/lib/types.ts",
        "src/**/*.test.{ts,tsx}"
      ],
      reporter: ["text", "json-summary", "html"],
      thresholds: {
        lines: 90,
        statements: 90,
        functions: 90,
        branches: 85
      }
    }
  },
  server: {
    port: 1420,
    strictPort: true
  }
});
