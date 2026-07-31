#!/usr/bin/env node
// =============================================================================
// scripts/abi-extractor/generate-types.js
//
// Reads extracted ABI JSON files and generates:
//   1. TypeScript type definitions for each contract
//   2. Rust client type stubs
//   3. A barrel index.ts re-exporting everything
//
// Usage:
//   node scripts/abi-extractor/generate-types.js [OPTIONS]
//
// Options:
//   --abi-dir <dir>    Directory containing ABI JSON files
//                      (default: packages/contract-abis/abi)
//   --out-dir <dir>    Output directory for generated types
//                      (default: packages/contract-abis/src)
//   --contract <name>  Generate types for a specific contract only
//   --rust             Also generate Rust client types
//   --help             Show help
// =============================================================================

const fs = require("fs");
const path = require("path");

// ---------------------------------------------------------------------------
// Resolve paths
// ---------------------------------------------------------------------------
const REPO_ROOT = path.resolve(__dirname, "../..");
const DEFAULT_ABI_DIR = path.join(REPO_ROOT, "packages/contract-abis/abi");
const DEFAULT_OUT_DIR = path.join(REPO_ROOT, "packages/contract-abis/src");

// ---------------------------------------------------------------------------
// Argument parsing
// ---------------------------------------------------------------------------
const args = process.argv.slice(2);
let abiDir = DEFAULT_ABI_DIR;
let outDir = DEFAULT_OUT_DIR;
let specificContract = null;
let generateRust = false;

for (let i = 0; i < args.length; i++) {
  switch (args[i]) {
    case "--abi-dir":
      abiDir = path.resolve(args[++i]);
      break;
    case "--out-dir":
      outDir = path.resolve(args[++i]);
      break;
    case "--contract":
      specificContract = args[++i];
      break;
    case "--rust":
      generateRust = true;
      break;
    case "--help":
      console.log(`Usage: node generate-types.js [OPTIONS]
Options:
  --abi-dir <dir>    ABI JSON directory (default: packages/contract-abis/abi)
  --out-dir <dir>    Output directory (default: packages/contract-abis/src)
  --contract <name>  Specific contract to generate
  --rust             Also generate Rust client types
  --help             Show this help`);
      process.exit(0);
    default:
      console.error(`Unknown argument: ${args[i]}`);
      process.exit(1);
  }
}

// ---------------------------------------------------------------------------
// Soroban type mapping
// ---------------------------------------------------------------------------

/** Map a Soroban XDR / spec type to a TypeScript type string. */
function sorobanTypeToTS(type) {
  if (typeof type === "string") {
    const PRIMITIVE_MAP = {
      bool: "boolean",
      i32: "number",
      u32: "number",
      i64: "bigint",
      u64: "bigint",
      i128: "bigint",
      u128: "bigint",
      i256: "bigint",
      u256: "bigint",
      symbol: "string",
      string: "string",
      bytes: "Buffer",
      address: "string",
      void: "void",
      val: "unknown",
      timepoint: "bigint",
      duration: "bigint",
    };
    return PRIMITIVE_MAP[type.toLowerCase()] || "unknown";
  }
  if (typeof type === "object" && type !== null) {
    if (type.vec) return `Array<${sorobanTypeToTS(type.vec.element_type || "unknown")}>`;
    if (type.map)
      return `Map<${sorobanTypeToTS(type.map.key_type || "unknown")}, ${sorobanTypeToTS(type.map.value_type || "unknown")}>`;
    if (type.option) return `${sorobanTypeToTS(type.option.value_type || "unknown")} | null`;
    if (type.result)
      return `Result<${sorobanTypeToTS(type.result.ok_type || "void")}, ${sorobanTypeToTS(type.result.err_type || "unknown")}>`;
    if (type.tuple) {
      const inner = (type.tuple.value_types || []).map(sorobanTypeToTS).join(", ");
      return `[${inner}]`;
    }
    if (type.bytes_n) return "Buffer";
    if (type.type) return sorobanTypeToTS(type.type);
    if (type.name) return toPascalCase(type.name);
  }
  return "unknown";
}

