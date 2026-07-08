import logger from '../utils/logger';
import { createHash, randomUUID } from 'crypto';

export interface SanctionsEntry {
  id: string;
  address: string;
  source: string;
  reason: string;
  sanctionedAt: string;
  expiresAt?: string;
  active: boolean;
}

export interface KycVerification {
  id: string;
  address: string;
  verified: boolean;
  tier: number;
  verifiedAt: string;
  expiresAt: string;
  jurisdiction: string;
  kycProvider: string;
}

export interface ComplianceEvent {
  id: string;
  eventType: string;
  address: string;
  amount?: string;
  assetAddress?: string;
  details?: string;
  timestamp: string;
}

export interface SAR {
  id: string;
  sarId: number;
  address: string;
  reason: string;
  amount: string;
  assetAddress: string;
  filedAt: string;
  filedBy: string;
  status: 'filed' | 'under_review' | 'resolved' | 'escalated';
  notes?: string;
}

export interface TransactionLimits {
  dailyLimit: string;
  weeklyLimit: string;
  maxSingleTx: string;
}

export interface ComplianceCheckResult {
  passed: boolean;
  sanctionsMatch: boolean;
  kycValid: boolean;
  withinLimits: boolean;
  geoRestricted: boolean;
  errors: string[];
}

export interface ComplianceReport {
  period: { from: string; to: string };
  totalTransactions: number;
  flaggedTransactions: number;
  sarCount: number;
  sanctionsMatches: number;
  jurisdictionBreakdown: Record<string, number>;
}

const sanctionsList: Map<string, SanctionsEntry> = new Map();
const kycStore: Map<string, KycVerification> = new Map();
const events: ComplianceEvent[] = [];
const sarStore: Map<number, SAR> = new Map();
let nextSarId = 1;

const OFAC_SANCTIONED_ADDRESSES: string[] = [];

function recordEvent(params: {
  eventType: string;
  address: string;
  amount?: string;
  assetAddress?: string;
  details?: string;
}): ComplianceEvent {
  const event: ComplianceEvent = {
    id: randomUUID(),
    eventType: params.eventType,
    address: params.address,
    amount: params.amount,
    assetAddress: params.assetAddress,
    details: params.details,
    timestamp: new Date().toISOString(),
  };
  events.push(event);
  logger.info('COMPLIANCE_EVENT', {
    id: event.id,
    type: event.eventType,
    address: event.address,
    timestamp: event.timestamp,
  });
  return event;
}

class ComplianceService {
  addSanction(address: string, source: string, reason: string, expiresAt?: string): SanctionsEntry {
    const entry: SanctionsEntry = {
      id: randomUUID(),
      address,
      source,
      reason,
      sanctionedAt: new Date().toISOString(),
      expiresAt,
      active: true,
    };
    sanctionsList.set(address.toLowerCase(), entry);
    recordEvent({
      eventType: 'SANCTION_ADDED',
      address,
      details: `${source}: ${reason}`,
    });
    return entry;
  }

  removeSanction(address: string): void {
    const entry = sanctionsList.get(address.toLowerCase());
    if (entry) {
      entry.active = false;
      recordEvent({
        eventType: 'SANCTION_REMOVED',
        address,
        details: 'admin_remove',
      });
    }
  }

  checkSanctioned(address: string): boolean {
    const entry = sanctionsList.get(address.toLowerCase());
    if (!entry || !entry.active) return false;
    if (entry.expiresAt && new Date(entry.expiresAt) < new Date()) return false;
    return true;
  }

  screenAgainstOFAC(address: string): { match: boolean; confidence: number } {
    const normalized = address.toLowerCase();
    const exactMatch = OFAC_SANCTIONED_ADDRESSES.some(
      (sanctioned) => sanctioned.toLowerCase() === normalized
    );
    if (exactMatch) {
      return { match: true, confidence: 1.0 };
    }
    return { match: false, confidence: 0 };
  }

  setKycVerification(params: {
    address: string;
    tier: number;
    jurisdiction: string;
    kycProvider: string;
    validityDays?: number;
  }): KycVerification {
    const validityDays = params.validityDays ?? 365;
    const now = new Date();
    const expiresAt = new Date(now.getTime() + validityDays * 24 * 60 * 60 * 1000);

    const kyc: KycVerification = {
      id: randomUUID(),
      address: params.address,
      verified: true,
      tier: params.tier,
      verifiedAt: now.toISOString(),
      expiresAt: expiresAt.toISOString(),
      jurisdiction: params.jurisdiction,
      kycProvider: params.kycProvider,
    };
    kycStore.set(params.address.toLowerCase(), kyc);
    recordEvent({
      eventType: 'KYC_VERIFIED',
      address: params.address,
      details: `tier=${params.tier} jurisdiction=${params.jurisdiction}`,
    });
    return kyc;
  }

