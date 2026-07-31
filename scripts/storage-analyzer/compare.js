#!/usr/bin/env node
/**
 * compare.js — Compare two storage layout JSON files (base vs head) and
 * produce a Markdown impact report.
 *
 * Usage:
 *   node compare.js <base.json> <head.json>
 *
 * Exit codes:
 *   0 — no breaking changes
 *   1 — breaking changes detected
 */

"use strict";

const fs = require("fs");
const path = require("path");

// ── Helpers ──────────────────────────────────────────────────────────

function loadJSON(filePath) {
  const raw = fs.readFileSync(filePath, "utf-8");
  return JSON.parse(raw);
}

/**
 * Flatten accesses from all contracts into a unified list with
 * a composite key for dedup / comparison.
 */
function flattenAccesses(layout) {
  const result = [];
  for (const [contract, data] of Object.entries(layout.contracts || {})) {
    for (const acc of data.accesses || []) {
      result.push({
        contract,
        tier: acc.tier,
        key: acc.key,
        keyKind: acc.key_kind,
        operation: acc.operation,
        file: acc.file,
        id: `${contract}::${acc.tier}::${acc.key}`,
      });
    }
  }
  return result;
}

/**
 * Build a map of contracttype structs keyed by "contract::name".
 */
function flattenTypes(layout) {
  const structs = new Map();
  const enums = new Map();
  for (const [contract, data] of Object.entries(layout.contracts || {})) {
    for (const t of data.types || []) {
      const id = `${contract}::${t.name}`;
      if (t.kind === "struct") {
        structs.set(id, { ...t, contract });
      } else if (t.kind === "enum") {
        enums.set(id, { ...t, contract });
      }
    }
  }
  return { structs, enums };
}

/**
 * Collect all key enum definitions.
 */
function flattenKeyEnums(layout) {
  const result = new Map();
  for (const [contract, data] of Object.entries(layout.contracts || {})) {
    for (const k of data.key_enums || []) {
      result.set(`${contract}::${k.name}`, { ...k, contract });
    }
  }
  return result;
}

// ── Diff logic ───────────────────────────────────────────────────────

function diffAccesses(baseList, headList) {
  const baseIds = new Set(baseList.map((a) => a.id));
  const headIds = new Set(headList.map((a) => a.id));

  const added = headList.filter((a) => !baseIds.has(a.id));
  const removed = baseList.filter((a) => !headIds.has(a.id));
  return { added, removed };
}

function diffStructs(baseMap, headMap) {
  const changes = [];

  // Removed structs
  for (const [id, base] of baseMap) {
    if (!headMap.has(id)) {
      changes.push({ id, type: "removed", base, head: null });
    }
  }

  // Added structs
  for (const [id, head] of headMap) {
    if (!baseMap.has(id)) {
      changes.push({ id, type: "added", base: null, head });
    }
  }

  // Modified structs — compare fields by name, order, and type
  for (const [id, base] of baseMap) {
    const head = headMap.get(id);
    if (!head) continue;

    const baseFields = base.fields || [];
    const headFields = head.fields || [];
    const fieldChanges = diffFields(baseFields, headFields);
    if (fieldChanges.length > 0) {
      changes.push({ id, type: "modified", base, head, fieldChanges });
    }
  }

  return changes;
}

