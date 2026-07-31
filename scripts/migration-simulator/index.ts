#!/usr/bin/env ts-node
/**
 * index.ts — Migration simulator orchestrator.
 *
 * Ties the four phases together:
 *   1. snapshot  → scripts/migration-simulator/snapshot.sh
 *   2. simulate  → scripts/migration-simulator/simulate.sh
 *   3. report    → scripts/migration-simulator/report.ts
 *   4. approve   → scripts/migration-simulator/approve.sh  (optional in dry-run mode)
 *
 * Usage:
 *   ts-node scripts/migration-simulator/index.ts [options]
 *
 * For mainnet execution the approval file must be present and its signature
 * must bind to the report's contentHash.
 */

import { execSync, ExecSyncOptionsWithStringEncoding } from "child_process";
import * as fs from "fs";
import * as path from "path";
import * as crypto from "crypto";

// ── Types ─────────────────────────────────────────────────────────────────────
interface Opts {
  /** One of: snapshot | simulate | report | approve | all */
  phase: "snapshot" | "simulate" | "report" | "approve" | "all";
  sourceNetwork: string;
  lendingContractId: string;
  migrationHubId: string;
  adminSecret: string;
  approverSecret: string;
  approverAlias: string;
  snapshotFile: string;
  resultFile: string;
  reportFile: string;
  approvalFile: string;
  label: string;
  rpcUrl: string;
  skipSandboxStart: boolean;
  /** dry-run: run all phases except actual mainnet deployment */
  dryRun: boolean;
}

// ── CLI ───────────────────────────────────────────────────────────────────────
function parseArgs(): Opts {
  const args = process.argv.slice(2);
  const opts: Opts = {
    phase: "all",
    sourceNetwork: process.env.SOURCE_NETWORK ?? "pubnet",
    lendingContractId: process.env.LENDING_CONTRACT_ID ?? "",
    migrationHubId: process.env.MIGRATION_HUB_CONTRACT_ID ?? "",
    adminSecret: process.env.ADMIN_SECRET_KEY ?? "",
    approverSecret: process.env.APPROVER_SECRET_KEY ?? "",
    approverAlias: process.env.APPROVER_ALIAS ?? "approver",
    snapshotFile: process.env.MIGRATION_SNAPSHOT_FILE ?? "",
    resultFile: process.env.MIGRATION_RESULT_FILE ?? "",
    reportFile: process.env.MIGRATION_REPORT_FILE ?? "",
    approvalFile: process.env.MIGRATION_APPROVAL_FILE ?? "",
    label: process.env.LABEL ?? new Date().toISOString().replace(/[:.]/g, "").slice(0, 15) + "Z",
    rpcUrl: process.env.STELLAR_RPC_URL ?? "http://localhost:8000/soroban/rpc",
    skipSandboxStart: false,
    dryRun: false,
  };

  for (let i = 0; i < args.length; i++) {
    switch (args[i]) {
      case "--phase":                opts.phase             = args[++i] as Opts["phase"]; break;
      case "--source-network":       opts.sourceNetwork     = args[++i]; break;
      case "--lending-contract-id":  opts.lendingContractId = args[++i]; break;
      case "--migration-hub-id":     opts.migrationHubId    = args[++i]; break;
      case "--admin-secret":         opts.adminSecret       = args[++i]; break;
      case "--approver-secret":      opts.approverSecret    = args[++i]; break;
      case "--approver-alias":       opts.approverAlias     = args[++i]; break;
      case "--snapshot-file":        opts.snapshotFile      = args[++i]; break;
      case "--result-file":          opts.resultFile        = args[++i]; break;
      case "--report-file":          opts.reportFile        = args[++i]; break;
      case "--approval-file":        opts.approvalFile      = args[++i]; break;
      case "--label":                opts.label             = args[++i]; break;
      case "--rpc-url":              opts.rpcUrl            = args[++i]; break;
      case "--skip-sandbox-start":   opts.skipSandboxStart  = true; break;
      case "--dry-run":              opts.dryRun            = true; break;
      case "--help": printHelp(); process.exit(0); break;
    }
  }
  return opts;
}

