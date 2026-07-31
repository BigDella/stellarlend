import { ConfigAuditEntry } from '../config/types';

const auditLog: ConfigAuditEntry[] = [];
const MAX_ENTRIES = 1000;

class ConfigAuditService {
  record(entry: ConfigAuditEntry): void {
    auditLog.unshift(entry);
    if (auditLog.length > MAX_ENTRIES) {
      auditLog.length = MAX_ENTRIES;
    }
  }

  getLog(limit = 100, offset = 0): ConfigAuditEntry[] {
    return auditLog.slice(offset, offset + limit);
  }

  clear(): void {
    auditLog.length = 0;
  }

  count(): number {
    return auditLog.length;
  }
}

export const configAuditService = new ConfigAuditService();
