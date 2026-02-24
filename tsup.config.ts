import { defineConfig } from "tsup";

export default defineConfig({
  entry: {
    cli: "src/cli.ts",
    "tray/main": "src/tray/main.ts",
    "tray/preload": "src/tray/preload.ts",
  },
  external: ["electron"],
  format: ["esm"],
  target: "node20",
  outDir: "dist",
  clean: true,
  sourcemap: true,
  splitting: false,
  dts: false,
  banner: {
    js: "#!/usr/bin/env node",
  },
});