function diffFields(baseFields, headFields) {
  const changes = [];
  const baseNames = baseFields.map((f) => f.name);
  const headNames = headFields.map((f) => f.name);

  // Added fields
  for (const f of headFields) {
    if (!baseNames.includes(f.name)) {
      changes.push({ field: f.name, change: "added", newType: f.type });
    }
  }

  // Removed fields
  for (const f of baseFields) {
    if (!headNames.includes(f.name)) {
      changes.push({ field: f.name, change: "removed", oldType: f.type });
    }
  }

  // Reordered — check if common fields kept their relative order
  const commonBase = baseFields.filter((f) => headNames.includes(f.name));
  const commonHead = headFields.filter((f) => baseNames.includes(f.name));
  if (commonBase.length > 1) {
    const baseOrder = commonBase.map((f) => f.name);
    const headOrder = commonHead.map((f) => f.name);
    if (JSON.stringify(baseOrder) !== JSON.stringify(headOrder)) {
      changes.push({
        field: "(ordering)",
        change: "reordered",
        from: baseOrder,
        to: headOrder,
      });
    }
  }

  // Type changes on surviving fields
  for (const bf of baseFields) {
    const hf = headFields.find((f) => f.name === bf.name);
    if (hf && hf.type !== bf.type) {
      changes.push({
        field: bf.name,
        change: "type_changed",
        oldType: bf.type,
        newType: hf.type,
      });
    }
  }

  return changes;
}

function diffEnums(baseMap, headMap) {
  const changes = [];

  for (const [id, base] of baseMap) {
    if (!headMap.has(id)) {
      changes.push({ id, type: "removed", base, head: null });
      continue;
    }
    const head = headMap.get(id);
    const baseVariants = (base.variants || []).map((v) => v.name);
    const headVariants = (head.variants || []).map((v) => v.name);

    const addedVariants = headVariants.filter((v) => !baseVariants.includes(v));
    const removedVariants = baseVariants.filter(
      (v) => !headVariants.includes(v)
    );

    // Check reordering of common variants
    const commonBase = baseVariants.filter((v) => headVariants.includes(v));
    const commonHead = headVariants.filter((v) => baseVariants.includes(v));
    const reordered =
      commonBase.length > 1 &&
      JSON.stringify(commonBase) !== JSON.stringify(commonHead);

    // Check param type changes on surviving variants
    const paramChanges = [];
    for (const bv of base.variants || []) {
      const hv = (head.variants || []).find((v) => v.name === bv.name);
      if (hv && (bv.params || "") !== (hv.params || "")) {
        paramChanges.push({
          variant: bv.name,
          oldParams: bv.params || "(none)",
          newParams: hv.params || "(none)",
        });
      }
    }

    if (
      addedVariants.length ||
      removedVariants.length ||
      reordered ||
      paramChanges.length
    ) {
      changes.push({
        id,
        type: "modified",
        addedVariants,
        removedVariants,
        reordered,
        paramChanges,
      });
    }
  }

  for (const [id, head] of headMap) {
    if (!baseMap.has(id)) {
      changes.push({ id, type: "added", base: null, head });
    }
  }

  return changes;
}

/**
 * Detect potential storage key collisions: different contracts using the
 * same string literal key in the same storage tier.
 */
function detectCollisions(headAccesses) {
  const keyMap = new Map(); // "tier::key" -> [contract, ...]
  for (const a of headAccesses) {
    if (a.keyKind !== "string_literal") continue;
    const composite = `${a.tier}::${a.key}`;
    if (!keyMap.has(composite)) keyMap.set(composite, new Set());
    keyMap.get(composite).add(a.contract);
  }
  const collisions = [];
  for (const [compositeKey, contracts] of keyMap) {
    if (contracts.size > 1) {
      const [tier, key] = compositeKey.split("::");
      collisions.push({
        tier,
        key,
        contracts: [...contracts],
      });
    }
  }
  return collisions;
}

// ── Breaking-change classification ───────────────────────────────────

