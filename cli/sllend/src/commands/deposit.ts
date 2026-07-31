/**
 * deposit.ts — deposit collateral into the lending pool.
 */
import type { Command } from "commander";
import ora from "ora";
import { StellarLendClient, addrVal, i128Val } from "../client.js";
import { formatAmount, formatTxResult, printResult, formatError } from "../format.js";
import type { NetworkConfig } from "../config.js";

interface DepositOpts {
  amount: string;
  asset?: string;
  json: boolean;
}

export function registerDeposit(
  program: Command,
  getClient: () => StellarLendClient,
  getContractId: () => string,
  getSecret: () => Promise<string>,
  getAccount: () => string,
  jsonFlag: () => boolean
): void {
  program
    .command("deposit <amount>")
    .description("Deposit collateral into the lending pool")
    .option("--asset <address>", "Asset contract address (default: native XLM)")
    .action(async (amount: string, opts: Pick<DepositOpts, "asset">) => {
      const json = jsonFlag();
      const spin = ora("Depositing…").start();
      try {
        const client = getClient();
        const contractId = getContractId();
        const secret = await getSecret();
        const account = getAccount();

        const amountBigInt = parseAmount(amount);
        const args = opts.asset
          ? [addrVal(account), addrVal(opts.asset), i128Val(amountBigInt)]
          : [addrVal(account), i128Val(amountBigInt)];

        const method = opts.asset ? "deposit_collateral_asset" : "deposit_collateral";
        const result = await client.invoke(contractId, method, args, secret);
        spin.succeed("Deposit confirmed");

        printResult(
          json ? result : formatTxResult(result.txHash, result.value),
          json
        );
      } catch (err) {
        spin.fail("Deposit failed");
        console.error(formatError(err));
        process.exit(1);
      }
    });
}

function parseAmount(s: string): bigint {
  // Accept "10", "10.5", "10.5000000"
  const [whole, frac = ""] = s.split(".");
  const padded = frac.padEnd(7, "0").slice(0, 7);
  return BigInt(whole) * 10_000_000n + BigInt(padded);
}
