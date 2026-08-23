import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    include: ["tests/live-convert-block.live.test.ts"],
    exclude: ["node_modules/**", "dist/**"],
  },
});
