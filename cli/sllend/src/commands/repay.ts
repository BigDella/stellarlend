/**
 * repay.ts — repay borrowed debt.
 */
import type { Command } from "commander";
import ora from "ora";
import { StellarLendClient, addrVal, i128Val } from "../client.js";
import { formatTxResult, printResult, formatError } from "../format.js";

function parseAmount(s: string): bigint {
  const [whole, frac = ""] = s.split(".");
  return BigInt(whole) * 10_000_000n + BigInt(frac.padEnd(7, "0").slice(0, 7));
}

export function registerRepay(
  program: Command,
  getClient: () => StellarLendClient,
  getContractId: () => string,
  getSecret: () => Promise<string>,
  getAccount: () => string,
  jsonFlag: () => boolean
): void {
  program
    .command("repay <amount>")
    .description("Repay borrowed debt (use 'max' to repay full balance)")
    .option("--asset <address>", "Asset contract address (default: native XLM)")
    .action(async (amount: string, opts: { asset?: string }) => {
      const json = jsonFlag();
      const spin = ora("Repaying…").start();
      try {
        const client = getClient();
        const contractId = getContractId();
        const secret = await getSecret();
        const account = getAccount();

        // 'max' sentinel: use i128::MAX so contract repays full balance
        const amountBigInt = amount === "max"
          ? 170141183460469231731687303715884105727n
          : parseAmount(amount);

        const method = opts.asset ? "repay_asset" : "repay";
        const args = opts.asset
          ? [addrVal(account), addrVal(opts.asset), i128Val(amountBigInt)]
          : [addrVal(account), i128Val(amountBigInt)];

        const result = await client.invoke(contractId, method, args, secret);
        spin.succeed("Repayment confirmed");
        printResult(json ? result : formatTxResult(result.txHash, result.value), json);
      } catch (err) {
        spin.fail("Repayment failed");
        console.error(formatError(err));
        process.exit(1);
      }
    });
}
