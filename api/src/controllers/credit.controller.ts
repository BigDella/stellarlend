import { Request, Response, NextFunction } from 'express';
import { creditDelegationService } from '../services/credit-delegation';
import logger from '../utils/logger';

export const createCreditLine = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const userAddress = req.headers['x-user-address'] as string;
    if (!userAddress) {
      return res.status(401).json({ success: false, error: 'Missing x-user-address header' });
    }
    const { delegateAddress, maxAmount, interestRate, maturityDate, collateral } = req.body;
    if (!delegateAddress || !maxAmount || !interestRate || !maturityDate) {
      return res.status(400).json({ success: false, error: 'Missing required fields' });
    }
    const creditLine = creditDelegationService.createCreditLine(
      userAddress, delegateAddress as string, maxAmount as string, interestRate as string, maturityDate as string, collateral as string | undefined
    );
    logger.info('Credit line created', { creditLineId: creditLine.id, userAddress });
    return res.status(201).json({ success: true, data: creditLine });
  } catch (error) {
    next(error);
    return;
  }
};

export const draw = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const userAddress = req.headers['x-user-address'] as string;
    if (!userAddress) {
      return res.status(401).json({ success: false, error: 'Missing x-user-address header' });
    }
    const id = req.params.id as string;
    const amount: string = req.body.amount as string;
    if (!amount) {
      return res.status(400).json({ success: false, error: 'Missing amount' });
    }
    const draw = creditDelegationService.draw(id, userAddress, amount);
    if (!draw) {
      return res.status(400).json({ success: false, error: 'Cannot draw on this credit line' });
    }
    return res.status(200).json({ success: true, data: draw });
  } catch (error) {
    next(error);
    return;
  }
};

export const repay = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const userAddress = req.headers['x-user-address'] as string;
    if (!userAddress) {
      return res.status(401).json({ success: false, error: 'Missing x-user-address header' });
    }
    const id = req.params.id as string;
    const amount: string = req.body.amount as string;
    if (!amount) {
      return res.status(400).json({ success: false, error: 'Missing amount' });
    }
    const repayment = creditDelegationService.repay(id, userAddress, amount);
    if (!repayment) {
      return res.status(400).json({ success: false, error: 'Cannot repay on this credit line' });
    }
    return res.status(200).json({ success: true, data: repayment });
  } catch (error) {
    next(error);
    return;
  }
};

export const getCreditLine = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const id = req.params.id as string;
    const creditLine = creditDelegationService.getCreditLine(id);
    if (!creditLine) {
      return res.status(404).json({ success: false, error: 'Credit line not found' });
    }
    const draws = creditDelegationService.getDraws(id);
    const repayments = creditDelegationService.getRepayments(id);
    return res.status(200).json({ success: true, data: { ...creditLine, draws, repayments } });
  } catch (error) {
    next(error);
    return;
  }
};

export const getMyCreditLines = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const userAddress = req.headers['x-user-address'] as string;
    if (!userAddress) {
      return res.status(401).json({ success: false, error: 'Missing x-user-address header' });
    }
    const asDelegator = creditDelegationService.getCreditLinesByDelegator(userAddress);
    const asDelegate = creditDelegationService.getCreditLinesByDelegate(userAddress);
    return res.status(200).json({ success: true, data: { asDelegator, asDelegate } });
  } catch (error) {
    next(error);
    return;
  }
};

export const claimDefault = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const userAddress = req.headers['x-user-address'] as string;
    if (!userAddress) {
      return res.status(401).json({ success: false, error: 'Missing x-user-address header' });
    }
    const id = req.params.id as string;
    const creditLine = creditDelegationService.claimDefault(id, userAddress);
    if (!creditLine) {
      return res.status(400).json({ success: false, error: 'Cannot claim default' });
    }
    return res.status(200).json({ success: true, data: creditLine });
  } catch (error) {
    next(error);
    return;
  }
};

export const adjustLimit = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const userAddress = req.headers['x-user-address'] as string;
    if (!userAddress) {
      return res.status(401).json({ success: false, error: 'Missing x-user-address header' });
    }
    const id = req.params.id as string;
    const maxAmount: string = req.body.maxAmount as string;
    const creditLine = creditDelegationService.adjustLimit(id, userAddress, maxAmount);
    if (!creditLine) {
      return res.status(404).json({ success: false, error: 'Credit line not found' });
    }
    return res.status(200).json({ success: true, data: creditLine });
  } catch (error) {
    next(error);
    return;
  }
};

export const transfer = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const userAddress = req.headers['x-user-address'] as string;
    if (!userAddress) {
      return res.status(401).json({ success: false, error: 'Missing x-user-address header' });
    }
    const id = req.params.id as string;
    const newDelegatorAddress: string = req.body.newDelegatorAddress as string;
    const creditLine = creditDelegationService.transfer(id, userAddress, newDelegatorAddress);
    if (!creditLine) {
      return res.status(404).json({ success: false, error: 'Credit line not found' });
    }
    return res.status(200).json({ success: true, data: creditLine });
  } catch (error) {
    next(error);
    return;
  }
};
