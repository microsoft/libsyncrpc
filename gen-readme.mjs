import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from 'node:url';

import dstmd from "@suchipi/dtsmd";

// Earlier versions of node@20 don't have `import.meta.dirname`.
const __dirname = import.meta.dirname || dirname(fileURLToPath(import.meta.url));

let dts;
try {
    dts = readFileSync(join(__dirname, "index.d.ts"), "utf8");
} catch {
    console.error("No .d.ts");
    process.exit(0);
}

const dtsDocs = (await dstmd.processSource(dts, { headingOffset: 2 })).markdown;

const tpl = readFileSync(join(__dirname, "README_tpl.md"), "utf8");

const final = tpl.replace("__DTSMD_PLACEHOLDER__", dtsDocs);

writeFileSync(join(__dirname, "README.md"), final);
