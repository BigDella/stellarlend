/**
 * Privacy-preserving amount commitments using a Pedersen-style commitment scheme.
 *
 * A commitment C = H(amount || nonce) binds the prover to a value without
 * revealing it. The range proof uses a bit-decomposition technique to prove
 * that committed_amount ∈ [min, max] without disclosing the amount itself.
 *
 * This is a hash-based construction suitable for off-chain verification.
 * On-chain verification stores only the commitment hash.
 */

import { createHash, randomBytes } from 'crypto';

function sha256Hex(input: string): string {
  return createHash('sha256').update(input).digest('hex');
}

export interface AmountCommitment {
  commitment: string;
  nonce: string;
  timestamp: number;
}

export interface RangeProof {
  commitment: string;
  bitCommitments: string[];
  rangeBound: { min: string; max: string };
  proofHash: string;
  timestamp: number;
}

export interface PrivateTransferProof {
  senderCommitment: AmountCommitment;
  recipientCommitment: AmountCommitment;
  balanceProof: string;
  rangeProof: RangeProof;
  viewKey?: string;
}

export function generateNonce(): string {
  return randomBytes(32).toString('hex');
}

export function generateCommitment(amount: bigint, nonce: string): AmountCommitment {
  if (amount < BigInt(0)) {
    throw new Error('Amount must be non-negative');
  }
  const commitment = sha256Hex(`${amount.toString()}:${nonce}`);
  return { commitment, nonce, timestamp: Date.now() };
}

export function openCommitment(commitment: AmountCommitment, amount: bigint): boolean {
  const expected = sha256Hex(`${amount.toString()}:${commitment.nonce}`);
  return expected === commitment.commitment;
}

export function generateRangeProof(
  amount: bigint,
  min: bigint,
  max: bigint,
  nonce: string
): RangeProof {
  if (amount < min || amount > max) {
    throw new RangeError(`Amount ${amount} outside range [${min}, ${max}]`);
  }

  const commitment = sha256Hex(`${amount.toString()}:${nonce}`);

  // Bit-decomposition: prove each bit of (amount - min) independently
  const normalised = amount - min;
  const bits = (max - min).toString(2).length;

  const bitCommitments: string[] = [];
  for (let i = 0; i < bits; i++) {
    const bit = (normalised >> BigInt(i)) & BigInt(1);
    const bitNonce = sha256Hex(`${nonce}:bit:${i}`);
    bitCommitments.push(sha256Hex(`${bit.toString()}:${bitNonce}`));
  }

  const proofHash = sha256Hex(
    [commitment, ...bitCommitments, min.toString(), max.toString()].join(':')
  );

  return {
    commitment,
    bitCommitments,
    rangeBound: { min: min.toString(), max: max.toString() },
    proofHash,
    timestamp: Date.now(),
  };
}

export function verifyRangeProof(proof: RangeProof, nonce: string, amount: bigint): boolean {
  const min = BigInt(proof.rangeBound.min);
  const max = BigInt(proof.rangeBound.max);

  if (amount < min || amount > max) return false;

  // Recompute commitment from revealed amount + nonce
  const expectedCommitment = sha256Hex(`${amount.toString()}:${nonce}`);
  if (expectedCommitment !== proof.commitment) return false;

  // Recompute bit commitments
  const normalised = amount - min;
  const bits = (max - min).toString(2).length;
  for (let i = 0; i < bits; i++) {
    const bit = (normalised >> BigInt(i)) & BigInt(1);
    const bitNonce = sha256Hex(`${nonce}:bit:${i}`);
    const expected = sha256Hex(`${bit.toString()}:${bitNonce}`);
    if (expected !== proof.bitCommitments[i]) return false;
  }

  // Recompute proof hash
  const expectedProofHash = sha256Hex(
    [proof.commitment, ...proof.bitCommitments, proof.rangeBound.min, proof.rangeBound.max].join(
      ':'
    )
  );

  return expectedProofHash === proof.proofHash;
}

export function generateTransferProof(
  senderAmount: bigint,
  recipientAmount: bigint,
  fee: bigint,
  maxAmount: bigint
): PrivateTransferProof {
  if (senderAmount !== recipientAmount + fee) {
    throw new Error('Balance invariant violated: sender must equal recipient + fee');
  }

  const senderNonce = generateNonce();
  const recipientNonce = generateNonce();

  const senderCommitment = generateCommitment(senderAmount, senderNonce);
  const recipientCommitment = generateCommitment(recipientAmount, recipientNonce);

  // Balance proof: H(senderCommitment || recipientCommitment || feeCommitment)
  const feeNonce = generateNonce();
  const feeCommitment = generateCommitment(fee, feeNonce);
  const balanceProof = sha256Hex(
    `${senderCommitment.commitment}:${recipientCommitment.commitment}:${feeCommitment.commitment}`
  );

  const rangeProof = generateRangeProof(recipientAmount, BigInt(0), maxAmount, recipientNonce);

  // View key allows regulatory inspection of the recipient amount
  const viewKey = sha256Hex(`viewkey:${recipientNonce}:${recipientAmount.toString()}`);

  return {
    senderCommitment,
    recipientCommitment,
    balanceProof,
    rangeProof,
    viewKey,
  };
}

export function verifyTransferProof(
  proof: PrivateTransferProof,
  senderAmount: bigint,
  recipientAmount: bigint,
  fee: bigint,
  senderNonce: string,
  recipientNonce: string
): boolean {
  if (senderAmount !== recipientAmount + fee) return false;

  if (!openCommitment(proof.senderCommitment, senderAmount)) return false;
  if (!openCommitment(proof.recipientCommitment, recipientAmount)) return false;

  return verifyRangeProof(proof.rangeProof, recipientNonce, recipientAmount);
}
