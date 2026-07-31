import { Request, Response, NextFunction } from 'express';
import { disputeResolutionService } from '../services/dispute-resolution';
import logger from '../utils/logger';

export const fileDispute = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const userAddress = req.headers['x-user-address'] as string;
    if (!userAddress) {
      return res.status(401).json({ success: false, error: 'Missing x-user-address header' });
    }
    const liquidationTxHash: string = req.body.liquidationTxHash as string;
    const collateralAmount: string = req.body.collateralAmount as string;
    const evidence: string = req.body.evidence as string;
    const disputeFee: string = req.body.disputeFee as string;
    if (!liquidationTxHash || !collateralAmount || !evidence || !disputeFee) {
      return res.status(400).json({ success: false, error: 'Missing required fields' });
    }
    const dispute = disputeResolutionService.fileDispute(
      userAddress, '', liquidationTxHash, collateralAmount, evidence, disputeFee
    );
    logger.info('Dispute filed', { disputeId: dispute.id, userAddress });
    return res.status(201).json({ success: true, data: dispute });
  } catch (error) {
    next(error);
    return;
  }
};

export const getDispute = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const id = req.params.id as string;
    const dispute = disputeResolutionService.getDispute(id);
    if (!dispute) {
      return res.status(404).json({ success: false, error: 'Dispute not found' });
    }
    return res.status(200).json({ success: true, data: dispute });
  } catch (error) {
    next(error);
    return;
  }
};

export const submitEvidence = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const userAddress = req.headers['x-user-address'] as string;
    if (!userAddress) {
      return res.status(401).json({ success: false, error: 'Missing x-user-address header' });
    }
    const id = req.params.id as string;
    const description: string = req.body.description as string;
    const data: string = req.body.data as string;
    const evidence = disputeResolutionService.submitEvidence(id, userAddress, description, data);
    if (!evidence) {
      return res.status(400).json({ success: false, error: 'Cannot submit evidence at this stage' });
    }
    return res.status(200).json({ success: true, data: evidence });
  } catch (error) {
    next(error);
    return;
  }
};

export const vote = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const userAddress = req.headers['x-user-address'] as string;
    if (!userAddress) {
      return res.status(401).json({ success: false, error: 'Missing x-user-address header' });
    }
    const id = req.params.id as string;
    const voteValue: string = req.body.vote as string;
    const rationale: string = req.body.rationale as string;
    if (!voteValue || !['valid', 'invalid'].includes(voteValue)) {
      return res.status(400).json({ success: false, error: 'Vote must be valid or invalid' });
    }
    const record = disputeResolutionService.castVote(id, userAddress, voteValue as any, rationale);
    if (!record) {
      return res.status(400).json({ success: false, error: 'Cannot vote at this stage or not a juror' });
    }
    return res.status(200).json({ success: true, data: record });
  } catch (error) {
    next(error);
    return;
  }
};

export const appeal = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const userAddress = req.headers['x-user-address'] as string;
    if (!userAddress) {
      return res.status(401).json({ success: false, error: 'Missing x-user-address header' });
    }
    const id = req.params.id as string;
    const stake: string = req.body.stake as string;
    const appealed = disputeResolutionService.appeal(id, userAddress, stake);
    if (!appealed) {
      return res.status(400).json({ success: false, error: 'Cannot appeal at this stage' });
    }
    return res.status(201).json({ success: true, data: appealed });
  } catch (error) {
    next(error);
    return;
  }
};

export const getMyDisputes = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const userAddress = req.headers['x-user-address'] as string;
    if (!userAddress) {
      return res.status(401).json({ success: false, error: 'Missing x-user-address header' });
    }
    const disputes = disputeResolutionService.getDisputesByUser(userAddress);
    return res.status(200).json({ success: true, data: disputes });
  } catch (error) {
    next(error);
    return;
  }
};

export const registerJuror = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const userAddress = req.headers['x-user-address'] as string;
    if (!userAddress) {
      return res.status(401).json({ success: false, error: 'Missing x-user-address header' });
    }
    disputeResolutionService.registerJuror(userAddress);
    return res.status(200).json({ success: true, data: { address: userAddress, registered: true } });
  } catch (error) {
    next(error);
    return;
  }
};
