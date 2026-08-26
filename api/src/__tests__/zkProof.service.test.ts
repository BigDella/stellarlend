import { verifyTransferProof } from '../utils/zkProof';
import { ZkProofService } from '../services/zkProof.service';

describe('ZkProofService', () => {
  let service: ZkProofService;

  beforeEach(() => {
    service = new ZkProofService();
  });

  it('creates commitments that can be reopened with the original amount', () => {
    const { commitment, nonce } = service.commit({ amount: '2500' });

    expect(commitment.nonce).toBe(nonce);
    expect(service.openCommitment(commitment, '2500')).toBe(true);
    expect(service.openCommitment(commitment, '2499')).toBe(false);
  });

  it('rejects negative commitments and invalid range bounds', () => {
    expect(() => service.commit({ amount: '-1' })).toThrow('Amount must be non-negative');
    expect(() =>
      service.rangeProof({
        amount: '10',
        min: '50',
        max: '40',
        nonce: 'nonce',
      })
    ).toThrow('min must be <= max');
  });

  it('generates and verifies range proofs', () => {
    const { nonce } = service.commit({ amount: '125' });
    const proof = service.rangeProof({
      amount: '125',
      min: '100',
      max: '200',
      nonce,
    });

    expect(service.verifyRange({ proof, nonce, amount: '125' })).toEqual({ valid: true });
    expect(service.verifyRange({ proof, nonce, amount: '201' })).toEqual({ valid: false });
  });

  it('generates transfer proofs that satisfy the balance invariant', () => {
    const proof = service.transferProof({
      senderAmount: '110',
      recipientAmount: '100',
      fee: '10',
    });

    expect(
      verifyTransferProof(
        proof,
        BigInt(110),
        BigInt(100),
        BigInt(10),
        proof.senderCommitment.nonce,
        proof.recipientCommitment.nonce
      )
    ).toBe(true);
  });

  it('rejects invalid transfer inputs', () => {
    expect(() =>
      service.transferProof({
        senderAmount: '0',
        recipientAmount: '0',
        fee: '0',
      })
    ).toThrow('Sender amount must be positive');

    expect(() =>
      service.transferProof({
        senderAmount: '100',
        recipientAmount: '95',
        fee: '4',
      })
    ).toThrow('Balance invariant violated: sender must equal recipient + fee');
  });
});
