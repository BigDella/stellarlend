#!/usr/bin/env node
/**
 * index.ts — sllend CLI entry point.
 *
 * Features:
 *  • All contract commands (deposit, withdraw, borrow, repay, liquidate, get-position, get-pool)
 *  • --json flag for programmatic output
 *  • --network flag for testnet / mainnet / local
 *  • --key flag (keystore alias) + --secret flag (raw, dev only)
 *  • --contract flag to override contract ID
 *  • Interactive mode (sllend interactive)
 *  • Batch mode: sllend batch <commands.json>
 *  • Key management: sllend keys add/list/remove
 *  • Config management: sllend config set/show
 */

import { Command } from "commander";
import * as readline from "readline";
import * as fs from "fs";
import inquirer from "inquirer";

import { loadConfig, getNetwork, addKey, unlockKey, listKeys, removeKey, saveConfig } from "./config.js";
import { StellarLendClient } from "./client.js";
import { formatError, printResult } from "./format.js";

import { registerDeposit }     from "./commands/deposit.js";
import { registerWithdraw }    from "./commands/withdraw.js";
import { registerBorrow }      from "./commands/borrow.js";
import { registerRepay }       from "./commands/repay.js";
import { registerLiquidate }   from "./commands/liquidate.js";
import { registerGetPosition } from "./commands/get-position.js";
import { registerGetPool }     from "./commands/get-pool.js";

// ── Program setup ─────────────────────────────────────────────────────────────

const program = new Command();

program
  .name("sllend")
  .description("StellarLend developer CLI — interact with lending contracts")
  .version("0.1.0")
  .option("--network <name>",   "Network to use: testnet | mainnet | local (default: config)")
  .option("--contract <id>",    "Override contract ID")
  .option("--key <alias>",      "Keystore alias to use for signing")
  .option("--secret <key>",     "Raw Stellar secret key (dev only, prefer --key)")
  .option("--account <addr>",   "Override sender address (read-only queries)")
  .option("--json",             "Output raw JSON (for scripting)")
  .option("--rpc-url <url>",    "Override Soroban RPC URL");

// ── Shared context helpers ────────────────────────────────────────────────────

let _client: StellarLendClient | null = null;
let _secretCache: string | null = null;

function getClient(): StellarLendClient {
  if (_client) return _client;
  const cfg = loadConfig();
  const opts = program.opts<{ network?: string; rpcUrl?: string }>();
  const net = getNetwork(cfg, opts.network);
  if (opts.rpcUrl) net.rpcUrl = opts.rpcUrl;
  _client = new StellarLendClient(net);
  return _client;
}

function getContractId(): string {
  const opts = program.opts<{ contract?: string; network?: string }>();
  if (opts.contract) return opts.contract;
  const cfg = loadConfig();
  const net = getNetwork(cfg, opts.network);
  if (!net.contractId) {
    console.error(formatError(
      "No contract ID set. Pass --contract <id> or set it in ~/.sllend/config.toml"
    ));
    process.exit(1);
  }
  return net.contractId;
}

async function getSecret(): Promise<string> {
  if (_secretCache) return _secretCache;
  const opts = program.opts<{ secret?: string; key?: string }>();
  if (opts.secret) return (_secretCache = opts.secret);
  if (opts.key) {
    const { password } = await inquirer.prompt<{ password: string }>([{
      type: "password", name: "password",
      message: `Password for key "${opts.key}":`,
      mask: "*",
    }]);
    return (_secretCache = unlockKey(opts.key, password));
  }
  console.error(formatError("No signing key. Pass --key <alias> or --secret <key>"));
  process.exit(1);
}

function getAccount(): string {
  const opts = program.opts<{ account?: string; key?: string; secret?: string }>();
  if (opts.account) return opts.account;
  if (opts.secret) {
    const { Keypair } = require("@stellar/stellar-sdk");
    return Keypair.fromSecret(opts.secret).publicKey();
  }
  if (opts.key) {
    const keys = listKeys();
    const k = keys.find((e) => e.alias === opts.key);
    if (k) return k.publicKey;
  }
  console.error(formatError("No account address. Pass --account or --key."));
  process.exit(1);
}

function jsonFlag(): boolean {
  return !!program.opts<{ json?: boolean }>().json;
}

// ── Register all contract commands ────────────────────────────────────────────

registerDeposit(program, getClient, getContractId, getSecret, getAccount, jsonFlag);
registerWithdraw(program, getClient, getContractId, getSecret, getAccount, jsonFlag);
registerBorrow(program, getClient, getContractId, getSecret, getAccount, jsonFlag);
registerRepay(program, getClient, getContractId, getSecret, getAccount, jsonFlag);
registerLiquidate(program, getClient, getContractId, getSecret, getAccount, jsonFlag);
registerGetPosition(program, getClient, getContractId, getAccount, jsonFlag);
registerGetPool(program, getClient, getContractId, jsonFlag);

