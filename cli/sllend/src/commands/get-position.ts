/**
 * get-position.ts — query a user's collateral/debt position.
 */
import type { Command } from "commander";
import ora from "ora";
import { StellarLendClient, addrVal } from "../client.js";
import { formatPosition, printResult, formatError } from "../format.js";

export function registerGetPosition(
  program: Command,
  getClient: () => StellarLendClient,
  getContractId: () => string,
  getAccount: () => string,
  jsonFlag: () => boolean
): void {
  program
    .command("get-position [address]")
    .description("Query a user's collateral and debt position")
    .action(async (address?: string) => {
      const json = jsonFlag();
      const spin = ora("Fetching position…").start();
      try {
        const client = getClient();
        const contractId = getContractId();
        const target = address ?? getAccount();

        const value = await client.query(contractId, "get_position", [addrVal(target)]);
        spin.stop();

        if (json) {
          printResult(value, true);
        } else {
          const pos = value as Record<string, unknown>;
          console.log(formatPosition(pos));
        }
      } catch (err) {
        spin.fail("Failed to fetch position");
        console.error(formatError(err));
        process.exit(1);
      }
    });
}
