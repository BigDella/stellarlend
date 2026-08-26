import { MerkleProofService, resetMerkleProofStore } from '../services/merkleProof.service';

describe('MerkleProofService', () => {
  beforeEach(() => {
    resetMerkleProofStore();
  });

  it('returns a 503-style error when no tree exists yet', () => {
    const service = new MerkleProofService();

    expect(() => service.getTreeInfo()).toThrow('No account tree available');
    expect(() => service.generateProof('missing')).toThrow('No account tree available');
  });

  it('builds proofs, verifies them, and exposes account snapshots', () => {
    const service = new MerkleProofService();

    const first = service.upsertAccount({
      userAddress: 'GA123FIRST',
      collateral: '1000',
      debt: '250',
      lastUpdated: 1_710_000_000,
    });
    const second = service.upsertAccount({
      userAddress: 'GA456SECOND',
      collateral: '500',
      debt: '100',
      lastUpdated: 1_710_000_100,
    });

    expect(first.leafCount).toBe(1);
    expect(second.leafCount).toBe(2);
    expect(service.getTreeInfo()).toMatchObject({
      root: second.root,
      leafCount: 2,
    });

    const proof = service.generateProof('GA123FIRST');
    expect(proof.root).toBe(second.root);
    expect(proof.siblings.length).toBeGreaterThan(0);
    expect(service.verifyProof(proof)).toEqual({ valid: true, root: second.root });

    expect(service.getAccount('GA456SECOND')).toMatchObject({
      collateral: '500',
      debt: '100',
    });
    expect(service.listAccounts()).toHaveLength(2);
  });

  it('rejects missing accounts and detects tampered proofs', () => {
    const service = new MerkleProofService();

    service.upsertAccount({
      userAddress: 'GA789THIRD',
      collateral: '42',
      debt: '0',
      lastUpdated: 1_710_000_200,
    });

    expect(() => service.generateProof('UNKNOWN')).toThrow('Account not found in registry');
    expect(() => service.getAccount('UNKNOWN')).toThrow('Account not found');

    const proof = service.generateProof('GA789THIRD');
    const tampered = {
      ...proof,
      leaf: proof.leaf.replace(/^./, proof.leaf[0] === 'a' ? 'b' : 'a'),
    };

    expect(service.verifyProof(tampered).valid).toBe(false);
  });
});