// ── keys command ──────────────────────────────────────────────────────────────

const keysCmd = program.command("keys").description("Manage encrypted keystores");

keysCmd
  .command("add <alias> <secret>")
  .description("Encrypt and store a Stellar secret key")
  .action(async (alias: string, secret: string) => {
    const { Keypair } = await import("@stellar/stellar-sdk");
    const { password, confirm } = await inquirer.prompt<{ password: string; confirm: string }>([
      { type: "password", name: "password", message: "Set keystore password:", mask: "*" },
      { type: "password", name: "confirm",  message: "Confirm password:", mask: "*" },
    ]);
    if (password !== confirm) { console.error(formatError("Passwords do not match.")); process.exit(1); }
    const publicKey = Keypair.fromSecret(secret).publicKey();
    addKey(alias, secret, password, publicKey);
    console.log(`Key "${alias}" stored. Public key: ${publicKey}`);
  });

keysCmd
  .command("list")
  .description("List stored key aliases")
  .action(() => {
    const keys = listKeys();
    if (keys.length === 0) { console.log("No keys stored."); return; }
    printResult(keys, jsonFlag());
  });

keysCmd
  .command("remove <alias>")
  .description("Remove a stored key")
  .action((alias: string) => {
    removeKey(alias);
    console.log(`Key "${alias}" removed.`);
  });

// ── config command ────────────────────────────────────────────────────────────

const configCmd = program.command("config").description("Manage sllend configuration");

configCmd
  .command("show")
  .description("Print current configuration")
  .action(() => printResult(loadConfig(), jsonFlag()));

configCmd
  .command("set-network <name> <rpcUrl> <horizonUrl> <passphrase>")
  .description("Add or update a network entry")
  .action((name: string, rpcUrl: string, horizonUrl: string, networkPassphrase: string) => {
    const cfg = loadConfig();
    cfg.networks[name] = { rpcUrl, horizonUrl, networkPassphrase };
    saveConfig(cfg);
    console.log(`Network "${name}" saved.`);
  });

configCmd
  .command("set-contract <network> <contractId>")
  .description("Set the lending contract ID for a network")
  .action((network: string, contractId: string) => {
    const cfg = loadConfig();
    if (!cfg.networks[network]) { console.error(formatError(`Network "${network}" not found.`)); process.exit(1); }
    cfg.networks[network].contractId = contractId;
    saveConfig(cfg);
    console.log(`Contract ID for "${network}" set to ${contractId}`);
  });

configCmd
  .command("set-default-network <name>")
  .description("Set the default network")
  .action((name: string) => {
    const cfg = loadConfig();
    cfg.defaultNetwork = name;
    saveConfig(cfg);
    console.log(`Default network set to "${name}"`);
  });

// ── interactive command ───────────────────────────────────────────────────────

program
  .command("interactive")
  .alias("i")
  .description("Interactive REPL mode with prompts for all parameters")
  .action(async () => {
    console.log("sllend interactive mode. Type 'exit' to quit.\n");
    const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
    const ask = (q: string) => new Promise<string>((res) => rl.question(q, res));

    while (true) {
      const input = (await ask("sllend> ")).trim();
      if (input === "exit" || input === "quit") { rl.close(); break; }
      if (!input) continue;

      // Re-parse as if it were a top-level command invocation
      try {
        // Insert a dummy argv[0]/argv[1] so Commander parses correctly
        await program.parseAsync(["node", "sllend", ...input.split(/\s+/)]);
      } catch (err) {
        console.error(formatError(err));
      }
    }
  });

// ── batch command ─────────────────────────────────────────────────────────────

program
  .command("batch <file>")
  .description("Run a JSON batch of commands: [{\"cmd\": \"deposit\", \"args\": [\"10\"]}]")
  .action(async (file: string) => {
    if (!fs.existsSync(file)) { console.error(formatError(`File not found: ${file}`)); process.exit(1); }
    const batch = JSON.parse(fs.readFileSync(file, "utf8")) as Array<{ cmd: string; args: string[] }>;
    for (const entry of batch) {
      console.log(`\n>>> ${entry.cmd} ${entry.args.join(" ")}`);
      try {
        await program.parseAsync(["node", "sllend", entry.cmd, ...entry.args]);
      } catch (err) {
        console.error(formatError(err));
        // Continue with next command in batch
      }
    }
  });

// ── Parse ─────────────────────────────────────────────────────────────────────

program.parseAsync(process.argv).catch((err) => {
  console.error(formatError(err));
  process.exit(1);
});