function classifyBreaking(accessDiff, structChanges, enumChanges) {
  const breaking = [];

  // Removed storage accesses might indicate dropped state
  for (const a of accessDiff.removed) {
    if (a.operation === "set" || a.operation === "get") {
      breaking.push(
        `Removed ${a.tier} storage key \`${a.key}\` in contract \`${a.contract}\``
      );
    }
  }

  // Removed structs
  for (const c of structChanges) {
    if (c.type === "removed") {
      breaking.push(
        `Removed contracttype struct \`${c.id}\` — existing on-chain data will be unreadable`
      );
    }
    if (c.type === "modified" && c.fieldChanges) {
      for (const fc of c.fieldChanges) {
        if (fc.change === "removed") {
          breaking.push(
            `Removed field \`${fc.field}\` from \`${c.id}\` — deserialization of existing data will fail`
          );
        }
        if (fc.change === "reordered") {
          breaking.push(
            `Reordered fields in \`${c.id}\` — Soroban XDR serialization is order-dependent`
          );
        }
        if (fc.change === "type_changed") {
          breaking.push(
            `Type of field \`${fc.field}\` in \`${c.id}\` changed from \`${fc.oldType}\` to \`${fc.newType}\``
          );
        }
      }
    }
  }

  // Enum changes
  for (const c of enumChanges) {
    if (c.type === "removed") {
      breaking.push(
        `Removed contracttype enum \`${c.id}\` — storage keys referencing it will break`
      );
    }
    if (c.type === "modified") {
      if (c.removedVariants && c.removedVariants.length) {
        breaking.push(
          `Removed variants [${c.removedVariants.join(", ")}] from \`${c.id}\` — existing keys will be unresolvable`
        );
      }
      if (c.reordered) {
        breaking.push(
          `Reordered variants in \`${c.id}\` — Soroban enum discriminants are order-dependent`
        );
      }
      if (c.paramChanges && c.paramChanges.length) {
        for (const pc of c.paramChanges) {
          breaking.push(
            `Variant \`${pc.variant}\` in \`${c.id}\` changed params from \`${pc.oldParams}\` to \`${pc.newParams}\``
          );
        }
      }
    }
  }

  return breaking;
}

// ── Markdown report generation ───────────────────────────────────────

