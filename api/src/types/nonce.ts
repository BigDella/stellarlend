export interface PendingNonce {
  nonce: string;
  allocatedAt: number;
  status: 'pending' | 'confirmed' | 'failed';
  txHash?: string;
}

export interface NonceGap {
  start: string;
  end: string;
  reason: 'transaction_failed' | 'timeout';
  filledAt?: number;
}

export interface NonceState {
  address: string;
  currentNonce: string;
  nextNonce: string;
  pendingNonces: PendingNonce[];
  failedNonces: string[];
  gaps: NonceGap[];
  lastUpdated: number;
}

export interface NonceAllocationRequest {
  address: string;
}

export interface NonceRecoveryRequest {
  address: string;
  nonce: string;
}

export interface NonceStateResponse {
  address: string;
  currentNonce: string;
  nextNonce: string;
  pendingNonceCount: number;
  failedNonceCount: number;
  gapCount: number;
}
