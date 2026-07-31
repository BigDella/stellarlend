# Compliance Features Documentation

## Overview

StellarLend's compliance module provides institutional-grade KYC/AML integration, regulatory limit enforcement, compliance reporting, audit trail, and a compliance dashboard — all designed for DeFi protocols that need to meet regulatory requirements.

---

## Features

### 1. KYC Integration

- **KYC Verification**: Set, check, revoke, and list KYC verifications for addresses.
- **Tiered KYC**: Supports multiple KYC tiers (1, 2, 3+) with different verification levels.
- **KYC Providers**: Pluggable KYC provider integration (Jumio, Onfido, etc.).
- **Jurisdiction Tracking**: Each KYC record tracks the user's jurisdiction.
- **Expiry Management**: KYC verifications have configurable validity periods.

**Endpoints:**
- `POST /api/v1/compliance/kyc` — Set KYC verification
- `DELETE /api/v1/compliance/kyc` — Revoke KYC
- `GET /api/v1/compliance/kyc/check?address=...` — Check KYC status
- `GET /api/v1/compliance/kyc/list` — List all KYC verifications

### 2. AML Integration

- **AML Risk Scoring**: Each address receives a risk score (0-100) based on multiple factors.
- **Risk Levels**: `low` (0-39), `medium` (40-69), `high` (70-89), `critical` (90-100).
- **Automated Detection**:
  - Rapid movement detection (multiple transactions in short time)
  - Structuring detection (transactions just below reporting thresholds)
  - High-frequency transaction detection
  - High-risk jurisdiction detection
  - Sanctions list screening
- **Auto SAR Filing**: Optionally auto-file SARs when risk exceeds a configurable threshold.

**Endpoints:**
- `POST /api/v1/compliance/aml/assess/:address` — Trigger AML risk assessment
- `GET /api/v1/compliance/aml/risk/:address` — Get existing AML assessment
- `GET /api/v1/compliance/aml/assessments?riskLevel=...` — List assessments by risk level

### 3. Sanctions Screening

- **Internal Sanctions List**: Add/remove addresses from the internal sanctions list.
- **OFAC Screening**: Screen addresses against OFAC sanctions list.
- **Expiry Support**: Sanctions can have expiry dates.

**Endpoints:**
- `POST /api/v1/compliance/sanctions` — Add sanction
- `DELETE /api/v1/compliance/sanctions` — Remove sanction
- `GET /api/v1/compliance/sanctions/check?address=...` — Check if address is sanctioned

### 4. Compliance Reporting

- **Comprehensive Reports**: Generate reports for any time period.
- **Metrics Include**: Total transactions, flagged transactions, SAR count, sanctions matches, KYC verifications/revocations, AML alerts.
- **Jurisdiction Breakdown**: Breakdown of KYC verifications by jurisdiction.
- **Event Type Breakdown**: Breakdown of all compliance events by type.

**Endpoints:**
- `GET /api/v1/compliance/report?from=...&to=...` — Get compliance report

### 5. Regulatory Limits

- **Configurable Limits**: Daily, weekly, and per-transaction limits.
- **Jurisdiction-Specific Limits**: Different limits per jurisdiction.
- **Non-KYC Limits**: Separate (lower) limits for non-KYC users.
- **Real-Time Enforcement**: Limits checked on every transaction.

**Endpoints:**
- `POST /api/v1/compliance/regulatory-limits/check` — Check if a transaction is within limits
- `PUT /api/v1/compliance/config` — Update limit configuration
- `POST /api/v1/compliance/config/jurisdiction-limits` — Set jurisdiction-specific limits

### 6. Compliance Audit Trail

- **Comprehensive Logging**: Every compliance action is logged with a timestamp.
- **Hash Chain Integrity**: Events are linked via SHA-256 hash chain for tamper detection.
- **Filtering**: Filter by address, event type, and limit.
- **Integrity Verification**: Verify the entire audit trail has not been tampered with.

**Endpoints:**
- `GET /api/v1/compliance/audit-trail?address=...&limit=...&eventType=...` — Get audit trail
- `GET /api/v1/compliance/audit-trail/verify` — Verify audit trail integrity

### 7. Compliance Configuration

- **Dynamic Configuration**: All compliance settings can be updated at runtime.
- **Configurable Settings**:
  - Default transaction limits
  - Per-jurisdiction limits
  - Restricted jurisdictions
  - KYC threshold amount
  - AML risk threshold
  - AML monitoring toggle
  - Auto-SAR filing toggle
  - KYC requirements for deposit/withdrawal
  - Non-KYC volume limits

**Endpoints:**
- `GET /api/v1/compliance/config` — Get current configuration
- `PUT /api/v1/compliance/config` — Update configuration
- `POST /api/v1/compliance/config/jurisdiction-limits` — Set jurisdiction-specific limits
- `POST /api/v1/compliance/config/restricted-jurisdictions` — Add restricted jurisdiction
- `DELETE /api/v1/compliance/config/restricted-jurisdictions` — Remove restricted jurisdiction

### 8. Compliance Dashboard

- **Summary Statistics**: Total KYC verifications, sanctioned addresses, SARs, AML alerts, pending reviews, compliance rate.
- **Risk Distribution**: Breakdown of addresses by risk level.
- **Jurisdiction Stats**: Per-jurisdiction user count and risk scores.
- **Top Flagged Addresses**: Most frequently flagged addresses.
- **Regulatory Limits Overview**: Current volume usage vs. limits.
- **Recent Events**: Last 20 compliance events.