function generateReport({
  accessDiff,
  structChanges,
  enumChanges,
  collisions,
  breakingReasons,
}) {
  const lines = [];
  const hasBreaking = breakingReasons.length > 0;
  const hasChanges =
    accessDiff.added.length > 0 ||
    accessDiff.removed.length > 0 ||
    structChanges.length > 0 ||
    enumChanges.length > 0;

  lines.push("## Storage Impact Report");
  lines.push("");

  if (!hasChanges && collisions.length === 0) {
    lines.push(
      "> **No storage layout changes detected.** This PR does not modify contract storage."
    );
    return lines.join("\n");
  }

  if (hasBreaking) {
    lines.push(
      "> **:rotating_light: BREAKING CHANGES DETECTED** — This PR modifies the on-chain storage layout in ways that are incompatible with existing deployed state. A data migration strategy is required."
    );
  } else {
    lines.push(
      "> **:white_check_mark: No breaking changes.** Storage modifications are additive / safe."
    );
  }
  lines.push("");

  // ── Summary table ──
  lines.push("### Summary");
  lines.push("");
  lines.push("| Metric | Count |");
  lines.push("|--------|-------|");
  lines.push(`| New storage accesses | ${accessDiff.added.length} |`);
  lines.push(`| Removed storage accesses | ${accessDiff.removed.length} |`);
  lines.push(
    `| Modified contracttype structs | ${structChanges.filter((c) => c.type === "modified").length} |`
  );
  lines.push(
    `| Added contracttype structs | ${structChanges.filter((c) => c.type === "added").length} |`
  );
  lines.push(
    `| Removed contracttype structs | ${structChanges.filter((c) => c.type === "removed").length} |`
  );
  lines.push(
    `| Modified storage key enums | ${enumChanges.filter((c) => c.type === "modified").length} |`
  );
  lines.push(`| Storage key collisions | ${collisions.length} |`);
  lines.push(
    `| **Breaking changes** | **${breakingReasons.length}** |`
  );
  lines.push("");

  // ── Breaking details ──
  if (hasBreaking) {
    lines.push("### Breaking Changes");
    lines.push("");
    for (const reason of breakingReasons) {
      lines.push(`- :x: ${reason}`);
    }
    lines.push("");
  }

  // ── Storage access changes ──
  if (accessDiff.added.length > 0 || accessDiff.removed.length > 0) {
    lines.push("### Storage Access Changes");
    lines.push("");
    lines.push(
      "| Status | Contract | Tier | Key | Kind | File |"
    );
    lines.push(
      "|--------|----------|------|-----|------|------|"
    );
    for (const a of accessDiff.added) {
      lines.push(
        `| :heavy_plus_sign: Added | \`${a.contract}\` | ${a.tier} | \`${a.key}\` | ${a.keyKind} | \`${a.file}\` |`
      );
    }
    for (const a of accessDiff.removed) {
      lines.push(
        `| :heavy_minus_sign: Removed | \`${a.contract}\` | ${a.tier} | \`${a.key}\` | ${a.keyKind} | \`${a.file}\` |`
      );
    }
    lines.push("");
  }

  // ── Struct changes ──
  const modifiedStructs = structChanges.filter((c) => c.type === "modified");
  if (modifiedStructs.length > 0) {
    lines.push("### Contracttype Struct Changes");
    lines.push("");
    for (const c of modifiedStructs) {
      lines.push(`#### \`${c.id}\``);
      lines.push("");
      lines.push("| Change | Field | Details |");
      lines.push("|--------|-------|---------|");
      for (const fc of c.fieldChanges || []) {
        switch (fc.change) {
          case "added":
            lines.push(
              `| :heavy_plus_sign: Added | \`${fc.field}\` | type: \`${fc.newType}\` |`
            );
            break;
          case "removed":
            lines.push(
              `| :heavy_minus_sign: Removed | \`${fc.field}\` | was: \`${fc.oldType}\` |`
            );
            break;
          case "type_changed":
            lines.push(
              `| :arrows_counterclockwise: Type changed | \`${fc.field}\` | \`${fc.oldType}\` -> \`${fc.newType}\` |`
            );
            break;
          case "reordered":
            lines.push(
              `| :warning: Reordered | (all) | ${fc.from.join(", ")} -> ${fc.to.join(", ")} |`
            );
            break;
        }
      }
      lines.push("");
    }
  }

  // ── Enum changes ──
  const modifiedEnums = enumChanges.filter((c) => c.type === "modified");
  if (modifiedEnums.length > 0) {
    lines.push("### Storage Key Enum Changes");
    lines.push("");
    for (const c of modifiedEnums) {
      lines.push(`#### \`${c.id}\``);
      lines.push("");
      if (c.addedVariants && c.addedVariants.length) {
        lines.push(
          `- **Added variants:** ${c.addedVariants.map((v) => `\`${v}\``).join(", ")}`
        );
      }
      if (c.removedVariants && c.removedVariants.length) {
        lines.push(
          `- **Removed variants:** ${c.removedVariants.map((v) => `\`${v}\``).join(", ")}`
        );
      }
      if (c.reordered) {
        lines.push("- **Variants were reordered** (discriminant values changed)");
      }
      if (c.paramChanges && c.paramChanges.length) {
        for (const pc of c.paramChanges) {
          lines.push(
            `- Variant \`${pc.variant}\` params: \`${pc.oldParams}\` -> \`${pc.newParams}\``
          );
        }
      }
      lines.push("");
    }
  }

  // ── Collision warnings ──
  if (collisions.length > 0) {
    lines.push("### Storage Key Collision Risks");
    lines.push("");
    lines.push(
      "The following string-literal storage keys are used by **multiple contracts** in the same tier. If these contracts share a storage namespace (e.g., via cross-contract calls), data corruption may occur."
    );
    lines.push("");
    lines.push("| Tier | Key | Contracts |");
    lines.push("|------|-----|-----------|");
    for (const col of collisions) {
      lines.push(
        `| ${col.tier} | \`${col.key}\` | ${col.contracts.map((c) => `\`${c}\``).join(", ")} |`
      );
    }
    lines.push("");
  }

  // ── Added / removed structs ──
  const addedStructs = structChanges.filter((c) => c.type === "added");
  const removedStructs = structChanges.filter((c) => c.type === "removed");
  if (addedStructs.length > 0 || removedStructs.length > 0) {
    lines.push("### Contracttype Definitions Added / Removed");
    lines.push("");
    lines.push("| Status | Name | Contract | Fields / Variants |");
    lines.push("|--------|------|----------|-------------------|");
    for (const s of addedStructs) {
      const fields = (s.head.fields || []).map((f) => f.name).join(", ");
      lines.push(
        `| :heavy_plus_sign: Added | \`${s.head.name}\` | \`${s.head.contract}\` | ${fields} |`
      );
    }
    for (const s of removedStructs) {
      const fields = (s.base.fields || []).map((f) => f.name).join(", ");
      lines.push(
        `| :heavy_minus_sign: Removed | \`${s.base.name}\` | \`${s.base.contract}\` | ${fields} |`
      );
    }
    lines.push("");
  }

  // ── Added / removed enums ──
  const addedEnums = enumChanges.filter((c) => c.type === "added");
  const removedEnums = enumChanges.filter((c) => c.type === "removed");
  if (addedEnums.length > 0 || removedEnums.length > 0) {
    lines.push("### Storage Key Enums Added / Removed");
    lines.push("");
    lines.push("| Status | Name | Contract | Variants |");
    lines.push("|--------|------|----------|----------|");
    for (const e of addedEnums) {
      const vars = (e.head.variants || []).map((v) => v.name).join(", ");
      lines.push(
        `| :heavy_plus_sign: Added | \`${e.head.name}\` | \`${e.head.contract}\` | ${vars} |`
      );
    }
    for (const e of removedEnums) {
      const vars = (e.base.variants || []).map((v) => v.name).join(", ");
      lines.push(
        `| :heavy_minus_sign: Removed | \`${e.base.name}\` | \`${e.base.contract}\` | ${vars} |`
      );
    }
    lines.push("");
  }

  lines.push("---");
  lines.push(
    "*Generated by [storage-analyzer](../scripts/storage-analyzer/) — PR storage impact check*"
  );

  return lines.join("\n");
}

