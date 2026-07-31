import { CreditLine, CreditDraw, CreditRepayment, CreditStatus } from '../../types/credit';
import logger from '../../utils/logger';
import { v4 as uuid } from 'uuid';

class CreditDelegationService {
  private creditLines: Map<string, CreditLine> = new Map();
  private draws: Map<string, CreditDraw[]> = new Map();
  private repayments: Map<string, CreditRepayment[]> = new Map();

  createCreditLine(
    delegatorAddress: string,
    delegateAddress: string,
    maxAmount: string,
    interestRate: string,
    maturityDate: string,
    collateral?: string
  ): CreditLine {
    const creditLine: CreditLine = {
      id: uuid(),
      delegatorAddress,
      delegateAddress,
      maxAmount,
      interestRate,
      maturityDate,
      collateral,
      drawnAmount: '0',
      repaidAmount: '0',
      status: 'active',
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
      transferCount: 0,
    };

    this.creditLines.set(creditLine.id, creditLine);
    this.draws.set(creditLine.id, []);
    this.repayments.set(creditLine.id, []);

    logger.info('Credit line created', {
      creditLineId: creditLine.id,
      delegator: delegatorAddress,
      delegate: delegateAddress,
      maxAmount,
    });

    return creditLine;
  }

  draw(creditLineId: string, delegateAddress: string, amount: string): CreditDraw | null {
    const creditLine = this.creditLines.get(creditLineId);
    if (!creditLine) return null;
    if (creditLine.delegateAddress !== delegateAddress) {
      throw new Error('Only the delegate can draw on this credit line');
    }
    if (creditLine.status !== 'active' && creditLine.status !== 'drawn') return null;

    const drawnAmount = BigInt(creditLine.drawnAmount);
    const maxAmount = BigInt(creditLine.maxAmount);
    const drawAmount = BigInt(amount);

    if (drawnAmount + drawAmount > maxAmount) {
      throw new Error('Draw amount exceeds available credit');
    }

    if (new Date(creditLine.maturityDate) < new Date()) {
      throw new Error('Credit line has matured');
    }

    const draw: CreditDraw = {
      creditLineId,
      amount,
      drawnAt: new Date().toISOString(),
    };

    const existingDraws = this.draws.get(creditLineId) || [];
    existingDraws.push(draw);
    this.draws.set(creditLineId, existingDraws);

    const newDrawnAmount = (drawnAmount + drawAmount).toString();
    this.creditLines.set(creditLineId, {
      ...creditLine,
      drawnAmount: newDrawnAmount,
      status: 'drawn',
      updatedAt: new Date().toISOString(),
    });

    logger.info('Credit drawn', { creditLineId, amount, delegate: delegateAddress });
    return draw;
  }

  repay(creditLineId: string, delegateAddress: string, amount: string): CreditRepayment | null {
    const creditLine = this.creditLines.get(creditLineId);
    if (!creditLine) return null;
    if (creditLine.delegateAddress !== delegateAddress) return null;
    if (creditLine.status !== 'active' && creditLine.status !== 'drawn') return null;

    const drawnAmount = BigInt(creditLine.drawnAmount);
    const repaidAmount = BigInt(creditLine.repaidAmount);
    const repayAmount = BigInt(amount);

    if (repaidAmount + repayAmount > drawnAmount) {
      throw new Error('Repayment exceeds drawn amount');
    }

    const interestRate = BigInt(creditLine.interestRate);
    const accruedInterest = (repayAmount * interestRate) / 10000n;

    const repayment: CreditRepayment = {
      creditLineId,
      amount,
      repaidAt: new Date().toISOString(),
      accruedInterest: accruedInterest.toString(),
    };

    const existingRepayments = this.repayments.get(creditLineId) || [];
    existingRepayments.push(repayment);
    this.repayments.set(creditLineId, existingRepayments);

    const newRepaidAmount = (repaidAmount + repayAmount).toString();
    const remaining = drawnAmount - (repaidAmount + repayAmount);
    const newStatus: CreditStatus = remaining <= 0n ? 'repaid' : 'drawn';

    this.creditLines.set(creditLineId, {
      ...creditLine,
      repaidAmount: newRepaidAmount,
      status: newStatus,
      updatedAt: new Date().toISOString(),
    });

    logger.info('Credit repaid', { creditLineId, amount, delegate: delegateAddress });
    return repayment;
  }

  claimDefault(creditLineId: string, delegatorAddress: string): CreditLine | null {
    const creditLine = this.creditLines.get(creditLineId);
    if (!creditLine) return null;
    if (creditLine.delegatorAddress !== delegatorAddress) return null;
    if (new Date(creditLine.maturityDate) > new Date()) return null;
    if (creditLine.status !== 'active' && creditLine.status !== 'drawn') return null;

    const drawnAmount = BigInt(creditLine.drawnAmount);
    const repaidAmount = BigInt(creditLine.repaidAmount);
    if (drawnAmount <= repaidAmount) return null;

    const updated = { ...creditLine, status: 'defaulted' as CreditStatus, updatedAt: new Date().toISOString() };
    this.creditLines.set(creditLineId, updated);

    logger.info('Credit line defaulted', { creditLineId, delegator: delegatorAddress });
    return updated;
  }

  adjustLimit(creditLineId: string, delegatorAddress: string, newMaxAmount: string): CreditLine | null {
    const creditLine = this.creditLines.get(creditLineId);
    if (!creditLine) return null;
    if (creditLine.delegatorAddress !== delegatorAddress) return null;

    const updated = { ...creditLine, maxAmount: newMaxAmount, updatedAt: new Date().toISOString() };
    this.creditLines.set(creditLineId, updated);
    return updated;
  }

  transfer(creditLineId: string, delegatorAddress: string, newDelegatorAddress: string): CreditLine | null {
    const creditLine = this.creditLines.get(creditLineId);
    if (!creditLine) return null;
    if (creditLine.delegatorAddress !== delegatorAddress) return null;

    const updated = {
      ...creditLine,
      delegatorAddress: newDelegatorAddress,
      transferCount: creditLine.transferCount + 1,
      updatedAt: new Date().toISOString(),
    };
    this.creditLines.set(creditLineId, updated);

    logger.info('Credit line transferred', { creditLineId, from: delegatorAddress, to: newDelegatorAddress });
    return updated;
  }

  getCreditLine(creditLineId: string): CreditLine | null {
    return this.creditLines.get(creditLineId) || null;
  }

  getCreditLinesByDelegator(delegatorAddress: string): CreditLine[] {
    const result: CreditLine[] = [];
    for (const [, cl] of this.creditLines) {
      if (cl.delegatorAddress === delegatorAddress) result.push(cl);
    }
    return result.sort((a, b) => new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime());
  }

  getCreditLinesByDelegate(delegateAddress: string): CreditLine[] {
    const result: CreditLine[] = [];
    for (const [, cl] of this.creditLines) {
      if (cl.delegateAddress === delegateAddress) result.push(cl);
    }
    return result.sort((a, b) => new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime());
  }

  getDraws(creditLineId: string): CreditDraw[] {
    return this.draws.get(creditLineId) || [];
  }

  getRepayments(creditLineId: string): CreditRepayment[] {
    return this.repayments.get(creditLineId) || [];
  }
}

export const creditDelegationService = new CreditDelegationService();
