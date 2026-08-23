import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    exclude: ["tests/**/*.live.test.ts", "node_modules/**", "dist/**"],
    coverage: {
      provider: "v8",
      include: ["src/**/*.ts"],
      exclude: ["src/index.ts"],
      reporter: ["text", "html", "lcov"],
      reportsDirectory: "coverage",
    },
  },
});
