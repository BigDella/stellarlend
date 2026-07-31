/**
 * borrow.ts — borrow assets from the lending pool.
 */
import type { Command } from "commander";
import ora from "ora";
import { StellarLendClient, addrVal, i128Val } from "../client.js";
import { formatTxResult, printResult, formatError } from "../format.js";

function parseAmount(s: string): bigint {
  const [whole, frac = ""] = s.split(".");
  return BigInt(whole) * 10_000_000n + BigInt(frac.padEnd(7, "0").slice(0, 7));
}

export function registerBorrow(
  program: Command,
  getClient: () => StellarLendClient,
  getContractId: () => string,
  getSecret: () => Promise<string>,
  getAccount: () => string,
  jsonFlag: () => boolean
): void {
  program
    .command("borrow <amount>")
    .description("Borrow assets against your collateral")
    .option("--asset <address>", "Asset contract address (default: native XLM)")
    .action(async (amount: string, opts: { asset?: string }) => {
      const json = jsonFlag();
      const spin = ora("Borrowing…").start();
      try {
        const client = getClient();
        const contractId = getContractId();
        const secret = await getSecret();
        const account = getAccount();

        const amountBigInt = parseAmount(amount);
        const method = opts.asset ? "borrow_asset" : "borrow";
        const args = opts.asset
          ? [addrVal(account), addrVal(opts.asset), i128Val(amountBigInt)]
          : [addrVal(account), i128Val(amountBigInt)];

        const result = await client.invoke(contractId, method, args, secret);
        spin.succeed("Borrow confirmed");
        printResult(json ? result : formatTxResult(result.txHash, result.value), json);
      } catch (err) {
        spin.fail("Borrow failed");
        console.error(formatError(err));
        process.exit(1);
      }
    });
}
