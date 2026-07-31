import { complianceService } from '../services/compliance.service';

// Reset compliance state before each test
beforeEach(() => {
  complianceService.reset();
  // Reset config to defaults
  complianceService.updateConfig({
    defaultLimits: {
      dailyLimit: '1000000000000',
      weeklyLimit: '5000000000000',
      maxSingleTx: '500000000000',
    },
    jurisdictionLimits: {},
    restrictedJurisdictions: [],
    kycThresholdAmount: '100000000000',
    amlRiskThreshold: 70,
    amlMonitoringEnabled: true,
    autoFileSar: false,
    autoFileSarThreshold: 90,
    requireKycForWithdrawal: false,
    requireKycForDeposit: false,
    maxDailyVolumeWithoutKyc: '50000000000',
  });
});

const testAddress = 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF';
const testAddress2 = 'GBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBWHF';

describe('ComplianceService - Sanctions', () => {
  it('adds a sanction', () => {
    const entry = complianceService.addSanction(testAddress, 'OFAC', 'test reason');
    expect(entry.address).toBe(testAddress);
    expect(entry.active).toBe(true);
    expect(entry.source).toBe('OFAC');
  });

  it('checks if address is sanctioned', () => {
    complianceService.addSanction(testAddress, 'OFAC', 'test reason');
    expect(complianceService.checkSanctioned(testAddress)).toBe(true);
    expect(complianceService.checkSanctioned(testAddress2)).toBe(false);
  });

  it('removes a sanction', () => {
    complianceService.addSanction(testAddress, 'OFAC', 'test reason');
    complianceService.removeSanction(testAddress);
    expect(complianceService.checkSanctioned(testAddress)).toBe(false);
  });

  it('handles expired sanctions', () => {
    const pastDate = new Date(Date.now() - 10000).toISOString();
    complianceService.addSanction(testAddress, 'OFAC', 'test reason', pastDate);
    expect(complianceService.checkSanctioned(testAddress)).toBe(false);
  });

  it('screens against OFAC', () => {
    const result = complianceService.screenAgainstOFAC(testAddress);
    expect(result).toHaveProperty('match');
    expect(result).toHaveProperty('confidence');
    expect(typeof result.match).toBe('boolean');
    expect(typeof result.confidence).toBe('number');
  });
});

describe('ComplianceService - KYC', () => {
  it('sets KYC verification', () => {
    const kyc = complianceService.setKycVerification({
      address: testAddress,
      tier: 1,
      jurisdiction: 'US',
      kycProvider: 'Jumio',
      validityDays: 365,
    });
    expect(kyc.address).toBe(testAddress);
    expect(kyc.verified).toBe(true);
    expect(kyc.tier).toBe(1);
    expect(kyc.jurisdiction).toBe('US');
    expect(kyc.kycProvider).toBe('Jumio');
  });

  it('checks KYC validity', () => {
    complianceService.setKycVerification({
      address: testAddress,
      tier: 1,
      jurisdiction: 'US',
      kycProvider: 'Jumio',
    });
    expect(complianceService.checkKyc(testAddress)).toBe(true);
    expect(complianceService.checkKyc(testAddress2)).toBe(false);
  });

  it('revokes KYC', () => {
    complianceService.setKycVerification({
      address: testAddress,
      tier: 1,
      jurisdiction: 'US',
      kycProvider: 'Jumio',
    });
    complianceService.revokeKyc(testAddress);
    expect(complianceService.checkKyc(testAddress)).toBe(false);
  });

  it('gets KYC details', () => {
    complianceService.setKycVerification({
      address: testAddress,
      tier: 2,
      jurisdiction: 'UK',
      kycProvider: 'Onfido',
    });
    const kyc = complianceService.getKyc(testAddress);
    expect(kyc).toBeDefined();
    expect(kyc!.tier).toBe(2);
    expect(kyc!.jurisdiction).toBe('UK');
  });

  it('lists KYC verifications', () => {
    complianceService.setKycVerification({
      address: testAddress,
      tier: 1,
      jurisdiction: 'US',
      kycProvider: 'Jumio',
    });
    complianceService.setKycVerification({
      address: testAddress2,
      tier: 2,
      jurisdiction: 'UK',
      kycProvider: 'Onfido',
    });
    const list = complianceService.listKycVerifications();
    expect(list.length).toBeGreaterThanOrEqual(2);
  });
});

