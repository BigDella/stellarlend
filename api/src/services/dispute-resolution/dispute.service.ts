import { Dispute, DisputeStatus, DisputeVote, DisputeEvidence, VoteRecord, JurorAssignment, DisputeResolution } from '../../types/disputes';
import logger from '../../utils/logger';
import { v4 as uuid } from 'uuid';

const FILING_PERIOD_MS = 24 * 60 * 60 * 1000;
const EVIDENCE_PERIOD_MS = 48 * 60 * 60 * 1000;
const VOTING_PERIOD_MS = 24 * 60 * 60 * 1000;
const REQUIRED_JURY_SIZE = 5;
const MAJORITY_THRESHOLD = 0.66;
const MIN_DISPUTE_FEE = 10000000n;
const APPEAL_MULTIPLIER = 2;

class DisputeResolutionService {
  private disputes: Map<string, Dispute> = new Map();
  private jurorPool: Set<string> = new Set();

  registerJuror(address: string): void {
    this.jurorPool.add(address);
  }

  fileDispute(
    disputerAddress: string,
    liquidatorAddress: string,
    liquidationTxHash: string,
    collateralAmount: string,
    evidenceData: string,
    disputeFee: string
  ): Dispute {
    const fee = BigInt(disputeFee);
    if (fee < MIN_DISPUTE_FEE) {
      throw new Error(`Dispute fee must be at least ${MIN_DISPUTE_FEE} stroops`);
    }

    const dispute: Dispute = {
      id: uuid(),
      disputerAddress,
      liquidatorAddress,
      liquidationTxHash,
      collateralAmount,
      disputeFee,
      status: 'filing',
      evidence: [{
        submittedBy: disputerAddress,
        description: 'Initial dispute filing',
        data: evidenceData,
        submittedAt: new Date().toISOString(),
      }],
      jurors: [],
      votes: [],
      createdAt: new Date().toISOString(),
      appealStake: '0',
    };

    this.disputes.set(dispute.id, dispute);
    logger.info('Dispute filed', { disputeId: dispute.id, disputer: disputerAddress });
    return dispute;
  }

  submitEvidence(disputeId: string, submitterAddress: string, description: string, data: string): DisputeEvidence | null {
    const dispute = this.disputes.get(disputeId);
    if (!dispute) return null;
    if (dispute.status !== 'filing' && dispute.status !== 'evidence') return null;
    if (Date.now() - new Date(dispute.createdAt).getTime() > EVIDENCE_PERIOD_MS + FILING_PERIOD_MS) return null;

    this.disputes.set(disputeId, { ...dispute, status: 'evidence' });

    const evidence: DisputeEvidence = {
      submittedBy: submitterAddress,
      description,
      data,
      submittedAt: new Date().toISOString(),
    };

    const updated = { ...dispute, evidence: [...dispute.evidence, evidence] };
    this.disputes.set(disputeId, updated);
    return evidence;
  }

  selectJurors(disputeId: string): JurorAssignment[] | null {
    const dispute = this.disputes.get(disputeId);
    if (!dispute) return null;
    if (dispute.status === 'voting' || dispute.status === 'resolved') return null;

    const pool = Array.from(this.jurorPool).filter(j => 
      j !== dispute.disputerAddress && j !== dispute.liquidatorAddress
    );

    if (pool.length < REQUIRED_JURY_SIZE) {
      throw new Error(`Insufficient jurors. Need ${REQUIRED_JURY_SIZE}, have ${pool.length}`);
    }

    const selected = this.shuffleArray(pool).slice(0, REQUIRED_JURY_SIZE);
    const jurors: JurorAssignment[] = selected.map(address => ({
      jurorAddress: address,
      selectedAt: new Date().toISOString(),
      voted: false,
    }));

    this.disputes.set(disputeId, {
      ...dispute,
      jurors,
      status: 'voting',
    });

    return jurors;
  }

