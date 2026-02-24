import fs from "node:fs/promises";
import path from "node:path";

const ROOT = process.cwd();
const src = path.join(ROOT, "resources");
const dst = path.join(ROOT, "dist", "resources");

await fs.mkdir(dst, { recursive: true });
await fs.cp(src, dst, { recursive: true, force: true });
