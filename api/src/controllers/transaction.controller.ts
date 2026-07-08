import { Request, Response, NextFunction } from 'express';
import { transactionBuilderService } from '../services/transactionBuilder.service';
import type {
  CreateTransactionRequest,
  ApproveStepRequest,
  RejectStepRequest,
} from '../types/transaction';

export const createTransaction = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const body = req.body as CreateTransactionRequest;
    const tx = transactionBuilderService.create(body);
    return res.status(201).json({ success: true, transaction: tx });
  } catch (err) {
    next(err);
    return;
  }
};

export const prepareStep = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { txId, stepId } = req.params;
    const tx = await transactionBuilderService.prepareStep(txId!, stepId!);
    return res.status(200).json({ success: true, transaction: tx });
  } catch (err) {
    next(err);
    return;
  }
};

export const approveStep = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const body = req.body as ApproveStepRequest;
    const tx = await transactionBuilderService.approveStep(body);
    return res.status(200).json({ success: true, transaction: tx });
  } catch (err) {
    next(err);
    return;
  }
};

export const rejectStep = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const body = req.body as RejectStepRequest;
    const tx = transactionBuilderService.rejectStep(body);
    return res.status(200).json({ success: true, transaction: tx });
  } catch (err) {
    next(err);
    return;
  }
};

export const getTransaction = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { txId } = req.params;
    const tx = transactionBuilderService.getTransaction(txId!);
    return res.status(200).json({ success: true, transaction: tx });
  } catch (err) {
    next(err);
    return;
  }
};

export const listUserTransactions = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { userAddress } = req.params;
    const transactions = transactionBuilderService.listForUser(userAddress!);
    return res.status(200).json({ success: true, transactions, total: transactions.length });
  } catch (err) {
    next(err);
    return;
  }
};
