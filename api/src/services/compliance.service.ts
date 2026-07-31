import logger from '../utils/logger';
import { createHash, randomUUID } from 'crypto';

// ─── Interfaces ──────────────────────────────────────────────────────────────

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
  hash?: string;
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
  amlRiskScore: number;
  amlFlags: string[];
  errors: string[];
}

export interface ComplianceReport {
  period: { from: string; to: string };
  totalTransactions: number;
  flaggedTransactions: number;
  sarCount: number;
  sanctionsMatches: number;
  kycVerifications: number;
  kycRevocations: number;
  amlAlerts: number;
  jurisdictionBreakdown: Record<string, number>;
  eventTypeBreakdown: Record<string, number>;
}

export interface ComplianceConfig {
  defaultLimits: TransactionLimits;
  jurisdictionLimits: Record<string, TransactionLimits>;
  restrictedJurisdictions: string[];
  kycThresholdAmount: string;
  amlRiskThreshold: number;
  amlMonitoringEnabled: boolean;
  autoFileSar: boolean;
  autoFileSarThreshold: number;
  requireKycForWithdrawal: boolean;
  requireKycForDeposit: boolean;
  maxDailyVolumeWithoutKyc: string;
}

export interface AmlRiskAssessment {
  address: string;
  riskScore: number;
  riskLevel: 'low' | 'medium' | 'high' | 'critical';
  flags: string[];
  assessedAt: string;
  transactionCount: number;
  totalVolume: string;
  uniqueCounterparties: number;
  highRiskJurisdiction: boolean;
  rapidMovementDetected: boolean;
  structuringDetected: boolean;
}

export interface ComplianceDashboardData {
  summary: {
    totalKycVerified: number;
    totalSanctioned: number;
    totalSars: number;
    totalAmlAlerts: number;
    pendingReviews: number;
    complianceRate: number;
  };
  recentEvents: ComplianceEvent[];
  riskDistribution: {
    low: number;
    medium: number;
    high: number;
    critical: number;
  };
  jurisdictionStats: Record<string, { count: number; riskScore: number }>;
  topFlaggedAddresses: Array<{ address: string; flagCount: number; lastFlag: string }>;
  regulatoryLimits: {
    dailyVolumeUsed: string;
    dailyVolumeLimit: string;
    weeklyVolumeUsed: string;
    weeklyVolumeLimit: string;
  };
}

// ─── In-memory stores ────────────────────────────────────────────────────────

const sanctionsList: Map<string, SanctionsEntry> = new Map();
const kycStore: Map<string, KycVerification> = new Map();
const events: ComplianceEvent[] = [];
const sarStore: Map<number, SAR> = new Map();
let nextSarId = 1;

const OFAC_SANCTIONED_ADDRESSES: string[] = [];

// AML risk store: address -> risk assessment
const amlRiskStore: Map<string, AmlRiskAssessment> = new Map();

// Transaction volume tracking for regulatory limits
const dailyVolumeStore: Map<string, { volume: bigint; date: string }> = new Map();
const weeklyVolumeStore: Map<string, { volume: bigint; weekStart: string }> = new Map();

// Compliance configuration
const defaultConfig: ComplianceConfig = {
  defaultLimits: {
    dailyLimit: '1000000000000', // 10,000 XLM (in stroops)
    weeklyLimit: '5000000000000', // 50,000 XLM
    maxSingleTx: '500000000000',  // 5,000 XLM
  },
  jurisdictionLimits: {},
  restrictedJurisdictions: [],
  kycThresholdAmount: '100000000000', // 1,000 XLM
  amlRiskThreshold: 70,
  amlMonitoringEnabled: true,
  autoFileSar: false,
  autoFileSarThreshold: 90,
  requireKycForWithdrawal: false,
  requireKycForDeposit: false,
  maxDailyVolumeWithoutKyc: '50000000000', // 500 XLM
};

let complianceConfig: ComplianceConfig = { ...defaultConfig };

// ─── Helpers ─────────────────────────────────────────────────────────────────

