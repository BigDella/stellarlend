import axios from 'axios';
jest.mock('axios');
const mockedAxios = axios as jest.Mocked<typeof axios>;
beforeAll(() => {
  mockedAxios.create.mockReturnThis();
  const axiosResponse = { data: {}, status: 200, statusText: 'OK', headers: {}, config: { url: '' } };
  mockedAxios.get.mockResolvedValue(axiosResponse);
  mockedAxios.post.mockResolvedValue(axiosResponse);
});
afterEach(() => { jest.clearAllMocks(); });

import { creditDelegationService } from '../services/credit-delegation';

const delegator = 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF';
const delegate = 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWJF';

describe('CreditDelegationService', () => {
  let creditLineId: string;

  it('creates a credit line', () => {
    const creditLine = creditDelegationService.createCreditLine(
      delegator, delegate, '1000000000', '500', '9999999999'
    );
    expect(creditLine.delegatorAddress).toBe(delegator);
    expect(creditLine.delegateAddress).toBe(delegate);
    expect(creditLine.maxAmount).toBe('1000000000');
    expect(creditLine.status).toBe('active');
    creditLineId = creditLine.id;
  });

  it('creates credit line with collateral', () => {
    const cl = creditDelegationService.createCreditLine(
      delegator, delegate, '500000000', '300', '9999999999', '200000000'
    );
    expect(cl.collateral).toBe('200000000');
  });

  it('draws from credit line', () => {
    const draw = creditDelegationService.draw(creditLineId, delegate, '500000000');
    expect(draw).not.toBeNull();
    expect(draw!.amount).toBe('500000000');
  });

  it('rejects draw exceeding limit', () => {
    expect(() => creditDelegationService.draw(creditLineId, delegate, '1000000000'))
      .toThrow('exceeds available credit');
  });

  it('rejects draw by non-delegate', () => {
    expect(() => creditDelegationService.draw(creditLineId, 'GAAAAAANOTALLOWED12345678901234567890123456789012', '1000000'))
      .toThrow();
  });

  it('repays credit line', () => {
    const repayment = creditDelegationService.repay(creditLineId, delegate, '200000000');
    expect(repayment).not.toBeNull();
    expect(repayment!.amount).toBe('200000000');
  });

  it('adjusts credit limit', () => {
    const updated = creditDelegationService.adjustLimit(creditLineId, delegator, '2000000000');
    expect(updated).not.toBeNull();
    expect(updated!.maxAmount).toBe('2000000000');
  });

  it('transfers credit line', () => {
    const newDelegator = 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAANEW';
    const updated = creditDelegationService.transfer(creditLineId, delegator, newDelegator);
    expect(updated).not.toBeNull();
    expect(updated!.delegatorAddress).toBe(newDelegator);
    expect(updated!.transferCount).toBe(1);
  });

  it('gets credit line by id', () => {
    const cl = creditDelegationService.getCreditLine(creditLineId);
    expect(cl).not.toBeNull();
    expect(cl!.id).toBe(creditLineId);
  });

  it('returns null for unknown credit line', () => {
    const cl = creditDelegationService.getCreditLine('unknown');
    expect(cl).toBeNull();
  });

  it('lists credit lines by delegator', () => {
    const lines = creditDelegationService.getCreditLinesByDelegator(delegator);
    expect(Array.isArray(lines)).toBe(true);
  });

  it('lists credit lines by delegate', () => {
    const lines = creditDelegationService.getCreditLinesByDelegate(delegate);
    expect(Array.isArray(lines)).toBe(true);
  });

  it('returns draws for credit line', () => {
    const draws = creditDelegationService.getDraws(creditLineId);
    expect(Array.isArray(draws)).toBe(true);
    expect(draws.length).toBeGreaterThan(0);
  });

  it('returns repayments for credit line', () => {
    const repayments = creditDelegationService.getRepayments(creditLineId);
    expect(Array.isArray(repayments)).toBe(true);
    expect(repayments.length).toBeGreaterThan(0);
  });

  it('claims default on matured unpaid credit line', () => {
    const cl = creditDelegationService.createCreditLine(delegator, delegate, '1000000000', '500', '100');
    const stored = (creditDelegationService as any).creditLines.get(cl.id);
    stored.drawnAmount = '500000000';
    stored.status = 'drawn';
    const defaulted = creditDelegationService.claimDefault(cl.id, delegator);
    expect(defaulted).not.toBeNull();
    expect(defaulted!.status).toBe('defaulted');
  });
});
