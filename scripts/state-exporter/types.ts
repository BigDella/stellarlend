/**
 * Domain types for the contract state-exporter (#499).
 *
 * These mirror the protocol's on-chain state at a structural level so the
 * exporter can run against any source — a live RPC reader or a captured JSON
 * snapshot — without coupling to a specific transport.
 */

/** A borrower/supplier position. */
export interface UserPosition {
  address: string;
  collateral: string; // stringified i128 to avoid precision loss
  debt: string;
  healthFactor: string;
}

/** Per-asset pool configuration. */
export interface PoolConfig {
  asset: string;
  supplyCap: string;
  interestRateBps: number;
  collateralFactorBps: number;
}

/** A governance proposal and its tally. */
export interface GovernanceRecord {
  proposalId: number;
  status: string;
  votesFor: string;
  votesAgainst: string;
  executed: boolean;
}

/** Full structured contract state. */
export interface ContractState {
  positions: UserPosition[];
  pools: PoolConfig[];
  governance: GovernanceRecord[];
}

export type StateSection = keyof ContractState;

export const STATE_SECTIONS: StateSection[] = ["positions", "pools", "governance"];

/** Stable primary key for each section, used for diffing (CDC) and CSV ordering. */
export const SECTION_KEY: Record<StateSection, string> = {
  positions: "address",
  pools: "asset",
  governance: "proposalId",
};

/**
 * Abstraction over where state comes from. The default file source reads a JSON
 * snapshot; a live Soroban RPC source (reading view functions + events) is a
 * documented follow-up.
 */
export interface StateSource {
  read(): Promise<ContractState>;
}

export type ExportFormat = "json" | "csv";

/** Verifiable metadata accompanying every export. */
export interface ExportManifest {
  generatedAt: string;
  format: ExportFormat;
  compressed: boolean;
  incremental: boolean;
  rowCount: Record<StateSection, number> & { total: number };
  checksum: string; // sha256 of the uncompressed payload
}
