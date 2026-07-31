import { test } from "node:test";
import assert from "node:assert/strict";
import {
  buildGraph,
  dependencies,
  dependents,
  detectCycles,
  detectDependencies,
  impactReport,
  toDOT,
} from "./analyzer.ts";
import { type ModuleSource } from "./types.ts";

function modules(): ModuleSource[] {
  return [
    // lending calls oracle (client) and imports shared
    {
      name: "lending",
      source: `use shared::types::Asset;\nlet px = OracleClient::new(&env, &id).price();`,
    },
    // oracle imports shared
    { name: "oracle", source: `use shared::math;\npub fn price() {}` },
    // shared depends on nothing
    { name: "shared", source: `pub struct Asset {}` },
    // governance calls lending
    { name: "governance", source: `let c = LendingClient::new(&env, &id);` },
  ];
}

test("detectDependencies finds call and import edges", () => {
  const edges = detectDependencies(modules()[0], ["lending", "oracle", "shared", "governance"]);
  const oracle = edges.find((e) => e.to === "oracle");
  const shared = edges.find((e) => e.to === "shared");
  assert.equal(oracle?.kind, "call");
  assert.equal(shared?.kind, "import");
  // does not depend on itself
  assert.equal(edges.some((e) => e.to === "lending"), false);
});

test("detectDependencies includes library deps", () => {
  const edges = detectDependencies(modules()[2], ["lending", "oracle", "shared"], ["soroban-sdk"]);
  assert.ok(edges.some((e) => e.to === "soroban-sdk" && e.kind === "library"));
});

test("buildGraph assembles nodes and edges", () => {
  const g = buildGraph(modules());
  assert.ok(g.nodes.includes("lending"));
  assert.ok(g.edges.some((e) => e.from === "lending" && e.to === "oracle" && e.kind === "call"));
  assert.ok(g.edges.some((e) => e.from === "governance" && e.to === "lending"));
});

test("dependencies returns transitive out-edges", () => {
  const g = buildGraph(modules());
  // lending -> oracle -> shared, lending -> shared
  assert.deepEqual(dependencies(g, "lending"), ["oracle", "shared"]);
});

test("dependents returns transitive in-edges (upgrade impact)", () => {
  const g = buildGraph(modules());
  // who is affected if shared is upgraded? lending, oracle, and governance (via lending)
  assert.deepEqual(dependents(g, "shared"), ["governance", "lending", "oracle"]);
});

test("impactReport classifies risk by blast radius", () => {
  const g = buildGraph(modules());
  const shared = impactReport(g, "shared");
  assert.equal(shared.riskLevel, "high"); // 3 transitive dependents
  assert.ok(shared.directDependents.includes("lending"));
  assert.ok(shared.directDependents.includes("oracle"));

  const governance = impactReport(g, "governance");
  assert.equal(governance.riskLevel, "low"); // nothing depends on governance
  assert.deepEqual(governance.transitiveDependents, []);
});

test("detectCycles finds a dependency cycle", () => {
  const g = buildGraph([
    { name: "a", source: "BClient::new();" },
    { name: "b", source: "AClient::new();" },
  ]);
  const cycles = detectCycles(g);
  assert.ok(cycles.length >= 1);
});

test("acyclic graph reports no cycles", () => {
  assert.deepEqual(detectCycles(buildGraph(modules())), []);
});

test("toDOT emits a digraph with styled edges", () => {
  const dot = toDOT(buildGraph(modules()));
  assert.ok(dot.startsWith("digraph contract_dependencies"));
  assert.ok(dot.includes('"lending" -> "oracle"'));
  assert.ok(dot.includes("style=solid")); // call edge
  assert.ok(dot.includes("style=dashed")); // import edge
});
