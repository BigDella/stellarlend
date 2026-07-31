/**
 * client.ts — Soroban RPC + transaction builder for StellarLend contract calls.
 */
import {
  SorobanRpc,
  TransactionBuilder,
  Networks,
  Keypair,
  Contract,
  nativeToScVal,
  scValToNative,
  xdr,
  BASE_FEE,
} from "@stellar/stellar-sdk";
import type { NetworkConfig } from "./config.js";

export interface CallResult {
  /** Decoded return value */
  value: unknown;
  /** Raw XDR (for --json output) */
  rawXdr?: string;
  /** Simulated resource cost */
  cost?: { cpuInsns: string; memBytes: string };
}

// Soroban ledger close is ~5s; give tx 3 minutes to confirm
const TX_TIMEOUT_LEDGERS = 36;

export class StellarLendClient {
  private rpc: SorobanRpc.Server;
  private net: NetworkConfig & { name: string };

  constructor(net: NetworkConfig & { name: string }) {
    this.net  = net;
    this.rpc  = new SorobanRpc.Server(net.rpcUrl, { allowHttp: net.rpcUrl.startsWith("http://") });
  }

  /** Read-only simulation — no fee, no signing required. */
  async simulate(contractId: string, method: string, args: xdr.ScVal[]): Promise<CallResult> {
    const account  = await this.rpc.getAccount("GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN");
    const contract = new Contract(contractId);
    const tx = new TransactionBuilder(account, {
      fee: BASE_FEE,
      networkPassphrase: this.net.networkPassphrase,
    })
      .addOperation(contract.call(method, ...args))
      .setTimeout(30)
      .build();

    const simResult = await this.rpc.simulateTransaction(tx);
    if (SorobanRpc.Api.isSimulationError(simResult)) {
      throw new Error(`Simulation error: ${simResult.error}`);
    }
    const retval = (simResult as SorobanRpc.Api.SimulateTransactionSuccessResponse).result?.retval;
    return {
      value: retval ? scValToNative(retval) : null,
      rawXdr: retval?.toXDR("base64"),
      cost: simResult.cost,
    };
  }

  /** Submit a state-changing transaction, wait for confirmation. */
  async invoke(
    contractId: string,
    method: string,
    args: xdr.ScVal[],
    secret: string
  ): Promise<CallResult & { txHash: string }> {
    const keypair  = Keypair.fromSecret(secret);
    const account  = await this.rpc.getAccount(keypair.publicKey());
    const contract = new Contract(contractId);

    const tx = new TransactionBuilder(account, {
      fee: BASE_FEE,
      networkPassphrase: this.net.networkPassphrase,
    })
      .addOperation(contract.call(method, ...args))
      .setTimeout(TX_TIMEOUT_LEDGERS * 5)
      .build();

    // Simulate first to get resource footprint
    const simResult = await this.rpc.simulateTransaction(tx);
    if (SorobanRpc.Api.isSimulationError(simResult)) {
      throw new Error(`Simulation error: ${simResult.error}`);
    }

    const preparedTx = SorobanRpc.assembleTransaction(tx, simResult).build();
    preparedTx.sign(keypair);

    const sendResp = await this.rpc.sendTransaction(preparedTx);
    if (sendResp.status === "ERROR") {
      throw new Error(`Send error: ${sendResp.errorResult?.toXDR("base64")}`);
    }

    // Poll for confirmation
    const hash = sendResp.hash;
    for (let i = 0; i < TX_TIMEOUT_LEDGERS; i++) {
      await sleep(5000);
      const status = await this.rpc.getTransaction(hash);
      if (status.status === SorobanRpc.Api.GetTransactionStatus.SUCCESS) {
        const retval = (status as SorobanRpc.Api.GetSuccessfulTransactionResponse).returnValue;
        return {
          value: retval ? scValToNative(retval) : null,
          rawXdr: retval?.toXDR("base64"),
          txHash: hash,
        };
      }
      if (status.status === SorobanRpc.Api.GetTransactionStatus.FAILED) {
        throw new Error(`Transaction failed: ${hash}`);
      }
    }
    throw new Error(`Transaction timed out after ${TX_TIMEOUT_LEDGERS} ledgers: ${hash}`);
  }

  /** Pure sim-only — used for read-only contract views. */
  async query(contractId: string, method: string, args: xdr.ScVal[]): Promise<unknown> {
    const r = await this.simulate(contractId, method, args);
    return r.value;
  }
}

// ── Arg helpers ───────────────────────────────────────────────────────────────

export function addrVal(address: string): xdr.ScVal {
  return nativeToScVal(address, { type: "address" });
}

export function i128Val(amount: bigint): xdr.ScVal {
  return nativeToScVal(amount, { type: "i128" });
}

export function u32Val(n: number): xdr.ScVal {
  return nativeToScVal(n, { type: "u32" });
}

export function symbolVal(s: string): xdr.ScVal {
  return xdr.ScVal.scvSymbol(s);
}

function sleep(ms: number): Promise<void> {
  return new Promise((res) => setTimeout(res, ms));
}
