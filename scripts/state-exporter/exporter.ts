/**
 * Core, transport-free export logic for the contract state-exporter (#499).
 *
 * Pure functions only — serialization, checksums, row counts, change-data-capture
 * (incremental) diffing, gzip (de)compression, and integrity verification — so
 * the behaviour is fully unit-testable without a network or filesystem.
 */

import { createHash } from "node:crypto";
import { gzipSync, gunzipSync } from "node:zlib";
import {
  type ContractState,
  type ExportFormat,
  type ExportManifest,
  type StateSection,
  SECTION_KEY,
  STATE_SECTIONS,
} from "./types.ts";

const EMPTY_STATE: ContractState = { positions: [], pools: [], governance: [] };

/** Deterministic SHA-256 hex digest of a payload. */
export function checksum(payload: string): string {
  return createHash("sha256").update(payload).digest("hex");
}

/** Row counts per section plus a total. */
export function rowCount(state: ContractState): ExportManifest["rowCount"] {
  const counts = {
    positions: state.positions.length,
    pools: state.pools.length,
    governance: state.governance.length,
    total: 0,
  };
  counts.total = counts.positions + counts.pools + counts.governance;
  return counts;
}

/** Serialize full state to canonical (stable-key-order) JSON. */
export function toJSON(state: ContractState): string {
  return JSON.stringify(
    {
      positions: state.positions,
      pools: state.pools,
      governance: state.governance,
    },
    null,
    2,
  );
}

function csvEscape(value: unknown): string {
  const s = String(value ?? "");
  return /[",\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
}

/** Serialize a single section to CSV (header + rows). */
export function sectionToCSV(state: ContractState, section: StateSection): string {
  const rows = state[section] as Array<Record<string, unknown>>;
  if (rows.length === 0) return "";
  const headers = Object.keys(rows[0]);
  const lines = [headers.join(",")];
  for (const row of rows) {
    lines.push(headers.map((h) => csvEscape(row[h])).join(","));
  }
  return lines.join("\n") + "\n";
}

/** Serialize all sections to CSV, concatenated with section markers. */
export function toCSV(state: ContractState): string {
  return STATE_SECTIONS.map((s) => `# section: ${s}\n${sectionToCSV(state, s)}`).join("\n");
}

export function serialize(state: ContractState, format: ExportFormat): string {
  return format === "csv" ? toCSV(state) : toJSON(state);
}

/**
 * Change-data-capture: return only rows that are new or changed relative to
 * `previous`, keyed by each section's primary key. Removed rows are intentionally
 * omitted (snapshot exports capture deletions; incremental exports capture
 * upserts) — the manifest's row counts make the delta size explicit.
 */
export function diff(previous: ContractState, next: ContractState): ContractState {
  const result: ContractState = { positions: [], pools: [], governance: [] };
  for (const section of STATE_SECTIONS) {
    const key = SECTION_KEY[section];
    const prevByKey = new Map<string, string>();
    for (const row of previous[section] as Array<Record<string, unknown>>) {
      prevByKey.set(String(row[key]), JSON.stringify(row));
    }
    const changed = (next[section] as Array<Record<string, unknown>>).filter((row) => {
      const prev = prevByKey.get(String(row[key]));
      return prev === undefined || prev !== JSON.stringify(row);
    });
    (result[section] as unknown[]) = changed;
  }
  return result;
}

export function compress(payload: string): Buffer {
  return gzipSync(Buffer.from(payload, "utf8"));
}

export function decompress(buf: Buffer): string {
  return gunzipSync(buf).toString("utf8");
}

export interface BuildExportOptions {
  format: ExportFormat;
  compressed?: boolean;
  /** Previous full state; when provided the export is an incremental (CDC) delta. */
  previous?: ContractState;
}

export interface BuiltExport {
  manifest: ExportManifest;
  /** UTF-8 payload (or gzip buffer when compressed). */
  payload: string | Buffer;
  /** The (possibly diffed) state that was exported. */
  state: ContractState;
}

/** Build a complete, verifiable export (payload + manifest). */
export function buildExport(state: ContractState, options: BuildExportOptions): BuiltExport {
  const incremental = options.previous !== undefined;
  const effective = incremental ? diff(options.previous ?? EMPTY_STATE, state) : state;
  const text = serialize(effective, options.format);
  const manifest: ExportManifest = {
    generatedAt: new Date().toISOString(),
    format: options.format,
    compressed: Boolean(options.compressed),
    incremental,
    rowCount: rowCount(effective),
    checksum: checksum(text),
  };
  return {
    manifest,
    payload: options.compressed ? compress(text) : text,
    state: effective,
  };
}

export interface IntegrityResult {
  ok: boolean;
  expectedChecksum: string;
  actualChecksum: string;
  expectedRows: number;
  actualRows: number;
}

/** Verify a payload against its manifest (checksum + row count). */
export function verifyIntegrity(
  payload: string | Buffer,
  manifest: ExportManifest,
): IntegrityResult {
  const text = manifest.compressed
    ? decompress(Buffer.isBuffer(payload) ? payload : Buffer.from(payload))
    : String(payload);
  const actualChecksum = checksum(text);
  const actualRows = countRowsInPayload(text, manifest.format);
  return {
    ok: actualChecksum === manifest.checksum && actualRows === manifest.rowCount.total,
    expectedChecksum: manifest.checksum,
    actualChecksum,
    expectedRows: manifest.rowCount.total,
    actualRows,
  };
}

function countRowsInPayload(text: string, format: ExportFormat): number {
  if (format === "json") {
    const parsed = JSON.parse(text) as ContractState;
    return rowCount(parsed).total;
  }
  // CSV: each "# section:" marker is followed by a header line, then data rows.
  // Count only the data rows.
  let count = 0;
  let expectHeader = false;
  for (const raw of text.split("\n")) {
    const line = raw.trim();
    if (line === "") continue;
    if (line.startsWith("# section:")) {
      expectHeader = true;
      continue;
    }
    if (expectHeader) {
      expectHeader = false; // this is the header line for the section
      continue;
    }
    count += 1;
  }
  return count;
}
