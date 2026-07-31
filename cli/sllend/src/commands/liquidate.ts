/**
 * liquidate.ts — liquidate an undercollateralized position.
 */
import type { Command } from "commander";
import ora from "ora";
import { StellarLendClient, addrVal, i128Val } from "../client.js";
import { formatTxResult, printResult, formatError } from "../format.js";

function parseAmount(s: string): bigint {
  const [whole, frac = ""] = s.split(".");
  return BigInt(whole) * 10_000_000n + BigInt(frac.padEnd(7, "0").slice(0, 7));
}

export function registerLiquidate(
  program: Command,
  getClient: () => StellarLendClient,
  getContractId: () => string,
  getSecret: () => Promise<string>,
  getAccount: () => string,
  jsonFlag: () => boolean
): void {
  program
    .command("liquidate <borrower> <repay-amount>")
    .description("Liquidate an undercollateralized position")
    .option("--collateral-asset <address>", "Collateral asset to seize")
    .option("--debt-asset <address>", "Debt asset to repay")
    .action(async (
      borrower: string,
      repayAmount: string,
      opts: { collateralAsset?: string; debtAsset?: string }
    ) => {
      const json = jsonFlag();
      const spin = ora(`Liquidating ${borrower.slice(0, 8)}…`).start();
      try {
        const client = getClient();
        const contractId = getContractId();
        const secret = await getSecret();
        const liquidator = getAccount();

        const amount = parseAmount(repayAmount);
        // liquidate(liquidator, borrower, repay_amount) — matches contract signature
        const args = [addrVal(liquidator), addrVal(borrower), i128Val(amount)];

        const result = await client.invoke(contractId, "liquidate", args, secret);
        spin.succeed("Liquidation confirmed");
        printResult(json ? result : formatTxResult(result.txHash, result.value), json);
      } catch (err) {
        spin.fail("Liquidation failed");
        console.error(formatError(err));
        process.exit(1);
      }
    });
}