describe('ComplianceService - AML', () => {
  it('assesses AML risk for a new address', () => {
    const assessment = complianceService.assessAmlRisk(testAddress);
    expect(assessment).toHaveProperty('riskScore');
    expect(assessment).toHaveProperty('riskLevel');
    expect(assessment).toHaveProperty('flags');
    expect(assessment).toHaveProperty('address');
    expect(['low', 'medium', 'high', 'critical']).toContain(assessment.riskLevel);
    expect(assessment.riskScore).toBeGreaterThanOrEqual(0);
    expect(assessment.riskScore).toBeLessThanOrEqual(100);
  });

  it('flags sanctioned address as high risk', () => {
    complianceService.addSanction(testAddress, 'OFAC', 'test reason');
    const assessment = complianceService.assessAmlRisk(testAddress);
    expect(assessment.riskScore).toBeGreaterThan(0);
    expect(assessment.flags).toContain('SANCTIONED');
    expect(assessment.riskLevel).toBe('critical');
  });

  it('flags no-KYC address', () => {
    const assessment = complianceService.assessAmlRisk(testAddress2);
    expect(assessment.flags).toContain('NO_KYC');
  });

  it('gets AML risk assessment', () => {
    complianceService.assessAmlRisk(testAddress);
    const assessment = complianceService.getAmlRisk(testAddress);
    expect(assessment).toBeDefined();
    expect(assessment!.address).toBe(testAddress);
  });

  it('lists AML assessments', () => {
    complianceService.assessAmlRisk(testAddress);
    complianceService.assessAmlRisk(testAddress2);
    const all = complianceService.listAmlAssessments();
    expect(all.length).toBeGreaterThanOrEqual(2);
  });

  it('lists AML assessments by risk level', () => {
    complianceService.addSanction(testAddress, 'OFAC', 'test');
    complianceService.assessAmlRisk(testAddress);
    const critical = complianceService.listAmlAssessments('critical');
    expect(critical.length).toBeGreaterThanOrEqual(1);
    expect(critical.every((a) => a.riskLevel === 'critical')).toBe(true);
  });

  it('auto-files SAR when threshold is reached', () => {
    complianceService.updateConfig({
      autoFileSar: true,
      autoFileSarThreshold: 50,
    });
    complianceService.addSanction(testAddress, 'OFAC', 'test');
    complianceService.assessAmlRisk(testAddress);
    // SAR should be auto-filed since sanctioned address has critical risk
    const sars = complianceService.listSars();
    const autoSar = sars.find((s) => s.filedBy === 'AML_SYSTEM');
    expect(autoSar).toBeDefined();
  });
});

describe('ComplianceService - Transaction Compliance', () => {
  it('checks a valid transaction', () => {
    complianceService.setKycVerification({
      address: testAddress,
      tier: 1,
      jurisdiction: 'US',
      kycProvider: 'Jumio',
    });
    const result = complianceService.checkTransaction({
      from: testAddress,
      to: testAddress2,
      amount: '1000000',
      asset: 'XLM',
    });
    expect(result).toHaveProperty('passed');
    expect(result).toHaveProperty('sanctionsMatch');
    expect(result).toHaveProperty('kycValid');
    expect(result).toHaveProperty('withinLimits');
    expect(result).toHaveProperty('geoRestricted');
    expect(result).toHaveProperty('amlRiskScore');
    expect(result).toHaveProperty('amlFlags');
    expect(result).toHaveProperty('errors');
    expect(result.amlRiskScore).toBeGreaterThanOrEqual(0);
  });

  it('blocks sanctioned address transactions', () => {
    complianceService.addSanction(testAddress, 'OFAC', 'test reason');
    const result = complianceService.checkTransaction({
      from: testAddress,
      to: testAddress2,
      amount: '1000000',
      asset: 'XLM',
    });
    expect(result.passed).toBe(false);
    expect(result.sanctionsMatch).toBe(true);
    expect(result.errors).toContain('Address is sanctioned');
  });

  it('requires KYC for large transactions', () => {
    // No KYC set for testAddress2
    const result = complianceService.checkTransaction({
      from: testAddress2,
      to: testAddress,
      amount: '500000000000', // 50,000 XLM - above threshold
      asset: 'XLM',
    });
    expect(result.kycValid).toBe(false);
  });

  it('enforces regulatory limits', () => {
    complianceService.setKycVerification({
      address: testAddress,
      tier: 1,
      jurisdiction: 'US',
      kycProvider: 'Jumio',
    });
    const result = complianceService.checkTransaction({
      from: testAddress,
      to: testAddress2,
      amount: '999999999999999', // Way above max single tx
      asset: 'XLM',
    });
    expect(result.withinLimits).toBe(false);
  });

  it('checks geo-restrictions', () => {
    complianceService.addRestrictedJurisdiction('KP', 'admin');
    complianceService.setKycVerification({
      address: testAddress,
      tier: 1,
      jurisdiction: 'KP',
      kycProvider: 'Jumio',
    });
    const result = complianceService.checkTransaction({
      from: testAddress,
      to: testAddress2,
      amount: '1000000',
      asset: 'XLM',
    });
    expect(result.geoRestricted).toBe(true);
    expect(result.errors.some((e) => e.includes('restricted'))).toBe(true);
  });
});