// ── Main ─────────────────────────────────────────────────────────────

function main() {
  const args = process.argv.slice(2);
  if (args.length < 2) {
    console.error("Usage: node compare.js <base.json> <head.json>");
    process.exit(2);
  }

  const [baseFile, headFile] = args;

  const baseLayout = loadJSON(baseFile);
  const headLayout = loadJSON(headFile);

  // Flatten for comparison
  const baseAccesses = flattenAccesses(baseLayout);
  const headAccesses = flattenAccesses(headLayout);

  const { structs: baseStructs, enums: baseEnums } = flattenTypes(baseLayout);
  const { structs: headStructs, enums: headEnums } = flattenTypes(headLayout);

  const baseKeyEnums = flattenKeyEnums(baseLayout);
  const headKeyEnums = flattenKeyEnums(headLayout);

  // Compute diffs
  const accessDiff = diffAccesses(baseAccesses, headAccesses);
  const structChanges = diffStructs(baseStructs, headStructs);
  const enumChanges = diffEnums(baseKeyEnums, headKeyEnums);
  const collisions = detectCollisions(headAccesses);
  const breakingReasons = classifyBreaking(
    accessDiff,
    structChanges,
    enumChanges
  );

  // Generate and print report
  const report = generateReport({
    accessDiff,
    structChanges,
    enumChanges,
    collisions,
    breakingReasons,
  });

  console.log(report);

  // Exit with 1 if breaking changes found
  if (breakingReasons.length > 0) {
    process.exit(1);
  }
}

main();