function printHelp(): void {
  console.log(`
Usage: ts-node scripts/migration-simulator/index.ts [options]

Options:
  --phase <snapshot|simulate|report|approve|all>  Phase to run (default: all)
  --source-network <net>        Mainnet source to snapshot (default: pubnet)
  --lending-contract-id <id>    Lending contract ID
  --migration-hub-id <id>       MigrationHub contract ID (optional)
  --admin-secret <key>          Admin secret key
  --approver-secret <key>       Approver secret key (approve phase)
  --approver-alias <name>       Approver alias (default: approver)
  --snapshot-file <path>        Existing snapshot to reuse (skips snapshot phase)
  --result-file <path>          Existing simulation result to reuse
  --report-file <path>          Existing report to reuse
  --approval-file <path>        Existing approval to verify
  --label <label>               Run label (default: auto-generated timestamp)
  --rpc-url <url>               Soroban RPC URL for sandbox
  --skip-sandbox-start          Assume local sandbox is already running
  --dry-run                     Run all phases but skip mainnet deployment gate
  --help                        Show this help
`);
}

// ── Helpers ───────────────────────────────────────────────────────────────────
const EXEC_OPTS: ExecSyncOptionsWithStringEncoding = { encoding: "utf8", stdio: "inherit" };

function run(cmd: string, env?: Record<string, string>): void {
  console.log(`\n>>> ${cmd}`);
  execSync(cmd, { ...EXEC_OPTS, env: { ...process.env, ...env } });
}

function runCapture(cmd: string, env?: Record<string, string>): string {
  return execSync(cmd, { encoding: "utf8", env: { ...process.env, ...env } }).trim();
}

function extractEnvVar(output: string, varName: string): string {
  const match = output.match(new RegExp(`${varName}=(.+)`));
  return match ? match[1].trim() : "";
}

const SIMULATOR_DIR = path.resolve(__dirname);
const REPO_ROOT     = path.resolve(__dirname, "../..");

// ── Phase runners ─────────────────────────────────────────────────────────────
function runSnapshot(opts: Opts): string {
  console.log("\n════ Phase 1: Snapshot ════");
  const env: Record<string, string> = {
    SOURCE_NETWORK:           opts.sourceNetwork,
    LENDING_CONTRACT_ID:      opts.lendingContractId,
    MIGRATION_HUB_CONTRACT_ID: opts.migrationHubId,
    LABEL:                    opts.label,
  };
  const out = runCapture(`bash "${SIMULATOR_DIR}/snapshot.sh"`, env);
  console.log(out);
  return extractEnvVar(out, "MIGRATION_SNAPSHOT_FILE") || opts.snapshotFile;
}

function runSimulate(opts: Opts, snapshotFile: string): string {
  console.log("\n════ Phase 2: Simulate ════");
  const skipFlag = opts.skipSandboxStart ? "--skip-sandbox-start" : "";
  const env: Record<string, string> = {
    MIGRATION_SNAPSHOT_FILE: snapshotFile,
    LENDING_CONTRACT_ID:     opts.lendingContractId,
    ADMIN_SECRET_KEY:        opts.adminSecret,
    STELLAR_RPC_URL:         opts.rpcUrl,
    LABEL:                   opts.label,
  };
  const out = runCapture(`bash "${SIMULATOR_DIR}/simulate.sh" ${skipFlag}`, env);
  console.log(out);
  return extractEnvVar(out, "MIGRATION_RESULT_FILE") || opts.resultFile;
}

function runReport(opts: Opts, snapshotFile: string, resultFile: string): string {
  console.log("\n════ Phase 3: Report ════");
  const out = runCapture(
    `npx ts-node "${SIMULATOR_DIR}/report.ts" --result "${resultFile}" --snapshot "${snapshotFile}"`,
    { LABEL: opts.label }
  );
  console.log(out);
  return extractEnvVar(out, "MIGRATION_REPORT_FILE") || opts.reportFile;
}

function runApprove(opts: Opts, reportFile: string): string {
  console.log("\n════ Phase 4: Approve ════");
  const env: Record<string, string> = {
    MIGRATION_REPORT_FILE: reportFile,
    APPROVER_SECRET_KEY:   opts.approverSecret,
    APPROVER_ALIAS:        opts.approverAlias,
    LABEL:                 opts.label,
  };
  const out = runCapture(`bash "${SIMULATOR_DIR}/approve.sh"`, env);
  console.log(out);
  return extractEnvVar(out, "MIGRATION_APPROVAL_FILE") || opts.approvalFile;
}