describe('ComplianceService - SAR', () => {
  it('files a SAR', () => {
    const sar = complianceService.fileSar({
      address: testAddress,
      reason: 'Suspicious activity',
      amount: '1000000000',
      assetAddress: 'XLM',
      filedBy: 'compliance_officer',
    });
    expect(sar.sarId).toBeGreaterThan(0);
    expect(sar.status).toBe('filed');
    expect(sar.address).toBe(testAddress);
  });

  it('gets a SAR by ID', () => {
    const sar = complianceService.fileSar({
      address: testAddress,
      reason: 'Suspicious activity',
      amount: '1000000000',
      assetAddress: 'XLM',
      filedBy: 'compliance_officer',
    });
    const retrieved = complianceService.getSar(sar.sarId);
    expect(retrieved).toBeDefined();
    expect(retrieved!.sarId).toBe(sar.sarId);
  });

  it('lists SARs', () => {
    complianceService.fileSar({
      address: testAddress,
      reason: 'Suspicious activity',
      amount: '1000000000',
      assetAddress: 'XLM',
      filedBy: 'compliance_officer',
    });
    const sars = complianceService.listSars();
    expect(sars.length).toBeGreaterThan(0);
  });

  it('lists SARs by status', () => {
    complianceService.fileSar({
      address: testAddress,
      reason: 'Suspicious activity',
      amount: '1000000000',
      assetAddress: 'XLM',
      filedBy: 'compliance_officer',
    });
    const filed = complianceService.listSars('filed');
    expect(filed.length).toBeGreaterThan(0);
    expect(filed.every((s) => s.status === 'filed')).toBe(true);
  });

  it('updates SAR status', () => {
    const sar = complianceService.fileSar({
      address: testAddress,
      reason: 'Suspicious activity',
      amount: '1000000000',
      assetAddress: 'XLM',
      filedBy: 'compliance_officer',
    });
    const updated = complianceService.updateSarStatus(sar.sarId, 'under_review', 'Reviewing');
    expect(updated).toBeDefined();
    expect(updated!.status).toBe('under_review');
    expect(updated!.notes).toBe('Reviewing');
  });
});

describe('ComplianceService - Reporting', () => {
  it('generates compliance report', () => {
    const from = new Date(Date.now() - 7 * 24 * 60 * 60 * 1000).toISOString();
    const to = new Date().toISOString();
    const report = complianceService.getComplianceReport(from, to);
    expect(report).toHaveProperty('period');
    expect(report).toHaveProperty('totalTransactions');
    expect(report).toHaveProperty('flaggedTransactions');
    expect(report).toHaveProperty('sarCount');
    expect(report).toHaveProperty('sanctionsMatches');
    expect(report).toHaveProperty('kycVerifications');
    expect(report).toHaveProperty('kycRevocations');
    expect(report).toHaveProperty('amlAlerts');
    expect(report).toHaveProperty('jurisdictionBreakdown');
    expect(report).toHaveProperty('eventTypeBreakdown');
    expect(report.period.from).toBe(from);
    expect(report.period.to).toBe(to);
  });

  it('includes jurisdiction breakdown in report', () => {
    complianceService.setKycVerification({
      address: testAddress,
      tier: 1,
      jurisdiction: 'US',
      kycProvider: 'Jumio',
    });
    const from = new Date(Date.now() - 7 * 24 * 60 * 60 * 1000).toISOString();
    const to = new Date(Date.now() + 24 * 60 * 60 * 1000).toISOString();
    const report = complianceService.getComplianceReport(from, to);
    expect(Object.keys(report.jurisdictionBreakdown).length).toBeGreaterThan(0);
  });
});