function recordEvent(params: {
  eventType: string;
  address: string;
  amount?: string;
  assetAddress?: string;
  details?: string;
}): ComplianceEvent {
  const prevHash = events.length > 0 ? (events[events.length - 1]!.hash ?? 'genesis') : 'genesis';
  const event: ComplianceEvent = {
    id: randomUUID(),
    eventType: params.eventType,
    address: params.address,
    amount: params.amount,
    assetAddress: params.assetAddress,
    details: params.details,
    timestamp: new Date().toISOString(),
  };
  // Compute hash chain for audit integrity
  event.hash = createHash('sha256')
    .update(`${prevHash}:${event.id}:${event.eventType}:${event.address}:${event.timestamp}`)
    .digest('hex');
  events.push(event);
  logger.info('COMPLIANCE_EVENT', {
    id: event.id,
    type: event.eventType,
    address: event.address,
    timestamp: event.timestamp,
    hash: event.hash,
  });
  return event;
}

function getTodayStr(): string {
  return new Date().toISOString().split('T')[0] as string;
}

function getWeekStartStr(): string {
  const now = new Date();
  const day = now.getDay();
  const diff = now.getDate() - day + (day === 0 ? -6 : 1);
  const monday = new Date(now.setDate(diff));
  return monday.toISOString().split('T')[0] as string;
}

function getLimitsForJurisdiction(jurisdiction?: string): TransactionLimits {
  if (jurisdiction && complianceConfig.jurisdictionLimits[jurisdiction]) {
    return complianceConfig.jurisdictionLimits[jurisdiction];
  }
  return complianceConfig.defaultLimits;
}

function isJurisdictionRestricted(jurisdiction: string): boolean {
  return complianceConfig.restrictedJurisdictions.includes(jurisdiction.toUpperCase());
}

// ─── ComplianceService ───────────────────────────────────────────────────────

class ComplianceService {
  // ─── Reset (for testing) ────────────────────────────────────────────────

  reset(): void {
    sanctionsList.clear();
    kycStore.clear();
    events.length = 0;
    sarStore.clear();
    nextSarId = 1;
    amlRiskStore.clear();
    dailyVolumeStore.clear();
    weeklyVolumeStore.clear();
    complianceConfig = { ...defaultConfig };
  }

  // ─── Configuration ──────────────────────────────────────────────────────

  getConfig(): ComplianceConfig {
    return { ...complianceConfig };
  }

  updateConfig(updates: Partial<ComplianceConfig>): ComplianceConfig {
    complianceConfig = { ...complianceConfig, ...updates };
    recordEvent({
      eventType: 'CONFIG_UPDATED',
      address: 'system',
      details: JSON.stringify(Object.keys(updates)),
    });
    return complianceConfig;
  }

  setJurisdictionLimits(jurisdiction: string, limits: TransactionLimits): void {
    complianceConfig.jurisdictionLimits[jurisdiction.toUpperCase()] = limits;
    recordEvent({
      eventType: 'JURISDICTION_LIMITS_SET',
      address: 'system',
      details: `jurisdiction=${jurisdiction}`,
    });
  }

  addRestrictedJurisdiction(jurisdiction: string, addedBy: string): void {
    const upper = jurisdiction.toUpperCase();
    if (!complianceConfig.restrictedJurisdictions.includes(upper)) {
      complianceConfig.restrictedJurisdictions.push(upper);
      recordEvent({
        eventType: 'JURISDICTION_RESTRICTED',
        address: addedBy,
        details: `jurisdiction=${upper}`,
      });
    }
  }

  removeRestrictedJurisdiction(jurisdiction: string, removedBy: string): void {
    const upper = jurisdiction.toUpperCase();
    complianceConfig.restrictedJurisdictions = complianceConfig.restrictedJurisdictions.filter(
      (j) => j !== upper
    );
    recordEvent({
      eventType: 'JURISDICTION_UNRESTRICTED',
      address: removedBy,
      details: `jurisdiction=${upper}`,
    });
  }

  // ─── Sanctions ──────────────────────────────────────────────────────────

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

  // ─── KYC ────────────────────────────────────────────────────────────────

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
      details: `tier=${params.tier} jurisdiction=${params.jurisdiction} provider=${params.kycProvider}`,
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

  listKycVerifications(): KycVerification[] {
    return Array.from(kycStore.values());
  }

  // ─── AML Risk Assessment ────────────────────────────────────────────────

