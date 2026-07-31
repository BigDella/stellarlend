import { NextFunction, Request, Response } from 'express';
import { UnauthorizedError, ValidationError } from '../utils/errors';
import logger from '../utils/logger';

export type Role = 'admin' | 'operator' | 'user' | 'viewer';

// Resources and the actions that can be performed on them
export type Resource =
  | 'lending'
  | 'staking'
  | 'subscriptions'
  | 'portfolio'
  | 'analytics'
  | 'config'
  | 'protocol'
  | 'admin';

export type Action = 'read' | 'write' | 'delete' | 'manage';

// Higher weight = more privilege
const ROLE_WEIGHT: Record<Role, number> = {
  admin: 4,
  operator: 3,
  user: 2,
  viewer: 1,
};

// Permission matrix: role → resource → allowed actions
const PERMISSION_MATRIX: Record<Role, Partial<Record<Resource, Action[]>>> = {
  admin: {
    lending: ['read', 'write', 'delete', 'manage'],
    staking: ['read', 'write', 'delete', 'manage'],
    subscriptions: ['read', 'write', 'delete', 'manage'],
    portfolio: ['read', 'write', 'delete', 'manage'],
    analytics: ['read', 'write', 'delete', 'manage'],
    config: ['read', 'write', 'delete', 'manage'],
    protocol: ['read', 'write', 'delete', 'manage'],
    admin: ['read', 'write', 'delete', 'manage'],
  },
  operator: {
    lending: ['read', 'write'],
    staking: ['read', 'write'],
    subscriptions: ['read', 'write', 'delete'],
    portfolio: ['read', 'write'],
    analytics: ['read'],
    config: ['read'],
    protocol: ['read', 'write'],
    admin: [],
  },
  user: {
    lending: ['read', 'write'],
    staking: ['read', 'write'],
    subscriptions: ['read', 'write'],
    portfolio: ['read'],
    analytics: ['read'],
    config: [],
    protocol: ['read'],
    admin: [],
  },
  viewer: {
    lending: ['read'],
    staking: ['read'],
    subscriptions: ['read'],
    portfolio: ['read'],
    analytics: ['read'],
    config: [],
    protocol: ['read'],
    admin: [],
  },
};

type PendingRevocation = {
  role: Role;
  actor: string;
  effectiveAt: number;
};

const pendingRevocations = new Map<string, PendingRevocation>();
const currentRoles = new Map<string, Role>();

function resolveRole(req: Request): Role {
  const role = (req.headers['x-user-role'] || 'viewer').toString().toLowerCase();
  if (role === 'admin' || role === 'operator' || role === 'user' || role === 'viewer') {
    return role as Role;
  }
  throw new ValidationError('x-user-role must be one of: admin, operator, user, viewer');
}

function resolveActor(req: Request): string {
  const actor = (req.headers['x-user-address'] || '').toString().trim();
  if (!actor) throw new UnauthorizedError('x-user-address header is required');
  return actor;
}

export function requireRole(minimum: Role) {
  return (req: Request, _res: Response, next: NextFunction) => {
    const callerRole = resolveRole(req);
    if (ROLE_WEIGHT[callerRole] < ROLE_WEIGHT[minimum]) {
      const actor = (req.headers['x-user-address'] || 'unknown').toString();
      logger.warn('Unauthorized role access attempt', {
        actor,
        callerRole,
        requiredRole: minimum,
        method: req.method,
        path: req.path,
        requestId: req.id,
      });
      throw new UnauthorizedError(`Role ${minimum} or higher required`);
    }
    next();
  };
}

export function requirePermission(resource: Resource, action: Action) {
  return (req: Request, _res: Response, next: NextFunction) => {
    const callerRole = resolveRole(req);
    const allowed = PERMISSION_MATRIX[callerRole][resource] ?? [];
    if (!allowed.includes(action)) {
      const actor = (req.headers['x-user-address'] || 'unknown').toString();
      logger.warn('Unauthorized permission access attempt', {
        actor,
        callerRole,
        resource,
        action,
        method: req.method,
        path: req.path,
        requestId: req.id,
      });
      throw new UnauthorizedError(
        `Permission denied: '${action}' on '${resource}' requires a higher role`
      );
    }
    next();
  };
}

export function hasPermission(role: Role, resource: Resource, action: Action): boolean {
  return (PERMISSION_MATRIX[role][resource] ?? []).includes(action);
}

export function scheduleRoleRevocation(
  actor: string,
  role: Role,
  target: string,
  coolOffMs: number
): void {
  if (ROLE_WEIGHT[role] >= ROLE_WEIGHT.admin && actor === target) {
    throw new ValidationError('admin self-revocation is not allowed');
  }
  pendingRevocations.set(target, {
    role,
    actor,
    effectiveAt: Date.now() + coolOffMs,
  });
}

export function applyMatureRoleRevocations(currentRoles: Map<string, Role>): void {
  const now = Date.now();
  for (const [target, revocation] of pendingRevocations.entries()) {
    if (revocation.effectiveAt <= now) {
      const current = currentRoles.get(target);
      if (current === revocation.role) currentRoles.set(target, 'viewer');
      pendingRevocations.delete(target);
    }
  }
}

export function assignRole(actorRole: Role, target: string, targetRole: Role): void {
  if (ROLE_WEIGHT[actorRole] <= ROLE_WEIGHT[targetRole]) {
    throw new UnauthorizedError('Role assignment requires a higher role than target');
  }
  currentRoles.set(target, targetRole);
}

export function scheduleRevocation(
  actor: string,
  actorRole: Role,
  target: string,
  role: Role,
  coolOffMs: number
): void {
  if (ROLE_WEIGHT[actorRole] <= ROLE_WEIGHT[role]) {
    throw new UnauthorizedError('Role revocation requires a higher role than target');
  }
  scheduleRoleRevocation(actor, role, target, coolOffMs);
}

export function getCurrentRoleAssignments(): Record<string, Role> {
  applyMatureRoleRevocations(currentRoles);
  return Object.fromEntries(currentRoles.entries());
}

export function roleAssignmentAllowed(req: Request, targetRole: Role): boolean {
  const callerRole = resolveRole(req);
  return ROLE_WEIGHT[callerRole] > ROLE_WEIGHT[targetRole];
}

export function getRbacAuditContext(req: Request): { actor: string; role: Role } {
  return {
    actor: resolveActor(req),
    role: resolveRole(req),
  };
}

export { PERMISSION_MATRIX, ROLE_WEIGHT };