describe('ComplianceService - Audit Trail', () => {
  it('records audit trail events', () => {
    complianceService.addSanction(testAddress, 'OFAC', 'test reason');
    const trail = complianceService.getAuditTrail();
    expect(trail.length).toBeGreaterThan(0);
  });

  it('filters audit trail by address', () => {
    complianceService.addSanction(testAddress, 'OFAC', 'test reason');
    const trail = complianceService.getAuditTrail(testAddress);
    expect(trail.every((e) => e.address === testAddress)).toBe(true);
  });

  it('filters audit trail by event type', () => {
    complianceService.addSanction(testAddress, 'OFAC', 'test reason');
    const trail = complianceService.getAuditTrail(undefined, 100, 'SANCTION_ADDED');
    expect(trail.every((e) => e.eventType === 'SANCTION_ADDED')).toBe(true);
  });

  it('limits audit trail results', () => {
    complianceService.addSanction(testAddress, 'OFAC', 'test1');
    complianceService.addSanction(testAddress2, 'OFAC', 'test2');
    const trail = complianceService.getAuditTrail(undefined, 1);
    expect(trail.length).toBeLessThanOrEqual(1);
  });

  it('includes hash chain for integrity', () => {
    complianceService.addSanction(testAddress, 'OFAC', 'test reason');
    const trail = complianceService.getAuditTrail();
    expect(trail.some((e) => e.hash)).toBe(true);
  });

  it('verifies audit trail integrity', () => {
    complianceService.addSanction(testAddress, 'OFAC', 'test reason');
    const result = complianceService.verifyAuditTrailIntegrity();
    expect(result).toHaveProperty('valid');
    expect(result.valid).toBe(true);
  });
});

describe('ComplianceService - Configuration', () => {
  it('gets current configuration', () => {
    const config = complianceService.getConfig();
    expect(config).toHaveProperty('defaultLimits');
    expect(config).toHaveProperty('jurisdictionLimits');
    expect(config).toHaveProperty('restrictedJurisdictions');
    expect(config).toHaveProperty('kycThresholdAmount');
    expect(config).toHaveProperty('amlRiskThreshold');
    expect(config).toHaveProperty('amlMonitoringEnabled');
    expect(config).toHaveProperty('autoFileSar');
    expect(config).toHaveProperty('requireKycForWithdrawal');
    expect(config).toHaveProperty('requireKycForDeposit');
    expect(config).toHaveProperty('maxDailyVolumeWithoutKyc');
  });

  it('updates configuration', () => {
    const updated = complianceService.updateConfig({
      kycThresholdAmount: '50000000000',
    });
    expect(updated.kycThresholdAmount).toBe('50000000000');
  });

  it('sets jurisdiction-specific limits', () => {
    complianceService.setJurisdictionLimits('US', {
      dailyLimit: '2000000000000',
      weeklyLimit: '10000000000000',
      maxSingleTx: '1000000000000',
    });
    const config = complianceService.getConfig();
    expect(config.jurisdictionLimits['US']).toBeDefined();
    expect(config.jurisdictionLimits['US'].dailyLimit).toBe('2000000000000');
  });

  it('adds restricted jurisdiction', () => {
    complianceService.addRestrictedJurisdiction('KP', 'admin');
    const config = complianceService.getConfig();
    expect(config.restrictedJurisdictions).toContain('KP');
  });

  it('removes restricted jurisdiction', () => {
    complianceService.addRestrictedJurisdiction('KP', 'admin');
    complianceService.removeRestrictedJurisdiction('KP', 'admin');
    const config = complianceService.getConfig();
    expect(config.restrictedJurisdictions).not.toContain('KP');
  });

  it('enforces jurisdiction-specific limits', () => {
    complianceService.setJurisdictionLimits('US', {
      dailyLimit: '1000000', // Very low limit
      weeklyLimit: '5000000',
      maxSingleTx: '500000',
    });
    complianceService.setKycVerification({
      address: testAddress,
      tier: 1,
      jurisdiction: 'US',
      kycProvider: 'Jumio',
    });
    const result = complianceService.checkRegulatoryLimits(testAddress, '10000000', 'US');
    expect(result.withinLimits).toBe(false);
    expect(result.violations.length).toBeGreaterThan(0);
  });
});

