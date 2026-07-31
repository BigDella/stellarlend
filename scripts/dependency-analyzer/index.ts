#!/usr/bin/env node
/**
 * dependency-analyzer CLI (#496).
 *
 * Scans a workspace of Rust/Soroban crates, builds a contract dependency graph,
 * and reports upgrade impact. Each crate (a `Cargo.toml` with a `[package]`
 * name) becomes a node; its `.rs` sources are scanned for cross-contract
 * references and its `[dependencies]` table supplies library edges.
 *
 * Run (Node >= 22, no install):
 *   node --experimental-strip-types scripts/dependency-analyzer/index.ts \
 *     --root stellar-lend --out graph.json --dot graph.dot
 *   node --experimental-strip-types scripts/dependency-analyzer/index.ts \
 *     --root stellar-lend --impact oracle
 */

import * as fs from "node:fs";
import * as path from "node:path";
import { buildGraph, impactReport, detectCycles, toDOT } from "./analyzer.ts";
import { type ModuleSource } from "./types.ts";

function arg(name: string): string | undefined {
  const i = process.argv.indexOf(`--${name}`);
  return i >= 0 ? process.argv[i + 1] : undefined;
}

function walk(dir: string, acc: string[] = []): string[] {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (entry.name === "target" || entry.name === "node_modules" || entry.name.startsWith(".")) {
      continue;
    }
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) walk(full, acc);
    else acc.push(full);
  }
  return acc;
}

function packageName(cargoToml: string): string | null {
  const m = /\[package\][\s\S]*?\bname\s*=\s*"([^"]+)"/.exec(cargoToml);
  return m ? m[1] : null;
}

function dependencyNames(cargoToml: string): string[] {
  const depsSection = /\[dependencies\]([\s\S]*?)(\n\[|$)/.exec(cargoToml);
  if (!depsSection) return [];
  return [...depsSection[1].matchAll(/^\s*([A-Za-z0-9_-]+)\s*=/gm)].map((x) => x[1]);
}

function collectModules(root: string): {
  modules: ModuleSource[];
  libraryDeps: Record<string, string[]>;
} {
  const cargoFiles = walk(root).filter((f) => path.basename(f) === "Cargo.toml");
  const modules: ModuleSource[] = [];
  const libraryDeps: Record<string, string[]> = {};
  for (const cargo of cargoFiles) {
    const toml = fs.readFileSync(cargo, "utf8");
    const name = packageName(toml);
    if (!name) continue;
    const crateDir = path.dirname(cargo);
    const rs = walk(crateDir).filter((f) => f.endsWith(".rs"));
    const source = rs.map((f) => fs.readFileSync(f, "utf8")).join("\n");
    modules.push({ name, source });
    libraryDeps[name] = dependencyNames(toml);
  }
  return { modules, libraryDeps };
}

function main(): void {
  const root = arg("root") ?? ".";
  const { modules, libraryDeps } = collectModules(root);
  // Only keep library edges that point at another crate in this workspace.
  const localNames = new Set(modules.map((m) => m.name));
  const localLibDeps: Record<string, string[]> = {};
  for (const [name, deps] of Object.entries(libraryDeps)) {
    localLibDeps[name] = deps.filter((d) => localNames.has(d));
  }

  const graph = buildGraph(modules, localLibDeps);

  const impactTarget = arg("impact");
  if (impactTarget) {
    console.log(JSON.stringify(impactReport(graph, impactTarget), null, 2));
    return;
  }

  const cycles = detectCycles(graph);
  if (cycles.length) console.error(`⚠ ${cycles.length} dependency cycle(s) detected`);

  const out = arg("out");
  if (out) fs.writeFileSync(out, JSON.stringify(graph, null, 2));
  const dot = arg("dot");
  if (dot) fs.writeFileSync(dot, toDOT(graph));

  console.log(
    `Analyzed ${modules.length} crates: ${graph.nodes.length} nodes, ${graph.edges.length} edges` +
      (cycles.length ? `, ${cycles.length} cycle(s)` : ""),
  );
  if (!out && !dot) console.log(toDOT(graph));
}

main();
