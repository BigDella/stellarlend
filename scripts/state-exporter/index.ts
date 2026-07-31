#!/usr/bin/env node
/**
 * state-exporter CLI (#499).
 *
 * Exports structured contract state to JSON/CSV with checksum + row-count
 * verification, optional gzip compression, and incremental (CDC) exports.
 *
 * Run (no install needed on Node >= 22):
 *   node --experimental-strip-types scripts/state-exporter/index.ts \
 *     --in state.json --format json --out export.json [--gzip] [--since prev.json]
 *   node --experimental-strip-types scripts/state-exporter/index.ts \
 *     --verify export.json --manifest export.json.manifest.json
 *
 * The default source is a JSON snapshot file so the tool is runnable offline.
 * A live Soroban RPC source (view functions + events) and additional sinks
 * (Parquet/SQL, S3/GCS, scheduling) are documented follow-ups in the README.
 */

import * as fs from "node:fs";
import { buildExport, verifyIntegrity } from "./exporter.ts";
import { type ContractState, type ExportFormat, type ExportManifest } from "./types.ts";

function arg(name: string): string | undefined {
  const i = process.argv.indexOf(`--${name}`);
  return i >= 0 ? process.argv[i + 1] : undefined;
}
function flag(name: string): boolean {
  return process.argv.includes(`--${name}`);
}

function readState(path: string): ContractState {
  return JSON.parse(fs.readFileSync(path, "utf8")) as ContractState;
}

function main(): void {
  if (flag("verify")) {
    const payloadPath = arg("verify")!;
    const manifestPath = arg("manifest") ?? `${payloadPath}.manifest.json`;
    const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8")) as ExportManifest;
    const payload = manifest.compressed
      ? fs.readFileSync(payloadPath)
      : fs.readFileSync(payloadPath, "utf8");
    const result = verifyIntegrity(payload, manifest);
    console.log(JSON.stringify(result, null, 2));
    process.exit(result.ok ? 0 : 1);
  }

  const inPath = arg("in");
  if (!inPath) {
    console.error("Missing --in <state.json> (or use --verify <export>)");
    process.exit(2);
  }
  const format = (arg("format") ?? "json") as ExportFormat;
  const out = arg("out") ?? `export.${format}`;
  const compressed = flag("gzip");
  const since = arg("since");

  const state = readState(inPath);
  const previous = since ? readState(since) : undefined;
  const built = buildExport(state, { format, compressed, previous });

  const outPath = compressed ? `${out}.gz` : out;
  fs.writeFileSync(outPath, built.payload);
  fs.writeFileSync(`${outPath}.manifest.json`, JSON.stringify(built.manifest, null, 2));

  console.log(`Exported ${built.manifest.rowCount.total} rows -> ${outPath}`);
  console.log(`Manifest -> ${outPath}.manifest.json (checksum ${built.manifest.checksum.slice(0, 12)}…)`);
}

main();