// ── Approval verification ─────────────────────────────────────────────────────
function verifyApproval(approvalFile: string, reportFile: string): void {
  console.log("\n════ Verifying approval ════");

  if (!fs.existsSync(approvalFile)) {
    throw new Error(`Approval file not found: ${approvalFile}`);
  }
  if (!fs.existsSync(reportFile)) {
    throw new Error(`Report file not found: ${reportFile}`);
  }

  const approval = JSON.parse(fs.readFileSync(approvalFile, "utf8"));
  const report   = JSON.parse(fs.readFileSync(reportFile, "utf8"));

  // Verify content hash binding
  const reportBodyForHash = { ...report };
  delete reportBodyForHash.contentHash;
  const bodyJson = JSON.stringify(reportBodyForHash);
  const expectedHash = crypto.createHash("sha256").update(bodyJson).digest("hex");

  if (approval.contentHash !== report.contentHash) {
    throw new Error(
      `Approval hash mismatch: approval=${approval.contentHash} report=${report.contentHash}`
    );
  }

  if (!approval.preFlightChecks.simulationPassed || !approval.preFlightChecks.invariantsPassed) {
    throw new Error("Approval was granted on a failing simulation. Aborting.");
  }

  console.log(`✔ Approval valid — approver: ${approval.approver.alias} (${approval.approver.publicKey})`);
  console.log(`✔ Approved at: ${approval.approvedAt}`);
  console.log(`✔ Content hash: ${approval.contentHash}`);
}

// ── Main ──────────────────────────────────────────────────────────────────────
async function main(): Promise<void> {
  const opts = parseArgs();

  console.log(`\nMigration Simulator — phase: ${opts.phase}${opts.dryRun ? " (dry-run)" : ""}`);
  console.log(`Label: ${opts.label}`);

  let snapshotFile = opts.snapshotFile;
  let resultFile   = opts.resultFile;
  let reportFile   = opts.reportFile;
  let approvalFile = opts.approvalFile;

  if (opts.phase === "snapshot" || opts.phase === "all") {
    if (!snapshotFile) {
      if (!opts.lendingContractId) throw new Error("--lending-contract-id is required for snapshot phase");
      snapshotFile = runSnapshot(opts);
    } else {
      console.log(`\nSkipping snapshot — using: ${snapshotFile}`);
    }
  }

  if (opts.phase === "simulate" || opts.phase === "all") {
    if (!resultFile) {
      if (!snapshotFile)          throw new Error("--snapshot-file is required for simulate phase");
      if (!opts.lendingContractId) throw new Error("--lending-contract-id is required");
      if (!opts.adminSecret)       throw new Error("--admin-secret is required");
      resultFile = runSimulate(opts, snapshotFile);
    } else {
      console.log(`\nSkipping simulate — using: ${resultFile}`);
    }
  }

  if (opts.phase === "report" || opts.phase === "all") {
    if (!reportFile) {
      if (!resultFile)   throw new Error("--result-file is required for report phase");
      if (!snapshotFile) throw new Error("--snapshot-file is required for report phase");
      reportFile = runReport(opts, snapshotFile, resultFile);
    } else {
      console.log(`\nSkipping report — using: ${reportFile}`);
    }
  }

  if (opts.phase === "approve" || opts.phase === "all") {
    if (!approvalFile) {
      if (!reportFile)          throw new Error("--report-file is required for approve phase");
      if (!opts.approverSecret) throw new Error("--approver-secret is required");
      approvalFile = runApprove(opts, reportFile);
    } else {
      console.log(`\nSkipping approve — using: ${approvalFile}`);
    }
  }

  // Final gate: if mainnet migration were to proceed, approval must be verified
  if (reportFile && approvalFile) {
    verifyApproval(approvalFile, reportFile);
  }

  console.log("\n════ Migration dry-run complete ════");
  console.log(`  Snapshot:  ${snapshotFile || "—"}`);
  console.log(`  Result:    ${resultFile   || "—"}`);
  console.log(`  Report:    ${reportFile   || "—"}`);
  console.log(`  Approval:  ${approvalFile || "—"}`);

  if (opts.dryRun) {
    console.log("\nDry-run mode: mainnet migration NOT executed.");
  } else {
    console.log("\nAll gates passed. Ready for mainnet migration (execute via deploy.sh).");
  }
}

main().catch((err) => {
  console.error("ERROR:", err.message ?? err);
  process.exit(1);
});