  assessAmlRisk(address: string): AmlRiskAssessment {
    const existing = amlRiskStore.get(address.toLowerCase());
    const addressEvents = events.filter(
      (e) => e.address.toLowerCase() === address.toLowerCase()
    );

    const txEvents = addressEvents.filter((e) => e.eventType === 'TX_CHECKED');
    const transactionCount = txEvents.length;

    let totalVolume = 0n;
    const counterpartySet = new Set<string>();
    for (const e of txEvents) {
      if (e.amount) totalVolume += BigInt(e.amount);
      if (e.assetAddress) counterpartySet.add(e.assetAddress);
    }

    const kyc = kycStore.get(address.toLowerCase());
    const highRiskJurisdiction = kyc
      ? isJurisdictionRestricted(kyc.jurisdiction)
      : false;

    // Detect rapid movement: multiple transactions in short time
    let rapidMovementDetected = false;
    if (txEvents.length >= 3) {
      const timestamps = txEvents.map((e) => new Date(e.timestamp).getTime()).sort();
      const oneHour = 60 * 60 * 1000;
      for (let i = 2; i < timestamps.length; i++) {
        if (timestamps[i]! - timestamps[i - 2]! < oneHour) {
          rapidMovementDetected = true;
          break;
        }
      }
    }

    // Detect structuring: transactions just below reporting thresholds
    const kycThreshold = BigInt(complianceConfig.kycThresholdAmount);
    const structuringThreshold = (kycThreshold * 90n) / 100n; // 90% of threshold
    const structuringDetected = txEvents.some(
      (e) => e.amount && BigInt(e.amount) >= structuringThreshold && BigInt(e.amount) < kycThreshold
    );

    // Calculate risk score
    let riskScore = 0;
    const flags: string[] = [];

    if (!kyc || !kyc.verified) {
      riskScore += 20;
      flags.push('NO_KYC');
    }
    if (highRiskJurisdiction) {
      riskScore += 30;
      flags.push('HIGH_RISK_JURISDICTION');
    }
    if (this.checkSanctioned(address)) {
      riskScore += 70;
      flags.push('SANCTIONED');
    }
    if (rapidMovementDetected) {
      riskScore += 20;
      flags.push('RAPID_MOVEMENT');
    }
    if (structuringDetected) {
      riskScore += 25;
      flags.push('STRUCTURING');
    }
    if (transactionCount > 100) {
      riskScore += 10;
      flags.push('HIGH_FREQUENCY');
    }
    if (totalVolume > BigInt(complianceConfig.defaultLimits.weeklyLimit)) {
      riskScore += 15;
      flags.push('HIGH_VOLUME');
    }

    riskScore = Math.min(riskScore, 100);

    let riskLevel: AmlRiskAssessment['riskLevel'];
    if (riskScore >= 90) riskLevel = 'critical';
    else if (riskScore >= 70) riskLevel = 'high';
    else if (riskScore >= 40) riskLevel = 'medium';
    else riskLevel = 'low';

    const assessment: AmlRiskAssessment = {
      address,
      riskScore,
      riskLevel,
      flags,
      assessedAt: new Date().toISOString(),
      transactionCount,
      totalVolume: totalVolume.toString(),
      uniqueCounterparties: counterpartySet.size,
      highRiskJurisdiction,
      rapidMovementDetected,
      structuringDetected,
    };

    amlRiskStore.set(address.toLowerCase(), assessment);

    // Auto-file SAR if threshold reached
    if (complianceConfig.autoFileSar && riskScore >= complianceConfig.autoFileSarThreshold) {
      this.fileSar({
        address,
        reason: `Automated SAR: AML risk score ${riskScore} exceeds threshold. Flags: ${flags.join(', ')}`,
        amount: totalVolume.toString(),
        assetAddress: 'AUTO',
        filedBy: 'AML_SYSTEM',
      });
    }

    recordEvent({
      eventType: 'AML_ASSESSMENT',
      address,
      details: `riskScore=${riskScore} riskLevel=${riskLevel} flags=${flags.join(',')}`,
    });

    return assessment;
  }

  getAmlRisk(address: string): AmlRiskAssessment | undefined {
    return amlRiskStore.get(address.toLowerCase());
  }

  listAmlAssessments(riskLevel?: string): AmlRiskAssessment[] {
    const all = Array.from(amlRiskStore.values());
    if (riskLevel) {
      return all.filter((a) => a.riskLevel === riskLevel);
    }
    return all;
  }

  // ─── Regulatory Limits Enforcement ──────────────────────────────────────