**Endpoints:**
- `GET /api/v1/compliance/dashboard` — Get compliance dashboard

### 9. Compliance Middleware

The `complianceEnforcement` middleware can be applied to any route to enforce compliance checks automatically:

```typescript
import { complianceEnforcement } from './middleware/complianceEnforcement';

// Apply to specific routes
app.post('/api/lending/deposit', complianceEnforcement({ requireKyc: true }), lendingController.deposit);

// Apply to all lending routes
app.use('/api/lending', complianceEnforcement(), lendingRoutes);
```

**Middleware Options:**
- `requireKyc` — Require KYC verification for all requests
- `checkAml` — Enable AML risk checking (default: true)
- `skipLimits` — Skip regulatory limit checks (default: false)

**Middleware Checks:**
1. Sanctions screening (internal + OFAC)
2. KYC verification (if required)
3. Jurisdiction restrictions
4. Regulatory limits (daily/weekly/max-single)
5. AML risk assessment (if enabled)

---

## Configuration Reference

```typescript
interface ComplianceConfig {
  defaultLimits: {
    dailyLimit: string;      // Default daily limit in stroops
    weeklyLimit: string;     // Default weekly limit in stroops
    maxSingleTx: string;     // Max single transaction in stroops
  };
  jurisdictionLimits: Record<string, TransactionLimits>;  // Per-jurisdiction overrides
  restrictedJurisdictions: string[];                       // Blocked jurisdictions
  kycThresholdAmount: string;       // Amount above which KYC is required
  amlRiskThreshold: number;         // AML risk score threshold (0-100)
  amlMonitoringEnabled: boolean;    // Enable/disable AML monitoring
  autoFileSar: boolean;             // Auto-file SAR for high-risk addresses
  autoFileSarThreshold: number;     // Risk score threshold for auto-SAR
  requireKycForWithdrawal: boolean; // Require KYC for withdrawals
  requireKycForDeposit: boolean;    // Require KYC for deposits
  maxDailyVolumeWithoutKyc: string; // Max daily volume for non-KYC users
}
```

---

## Database Schema

The compliance module uses the following database tables (see `api/db/init/02_compliance.sql`):
- `sanctions_list` — Sanctioned addresses
- `kyc_verifications` — KYC verification records
- `compliance_events` — Compliance event log
- `suspicious_activity_reports` — SAR records
- `transaction_volume` — Volume tracking per address
- `restricted_jurisdictions` — Restricted jurisdiction list

---

## Compliance Event Types

| Event Type | Description |
|---|---|
| `SANCTION_ADDED` | Address added to sanctions list |
| `SANCTION_REMOVED` | Address removed from sanctions list |
| `KYC_VERIFIED` | KYC verification set |
| `KYC_REVOKED` | KYC verification revoked |
| `TX_CHECKED` | Transaction compliance check |
| `SAR_FILED` | SAR filed |
| `SAR_STATUS_UPDATED` | SAR status updated |
| `AML_ASSESSMENT` | AML risk assessment performed |
| `CONFIG_UPDATED` | Configuration updated |
| `JURISDICTION_LIMITS_SET` | Jurisdiction-specific limits set |
| `JURISDICTION_RESTRICTED` | Jurisdiction added to restricted list |
| `JURISDICTION_UNRESTRICTED` | Jurisdiction removed from restricted list |

---

## API Endpoints Summary

| Method | Path | Description |
|---|---|---|
| POST | `/api/v1/compliance/sanctions` | Add sanction |
| DELETE | `/api/v1/compliance/sanctions` | Remove sanction |
| GET | `/api/v1/compliance/sanctions/check` | Check sanctions |
| POST | `/api/v1/compliance/kyc` | Set KYC |
| DELETE | `/api/v1/compliance/kyc` | Revoke KYC |
| GET | `/api/v1/compliance/kyc/check` | Check KYC |
| GET | `/api/v1/compliance/kyc/list` | List KYC verifications |
| POST | `/api/v1/compliance/aml/assess/:address` | Assess AML risk |
| GET | `/api/v1/compliance/aml/risk/:address` | Get AML risk |
| GET | `/api/v1/compliance/aml/assessments` | List AML assessments |
| POST | `/api/v1/compliance/transaction/check` | Check transaction |
| POST | `/api/v1/compliance/sar` | File SAR |
| GET | `/api/v1/compliance/sar/:sarId` | Get SAR |
| GET | `/api/v1/compliance/sar` | List SARs |
| PATCH | `/api/v1/compliance/sar/:sarId/status` | Update SAR status |
| GET | `/api/v1/compliance/report` | Get compliance report |
| GET | `/api/v1/compliance/audit-trail` | Get audit trail |
| GET | `/api/v1/compliance/audit-trail/verify` | Verify audit trail |
| GET | `/api/v1/compliance/config` | Get configuration |
| PUT | `/api/v1/compliance/config` | Update configuration |
| POST | `/api/v1/compliance/config/jurisdiction-limits` | Set jurisdiction limits |
| POST | `/api/v1/compliance/config/restricted-jurisdictions` | Add restricted jurisdiction |
| DELETE | `/api/v1/compliance/config/restricted-jurisdictions` | Remove restricted jurisdiction |
| GET | `/api/v1/compliance/dashboard` | Get compliance dashboard |
| POST | `/api/v1/compliance/regulatory-limits/check` | Check regulatory limits |
