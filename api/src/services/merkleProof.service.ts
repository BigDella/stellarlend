import { MerkleTree, MerkleProof, hashAccountState } from '../utils/merkleTree';
import logger from '../utils/logger';

export interface AccountState {
  userAddress: string;
  collateral: string;
  debt: string;
  lastUpdated: number;
}

interface TreeSnapshot {
  root: string;
  depth: number;
  leafCount: number;
  createdAt: string;
}

// In-memory account registry. In production back this with the lending DB.
const accountRegistry = new Map<string, AccountState>();
let currentTree: MerkleTree | null = null;
let treeSnapshot: TreeSnapshot | null = null;

function rebuildTree(): void {
  const accounts = Array.from(accountRegistry.values());
  if (accounts.length === 0) {
    currentTree = null;
    treeSnapshot = null;
    return;
  }

  const leaves = accounts.map((a) =>
    hashAccountState(a.userAddress, a.collateral, a.debt, a.lastUpdated)
  );
  currentTree = new MerkleTree(leaves);
  treeSnapshot = {
    root: currentTree.root,
    depth: currentTree.depth,
    leafCount: accounts.length,
    createdAt: new Date().toISOString(),
  };
  logger.info('Merkle tree rebuilt', { root: treeSnapshot.root, leaves: leaves.length });
}

export class MerkleProofService {
  upsertAccount(state: AccountState): TreeSnapshot {
    accountRegistry.set(state.userAddress, {
      ...state,
      lastUpdated: state.lastUpdated ?? Date.now(),
    });
    rebuildTree();
    return treeSnapshot!;
  }

  generateProof(userAddress: string): MerkleProof {
    if (!currentTree) {
      throw Object.assign(new Error('No account tree available'), { status: 503 });
    }

    const accounts = Array.from(accountRegistry.values());
    const index = accounts.findIndex((a) => a.userAddress === userAddress);
    if (index === -1) {
      throw Object.assign(new Error('Account not found in registry'), { status: 404 });
    }

    return currentTree.getProof(index);
  }

  verifyProof(proof: MerkleProof): { valid: boolean; root: string } {
    const valid = MerkleTree.verify(proof);
    return { valid, root: proof.root };
  }

  getTreeInfo(): TreeSnapshot {
    if (!treeSnapshot) {
      throw Object.assign(new Error('No account tree available'), { status: 503 });
    }
    return treeSnapshot;
  }

  getAccount(userAddress: string): AccountState {
    const account = accountRegistry.get(userAddress);
    if (!account) {
      throw Object.assign(new Error('Account not found'), { status: 404 });
    }
    return account;
  }

  listAccounts(): AccountState[] {
    return Array.from(accountRegistry.values());
  }
}

export const merkleProofService = new MerkleProofService();

export function resetMerkleProofStore(): void {
  accountRegistry.clear();
  currentTree = null;
  treeSnapshot = null;
}
