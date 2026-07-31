# Security Policy & Bug Bounty Program

We take the security of StellarLend seriously. This document outlines our vulnerability disclosure policy and bug bounty program rules.

## Bug Bounty Program

Our Bug Bounty Program incentivizes external security researchers to find and responsibly disclose vulnerabilities. 

### Severity and Rewards
Rewards are based on the severity of the vulnerability and are paid in USDC.

| Severity | Description | Reward |
| --- | --- | --- |
| **Critical** | Vulnerabilities causing unauthorized access to protocol funds or significant state corruption. | $10,000 |
| **High** | Vulnerabilities allowing manipulation of protocol logic without direct fund theft, or disruption of key operations. | $5,000 |
| **Medium** | Issues that degrade user experience or cause temporary denial of service. | $2,000 |
| **Low** | Minor issues, edge cases, and cosmetic bugs with minimal impact on security. | $500 |

### Scope
**In-Scope:**
- Smart contracts within `contract/` and `packages/`
- Backend API services (`api/`) handling state and critical operations.

**Out-of-Scope:**
- Third-party oracle services (unless specifically integrated within our codebase and misconfigured by us)
- Phishing or Social Engineering attacks
- Physical attacks against servers

### Responsible Disclosure Policy
- Please submit vulnerability reports via our [Bug Bounty Submission Form](/bug-bounty) or use the GitHub issue template.
- Do not exploit the vulnerability further than necessary to prove its existence.
- Provide us a reasonable amount of time to resolve the issue before disclosing it to the public or a third party.

### SLAs
We aim to respond to reports based on severity:
- **Critical**: < 24h
- **High**: < 72h
- **Medium/Low**: within 7 days

### Payout Mechanism
Payouts are handled manually via stablecoin (USDC) transfers to the researcher's provided wallet address after the fix is deployed.

### Hall of Fame
We publicly acknowledge the contributions of researchers who help us secure our protocol on our Hall of Fame page!
