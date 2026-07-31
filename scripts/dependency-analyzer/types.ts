/**
 * Types for the contract dependency-graph analyzer (#496).
 */

export type DependencyKind = "call" | "import" | "library";

export interface DependencyEdge {
  from: string;
  to: string;
  kind: DependencyKind;
}

export interface DependencyGraph {
  nodes: string[];
  edges: DependencyEdge[];
}

/** A contract/crate node and its source text, used to detect references. */
export interface ModuleSource {
  name: string;
  source: string;
}

export interface ImpactReport {
  target: string;
  /** Contracts that directly depend on `target`. */
  directDependents: string[];
  /** All contracts transitively affected by upgrading `target`. */
  transitiveDependents: string[];
  riskLevel: "low" | "medium" | "high";
}
