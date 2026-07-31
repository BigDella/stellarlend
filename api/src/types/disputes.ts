export type DisputeStatus = 'filing' | 'evidence' | 'voting' | 'resolved' | 'appealed';
export type DisputeVote = 'valid' | 'invalid';
export type DisputeResolution = 'valid' | 'invalid';

export interface Dispute {
  id: string;
  disputerAddress: string;
  liquidatorAddress: string;
  liquidationTxHash: string;
  collateralAmount: string;
  disputeFee: string;
  status: DisputeStatus;
  evidence: DisputeEvidence[];
  jurors: JurorAssignment[];
  votes: VoteRecord[];
  resolution?: DisputeResolution;
  resolvedAt?: string;
  createdAt: string;
  appealParentId?: string;
  appealStake: string;
}

export interface DisputeEvidence {
  submittedBy: string;
  description: string;
  data: string;
  submittedAt: string;
}

export interface JurorAssignment {
  jurorAddress: string;
  selectedAt: string;
  voted: boolean;
}

export interface VoteRecord {
  jurorAddress: string;
  vote: DisputeVote;
  rationale?: string;
  votedAt: string;
}

export interface FileDisputeRequest {
  liquidationTxHash: string;
  collateralAmount: string;
  evidence: string;
}

export interface SubmitEvidenceRequest {
  description: string;
  data: string;
}

export interface VoteRequest {
  vote: DisputeVote;
  rationale?: string;
}

export interface AppealRequest {
  disputeId: string;
  stake: string;
}
