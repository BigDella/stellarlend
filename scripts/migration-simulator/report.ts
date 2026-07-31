#!/usr/bin/env ts-node
/**
 * report.ts — Generate a human-readable migration plan report with gas and time estimates.
 *
 * Reads a simulation result JSON (from simulate.sh) and a snapshot JSON (from snapshot.sh),
 * then writes a migration-plan-<label>.json report to the reports directory.
 *
 * Usage:
 *   ts-node scripts/migration-simulator/report.ts \
 *     --result .migration-sandbox/results/simulate-<label>.json \
 *     --snapshot .migration-sandbox/snapshots/snapshot-<label>.json
 */

import * as fs from "fs";
import * as path from "path";
import * as crypto from "crypto";

// ── Config ────────────────────────────────────────────────────────────────────
const CONFIG_PATH = path.resolve(__dirname, "../../environments/migration-sandbox/sandbox.config.json");
const GAS_BASELINE_PATH = path.resolve(__dirname, "../../stellar-lend/benchmarks/gas-baseline.json");

interface SandboxConfig {
  reportsDir: string;
  gasEstimation: {
    networkThroughputTxPerLedger: number;
    ledgerCloseTimeSec: number;
    cpuInsnsPerXlm: number;
  };
}

interface GasBaseline {
  benchmarks: Array<{ operation: string; cpu_insns: number; mem_bytes: number }>;
}

interface SimulationResult {
  label: string;
  timestamp: string;
  snapshotFile: string;
  contractId: string;
  migrationOk: boolean;
  migrationError: string;
  migrationDurationMs: number;
  totalCpuInsns: number;
  stepCpuInsns: Record<string, number>;
  invariantsPassed: boolean;
  invariantErrors: string[];
  rollbackOk: boolean;
  rollbackError: string;
}

interface Snapshot {
  label: string;
  timestamp: string;
  sourceNetwork: string;
  lendingContractId: string;
  state: Record<string, unknown>;
}

// ── CLI args ──────────────────────────────────────────────────────────────────
function parseArgs(): { resultFile: string; snapshotFile: string; outDir?: string } {
  const args = process.argv.slice(2);
  let resultFile = "";
  let snapshotFile = "";
  let outDir: string | undefined;

  for (let i = 0; i < args.length; i++) {
    if (args[i] === "--result")   resultFile  = args[++i];
    if (args[i] === "--snapshot") snapshotFile = args[++i];
    if (args[i] === "--out-dir")  outDir       = args[++i];
  }

  if (!resultFile)   { console.error("ERROR: --result is required"); process.exit(1); }
  if (!snapshotFile) { console.error("ERROR: --snapshot is required"); process.exit(1); }
  return { resultFile, snapshotFile, outDir };
}

// ── Gas / time estimation ─────────────────────────────────────────────────────
function estimateXlmCost(cpuInsns: number, cpuInsnsPerXlm: number): number {
  return cpuInsns / cpuInsnsPerXlm;
}

function estimateLedgers(txCount: number, throughput: number): number {
  return Math.ceil(txCount / throughput);
}

function estimateTimeSec(ledgers: number, ledgerCloseSec: number): number {
  return ledgers * ledgerCloseSec;
}

// ── Main ──────────────────────────────────────────────────────────────────────
function main(): void {
  const { resultFile, snapshotFile, outDir } = parseArgs();

  const config: SandboxConfig = JSON.parse(fs.readFileSync(CONFIG_PATH, "utf8"));
  const result: SimulationResult = JSON.parse(fs.readFileSync(resultFile, "utf8"));
  const snapshot: Snapshot = JSON.parse(fs.readFileSync(snapshotFile, "utf8"));

  const gasBaseline: GasBaseline = fs.existsSync(GAS_BASELINE_PATH)
    ? JSON.parse(fs.readFileSync(GAS_BASELINE_PATH, "utf8"))
    : { benchmarks: [] };

  const { networkThroughputTxPerLedger, ledgerCloseTimeSec, cpuInsnsPerXlm } = config.gasEstimation;

  // Build per-step gas estimates
  const steps = Object.entries(result.stepCpuInsns).map(([name, cpuInsns]) => {
    const baseline = gasBaseline.benchmarks.find((b) => b.operation === name);
    return {
      operation: name,
      cpuInsns,
      baselineCpuInsns: baseline?.cpu_insns ?? null,
      estimatedXlm: +estimateXlmCost(cpuInsns, cpuInsnsPerXlm).toFixed(6),
    };
  });

  const txCount = steps.length;
  const ledgers = estimateLedgers(txCount, networkThroughputTxPerLedger);
  const estimatedTimeSec = estimateTimeSec(ledgers, ledgerCloseTimeSec);
  const totalEstimatedXlm = +estimateXlmCost(result.totalCpuInsns, cpuInsnsPerXlm).toFixed(6);

  // Report content hash (used by approve.sh to bind the approval to this exact report)
  const reportBody = {
    schemaVersion: "1.0.0",
    generatedAt: new Date().toISOString(),
    label: result.label,
    sourceNetwork: snapshot.sourceNetwork,
    contractId: result.contractId,
    simulation: {
      ok: result.migrationOk,
      error: result.migrationError || null,
      durationMs: result.migrationDurationMs,
      invariantsPassed: result.invariantsPassed,
      invariantErrors: result.invariantErrors,
      rollbackOk: result.rollbackOk,
      rollbackError: result.rollbackError || null,
    },
    gasEstimate: {
      totalCpuInsns: result.totalCpuInsns,
      totalEstimatedXlm,
      steps,
    },
    timeEstimate: {
      txCount,
      ledgers,
      estimatedTimeSec,
      estimatedTimeHuman: `${Math.floor(estimatedTimeSec / 60)}m ${estimatedTimeSec % 60}s`,
    },
    preFlightChecks: {
      simulationPassed: result.migrationOk,
      invariantsPassed: result.invariantsPassed,
      rollbackValidated: result.rollbackOk,
    },
    snapshotRef: {
      file: path.basename(snapshotFile),
      label: snapshot.label,
      timestamp: snapshot.timestamp,
    },
  };

  const bodyJson = JSON.stringify(reportBody);
  const contentHash = crypto.createHash("sha256").update(bodyJson).digest("hex");

  const report = { ...reportBody, contentHash };

  const reportsDir = outDir ?? path.resolve(__dirname, "../../", config.reportsDir);
  fs.mkdirSync(reportsDir, { recursive: true });

  const reportFile = path.join(reportsDir, `migration-plan-${result.label}.json`);
  fs.writeFileSync(reportFile, JSON.stringify(report, null, 2));

  console.log(`Migration plan report written to: ${reportFile}`);
  console.log(`Content hash: ${contentHash}`);
  console.log(`MIGRATION_REPORT_FILE=${reportFile}`);
  console.log(`MIGRATION_REPORT_HASH=${contentHash}`);

  if (!report.preFlightChecks.simulationPassed || !report.preFlightChecks.invariantsPassed) {
    console.error("ERROR: Pre-flight checks failed. Review the report before proceeding.");
    process.exit(1);
  }
}

main();
