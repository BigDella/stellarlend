export type CreditStatus = 'active' | 'drawn' | 'repaid' | 'defaulted' | 'transferred';

export interface CreditLine {
  id: string;
  delegatorAddress: string;
  delegateAddress: string;
  maxAmount: string;
  interestRate: string;
  maturityDate: string;
  collateral?: string;
  drawnAmount: string;
  repaidAmount: string;
  status: CreditStatus;
  createdAt: string;
  updatedAt: string;
  transferCount: number;
}

export interface CreditDraw {
  creditLineId: string;
  amount: string;
  drawnAt: string;
}

export interface CreditRepayment {
  creditLineId: string;
  amount: string;
  repaidAt: string;
  accruedInterest: string;
}

export interface CreateCreditLineRequest {
  delegateAddress: string;
  maxAmount: string;
  interestRate: string;
  maturityDate: string;
  collateral?: string;
}

export interface DrawRequest {
  amount: string;
}

export interface RepayRequest {
  amount: string;
}

export interface TransferRequest {
  newDelegatorAddress: string;
}