/** Map a Soroban type to a Rust type string. */
function sorobanTypeToRust(type) {
  if (typeof type === "string") {
    const PRIMITIVE_MAP = {
      bool: "bool",
      i32: "i32",
      u32: "u32",
      i64: "i64",
      u64: "u64",
      i128: "i128",
      u128: "u128",
      i256: "soroban_sdk::I256",
      u256: "soroban_sdk::U256",
      symbol: "soroban_sdk::Symbol",
      string: "soroban_sdk::String",
      bytes: "soroban_sdk::Bytes",
      address: "soroban_sdk::Address",
      void: "()",
      val: "soroban_sdk::Val",
      timepoint: "u64",
      duration: "u64",
    };
    return PRIMITIVE_MAP[type.toLowerCase()] || type;
  }
  if (typeof type === "object" && type !== null) {
    if (type.vec) return `soroban_sdk::Vec<${sorobanTypeToRust(type.vec.element_type || "Val")}>`;
    if (type.map)
      return `soroban_sdk::Map<${sorobanTypeToRust(type.map.key_type || "Val")}, ${sorobanTypeToRust(type.map.value_type || "Val")}>`;
    if (type.option) return `Option<${sorobanTypeToRust(type.option.value_type || "()")}>`;
    if (type.result)
      return `Result<${sorobanTypeToRust(type.result.ok_type || "()")}, ${sorobanTypeToRust(type.result.err_type || "Error")}>`;
    if (type.bytes_n) return `soroban_sdk::BytesN<${type.bytes_n.n || 32}>`;
    if (type.type) return sorobanTypeToRust(type.type);
    if (type.name) return toPascalCase(type.name);
  }
  return "soroban_sdk::Val";
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function toPascalCase(str) {
  return str
    .split(/[-_]/)
    .map((s) => s.charAt(0).toUpperCase() + s.slice(1).toLowerCase())
    .join("");
}

function toCamelCase(str) {
  const pascal = toPascalCase(str);
  return pascal.charAt(0).toLowerCase() + pascal.slice(1);
}

function toSnakeCase(str) {
  return str.replace(/-/g, "_").toLowerCase();
}

function banner() {
  return `// =============================================================================
// AUTO-GENERATED by scripts/abi-extractor/generate-types.js
// DO NOT EDIT MANUALLY — re-run the ABI extractor to regenerate.
// Generated at: ${new Date().toISOString()}
// =============================================================================`;
}

// ---------------------------------------------------------------------------
// Type extraction from ABI spec
// ---------------------------------------------------------------------------

/**
 * Parse a Soroban contract spec into structured data:
 *   { functions: [...], structs: [...], enums: [...], errors: [...] }
 */
function parseSpec(spec) {
  const result = {
    functions: [],
    structs: [],
    enums: [],
    errors: [],
  };

  if (!spec || !Array.isArray(spec)) return result;

  for (const entry of spec) {
    if (!entry) continue;

    // Function entries
    if (entry.type === "function" || entry.function) {
      const fn = entry.function || entry;
      result.functions.push({
        name: fn.name || "unknown",
        doc: fn.doc || "",
        inputs: (fn.inputs || fn.args || []).map((inp) => ({
          name: inp.name || inp.arg_name || "arg",
          type: inp.type || inp.value || "unknown",
        })),
        outputs: fn.output || fn.outputs || fn.return_type || "void",
      });
    }

    // Struct entries
    if (entry.type === "struct" || entry.struct) {
      const st = entry.struct || entry;
      result.structs.push({
        name: st.name || "UnnamedStruct",
        doc: st.doc || "",
        fields: (st.fields || []).map((f) => ({
          name: f.name || f.field_name || "field",
          type: f.type || f.value || "unknown",
          doc: f.doc || "",
        })),
      });
    }

    // Enum entries
    if (entry.type === "union" || entry.union || entry.type === "enum" || entry.enum) {
      const en = entry.union || entry.enum || entry;
      result.enums.push({
        name: en.name || "UnnamedEnum",
        doc: en.doc || "",
        cases: (en.cases || en.variants || []).map((c) => ({
          name: c.name || c.variant_name || "Unknown",
          value: c.value,
          type: c.type || null,
        })),
      });
    }

    // Error enum entries
    if (entry.type === "error_enum" || entry.error_enum) {
      const err = entry.error_enum || entry;
      result.errors.push({
        name: err.name || "ContractError",
        cases: (err.cases || []).map((c) => ({
          name: c.name || "Unknown",
          value: c.value,
          doc: c.doc || "",
        })),
      });
    }
  }

  return result;
}

// ---------------------------------------------------------------------------
// TypeScript generation
// ---------------------------------------------------------------------------

function generateTypeScript(contractName, abi) {
  const parsed = parseSpec(abi.spec);
  const pascal = toPascalCase(contractName);
  const lines = [banner(), ""];

  // Result helper type
  lines.push(`/** Generic Result type for contract calls */`);
  lines.push(`export type Result<T, E> = { ok: true; value: T } | { ok: false; error: E };`);
  lines.push("");

  // Contract metadata
  lines.push(`/** Contract metadata */`);
  lines.push(`export const ${pascal}Metadata = {`);
  lines.push(`  contractName: "${contractName}",`);
  lines.push(`  wasmHash: "${abi.wasm_hash || ""}",`);
  lines.push(`  version: "${abi.version || ""}",`);
  lines.push(`  extractedAt: "${abi.extracted_at || ""}",`);
  lines.push(`} as const;`);
  lines.push("");

  // Struct types
  for (const struct of parsed.structs) {
    if (struct.doc) lines.push(`/** ${struct.doc} */`);
    lines.push(`export interface ${toPascalCase(struct.name)} {`);
    for (const field of struct.fields) {
      if (field.doc) lines.push(`  /** ${field.doc} */`);
      lines.push(`  ${toCamelCase(field.name)}: ${sorobanTypeToTS(field.type)};`);
    }
    lines.push(`}`);
    lines.push("");
  }

  // Enum types
  for (const en of parsed.enums) {
    if (en.doc) lines.push(`/** ${en.doc} */`);
    if (en.cases.every((c) => c.type === null || c.type === undefined)) {
      // Simple enum (no associated data)
      lines.push(`export enum ${toPascalCase(en.name)} {`);
      for (const c of en.cases) {
        const val = c.value !== undefined ? ` = ${c.value}` : "";
        lines.push(`  ${toPascalCase(c.name)}${val},`);
      }
      lines.push(`}`);
    } else {
      // Tagged union
      lines.push(`export type ${toPascalCase(en.name)} =`);
      for (let i = 0; i < en.cases.length; i++) {
        const c = en.cases[i];
        const sep = i < en.cases.length - 1 ? " |" : ";";
        if (c.type) {
          lines.push(`  | { tag: "${c.name}"; value: ${sorobanTypeToTS(c.type)} }${sep}`);
        } else {
          lines.push(`  | { tag: "${c.name}" }${sep}`);
        }
      }
    }
    lines.push("");
  }

  // Error enums
  for (const err of parsed.errors) {
    lines.push(`/** Contract error codes */`);
    lines.push(`export enum ${toPascalCase(err.name)} {`);
    for (const c of err.cases) {
      if (c.doc) lines.push(`  /** ${c.doc} */`);
      const val = c.value !== undefined ? ` = ${c.value}` : "";
      lines.push(`  ${toPascalCase(c.name)}${val},`);
    }
    lines.push(`}`);
    lines.push("");
  }

  // Contract client interface
  lines.push(`/** Type-safe client interface for the ${contractName} contract */`);
  lines.push(`export interface ${pascal}Client {`);
  for (const fn of parsed.functions) {
    if (fn.doc) lines.push(`  /** ${fn.doc} */`);
    const params = fn.inputs
      .map((inp) => `${toCamelCase(inp.name)}: ${sorobanTypeToTS(inp.type)}`)
      .join(", ");
    const returnType = sorobanTypeToTS(fn.outputs);
    lines.push(`  ${toCamelCase(fn.name)}(${params}): Promise<${returnType}>;`);
  }
  lines.push(`}`);
  lines.push("");

  // Function name constants (useful for invoke calls)
  if (parsed.functions.length > 0) {
    lines.push(`/** Contract function names */`);
    lines.push(`export const ${pascal}Functions = {`);
    for (const fn of parsed.functions) {
      lines.push(`  ${toCamelCase(fn.name)}: "${fn.name}",`);
    }
    lines.push(`} as const;`);
    lines.push("");
  }

  return lines.join("\n");
}

// ---------------------------------------------------------------------------
// Rust client generation
// ---------------------------------------------------------------------------

function generateRustClient(contractName, abi) {
  const parsed = parseSpec(abi.spec);
  const pascal = toPascalCase(contractName);
  const snake = toSnakeCase(contractName);
  const lines = [
    `// ${banner().replace(/\/\/ /g, "")}`,
    "",
    `//! Auto-generated client types for the \`${contractName}\` contract.`,
    "",
    "#![allow(dead_code)]",
    "#![allow(unused_imports)]",
    "",
    "use soroban_sdk::{Address, Env, BytesN, contracttype, contracterror};",
    "",
  ];

  // Struct definitions
  for (const struct of parsed.structs) {
    if (struct.doc) lines.push(`/// ${struct.doc}`);
    lines.push(`#[contracttype]`);
    lines.push(`#[derive(Clone, Debug, Eq, PartialEq)]`);
    lines.push(`pub struct ${toPascalCase(struct.name)} {`);
    for (const field of struct.fields) {
      if (field.doc) lines.push(`    /// ${field.doc}`);
      lines.push(`    pub ${toSnakeCase(field.name)}: ${sorobanTypeToRust(field.type)},`);
    }
    lines.push(`}`);
    lines.push("");
  }

  // Enum definitions
  for (const en of parsed.enums) {
    if (en.doc) lines.push(`/// ${en.doc}`);
    lines.push(`#[contracttype]`);
    lines.push(`#[derive(Clone, Debug, Eq, PartialEq)]`);
    lines.push(`pub enum ${toPascalCase(en.name)} {`);
    for (const c of en.cases) {
      if (c.type) {
        lines.push(`    ${toPascalCase(c.name)}(${sorobanTypeToRust(c.type)}),`);
      } else {
        lines.push(`    ${toPascalCase(c.name)},`);
      }
    }
    lines.push(`}`);
    lines.push("");
  }

  // Error enum definitions
  for (const err of parsed.errors) {
    lines.push(`#[contracterror]`);
    lines.push(`#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]`);
    lines.push(`#[repr(u32)]`);
    lines.push(`pub enum ${toPascalCase(err.name)} {`);
    for (const c of err.cases) {
      if (c.doc) lines.push(`    /// ${c.doc}`);
      const val = c.value !== undefined ? ` = ${c.value}` : "";
      lines.push(`    ${toPascalCase(c.name)}${val},`);
    }
    lines.push(`}`);
    lines.push("");
  }

  // Client trait
  lines.push(`/// Type-safe client trait for the \`${contractName}\` contract.`);
  lines.push(`pub trait ${pascal}ClientTrait {`);
  for (const fn of parsed.functions) {
    if (fn.doc) lines.push(`    /// ${fn.doc}`);
    const params = fn.inputs
      .map((inp) => `${toSnakeCase(inp.name)}: ${sorobanTypeToRust(inp.type)}`)
      .join(", ");
    const allParams = params ? `env: &Env, ${params}` : `env: &Env`;
    const returnType = sorobanTypeToRust(fn.outputs);
    lines.push(`    fn ${toSnakeCase(fn.name)}(${allParams}) -> ${returnType};`);
  }
  lines.push(`}`);
  lines.push("");

  return lines.join("\n");
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

function main() {
  console.log("======================================================================");
  console.log(" StellarLend Type Generator");
  console.log("======================================================================");
  console.log("");

  // Validate ABI directory
  if (!fs.existsSync(abiDir)) {
    console.error(`ERROR: ABI directory not found: ${abiDir}`);
    console.error("Run scripts/abi-extractor/extract.sh first.");
    process.exit(1);
  }

  // Find ABI files
  let abiFiles = fs
    .readdirSync(abiDir)
    .filter((f) => f.endsWith(".json"))
    .sort();

  if (specificContract) {
    abiFiles = abiFiles.filter((f) => f === `${specificContract}.json`);
    if (abiFiles.length === 0) {
      console.error(`ERROR: No ABI file found for contract '${specificContract}'`);
      process.exit(1);
    }
  }

  console.log(`Found ${abiFiles.length} ABI file(s) in ${abiDir}`);
  console.log("");

  // Ensure output directories
  const typesDir = path.join(outDir, "types");
  fs.mkdirSync(typesDir, { recursive: true });

  if (generateRust) {
    const rustDir = path.join(outDir, "rust");
    fs.mkdirSync(rustDir, { recursive: true });
  }

  // Process each ABI file
  const generatedContracts = [];

  for (const abiFile of abiFiles) {
    const contractName = path.basename(abiFile, ".json");
    console.log(`  Generating types for: ${contractName}`);

    let abi;
    try {
      const raw = fs.readFileSync(path.join(abiDir, abiFile), "utf-8");
      abi = JSON.parse(raw);
    } catch (err) {
      console.error(`    ERROR: Failed to parse ${abiFile}: ${err.message}`);
      continue;
    }

    // Generate TypeScript
    const tsContent = generateTypeScript(contractName, abi);
    const tsFile = path.join(typesDir, `${contractName}.ts`);
    fs.writeFileSync(tsFile, tsContent, "utf-8");
    console.log(`    -> ${path.relative(REPO_ROOT, tsFile)}`);

    // Generate Rust (if requested)
    if (generateRust) {
      const rustContent = generateRustClient(contractName, abi);
      const rustFile = path.join(outDir, "rust", `${toSnakeCase(contractName)}.rs`);
      fs.writeFileSync(rustFile, rustContent, "utf-8");
      console.log(`    -> ${path.relative(REPO_ROOT, rustFile)}`);
    }

    generatedContracts.push(contractName);
  }

  // Generate barrel index.ts
  if (generatedContracts.length > 0) {
    const indexLines = [banner(), ""];
    for (const name of generatedContracts) {
      indexLines.push(`export * from "./types/${name}";`);
    }
    indexLines.push("");
    const indexFile = path.join(outDir, "index.ts");
    fs.writeFileSync(indexFile, indexLines.join("\n"), "utf-8");
    console.log("");
    console.log(`  Generated barrel: ${path.relative(REPO_ROOT, indexFile)}`);

    // Generate Rust mod.rs if Rust was requested
    if (generateRust) {
      const modLines = [`// ${banner().replace(/\/\/ /g, "")}`, ""];
      for (const name of generatedContracts) {
        modLines.push(`pub mod ${toSnakeCase(name)};`);
      }
      modLines.push("");
      const modFile = path.join(outDir, "rust", "mod.rs");
      fs.writeFileSync(modFile, modLines.join("\n"), "utf-8");
      console.log(`  Generated Rust mod: ${path.relative(REPO_ROOT, modFile)}`);
    }
  }

  console.log("");
  console.log("======================================================================");
  console.log(` Type generation complete. ${generatedContracts.length} contract(s) processed.`);
  console.log("======================================================================");
}

main();