  checkRegulatoryLimits(address: string, amount: string, jurisdiction?: string): {
    withinLimits: boolean;
    violations: string[];
    dailyUsed: string;
    weeklyUsed: string;
  } {
    const amountBigInt = BigInt(amount);
    const violations: string[] = [];
    const limits = getLimitsForJurisdiction(jurisdiction);

    // Update daily volume
    const today = getTodayStr();
    const daily = dailyVolumeStore.get(address.toLowerCase());
    let dailyVolume = 0n;
    if (daily && daily.date === today) {
      dailyVolume = daily.volume;
    }
    const newDailyVolume = dailyVolume + amountBigInt;

    // Update weekly volume
    const weekStart = getWeekStartStr();
    const weekly = weeklyVolumeStore.get(address.toLowerCase());
    let weeklyVolume = 0n;
    if (weekly && weekly.weekStart === weekStart) {
      weeklyVolume = weekly.volume;
    }
    const newWeeklyVolume = weeklyVolume + amountBigInt;

    // Check single transaction limit
    if (amountBigInt > BigInt(limits.maxSingleTx)) {
      violations.push(`Single transaction ${amount} exceeds max ${limits.maxSingleTx}`);
    }

    // Check daily limit
    if (newDailyVolume > BigInt(limits.dailyLimit)) {
      violations.push(`Daily volume ${newDailyVolume.toString()} exceeds limit ${limits.dailyLimit}`);
    }

    // Check weekly limit
    if (newWeeklyVolume > BigInt(limits.weeklyLimit)) {
      violations.push(`Weekly volume ${newWeeklyVolume.toString()} exceeds limit ${limits.weeklyLimit}`);
    }

    // Check KYC requirement for non-KYC users
    if (!this.checkKyc(address)) {
      const noKycLimit = BigInt(complianceConfig.maxDailyVolumeWithoutKyc);
      if (newDailyVolume > noKycLimit) {
        violations.push(`Daily volume for non-KYC user exceeds ${complianceConfig.maxDailyVolumeWithoutKyc}`);
      }
    }

    return {
      withinLimits: violations.length === 0,
      violations,
      dailyUsed: newDailyVolume.toString(),
      weeklyUsed: newWeeklyVolume.toString(),
    };
  }

  recordTransactionVolume(address: string, amount: string): void {
    const amountBigInt = BigInt(amount);
    const today = getTodayStr();
    const weekStart = getWeekStartStr();

    const daily = dailyVolumeStore.get(address.toLowerCase());
    if (daily && daily.date === today) {
      daily.volume += amountBigInt;
    } else {
      dailyVolumeStore.set(address.toLowerCase(), { volume: amountBigInt, date: today });
    }

    const weekly = weeklyVolumeStore.get(address.toLowerCase());
    if (weekly && weekly.weekStart === weekStart) {
      weekly.volume += amountBigInt;
    } else {
      weeklyVolumeStore.set(address.toLowerCase(), { volume: amountBigInt, weekStart });
    }
  }

  // ─── Transaction Compliance Check (Enhanced) ────────────────────────────

  checkTransaction(params: {
    from: string;
    to: string;
    amount: string;
    asset: string;
    jurisdiction?: string;
  }): ComplianceCheckResult {
    const errors: string[] = [];
    let sanctionsMatch = false;
    let kycValid = true;
    let geoRestricted = false;
    const amlFlags: string[] = [];

    // 1. Sanctions check
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

    // 2. KYC check
    const amount = BigInt(params.amount);
    const kycThreshold = BigInt(complianceConfig.kycThresholdAmount);
    if (amount > kycThreshold) {
      if (!this.checkKyc(params.from)) {
        kycValid = false;
        errors.push('KYC verification required for large transactions');
      }
    }

    // 3. KYC requirement for deposit/withdrawal
    if (complianceConfig.requireKycForWithdrawal && !this.checkKyc(params.from)) {
      kycValid = false;
      errors.push('KYC verification required for withdrawal');
    }
    if (complianceConfig.requireKycForDeposit && !this.checkKyc(params.to)) {
      kycValid = false;
      errors.push('KYC verification required for deposit');
    }

    // 4. Regulatory limits check
    const fromKyc = kycStore.get(params.from.toLowerCase());
    const limitCheck = this.checkRegulatoryLimits(
      params.from,
      params.amount,
      fromKyc?.jurisdiction
    );
    if (!limitCheck.withinLimits) {
      errors.push(...limitCheck.violations);
    }

    // 5. Geo-restriction check
    if (fromKyc && isJurisdictionRestricted(fromKyc.jurisdiction)) {
      geoRestricted = true;
      errors.push(`Jurisdiction ${fromKyc.jurisdiction} is restricted`);
    }
    const toKyc = kycStore.get(params.to.toLowerCase());
    if (toKyc && isJurisdictionRestricted(toKyc.jurisdiction)) {
      geoRestricted = true;
      errors.push(`Jurisdiction ${toKyc.jurisdiction} is restricted`);
    }

    // 6. AML risk assessment
    let amlRiskScore = 0;
    if (complianceConfig.amlMonitoringEnabled) {
      const fromAssessment = this.assessAmlRisk(params.from);
      const toAssessment = this.assessAmlRisk(params.to);
      amlRiskScore = Math.max(fromAssessment.riskScore, toAssessment.riskScore);

      if (fromAssessment.riskLevel === 'critical' || toAssessment.riskLevel === 'critical') {
        errors.push('AML risk score critical - transaction blocked');
        amlFlags.push(...fromAssessment.flags, ...toAssessment.flags);
      } else if (fromAssessment.riskLevel === 'high' || toAssessment.riskLevel === 'high') {
        amlFlags.push(...fromAssessment.flags, ...toAssessment.flags);
        if (amlRiskScore >= complianceConfig.amlRiskThreshold) {
          errors.push('AML risk score exceeds threshold - transaction requires review');
        }
      }
    }

    // Record volume
    if (errors.length === 0) {
      this.recordTransactionVolume(params.from, params.amount);
    }

    recordEvent({
      eventType: 'TX_CHECKED',
      address: params.from,
      amount: params.amount,
      assetAddress: params.asset,
      details: errors.length > 0 ? 'flagged' : 'passed',
    });

    return {
      passed: errors.length === 0,
      sanctionsMatch,
      kycValid,
      withinLimits: limitCheck.withinLimits,
      geoRestricted,
      amlRiskScore,
      amlFlags: [...new Set(amlFlags)],
      errors,
    };
  }

