import { createHash } from 'crypto';

function sha256(data: string): string {
  return createHash('sha256').update(data).digest('hex');
}

function hashPair(left: string, right: string): string {
  // Canonical ordering ensures the same pair always produces the same hash
  const [a, b] = left <= right ? [left, right] : [right, left];
  return sha256(a + b);
}

export interface MerkleProof {
  leaf: string;
  index: number;
  siblings: Array<{ hash: string; position: 'left' | 'right' }>;
  root: string;
}

export class MerkleTree {
  private readonly leaves: string[];
  private readonly tree: string[][];

  constructor(leaves: string[]) {
    if (leaves.length === 0) {
      throw new Error('MerkleTree requires at least one leaf');
    }
    this.leaves = leaves.map((l) => sha256(l));
    this.tree = this.build(this.leaves);
  }

  get root(): string {
    const topLevel = this.tree[this.tree.length - 1];
    if (!topLevel) throw new Error('Empty tree');
    const root = topLevel[0];
    if (!root) throw new Error('Empty root level');
    return root;
  }

  get depth(): number {
    return this.tree.length - 1;
  }

  getProof(index: number): MerkleProof {
    if (index < 0 || index >= this.leaves.length) {
      throw new RangeError(`Leaf index ${index} out of bounds`);
    }

    const siblings: MerkleProof['siblings'] = [];
    let currentIndex = index;

    for (let level = 0; level < this.tree.length - 1; level++) {
      const levelNodes = this.tree[level];
      const siblingIndex = currentIndex % 2 === 0 ? currentIndex + 1 : currentIndex - 1;

      if (levelNodes && siblingIndex < levelNodes.length) {
        const siblingHash = levelNodes[siblingIndex];
        if (siblingHash !== undefined) {
          siblings.push({
            hash: siblingHash,
            position: currentIndex % 2 === 0 ? 'right' : 'left',
          });
        }
      }

      currentIndex = Math.floor(currentIndex / 2);
    }

    const leaf = this.leaves[index];
    if (!leaf) throw new Error('Leaf not found');

    return {
      leaf,
      index,
      siblings,
      root: this.root,
    };
  }

  verify(proof: MerkleProof): boolean {
    return MerkleTree.verify(proof);
  }

  static verify(proof: MerkleProof): boolean {
    let current = proof.leaf;

    for (const sibling of proof.siblings) {
      if (sibling.position === 'left') {
        current = hashPair(sibling.hash, current);
      } else {
        current = hashPair(current, sibling.hash);
      }
    }

    return current === proof.root;
  }

  private build(leaves: string[]): string[][] {
    const levels: string[][] = [leaves.slice()];

    let current = leaves.slice();
    while (current.length > 1) {
      const next: string[] = [];
      for (let i = 0; i < current.length; i += 2) {
        const left = current[i]!;
        const right = i + 1 < current.length ? current[i + 1]! : left;
        next.push(hashPair(left, right));
      }
      levels.push(next);
      current = next;
    }

    return levels;
  }
}

export function hashAccountState(
  userAddress: string,
  collateral: string,
  debt: string,
  timestamp: number
): string {
  return `${userAddress}:${collateral}:${debt}:${timestamp}`;
}
