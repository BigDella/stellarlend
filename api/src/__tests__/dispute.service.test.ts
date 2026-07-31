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

import { disputeResolutionService } from '../services/dispute-resolution';

const disputer = 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF';
const liquidator = 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWJF';
const juror1 = 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABC';
const juror2 = 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAADEF';
const juror3 = 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAGHI';
const juror4 = 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAJKL';
const juror5 = 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAMNO';

describe('DisputeResolutionService', () => {
  let disputeId: string;

  it('registers jurors', () => {
    disputeResolutionService.registerJuror(juror1);
    disputeResolutionService.registerJuror(juror2);
    disputeResolutionService.registerJuror(juror3);
    disputeResolutionService.registerJuror(juror4);
    disputeResolutionService.registerJuror(juror5);
    expect(true).toBe(true);
  });

  it('files a dispute', () => {
    const dispute = disputeResolutionService.fileDispute(
      disputer, liquidator, 'tx_hash_1', '1000000000', 'initial evidence', '10000000'
    );
    expect(dispute.disputerAddress).toBe(disputer);
    expect(dispute.status).toBe('filing');
    expect(dispute.disputeFee).toBe('10000000');
    disputeId = dispute.id;
  });

  it('rejects dispute fee below minimum', () => {
    expect(() => disputeResolutionService.fileDispute(
      disputer, liquidator, 'tx_hash_2', '1000000000', 'evidence', '100'
    )).toThrow();
  });

  it('submits evidence', () => {
    const evidence = disputeResolutionService.submitEvidence(
      disputeId, disputer, 'Additional evidence', 'more_data'
    );
    expect(evidence).not.toBeNull();
    expect(evidence!.description).toBe('Additional evidence');
  });

  it('selects jurors', () => {
    const jurors = disputeResolutionService.selectJurors(disputeId);
    expect(jurors).not.toBeNull();
    expect(jurors!.length).toBe(5);
    expect(disputeResolutionService.getDispute(disputeId)!.status).toBe('voting');
  });

  it('casts votes and resolves', () => {
    disputeResolutionService.castVote(disputeId, juror1, 'valid', 'Evidence supports validity');
    disputeResolutionService.castVote(disputeId, juror2, 'valid', 'Agree');
    disputeResolutionService.castVote(disputeId, juror3, 'valid', 'Liquidation was proper');

    const dispute = disputeResolutionService.getDispute(disputeId)!;
    expect(dispute.votes.length).toBe(3);
  });

  it('resolves dispute with majority', () => {
    disputeResolutionService.castVote(disputeId, juror4, 'valid', 'Concur');
    const dispute = disputeResolutionService.getDispute(disputeId)!;
    expect(dispute.status).toBe('resolved');
    expect(dispute.resolution).toBe('valid');
  });

  it('allows appeal', () => {
    const appealed = disputeResolutionService.appeal(disputeId, disputer, '20000000');
    expect(appealed).not.toBeNull();
    expect(appealed!.status).toBe('filing');
    expect(appealed!.appealParentId).toBe(disputeId);
  });

  it('gets dispute by id', () => {
    const dispute = disputeResolutionService.getDispute(disputeId);
    expect(dispute).not.toBeNull();
    expect(dispute!.id).toBe(disputeId);
  });

  it('returns null for unknown dispute', () => {
    const dispute = disputeResolutionService.getDispute('unknown');
    expect(dispute).toBeNull();
  });

  it('lists disputes by user', () => {
    const disputes = disputeResolutionService.getDisputesByUser(disputer);
    expect(disputes.length).toBeGreaterThan(0);
  });
});
