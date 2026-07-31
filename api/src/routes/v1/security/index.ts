/**
 * Security Domain Routes (v1)
 *
 * Aggregates all security/privacy routes under /v1/security:
 * - Merkle tree (accounts, proofs, verification)
 * - Zero-knowledge proofs (commit, range proofs, transfer proofs)
 * - Contract verification
 * - MEV protection (commit-reveal, auctions, dashboard)
 */

import { Router } from 'express';
import merkleRoutes from '../../merkle.routes';
import zkProofRoutes from '../../zkProof.routes';
import verificationRoutes from '../../verification.routes';
import mevRoutes from '../../mev.routes';

const router = Router();

// Merkle: /v1/security/merkle/*
router.use('/merkle', merkleRoutes);

// ZK Proofs: /v1/security/zk/*
router.use('/zk', zkProofRoutes);

// Contract verification: /v1/security/verification/*
router.use('/verification', verificationRoutes);

// MEV Protection: /v1/security/mev/*
router.use('/mev', mevRoutes);

export default router;
