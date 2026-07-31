/**
 * config.ts — ~/.sllend/config.toml management and encrypted keystore.
 */
import * as fs from "fs";
import * as os from "os";
import * as path from "path";
import * as crypto from "crypto";
import TOML from "toml";

// ── Types ─────────────────────────────────────────────────────────────────────

export interface NetworkConfig {
  rpcUrl: string;
  horizonUrl: string;
  networkPassphrase: string;
  contractId?: string;
}

export interface SllendConfig {
  defaultNetwork: string;
  networks: Record<string, NetworkConfig>;
  /** Active keystore alias */
  defaultKey?: string;
}

export interface KeystoreEntry {
  alias: string;
  /** AES-256-GCM encrypted Stellar secret key, hex-encoded */
  encryptedSecret: string;
  /** Hex-encoded IV */
  iv: string;
  /** Hex-encoded auth tag */
  tag: string;
  /** Stellar public key */
  publicKey: string;
}

// ── Built-in network presets ──────────────────────────────────────────────────

const NETWORK_PRESETS: Record<string, NetworkConfig> = {
  mainnet: {
    rpcUrl: "https://soroban-rpc.mainnet.stellar.org",
    horizonUrl: "https://horizon.stellar.org",
    networkPassphrase: "Public Global Stellar Network ; September 2015",
  },
  testnet: {
    rpcUrl: "https://soroban-testnet.stellar.org",
    horizonUrl: "https://horizon-testnet.stellar.org",
    networkPassphrase: "Test SDF Network ; September 2015",
  },
  local: {
    rpcUrl: "http://localhost:8000/soroban/rpc",
    horizonUrl: "http://localhost:8000",
    networkPassphrase: "Standalone Network ; February 2017",
  },
};

const DEFAULT_CONFIG: SllendConfig = {
  defaultNetwork: "testnet",
  networks: NETWORK_PRESETS,
};

// ── Paths ─────────────────────────────────────────────────────────────────────

export const CONFIG_DIR  = path.join(os.homedir(), ".sllend");
export const CONFIG_FILE = path.join(CONFIG_DIR, "config.toml");
export const KEYSTORE_FILE = path.join(CONFIG_DIR, "keystore.json");

// ── Config I/O ────────────────────────────────────────────────────────────────

export function loadConfig(): SllendConfig {
  if (!fs.existsSync(CONFIG_FILE)) return DEFAULT_CONFIG;
  const raw = fs.readFileSync(CONFIG_FILE, "utf8");
  const parsed = TOML.parse(raw) as Partial<SllendConfig>;
  return {
    defaultNetwork: parsed.defaultNetwork ?? DEFAULT_CONFIG.defaultNetwork,
    networks: { ...NETWORK_PRESETS, ...(parsed.networks ?? {}) },
    defaultKey: parsed.defaultKey,
  };
}

export function saveConfig(cfg: SllendConfig): void {
  fs.mkdirSync(CONFIG_DIR, { recursive: true, mode: 0o700 });
  const lines = [
    `defaultNetwork = "${cfg.defaultNetwork}"`,
    cfg.defaultKey ? `defaultKey = "${cfg.defaultKey}"` : "",
    "",
    ...Object.entries(cfg.networks)
      .filter(([name]) => !NETWORK_PRESETS[name] || cfg.networks[name] !== NETWORK_PRESETS[name])
      .flatMap(([name, net]) => [
        `[networks.${name}]`,
        `rpcUrl = "${net.rpcUrl}"`,
        `horizonUrl = "${net.horizonUrl}"`,
        `networkPassphrase = "${net.networkPassphrase}"`,
        net.contractId ? `contractId = "${net.contractId}"` : "",
        "",
      ]),
  ];
  fs.writeFileSync(CONFIG_FILE, lines.filter((l) => l !== "").join("\n") + "\n", { mode: 0o600 });
}

export function getNetwork(cfg: SllendConfig, networkName?: string): NetworkConfig & { name: string } {
  const name = networkName ?? cfg.defaultNetwork;
  const net = cfg.networks[name];
  if (!net) throw new Error(`Unknown network "${name}". Available: ${Object.keys(cfg.networks).join(", ")}`);
  return { ...net, name };
}

// ── Keystore ──────────────────────────────────────────────────────────────────

function loadKeystore(): KeystoreEntry[] {
  if (!fs.existsSync(KEYSTORE_FILE)) return [];
  return JSON.parse(fs.readFileSync(KEYSTORE_FILE, "utf8")) as KeystoreEntry[];
}

function saveKeystore(entries: KeystoreEntry[]): void {
  fs.mkdirSync(CONFIG_DIR, { recursive: true, mode: 0o700 });
  fs.writeFileSync(KEYSTORE_FILE, JSON.stringify(entries, null, 2), { mode: 0o600 });
}

/** Derive a 32-byte key from a password using scrypt. */
function deriveKey(password: string, salt: Buffer): Buffer {
  return crypto.scryptSync(password, salt, 32, { N: 32768, r: 8, p: 1 });
}

export function addKey(alias: string, secretKey: string, password: string, publicKey: string): void {
  const entries = loadKeystore();
  if (entries.find((e) => e.alias === alias)) {
    throw new Error(`Key alias "${alias}" already exists. Remove it first.`);
  }
  const salt = crypto.randomBytes(16);
  const iv   = crypto.randomBytes(12);
  const key  = deriveKey(password, salt);
  const cipher = crypto.createCipheriv("aes-256-gcm", key, iv);
  const encrypted = Buffer.concat([cipher.update(secretKey, "utf8"), cipher.final()]);
  const tag = cipher.getAuthTag();
  entries.push({
    alias,
    encryptedSecret: salt.toString("hex") + ":" + encrypted.toString("hex"),
    iv: iv.toString("hex"),
    tag: tag.toString("hex"),
    publicKey,
  });
  saveKeystore(entries);
}

export function unlockKey(alias: string, password: string): string {
  const entries = loadKeystore();
  const entry = entries.find((e) => e.alias === alias);
  if (!entry) throw new Error(`Key alias "${alias}" not found. Add it with: sllend keys add`);
  const [saltHex, encHex] = entry.encryptedSecret.split(":");
  const salt = Buffer.from(saltHex, "hex");
  const enc  = Buffer.from(encHex, "hex");
  const iv   = Buffer.from(entry.iv, "hex");
  const tag  = Buffer.from(entry.tag, "hex");
  const key  = deriveKey(password, salt);
  try {
    const decipher = crypto.createDecipheriv("aes-256-gcm", key, iv);
    decipher.setAuthTag(tag);
    return decipher.update(enc) + decipher.final("utf8");
  } catch {
    throw new Error("Incorrect password or corrupted keystore entry.");
  }
}

export function listKeys(): Array<{ alias: string; publicKey: string }> {
  return loadKeystore().map(({ alias, publicKey }) => ({ alias, publicKey }));
}

export function removeKey(alias: string): void {
  const entries = loadKeystore().filter((e) => e.alias !== alias);
  saveKeystore(entries);
}