  revokeKyc(address: string): void {
    const kyc = kycStore.get(address.toLowerCase());
    if (kyc) {
      kyc.verified = false;
      recordEvent({
        eventType: 'KYC_REVOKED',
        address,
        details: 'admin_revoke',
      });
    }
  }

  checkKyc(address: string): boolean {
    const kyc = kycStore.get(address.toLowerCase());
    if (!kyc || !kyc.verified) return false;
    if (new Date(kyc.expiresAt) < new Date()) return false;
    return true;
  }

  getKyc(address: string): KycVerification | undefined {
    return kycStore.get(address.toLowerCase());
  }

  checkTransaction(params: {
    from: string;
    to: string;
    amount: string;
    asset: string;
  }): ComplianceCheckResult {
    const errors: string[] = [];
    let sanctionsMatch = false;
    let kycValid = true;
    const withinLimits = true;
    const geoRestricted = false;

    if (this.checkSanctioned(params.from) || this.checkSanctioned(params.to)) {
      sanctionsMatch = true;
      errors.push('Address is sanctioned');
    }

    const ofacFrom = this.screenAgainstOFAC(params.from);
    const ofacTo = this.screenAgainstOFAC(params.to);
    if (ofacFrom.match || ofacTo.match) {
      sanctionsMatch = true;
      errors.push('OFAC sanctions match detected');
    }

    const amount = BigInt(params.amount);
    if (amount > 100_000_000_000n) {
      if (!this.checkKyc(params.from)) {
        kycValid = false;
        errors.push('KYC verification required for large transactions');
      }
    }

    recordEvent({
      eventType: 'TX_CHECKED',
      address: params.from,
      amount: params.amount,
      assetAddress: params.asset,
      details: sanctionsMatch ? 'flagged' : 'passed',
    });

    return {
      passed: errors.length === 0,
      sanctionsMatch,
      kycValid,
      withinLimits,
      geoRestricted,
      errors,
    };
  }

  fileSar(params: {
    address: string;
    reason: string;
    amount: string;
    assetAddress: string;
    filedBy: string;
  }): SAR {
    const sarId = nextSarId++;
    const sar: SAR = {
      id: randomUUID(),
      sarId,
      address: params.address,
      reason: params.reason,
      amount: params.amount,
      assetAddress: params.assetAddress,
      filedAt: new Date().toISOString(),
      filedBy: params.filedBy,
      status: 'filed',
    };
    sarStore.set(sarId, sar);
    recordEvent({
      eventType: 'SAR_FILED',
      address: params.address,
      amount: params.amount,
      assetAddress: params.assetAddress,
      details: params.reason,
    });
    logger.warn('SAR_FILED', { sarId, address: params.address, reason: params.reason });
    return sar;
  }

  getSar(sarId: number): SAR | undefined {
    return sarStore.get(sarId);
  }

  listSars(status?: string): SAR[] {
    const all = Array.from(sarStore.values());
    if (status) {
      return all.filter((s) => s.status === status);
    }
    return all;
  }

  getComplianceReport(from: string, to: string): ComplianceReport {
    const fromDate = new Date(from);
    const toDate = new Date(to);
    const periodEvents = events.filter((e) => {
      const d = new Date(e.timestamp);
      return d >= fromDate && d <= toDate;
    });

    const txChecked = periodEvents.filter((e) => e.eventType === 'TX_CHECKED');
    const flagged = txChecked.filter((e) => e.details === 'flagged');

    return {
      period: { from, to },
      totalTransactions: txChecked.length,
      flaggedTransactions: flagged.length,
      sarCount: this.listSars().filter((s) => {
        const d = new Date(s.filedAt);
        return d >= fromDate && d <= toDate;
      }).length,
      sanctionsMatches: periodEvents.filter((e) => e.eventType === 'SANCTION_ADDED').length,
      jurisdictionBreakdown: {},
    };
  }

  getAuditTrail(address?: string, limit: number = 100): ComplianceEvent[] {
    const filtered = address
      ? events.filter((e) => e.address.toLowerCase() === address.toLowerCase())
      : events;
    return filtered.slice(-limit);
  }
}

export const complianceService = new ComplianceService();