  // ─── SAR ────────────────────────────────────────────────────────────────

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

  updateSarStatus(sarId: number, status: SAR['status'], notes?: string): SAR | undefined {
    const sar = sarStore.get(sarId);
    if (!sar) return undefined;
    sar.status = status;
    if (notes) sar.notes = notes;
    recordEvent({
      eventType: 'SAR_STATUS_UPDATED',
      address: sar.address,
      details: `sarId=${sarId} status=${status}`,
    });
    return sar;
  }

  // ─── Compliance Reporting (Enhanced) ────────────────────────────────────

  getComplianceReport(from: string, to: string): ComplianceReport {
    const fromDate = new Date(from);
    const toDate = new Date(to);
    const periodEvents = events.filter((e) => {
      const d = new Date(e.timestamp);
      return d >= fromDate && d <= toDate;
    });

    const txChecked = periodEvents.filter((e) => e.eventType === 'TX_CHECKED');
    const flagged = txChecked.filter((e) => e.details === 'flagged');

    // Jurisdiction breakdown
    const jurisdictionBreakdown: Record<string, number> = {};
    for (const kyc of kycStore.values()) {
      if (kyc.verified) {
        jurisdictionBreakdown[kyc.jurisdiction] = (jurisdictionBreakdown[kyc.jurisdiction] || 0) + 1;
      }
    }

    // Event type breakdown
    const eventTypeBreakdown: Record<string, number> = {};
    for (const e of periodEvents) {
      eventTypeBreakdown[e.eventType] = (eventTypeBreakdown[e.eventType] || 0) + 1;
    }

    return {
      period: { from, to },
      totalTransactions: txChecked.length,
      flaggedTransactions: flagged.length,
      sarCount: this.listSars().filter((s) => {
        const d = new Date(s.filedAt);
        return d >= fromDate && d <= toDate;
      }).length,
      sanctionsMatches: periodEvents.filter((e) => e.eventType === 'SANCTION_ADDED').length,
      kycVerifications: periodEvents.filter((e) => e.eventType === 'KYC_VERIFIED').length,
      kycRevocations: periodEvents.filter((e) => e.eventType === 'KYC_REVOKED').length,
      amlAlerts: periodEvents.filter((e) => e.eventType === 'AML_ASSESSMENT').length,
      jurisdictionBreakdown,
      eventTypeBreakdown,
    };
  }

  // ─── Audit Trail (Enhanced with hash chain) ────────────────────────────

  getAuditTrail(address?: string, limit: number = 100, eventType?: string): ComplianceEvent[] {
    let filtered = address
      ? events.filter((e) => e.address.toLowerCase() === address.toLowerCase())
      : [...events];
    if (eventType) {
      filtered = filtered.filter((e) => e.eventType === eventType);
    }
    return filtered.slice(-limit);
  }

