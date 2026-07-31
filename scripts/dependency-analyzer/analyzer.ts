/**
 * Core dependency-graph analysis for contract upgrade-impact assessment (#496).
 *
 * Pure functions: detect cross-contract references in source, build a graph,
 * compute transitive dependents/dependencies (upgrade impact), detect cycles,
 * and emit Graphviz DOT. The interactive web visualization and CI wiring are
 * documented follow-ups; this is the analysis engine.
 */

import {
  type DependencyEdge,
  type DependencyGraph,
  type ImpactReport,
  type ModuleSource,
} from "./types.ts";

function escapeRegExp(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

/**
 * Detect dependency edges from `module` to other known contracts by scanning
 * its source. Heuristics tuned for Rust/Soroban:
 *  - `import`  : `use <C>` / `<C>::` / `mod <C>` (interface/module imports)
 *  - `call`    : `<C>Client` (generated cross-contract client = a direct call)
 *  - `library` : declared elsewhere (Cargo deps), supplied via `libraryDeps`
 */
export function detectDependencies(
  module: ModuleSource,
  knownContracts: string[],
  libraryDeps: string[] = [],
): DependencyEdge[] {
  const edges: DependencyEdge[] = [];
  const seen = new Set<string>();

  for (const target of knownContracts) {
    if (target === module.name) continue;
    const t = escapeRegExp(target);
    // Soroban generated clients are PascalCase (`OracleClient`) while crate/node
    // names are snake_case (`oracle`); compact underscores and match
    // case-insensitively so `oracle` detects `OracleClient` and `stellar_lend`
    // detects `StellarLendClient`.
    const compact = escapeRegExp(target.replace(/_/g, ""));

    const callRe = new RegExp(`\\b${compact}Client\\b`, "i");
    const importRe = new RegExp(`\\b(use|mod)\\s+${t}\\b|\\b${t}::`);

    let kind: DependencyEdge["kind"] | null = null;
    if (callRe.test(module.source)) kind = "call";
    else if (importRe.test(module.source)) kind = "import";

    if (kind) {
      const id = `${target}:${kind}`;
      if (!seen.has(id)) {
        seen.add(id);
        edges.push({ from: module.name, to: target, kind });
      }
    }
  }

  for (const lib of libraryDeps) {
    if (lib === module.name) continue;
    edges.push({ from: module.name, to: lib, kind: "library" });
  }

  return edges;
}

/** Build the full dependency graph from a set of module sources. */
export function buildGraph(
  modules: ModuleSource[],
  libraryDepsByModule: Record<string, string[]> = {},
): DependencyGraph {
  const names = modules.map((m) => m.name);
  const nodeSet = new Set(names);
  const edges: DependencyEdge[] = [];
  for (const m of modules) {
    for (const e of detectDependencies(m, names, libraryDepsByModule[m.name] ?? [])) {
      edges.push(e);
      nodeSet.add(e.to);
    }
  }
  return { nodes: [...nodeSet].sort(), edges };
}

function neighbors(graph: DependencyGraph, node: string, direction: "out" | "in"): string[] {
  return graph.edges
    .filter((e) => (direction === "out" ? e.from === node : e.to === node))
    .map((e) => (direction === "out" ? e.to : e.from));
}

function transitiveClosure(graph: DependencyGraph, start: string, direction: "out" | "in"): string[] {
  const visited = new Set<string>();
  const stack = [...neighbors(graph, start, direction)];
  while (stack.length) {
    const n = stack.pop() as string;
    if (visited.has(n) || n === start) continue;
    visited.add(n);
    stack.push(...neighbors(graph, n, direction));
  }
  return [...visited].sort();
}

/** What `target` depends on (transitively). */
export function dependencies(graph: DependencyGraph, target: string): string[] {
  return transitiveClosure(graph, target, "out");
}

/** What depends on `target` (transitively) — i.e. what an upgrade impacts. */
export function dependents(graph: DependencyGraph, target: string): string[] {
  return transitiveClosure(graph, target, "in");
}

/** Upgrade-impact report for a contract. */
export function impactReport(graph: DependencyGraph, target: string): ImpactReport {
  const direct = [...new Set(neighbors(graph, target, "in"))].sort();
  const transitive = dependents(graph, target);
  const riskLevel = transitive.length >= 3 ? "high" : transitive.length >= 1 ? "medium" : "low";
  return {
    target,
    directDependents: direct,
    transitiveDependents: transitive,
    riskLevel,
  };
}

/** Detect dependency cycles (each returned array is a cycle path). */
export function detectCycles(graph: DependencyGraph): string[][] {
  const cycles: string[][] = [];
  const state = new Map<string, number>(); // 0=visiting,1=done
  const path: string[] = [];

  const visit = (node: string): void => {
    state.set(node, 0);
    path.push(node);
    for (const next of neighbors(graph, node, "out")) {
      if (!state.has(next)) visit(next);
      else if (state.get(next) === 0) {
        const idx = path.indexOf(next);
        if (idx >= 0) cycles.push([...path.slice(idx), next]);
      }
    }
    path.pop();
    state.set(node, 1);
  };

  for (const node of graph.nodes) if (!state.has(node)) visit(node);
  return cycles;
}

/** Emit Graphviz DOT, with edge styling per dependency kind. */
export function toDOT(graph: DependencyGraph): string {
  const style: Record<DependencyEdge["kind"], string> = {
    call: "solid",
    import: "dashed",
    library: "dotted",
  };
  const lines = ["digraph contract_dependencies {", "  rankdir=LR;", "  node [shape=box];"];
  for (const n of graph.nodes) lines.push(`  "${n}";`);
  for (const e of graph.edges) {
    lines.push(`  "${e.from}" -> "${e.to}" [style=${style[e.kind]} label="${e.kind}"];`);
  }
  lines.push("}");
  return lines.join("\n");
}