  castVote(disputeId: string, jurorAddress: string, vote: DisputeVote, rationale?: string): VoteRecord | null {
    const dispute = this.disputes.get(disputeId);
    if (!dispute) return null;
    if (dispute.status !== 'voting') return null;
    if (Date.now() - new Date(dispute.createdAt).getTime() > VOTING_PERIOD_MS + EVIDENCE_PERIOD_MS + FILING_PERIOD_MS) return null;

    const jurorIndex = dispute.jurors.findIndex(j => j.jurorAddress === jurorAddress);
    if (jurorIndex === -1) return null;
    if (dispute.jurors[jurorIndex]!.voted) return null;

    const voteRecord: VoteRecord = {
      jurorAddress,
      vote,
      rationale,
      votedAt: new Date().toISOString(),
    };

    const updatedJurors = [...dispute.jurors];
    updatedJurors[jurorIndex] = { ...(updatedJurors[jurorIndex] as JurorAssignment), voted: true };

    const updated = {
      ...dispute,
      jurors: updatedJurors,
      votes: [...dispute.votes, voteRecord],
    };

    this.disputes.set(disputeId, updated);

    if (this.canResolve(updated)) {
      this.resolve(disputeId);
    }

    return voteRecord;
  }

  private canResolve(dispute: Dispute): boolean {
    const votedCount = dispute.jurors.filter(j => j.voted).length;
    return votedCount >= REQUIRED_JURY_SIZE || dispute.votes.length >= Math.ceil(REQUIRED_JURY_SIZE * MAJORITY_THRESHOLD);
  }

  private resolve(disputeId: string): void {
    const dispute = this.disputes.get(disputeId);
    if (!dispute) return;

    const validVotes = dispute.votes.filter(v => v.vote === 'valid').length;
    const totalVotes = dispute.votes.length;
    const ratio = totalVotes > 0 ? validVotes / totalVotes : 0;

    const resolution: DisputeResolution = ratio >= MAJORITY_THRESHOLD ? 'valid' : 'invalid';

    this.disputes.set(disputeId, {
      ...dispute,
      status: 'resolved',
      resolution,
      resolvedAt: new Date().toISOString(),
    });

    logger.info('Dispute resolved', { disputeId, resolution, validVotes, totalVotes });
  }

  appeal(disputeId: string, appellantAddress: string, stake: string): Dispute | null {
    const dispute = this.disputes.get(disputeId);
    if (!dispute) return null;
    if (dispute.status !== 'resolved') return null;

    const appealStake = BigInt(stake);
    const requiredStake = BigInt(dispute.disputeFee) * BigInt(APPEAL_MULTIPLIER);
    if (appealStake < requiredStake) {
      throw new Error(`Appeal stake must be at least ${requiredStake} stroops`);
    }

    const appealedDispute: Dispute = {
      ...dispute,
      id: uuid(),
      status: 'filing',
      evidence: [...dispute.evidence],
      jurors: [],
      votes: [],
      resolution: undefined,
      resolvedAt: undefined,
      createdAt: new Date().toISOString(),
      appealParentId: disputeId,
      appealStake: stake,
    };

    this.disputes.set(dispute.id, { ...dispute, status: 'appealed' });
    this.disputes.set(appealedDispute.id, appealedDispute);
    return appealedDispute;
  }

  getDispute(disputeId: string): Dispute | null {
    return this.disputes.get(disputeId) || null;
  }

  getDisputesByUser(address: string): Dispute[] {
    const result: Dispute[] = [];
    for (const [, dispute] of this.disputes) {
      if (dispute.disputerAddress === address || dispute.liquidatorAddress === address) {
        result.push(dispute);
      }
    }
    return result.sort((a, b) => new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime());
  }

  private shuffleArray<T>(array: T[]): T[] {
    const shuffled: T[] = [...array];
    for (let i = shuffled.length - 1; i > 0; i--) {
      const j = Math.floor(Math.random() * (i + 1));
      const tmp = shuffled[i] as T;
      shuffled[i] = shuffled[j] as T;
      shuffled[j] = tmp;
    }
    return shuffled;
  }
}

export const disputeResolutionService = new DisputeResolutionService();
