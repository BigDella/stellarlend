import { Request, Response, NextFunction } from 'express';
import { complianceService } from '../services/compliance.service';
import logger from '../utils/logger';

/**
 * Compliance middleware that enforces KYC/AML checks on protected endpoints.
 * 
 * Usage:
 *   app.post('/api/lending/deposit', complianceEnforcement(), lendingController.deposit);
 * 
 * Checks performed:
 *   1. Sanctions screening (OFAC + internal list)
 *   2. KYC verification (if required by config or transaction amount)
 *   3. Jurisdiction restrictions
 *   4. Regulatory limits (daily/weekly/max-single)
 *   5. AML risk assessment (if enabled)
 */
export function complianceEnforcement(options?: {
  requireKyc?: boolean;
  checkAml?: boolean;
  skipLimits?: boolean;
}) {
  const requireKyc = options?.requireKyc ?? false;
  const checkAml = options?.checkAml ?? true;
  const skipLimits = options?.skipLimits ?? false;

  return (req: Request, res: Response, next: NextFunction) => {
    try {
      const userAddress = req.body?.userAddress || req.body?.from || req.headers['x-user-address'] as string;
      if (!userAddress) {
        // No user address to check; skip compliance enforcement
        return next();
      }

      // 1. Sanctions check
      if (complianceService.checkSanctioned(userAddress)) {
        logger.warn('COMPLIANCE_BLOCKED: Sanctioned address', { address: userAddress });
        return res.status(403).json({
          error: 'Address is on sanctions list',
          code: 'SANCTIONED_ADDRESS',
        });
      }

      const ofacResult = complianceService.screenAgainstOFAC(userAddress);
      if (ofacResult.match) {
        logger.warn('COMPLIANCE_BLOCKED: OFAC match', { address: userAddress, confidence: ofacResult.confidence });
        return res.status(403).json({
          error: 'Address matches OFAC sanctions list',
          code: 'OFAC_MATCH',
        });
      }

      // 2. KYC check
      if (requireKyc || complianceService.getConfig().requireKycForDeposit) {
        if (!complianceService.checkKyc(userAddress)) {
          logger.warn('COMPLIANCE_BLOCKED: KYC required', { address: userAddress });
          return res.status(403).json({
            error: 'KYC verification required',
            code: 'KYC_REQUIRED',
          });
        }
      }

      // 3. Jurisdiction check
      const kyc = complianceService.getKyc(userAddress);
      if (kyc) {
        const config = complianceService.getConfig();
        if (config.restrictedJurisdictions.includes(kyc.jurisdiction.toUpperCase())) {
          logger.warn('COMPLIANCE_BLOCKED: Restricted jurisdiction', { address: userAddress, jurisdiction: kyc.jurisdiction });
          return res.status(403).json({
            error: `Jurisdiction ${kyc.jurisdiction} is restricted`,
            code: 'RESTRICTED_JURISDICTION',
          });
        }
      }

      // 4. Regulatory limits check
      if (!skipLimits) {
        const amount = req.body?.amount;
        if (amount) {
          const limitCheck = complianceService.checkRegulatoryLimits(
            userAddress,
            amount,
            kyc?.jurisdiction
          );
          if (!limitCheck.withinLimits) {
            logger.warn('COMPLIANCE_BLOCKED: Regulatory limit exceeded', {
              address: userAddress,
              violations: limitCheck.violations,
            });
            return res.status(403).json({
              error: 'Regulatory limits exceeded',
              code: 'LIMIT_EXCEEDED',
              violations: limitCheck.violations,
            });
          }
        }
      }

      // 5. AML risk check
      if (checkAml && complianceService.getConfig().amlMonitoringEnabled) {
        const assessment = complianceService.assessAmlRisk(userAddress);
        if (assessment.riskLevel === 'critical') {
          logger.warn('COMPLIANCE_BLOCKED: Critical AML risk', { address: userAddress, riskScore: assessment.riskScore });
          return res.status(403).json({
            error: 'Transaction blocked due to critical AML risk',
            code: 'AML_CRITICAL',
            riskScore: assessment.riskScore,
            flags: assessment.flags,
          });
        }
        if (assessment.riskLevel === 'high' && assessment.riskScore >= complianceService.getConfig().amlRiskThreshold) {
          logger.warn('COMPLIANCE_BLOCKED: High AML risk', { address: userAddress, riskScore: assessment.riskScore });
          return res.status(403).json({
            error: 'Transaction requires review due to high AML risk',
            code: 'AML_HIGH_RISK',
            riskScore: assessment.riskScore,
            flags: assessment.flags,
          });
        }
      }

      // Attach compliance info to request for downstream use
      (req as any).compliance = {
        address: userAddress,
        kycVerified: complianceService.checkKyc(userAddress),
        kyc: kyc || null,
        amlRisk: checkAml ? complianceService.getAmlRisk(userAddress) : null,
      };

      next();
    } catch (err: any) {
      logger.error('COMPLIANCE_MIDDLEWARE_ERROR', { error: err.message });
      next();
    }
  };
}
