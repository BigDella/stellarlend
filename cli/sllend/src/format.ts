/**
 * format.ts — Human-readable output formatters for the sllend CLI.
 */
import chalk from "chalk";

// Soroban amounts use 7 decimal places (stroops × 10^7 → XLM)
const STROOP_SCALE = 10_000_000n;

export function formatAmount(raw: bigint | number | string, symbol = "XLM"): string {
  const n = BigInt(raw);
  const whole = n / STROOP_SCALE;
  const frac  = (n % STROOP_SCALE).toString().replace("-", "").padStart(7, "0");
  return `${whole}.${frac} ${symbol}`;
}

export function formatPercent(basisPoints: bigint | number | string): string {
  // Contract stores rates in basis points (100 bp = 1%)
  const bp = Number(basisPoints);
  return `${(bp / 100).toFixed(2)}%`;
}

export function formatHealthFactor(raw: bigint | number | string): string {
  const hf = Number(raw) / 10_000; // contract stores with 4 decimal places
  const label =
    hf >= 2    ? chalk.green(`${hf.toFixed(4)} ✔ healthy`) :
    hf >= 1.05 ? chalk.yellow(`${hf.toFixed(4)} ⚠ caution`) :
                 chalk.red(`${hf.toFixed(4)} ✗ liquidatable`);
  return label;
}

export function formatRatio(raw: bigint | number | string): string {
  const r = Number(raw) / 100;
  return `${r.toFixed(2)}%`;
}

export function formatPosition(pos: Record<string, unknown>): string {
  const lines = [
    chalk.bold("Position"),
    `  Collateral : ${formatAmount(pos.collateral as bigint)}`,
    `  Debt       : ${formatAmount(pos.debt as bigint)}`,
    `  Ratio      : ${formatRatio(pos.collateral_ratio as bigint)}`,
    `  Health     : ${formatHealthFactor((pos.health_factor ?? pos.collateral_ratio) as bigint)}`,
  ];
  return lines.join("\n");
}

export function formatPool(pool: Record<string, unknown>): string {
  const lines = [
    chalk.bold("Pool"),
    `  Total deposits  : ${formatAmount(pool.total_deposits as bigint)}`,
    `  Total borrows   : ${formatAmount(pool.total_borrows as bigint)}`,
    `  Utilization     : ${formatPercent(pool.utilization_rate as bigint)}`,
    `  Borrow APY      : ${formatPercent(pool.borrow_rate as bigint)}`,
    `  Supply APY      : ${formatPercent(pool.supply_rate as bigint)}`,
    `  Reserve balance : ${formatAmount(pool.reserve_balance as bigint)}`,
  ];
  return lines.join("\n");
}

export function formatTxResult(txHash: string, value: unknown): string {
  return [
    chalk.green("✔ Transaction confirmed"),
    `  Hash  : ${txHash}`,
    value != null ? `  Value : ${JSON.stringify(value)}` : "",
  ].filter(Boolean).join("\n");
}

export function formatError(err: unknown): string {
  const msg = err instanceof Error ? err.message : String(err);

  // Map known contract errors to human-readable suggestions
  const suggestions: Array<[RegExp, string]> = [
    [/InsufficientCollateral/i,  "Your collateral ratio would fall below the minimum. Deposit more collateral first."],
    [/BorrowLimitExceeded/i,     "Requested borrow exceeds your collateral limit. Reduce the amount."],
    [/PositionNotLiquidatable/i, "This position is healthy and cannot be liquidated."],
    [/Paused/i,                  "The protocol is currently paused. Check https://stellarlend.io/status for updates."],
    [/InsufficientFunds/i,       "Your account balance is too low to cover this operation."],
    [/Unauthorized/i,            "This operation requires admin authority."],
    [/Simulation error.*HostError/i, "Contract execution failed. Check your arguments and contract ID."],
  ];

  const match = suggestions.find(([re]) => re.test(msg));
  const suggestion = match ? `\n  Suggestion: ${match[1]}` : "";
  return chalk.red(`Error: ${msg}`) + chalk.yellow(suggestion);
}

/** Print result as JSON or human-readable depending on flag. */
export function printResult(data: unknown, json: boolean): void {
  if (json) {
    process.stdout.write(JSON.stringify(data, (_k, v) =>
      typeof v === "bigint" ? v.toString() : v
    , 2) + "\n");
  } else if (typeof data === "string") {
    console.log(data);
  } else {
    console.log(JSON.stringify(data, (_k, v) =>
      typeof v === "bigint" ? v.toString() : v
    , 2));
  }
}
