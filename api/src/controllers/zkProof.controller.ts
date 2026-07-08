import { Request, Response, NextFunction } from 'express';
import { zkProofService } from '../services/zkProof.service';
import type {
  CommitRequest,
  RangeProofRequest,
  VerifyRangeProofRequest,
  TransferProofRequest,
} from '../services/zkProof.service';

export const commit = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const body = req.body as CommitRequest;
    const result = zkProofService.commit(body);
    return res.status(200).json({ success: true, ...result });
  } catch (err) {
    next(err);
    return;
  }
};

export const rangeProof = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const body = req.body as RangeProofRequest;
    const proof = zkProofService.rangeProof(body);
    return res.status(200).json({ success: true, proof });
  } catch (err) {
    next(err);
    return;
  }
};

export const verifyRange = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const body = req.body as VerifyRangeProofRequest;
    const result = zkProofService.verifyRange(body);
    return res.status(200).json({ success: true, ...result });
  } catch (err) {
    next(err);
    return;
  }
};

export const transferProof = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const body = req.body as TransferProofRequest;
    const proof = zkProofService.transferProof(body);
    return res.status(200).json({ success: true, proof });
  } catch (err) {
    next(err);
    return;
  }
};