describe('ComplianceService - Dashboard', () => {
  it('returns dashboard data', () => {
    complianceService.setKycVerification({
      address: testAddress,
      tier: 1,
      jurisdiction: 'US',
      kycProvider: 'Jumio',
    });
    const dashboard = complianceService.getDashboard();
    expect(dashboard).toHaveProperty('summary');
    expect(dashboard).toHaveProperty('recentEvents');
    expect(dashboard).toHaveProperty('riskDistribution');
    expect(dashboard).toHaveProperty('jurisdictionStats');
    expect(dashboard).toHaveProperty('topFlaggedAddresses');
    expect(dashboard).toHaveProperty('regulatoryLimits');
    expect(dashboard.summary).toHaveProperty('totalKycVerified');
    expect(dashboard.summary).toHaveProperty('totalSanctioned');
    expect(dashboard.summary).toHaveProperty('totalSars');
    expect(dashboard.summary).toHaveProperty('totalAmlAlerts');
    expect(dashboard.summary).toHaveProperty('pendingReviews');
    expect(dashboard.summary).toHaveProperty('complianceRate');
    expect(dashboard.riskDistribution).toHaveProperty('low');
    expect(dashboard.riskDistribution).toHaveProperty('medium');
    expect(dashboard.riskDistribution).toHaveProperty('high');
    expect(dashboard.riskDistribution).toHaveProperty('critical');
    expect(dashboard.regulatoryLimits).toHaveProperty('dailyVolumeUsed');
    expect(dashboard.regulatoryLimits).toHaveProperty('dailyVolumeLimit');
    expect(dashboard.regulatoryLimits).toHaveProperty('weeklyVolumeUsed');
    expect(dashboard.regulatoryLimits).toHaveProperty('weeklyVolumeLimit');
  });

  it('tracks KYC verified count in dashboard', () => {
    complianceService.setKycVerification({
      address: testAddress,
      tier: 1,
      jurisdiction: 'US',
      kycProvider: 'Jumio',
    });
    const dashboard = complianceService.getDashboard();
    expect(dashboard.summary.totalKycVerified).toBeGreaterThan(0);
  });

  it('tracks risk distribution in dashboard', () => {
    complianceService.addSanction(testAddress, 'OFAC', 'test');
    complianceService.assessAmlRisk(testAddress);
    const dashboard = complianceService.getDashboard();
    expect(dashboard.riskDistribution.critical).toBeGreaterThan(0);
  });
});

describe('ComplianceService - Regulatory Limits', () => {
  it('checks regulatory limits', () => {
    complianceService.setKycVerification({
      address: testAddress,
      tier: 1,
      jurisdiction: 'US',
      kycProvider: 'Jumio',
    });
    const result = complianceService.checkRegulatoryLimits(testAddress, '1000000');
    expect(result).toHaveProperty('withinLimits');
    expect(result).toHaveProperty('violations');
    expect(result).toHaveProperty('dailyUsed');
    expect(result).toHaveProperty('weeklyUsed');
  });

  it('detects exceeding daily limit', () => {
    complianceService.updateConfig({
      defaultLimits: {
        dailyLimit: '1000000',
        weeklyLimit: '5000000',
        maxSingleTx: '500000',
      },
    });
    const result = complianceService.checkRegulatoryLimits(testAddress, '999999999');
    expect(result.withinLimits).toBe(false);
    expect(result.violations.some((v) => v.includes('Daily'))).toBe(true);
  });

  it('detects exceeding max single tx limit', () => {
    complianceService.updateConfig({
      defaultLimits: {
        dailyLimit: '10000000000000',
        weeklyLimit: '50000000000000',
        maxSingleTx: '500000',
      },
    });
    const result = complianceService.checkRegulatoryLimits(testAddress, '999999999');
    expect(result.withinLimits).toBe(false);
    expect(result.violations.some((v) => v.includes('Single transaction'))).toBe(true);
  });

  it('enforces non-KYC daily volume limit', () => {
    complianceService.updateConfig({
      maxDailyVolumeWithoutKyc: '50000000',
    });
    const result = complianceService.checkRegulatoryLimits(testAddress2, '99999999999');
    expect(result.withinLimits).toBe(false);
    expect(result.violations.some((v) => v.includes('non-KYC'))).toBe(true);
  });

  it('records transaction volume', () => {
    complianceService.setKycVerification({
      address: testAddress,
      tier: 1,
      jurisdiction: 'US',
      kycProvider: 'Jumio',
    });
    complianceService.recordTransactionVolume(testAddress, '1000000000');
    const result = complianceService.checkRegulatoryLimits(testAddress, '0');
    expect(result.dailyUsed).toBe('1000000000');
  });
});
