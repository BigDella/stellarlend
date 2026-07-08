import { Request, Response, NextFunction } from 'express';
import { merkleProofService } from '../services/merkleProof.service';
import type { AccountState } from '../services/merkleProof.service';
import type { MerkleProof } from '../utils/merkleTree';

export const upsertAccount = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const state = req.body as AccountState;
    const snapshot = merkleProofService.upsertAccount(state);
    return res.status(200).json({ success: true, tree: snapshot });
  } catch (err) {
    next(err);
    return;
  }
};

export const getProof = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { userAddress } = req.params!;
    const proof = merkleProofService.generateProof(userAddress);
    return res.status(200).json({ success: true, proof });
  } catch (err) {
    next(err);
    return;
  }
};

export const verifyProof = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const proof = req.body as MerkleProof;
    const result = merkleProofService.verifyProof(proof);
    return res.status(200).json({ success: true, ...result });
  } catch (err) {
    next(err);
    return;
  }
};

export const getTreeInfo = async (_req: Request, res: Response, next: NextFunction) => {
  try {
    const info = merkleProofService.getTreeInfo();
    return res.status(200).json({ success: true, tree: info });
  } catch (err) {
    next(err);
    return;
  }
};

export const getAccount = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { userAddress } = req.params!;
    const account = merkleProofService.getAccount(userAddress);
    return res.status(200).json({ success: true, account });
  } catch (err) {
    next(err);
    return;
  }
};

export const listAccounts = async (_req: Request, res: Response, next: NextFunction) => {
  try {
    const accounts = merkleProofService.listAccounts();
    return res.status(200).json({ success: true, accounts, total: accounts.length });
  } catch (err) {
    next(err);
    return;
  }
};