  getAuditTrailByHash(hash: string): ComplianceEvent | undefined {
    return events.find((e) => e.hash === hash);
  }

  verifyAuditTrailIntegrity(): { valid: boolean; brokenAt?: string } {
    for (let i = 1; i < events.length; i++) {
      const prevEvent = events[i - 1]!;
      const currEvent = events[i]!;
      const prevHash = prevEvent.hash ?? 'genesis';
      const expectedHash = createHash('sha256')
        .update(`${prevHash}:${currEvent.id}:${currEvent.eventType}:${currEvent.address}:${currEvent.timestamp}`)
        .digest('hex');
      if (currEvent.hash !== expectedHash) {
        return { valid: false, brokenAt: currEvent.id };
      }
    }
    return { valid: true };
  }

  // ─── Compliance Dashboard ───────────────────────────────────────────────

  getDashboard(): ComplianceDashboardData {
    const totalKycVerified = Array.from(kycStore.values()).filter((k) => k.verified).length;
    const totalSanctioned = Array.from(sanctionsList.values()).filter((s) => s.active).length;
    const totalSars = sarStore.size;
    const totalAmlAlerts = amlRiskStore.size;

    const pendingReviews = Array.from(sarStore.values()).filter(
      (s) => s.status === 'filed' || s.status === 'under_review'
    ).length;

    const totalTxChecked = events.filter((e) => e.eventType === 'TX_CHECKED').length;
    const passedTx = events.filter((e) => e.eventType === 'TX_CHECKED' && e.details === 'passed').length;
    const complianceRate = totalTxChecked > 0 ? (passedTx / totalTxChecked) * 100 : 100;

    // Risk distribution
    const riskDistribution = { low: 0, medium: 0, high: 0, critical: 0 };
    for (const assessment of amlRiskStore.values()) {
      riskDistribution[assessment.riskLevel]++;
    }

    // Jurisdiction stats
    const jurisdictionStats: Record<string, { count: number; riskScore: number }> = {};
    for (const kyc of kycStore.values()) {
      if (!jurisdictionStats[kyc.jurisdiction]) {
        jurisdictionStats[kyc.jurisdiction] = { count: 0, riskScore: 0 };
      }
      const js = jurisdictionStats[kyc.jurisdiction]!;
      js.count++;
      const assessment = amlRiskStore.get(kyc.address.toLowerCase());
      if (assessment) {
        js.riskScore = Math.max(js.riskScore, assessment.riskScore);
      }
    }

    // Top flagged addresses
    const flaggedCounts: Map<string, { count: number; lastFlag: string }> = new Map();
    for (const e of events) {
      if (e.details === 'flagged' || e.eventType === 'SAR_FILED' || e.eventType === 'AML_ASSESSMENT') {
        const existing = flaggedCounts.get(e.address) || { count: 0, lastFlag: e.timestamp };
        existing.count++;
        if (e.timestamp > existing.lastFlag) existing.lastFlag = e.timestamp;
        flaggedCounts.set(e.address, existing);
      }
    }
    const topFlaggedAddresses = Array.from(flaggedCounts.entries())
      .map(([address, data]) => ({ address, flagCount: data.count, lastFlag: data.lastFlag }))
      .sort((a, b) => b.flagCount - a.flagCount)
      .slice(0, 10);

    // Regulatory limits summary
    const today = getTodayStr();
    const weekStart = getWeekStartStr();
    let totalDailyVolume = 0n;
    let totalWeeklyVolume = 0n;
    for (const [_, daily] of dailyVolumeStore) {
      if (daily.date === today) totalDailyVolume += daily.volume;
    }
    for (const [_, weekly] of weeklyVolumeStore) {
      if (weekly.weekStart === weekStart) totalWeeklyVolume += weekly.volume;
    }

    return {
      summary: {
        totalKycVerified,
        totalSanctioned,
        totalSars,
        totalAmlAlerts,
        pendingReviews,
        complianceRate: Math.round(complianceRate * 100) / 100,
      },
      recentEvents: events.slice(-20),
      riskDistribution,
      jurisdictionStats,
      topFlaggedAddresses,
      regulatoryLimits: {
        dailyVolumeUsed: totalDailyVolume.toString(),
        dailyVolumeLimit: complianceConfig.defaultLimits.dailyLimit,
        weeklyVolumeUsed: totalWeeklyVolume.toString(),
        weeklyVolumeLimit: complianceConfig.defaultLimits.weeklyLimit,
      },
    };
  }
}

export const complianceService = new ComplianceService();
