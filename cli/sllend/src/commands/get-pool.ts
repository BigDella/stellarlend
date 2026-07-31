/**
 * get-pool.ts — query protocol-wide pool stats.
 */
import type { Command } from "commander";
import ora from "ora";
import { StellarLendClient } from "../client.js";
import { formatPool, printResult, formatError } from "../format.js";

export function registerGetPool(
  program: Command,
  getClient: () => StellarLendClient,
  getContractId: () => string,
  jsonFlag: () => boolean
): void {
  program
    .command("get-pool")
    .description("Query protocol pool statistics (utilization, APYs, reserves)")
    .action(async () => {
      const json = jsonFlag();
      const spin = ora("Fetching pool stats…").start();
      try {
        const client = getClient();
        const contractId = getContractId();

        // get_system_stats returns aggregate pool data
        const value = await client.query(contractId, "get_system_stats", []);
        spin.stop();

        if (json) {
          printResult(value, true);
        } else {
          const pool = value as Record<string, unknown>;
          console.log(formatPool(pool));
        }
      } catch (err) {
        spin.fail("Failed to fetch pool stats");
        console.error(formatError(err));
        process.exit(1);
      }
    });
}
